use crate::editor_session::{
    CurveEditorHandle, EditorSession, KeyframeDragMember, KeyframeInterpolationPreset,
};
use rhythm_core::{
    animation::{Animated, Interpolation, evaluate_bezier_easing},
    ids::{EffectId, KeyframeId, ObjectId},
    project::{EffectKind, ObjectContent, Project},
    property::{
        AnimatableProperty, EffectAnimatableProperty, PropertyKeyframe, locate_property_keyframe,
        property_keyframe_by_id, property_keyframes,
    },
    time::{
        BeatDivision, MusicalTick, PPQ, ProjectTimeNs, TempoMap, floor_tick_position_to_grid,
        snap_tick_position_to_grid,
    },
};
use rhythm_engine::waveform::{WaveformData, WaveformSlice};

const RULER_ROW_HEIGHT: f32 = 28.0;
const WAVEFORM_ROW_HEIGHT: f32 = 64.0;
const CURVE_EDITOR_HEIGHT: f32 = 204.0;
const CURVE_SAMPLE_COUNT: usize = 64;
const MIN_RULER_LABEL_SPACING_PX: f32 = 72.0;
const MAX_GRID_LINES_PER_FRAME: usize = 100_000;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TimelineTransform {
    rect: egui::Rect,
    start_time: ProjectTimeNs,
    end_time: ProjectTimeNs,
}

impl TimelineTransform {
    #[must_use]
    pub fn new(
        rect: egui::Rect,
        start_time: ProjectTimeNs,
        end_time: ProjectTimeNs,
    ) -> Option<Self> {
        if rect.width() <= 0.0 || end_time <= start_time {
            return None;
        }

        Some(Self {
            rect,
            start_time,
            end_time,
        })
    }

    #[must_use]
    pub const fn start_time(self) -> ProjectTimeNs {
        self.start_time
    }

    #[must_use]
    pub const fn end_time(self) -> ProjectTimeNs {
        self.end_time
    }

    #[must_use]
    pub const fn rect(self) -> egui::Rect {
        self.rect
    }

    #[must_use]
    pub fn project_time_to_x(self, project_time: ProjectTimeNs) -> f32 {
        let span_ns = (i128::from(self.end_time.get()) - i128::from(self.start_time.get())) as f64;
        let offset_ns = (i128::from(project_time.get()) - i128::from(self.start_time.get())) as f64;
        let normalized = offset_ns / span_ns;
        self.rect.left() + (normalized * f64::from(self.rect.width())) as f32
    }

    #[must_use]
    pub fn x_to_project_time(self, x: f32) -> ProjectTimeNs {
        let normalized = ((x - self.rect.left()) / self.rect.width()).clamp(0.0, 1.0) as f64;
        let start_ns = i128::from(self.start_time.get());
        let span_ns = i128::from(self.end_time.get()) - start_ns;
        let offset_ns = (span_ns as f64 * normalized).round() as i128;
        let project_ns =
            (start_ns + offset_ns).clamp(i128::from(i64::MIN), i128::from(i64::MAX)) as i64;
        ProjectTimeNs::new(project_ns)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TimelineEffectProperty {
    BlurRadius,
    GlowRadius,
    GlowIntensity,
    GlowThreshold,
    GlowColor,
    TintColor,
    TintAmount,
    NoiseAmount,
    NoiseSize,
    NoiseEvolution,
    RgbSplitAmount,
    RgbSplitAngle,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TimelineProperty {
    Position,
    Scale,
    Rotation,
    Anchor,
    Opacity,
    RectangleSize,
    RectangleFill,
    RectangleCornerRadius,
    EllipseSize,
    EllipseFill,
    TextColor,
    Effect {
        effect_id: EffectId,
        property: TimelineEffectProperty,
    },
}

impl TimelineProperty {
    #[must_use]
    pub const fn animatable_property(self) -> AnimatableProperty {
        match self {
            Self::Position => AnimatableProperty::Position,
            Self::Scale => AnimatableProperty::Scale,
            Self::Rotation => AnimatableProperty::Rotation,
            Self::Anchor => AnimatableProperty::Anchor,
            Self::Opacity => AnimatableProperty::Opacity,
            Self::RectangleSize => AnimatableProperty::RectangleSize,
            Self::RectangleFill => AnimatableProperty::RectangleFill,
            Self::RectangleCornerRadius => AnimatableProperty::RectangleCornerRadius,
            Self::EllipseSize => AnimatableProperty::EllipseSize,
            Self::EllipseFill => AnimatableProperty::EllipseFill,
            Self::TextColor => AnimatableProperty::TextColor,
            Self::Effect {
                effect_id,
                property,
            } => AnimatableProperty::Effect {
                effect_id,
                property: match property {
                    TimelineEffectProperty::BlurRadius => EffectAnimatableProperty::BlurRadius,
                    TimelineEffectProperty::GlowRadius => EffectAnimatableProperty::GlowRadius,
                    TimelineEffectProperty::GlowIntensity => {
                        EffectAnimatableProperty::GlowIntensity
                    }
                    TimelineEffectProperty::GlowThreshold => {
                        EffectAnimatableProperty::GlowThreshold
                    }
                    TimelineEffectProperty::GlowColor => EffectAnimatableProperty::GlowColor,
                    TimelineEffectProperty::TintColor => EffectAnimatableProperty::TintColor,
                    TimelineEffectProperty::TintAmount => EffectAnimatableProperty::TintAmount,
                    TimelineEffectProperty::NoiseAmount => EffectAnimatableProperty::NoiseAmount,
                    TimelineEffectProperty::NoiseSize => EffectAnimatableProperty::NoiseSize,
                    TimelineEffectProperty::NoiseEvolution => {
                        EffectAnimatableProperty::NoiseEvolution
                    }
                    TimelineEffectProperty::RgbSplitAmount => {
                        EffectAnimatableProperty::RgbSplitAmount
                    }
                    TimelineEffectProperty::RgbSplitAngle => {
                        EffectAnimatableProperty::RgbSplitAngle
                    }
                },
            },
        }
    }

    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Position => "Position",
            Self::Scale => "Scale",
            Self::Rotation => "Rotation",
            Self::Anchor => "Anchor",
            Self::Opacity => "Opacity",
            Self::RectangleSize | Self::EllipseSize => "Size",
            Self::RectangleFill | Self::EllipseFill => "Fill",
            Self::RectangleCornerRadius => "Corner Radius",
            Self::TextColor => "Color",
            Self::Effect { property, .. } => match property {
                TimelineEffectProperty::BlurRadius => "Blur Radius",
                TimelineEffectProperty::GlowRadius => "Glow Radius",
                TimelineEffectProperty::GlowIntensity => "Glow Intensity",
                TimelineEffectProperty::GlowThreshold => "Glow Threshold",
                TimelineEffectProperty::GlowColor => "Glow Color",
                TimelineEffectProperty::TintColor => "Tint Color",
                TimelineEffectProperty::TintAmount => "Tint Amount",
                TimelineEffectProperty::NoiseAmount => "Noise Amount",
                TimelineEffectProperty::NoiseSize => "Noise Size",
                TimelineEffectProperty::NoiseEvolution => "Noise Evolution",
                TimelineEffectProperty::RgbSplitAmount => "RGB Split Amount",
                TimelineEffectProperty::RgbSplitAngle => "RGB Split Angle",
            },
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum TimelineRow<'a> {
    Object {
        object_id: ObjectId,
        name: &'a str,
        visible: bool,
        locked: bool,
    },
    Property {
        object_id: ObjectId,
        property: TimelineProperty,
        keyframe_count: usize,
    },
}

impl TimelineRow<'_> {
    #[must_use]
    pub const fn height(self) -> f32 {
        match self {
            Self::Object { .. } => 30.0,
            Self::Property { .. } => 28.0,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TimelineRowLayout {
    offsets: Vec<f32>,
}

impl TimelineRowLayout {
    #[must_use]
    pub fn new(rows: &[TimelineRow<'_>]) -> Self {
        let mut offsets = Vec::with_capacity(rows.len() + 1);
        offsets.push(0.0);
        let mut y = 0.0;
        for row in rows {
            y += row.height();
            offsets.push(y);
        }
        Self { offsets }
    }

    #[must_use]
    pub fn total_height(&self) -> f32 {
        self.offsets.last().copied().unwrap_or(0.0)
    }

    #[must_use]
    pub fn row_top(&self, index: usize) -> f32 {
        self.offsets
            .get(index)
            .copied()
            .unwrap_or_else(|| self.total_height())
    }

    #[must_use]
    pub fn visible_range(&self, scroll_top: f32, viewport_height: f32) -> std::ops::Range<usize> {
        let row_count = self.offsets.len().saturating_sub(1);
        if row_count == 0 || !scroll_top.is_finite() || !viewport_height.is_finite() {
            return 0..0;
        }

        let top = scroll_top.max(0.0);
        let bottom = (top + viewport_height.max(0.0)).max(top);
        let first = self
            .offsets
            .partition_point(|offset| *offset <= top)
            .saturating_sub(1)
            .min(row_count);
        let end = self
            .offsets
            .partition_point(|offset| *offset < bottom)
            .min(row_count);

        if first >= end {
            row_count..row_count
        } else {
            first..end
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimelineKeyframeRef {
    pub id: KeyframeId,
    pub tick: MusicalTick,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimelineEmptyStates {
    pub no_audio: bool,
    pub no_bpm: bool,
    pub no_objects: bool,
}

#[must_use]
pub fn timeline_empty_states(project: &Project) -> TimelineEmptyStates {
    TimelineEmptyStates {
        no_audio: project.audio_track.is_none(),
        no_bpm: project.tempo_map.initial_segment().is_none(),
        no_objects: project.composition.objects.is_empty(),
    }
}

#[must_use]
pub fn build_timeline_rows(project: &Project) -> Vec<TimelineRow<'_>> {
    let mut rows = Vec::new();

    for object in &project.composition.objects {
        rows.push(TimelineRow::Object {
            object_id: object.id,
            name: &object.name,
            visible: object.visible,
            locked: object.locked,
        });

        let mut push_property = |property, keyframe_count| {
            rows.push(TimelineRow::Property {
                object_id: object.id,
                property,
                keyframe_count,
            });
        };

        push_property(
            TimelineProperty::Position,
            object.transform.position.keyframes().len(),
        );
        push_property(
            TimelineProperty::Scale,
            object.transform.scale.keyframes().len(),
        );
        push_property(
            TimelineProperty::Rotation,
            object.transform.rotation_degrees.keyframes().len(),
        );
        push_property(
            TimelineProperty::Anchor,
            object.transform.anchor.keyframes().len(),
        );
        push_property(
            TimelineProperty::Opacity,
            object.transform.opacity.keyframes().len(),
        );

        match &object.content {
            ObjectContent::Rectangle(rectangle) => {
                push_property(
                    TimelineProperty::RectangleSize,
                    rectangle.size.keyframes().len(),
                );
                push_property(
                    TimelineProperty::RectangleFill,
                    rectangle.fill.keyframes().len(),
                );
                push_property(
                    TimelineProperty::RectangleCornerRadius,
                    rectangle.corner_radius.keyframes().len(),
                );
            }
            ObjectContent::Ellipse(ellipse) => {
                push_property(
                    TimelineProperty::EllipseSize,
                    ellipse.size.keyframes().len(),
                );
                push_property(
                    TimelineProperty::EllipseFill,
                    ellipse.fill.keyframes().len(),
                );
            }
            ObjectContent::Image(_) => {}
            ObjectContent::Text(text) => {
                push_property(TimelineProperty::TextColor, text.color.keyframes().len());
            }
        }

        for effect in &object.effects {
            let effect_id = effect.id;
            match &effect.kind {
                EffectKind::Blur(blur) => {
                    push_property(
                        TimelineProperty::Effect {
                            effect_id,
                            property: TimelineEffectProperty::BlurRadius,
                        },
                        blur.radius_px.keyframes().len(),
                    );
                }
                EffectKind::Glow(glow) => {
                    for (property, keyframe_count) in [
                        (
                            TimelineEffectProperty::GlowRadius,
                            glow.radius_px.keyframes().len(),
                        ),
                        (
                            TimelineEffectProperty::GlowIntensity,
                            glow.intensity.keyframes().len(),
                        ),
                        (
                            TimelineEffectProperty::GlowThreshold,
                            glow.threshold.keyframes().len(),
                        ),
                        (
                            TimelineEffectProperty::GlowColor,
                            glow.color.keyframes().len(),
                        ),
                    ] {
                        push_property(
                            TimelineProperty::Effect {
                                effect_id,
                                property,
                            },
                            keyframe_count,
                        );
                    }
                }
                EffectKind::Tint(tint) => {
                    for (property, keyframe_count) in [
                        (
                            TimelineEffectProperty::TintColor,
                            tint.color.keyframes().len(),
                        ),
                        (
                            TimelineEffectProperty::TintAmount,
                            tint.amount.keyframes().len(),
                        ),
                    ] {
                        push_property(
                            TimelineProperty::Effect {
                                effect_id,
                                property,
                            },
                            keyframe_count,
                        );
                    }
                }
                EffectKind::Noise(noise) => {
                    for (property, keyframe_count) in [
                        (
                            TimelineEffectProperty::NoiseAmount,
                            noise.amount.keyframes().len(),
                        ),
                        (
                            TimelineEffectProperty::NoiseSize,
                            noise.size_px.keyframes().len(),
                        ),
                        (
                            TimelineEffectProperty::NoiseEvolution,
                            noise.evolution.keyframes().len(),
                        ),
                    ] {
                        push_property(
                            TimelineProperty::Effect {
                                effect_id,
                                property,
                            },
                            keyframe_count,
                        );
                    }
                }
                EffectKind::RgbSplit(split) => {
                    for (property, keyframe_count) in [
                        (
                            TimelineEffectProperty::RgbSplitAmount,
                            split.amount_px.keyframes().len(),
                        ),
                        (
                            TimelineEffectProperty::RgbSplitAngle,
                            split.angle_degrees.keyframes().len(),
                        ),
                    ] {
                        push_property(
                            TimelineProperty::Effect {
                                effect_id,
                                property,
                            },
                            keyframe_count,
                        );
                    }
                }
            }
        }
    }

    rows
}

#[must_use]
pub fn query_visible_keyframes(
    project: &Project,
    object_id: ObjectId,
    property: TimelineProperty,
    start_tick: MusicalTick,
    end_tick: MusicalTick,
) -> Vec<TimelineKeyframeRef> {
    if end_tick < start_tick {
        return Vec::new();
    }

    let Some(object) = project
        .composition
        .objects
        .iter()
        .find(|object| object.id == object_id)
    else {
        return Vec::new();
    };

    match property {
        TimelineProperty::Position => {
            visible_keyframes_from_animated(&object.transform.position, start_tick, end_tick)
        }
        TimelineProperty::Scale => {
            visible_keyframes_from_animated(&object.transform.scale, start_tick, end_tick)
        }
        TimelineProperty::Rotation => visible_keyframes_from_animated(
            &object.transform.rotation_degrees,
            start_tick,
            end_tick,
        ),
        TimelineProperty::Anchor => {
            visible_keyframes_from_animated(&object.transform.anchor, start_tick, end_tick)
        }
        TimelineProperty::Opacity => {
            visible_keyframes_from_animated(&object.transform.opacity, start_tick, end_tick)
        }
        TimelineProperty::RectangleSize => match &object.content {
            ObjectContent::Rectangle(rectangle) => {
                visible_keyframes_from_animated(&rectangle.size, start_tick, end_tick)
            }
            _ => Vec::new(),
        },
        TimelineProperty::RectangleFill => match &object.content {
            ObjectContent::Rectangle(rectangle) => {
                visible_keyframes_from_animated(&rectangle.fill, start_tick, end_tick)
            }
            _ => Vec::new(),
        },
        TimelineProperty::RectangleCornerRadius => match &object.content {
            ObjectContent::Rectangle(rectangle) => {
                visible_keyframes_from_animated(&rectangle.corner_radius, start_tick, end_tick)
            }
            _ => Vec::new(),
        },
        TimelineProperty::EllipseSize => match &object.content {
            ObjectContent::Ellipse(ellipse) => {
                visible_keyframes_from_animated(&ellipse.size, start_tick, end_tick)
            }
            _ => Vec::new(),
        },
        TimelineProperty::EllipseFill => match &object.content {
            ObjectContent::Ellipse(ellipse) => {
                visible_keyframes_from_animated(&ellipse.fill, start_tick, end_tick)
            }
            _ => Vec::new(),
        },
        TimelineProperty::TextColor => match &object.content {
            ObjectContent::Text(text) => {
                visible_keyframes_from_animated(&text.color, start_tick, end_tick)
            }
            _ => Vec::new(),
        },
        TimelineProperty::Effect {
            effect_id,
            property,
        } => {
            let Some(effect) = object.effects.iter().find(|effect| effect.id == effect_id) else {
                return Vec::new();
            };

            match (&effect.kind, property) {
                (EffectKind::Blur(blur), TimelineEffectProperty::BlurRadius) => {
                    visible_keyframes_from_animated(&blur.radius_px, start_tick, end_tick)
                }
                (EffectKind::Glow(glow), TimelineEffectProperty::GlowRadius) => {
                    visible_keyframes_from_animated(&glow.radius_px, start_tick, end_tick)
                }
                (EffectKind::Glow(glow), TimelineEffectProperty::GlowIntensity) => {
                    visible_keyframes_from_animated(&glow.intensity, start_tick, end_tick)
                }
                (EffectKind::Glow(glow), TimelineEffectProperty::GlowThreshold) => {
                    visible_keyframes_from_animated(&glow.threshold, start_tick, end_tick)
                }
                (EffectKind::Glow(glow), TimelineEffectProperty::GlowColor) => {
                    visible_keyframes_from_animated(&glow.color, start_tick, end_tick)
                }
                (EffectKind::Tint(tint), TimelineEffectProperty::TintColor) => {
                    visible_keyframes_from_animated(&tint.color, start_tick, end_tick)
                }
                (EffectKind::Tint(tint), TimelineEffectProperty::TintAmount) => {
                    visible_keyframes_from_animated(&tint.amount, start_tick, end_tick)
                }
                (EffectKind::Noise(noise), TimelineEffectProperty::NoiseAmount) => {
                    visible_keyframes_from_animated(&noise.amount, start_tick, end_tick)
                }
                (EffectKind::Noise(noise), TimelineEffectProperty::NoiseSize) => {
                    visible_keyframes_from_animated(&noise.size_px, start_tick, end_tick)
                }
                (EffectKind::Noise(noise), TimelineEffectProperty::NoiseEvolution) => {
                    visible_keyframes_from_animated(&noise.evolution, start_tick, end_tick)
                }
                (EffectKind::RgbSplit(split), TimelineEffectProperty::RgbSplitAmount) => {
                    visible_keyframes_from_animated(&split.amount_px, start_tick, end_tick)
                }
                (EffectKind::RgbSplit(split), TimelineEffectProperty::RgbSplitAngle) => {
                    visible_keyframes_from_animated(&split.angle_degrees, start_tick, end_tick)
                }
                _ => Vec::new(),
            }
        }
    }
}

fn visible_keyframes_from_animated<T>(
    animated: &Animated<T>,
    start_tick: MusicalTick,
    end_tick: MusicalTick,
) -> Vec<TimelineKeyframeRef> {
    let keyframes = animated.keyframes();
    let start_index = keyframes.partition_point(|keyframe| keyframe.tick < start_tick);
    let end_index = keyframes.partition_point(|keyframe| keyframe.tick <= end_tick);

    keyframes[start_index..end_index]
        .iter()
        .map(|keyframe| TimelineKeyframeRef {
            id: keyframe.id,
            tick: keyframe.tick,
        })
        .collect()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MusicalGridLineKind {
    Bar,
    Beat,
    Subdivision,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct MusicalGridLine {
    tick: MusicalTick,
    kind: MusicalGridLineKind,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct CurveEditorSegment {
    object_id: ObjectId,
    property: AnimatableProperty,
    from: PropertyKeyframe,
    to: PropertyKeyframe,
}

fn selected_curve_editor_segment(
    session: &EditorSession,
    project: &Project,
) -> Option<CurveEditorSegment> {
    let selected = session.selected_keyframe_ids();
    if selected.len() != 1 {
        return None;
    }

    let located = locate_property_keyframe(project, selected[0])?;
    let keyframes = property_keyframes(project, located.object_id, located.property).ok()?;
    let from_index = keyframes
        .iter()
        .position(|keyframe| keyframe.id == located.keyframe.id)?;
    let to = *keyframes.get(from_index + 1)?;

    Some(CurveEditorSegment {
        object_id: located.object_id,
        property: located.property,
        from: located.keyframe,
        to,
    })
}

fn curve_editor_point(rect: egui::Rect, x: f32, y: f32) -> egui::Pos2 {
    egui::pos2(
        rect.left() + x * rect.width(),
        rect.bottom() - y * rect.height(),
    )
}

fn curve_editor_normalized_pointer(rect: egui::Rect, pointer: egui::Pos2) -> [f32; 2] {
    let x = if rect.width() > 0.0 {
        (pointer.x - rect.left()) / rect.width()
    } else {
        0.0
    };
    let y = if rect.height() > 0.0 {
        (rect.bottom() - pointer.y) / rect.height()
    } else {
        0.0
    };

    [x.clamp(0.0, 1.0), y.clamp(0.0, 1.0)]
}

fn curve_handle_hit_rect(center: egui::Pos2) -> egui::Rect {
    egui::Rect::from_center_size(center, egui::vec2(18.0, 18.0))
}

fn interpolation_label(interpolation: Interpolation) -> &'static str {
    match interpolation {
        Interpolation::Hold => "Hold",
        Interpolation::Linear => "Linear",
        Interpolation::CubicBezier(_) => "Cubic Bezier",
    }
}

fn draw_curve_editor(
    ui: &mut egui::Ui,
    session: &mut EditorSession,
    project: &Project,
    rows: &[TimelineRow<'_>],
) {
    let Some(segment) = selected_curve_editor_segment(session, project) else {
        return;
    };

    ui.add_space(6.0);
    let (panel_rect, _) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), CURVE_EDITOR_HEIGHT),
        egui::Sense::hover(),
    );
    let painter = ui.painter();
    let visuals = ui.visuals();
    let foreground = visuals.widgets.noninteractive.fg_stroke.color;
    let background = visuals.widgets.noninteractive.bg_fill;

    painter.rect_filled(panel_rect, 4.0, background);
    painter.rect_stroke(
        panel_rect,
        4.0,
        egui::Stroke::new(1.0, foreground.gamma_multiply(0.22)),
        egui::StrokeKind::Inside,
    );

    let object_name = project
        .composition
        .objects
        .iter()
        .find(|object| object.id == segment.object_id)
        .map_or("Object", |object| object.name.as_str());
    let property_label = rows
        .iter()
        .find_map(|row| match row {
            TimelineRow::Property {
                object_id,
                property,
                ..
            } if *object_id == segment.object_id
                && property.animatable_property() == segment.property =>
            {
                Some(property.label())
            }
            _ => None,
        })
        .unwrap_or("Property");

    painter.text(
        egui::pos2(panel_rect.left() + 10.0, panel_rect.top() + 8.0),
        egui::Align2::LEFT_TOP,
        format!("Curve Editor · {object_name} · {property_label}"),
        egui::FontId::proportional(12.0),
        foreground,
    );
    painter.text(
        egui::pos2(panel_rect.right() - 10.0, panel_rect.top() + 8.0),
        egui::Align2::RIGHT_TOP,
        format!(
            "{} → {} · {}",
            segment.from.tick.get(),
            segment.to.tick.get(),
            interpolation_label(segment.from.interpolation)
        ),
        egui::FontId::monospace(11.0),
        visuals.weak_text_color(),
    );

    let graph_rect = egui::Rect::from_min_max(
        egui::pos2(panel_rect.left() + 36.0, panel_rect.top() + 36.0),
        egui::pos2(panel_rect.right() - 18.0, panel_rect.bottom() - 24.0),
    );
    painter.rect_stroke(
        graph_rect,
        0.0,
        egui::Stroke::new(1.0, foreground.gamma_multiply(0.3)),
        egui::StrokeKind::Inside,
    );

    for step in 1..4 {
        let fraction = step as f32 / 4.0;
        let x = graph_rect.left() + fraction * graph_rect.width();
        let y = graph_rect.bottom() - fraction * graph_rect.height();
        let grid_stroke = egui::Stroke::new(1.0, foreground.gamma_multiply(0.12));
        painter.line_segment(
            [
                egui::pos2(x, graph_rect.top()),
                egui::pos2(x, graph_rect.bottom()),
            ],
            grid_stroke,
        );
        painter.line_segment(
            [
                egui::pos2(graph_rect.left(), y),
                egui::pos2(graph_rect.right(), y),
            ],
            grid_stroke,
        );
    }

    let start = curve_editor_point(graph_rect, 0.0, 0.0);
    let end = curve_editor_point(graph_rect, 1.0, 1.0);
    let curve_stroke = egui::Stroke::new(2.0, visuals.selection.stroke.color);

    match segment.from.interpolation {
        Interpolation::Hold => {
            let corner = curve_editor_point(graph_rect, 1.0, 0.0);
            painter.line_segment([start, corner], curve_stroke);
            painter.line_segment([corner, end], curve_stroke);
            painter.text(
                graph_rect.center(),
                egui::Align2::CENTER_CENTER,
                "Hold has no editable Bezier handles",
                egui::FontId::proportional(12.0),
                visuals.weak_text_color(),
            );
        }
        Interpolation::Linear => {
            painter.line_segment([start, end], curve_stroke);
            painter.text(
                egui::pos2(graph_rect.left() + 8.0, graph_rect.top() + 8.0),
                egui::Align2::LEFT_TOP,
                "Linear timing",
                egui::FontId::proportional(11.0),
                visuals.weak_text_color(),
            );
        }
        Interpolation::CubicBezier(easing) => {
            let mut display_easing = session
                .curve_handle_drag_easing(segment.from.id)
                .unwrap_or(easing);
            let first_handle =
                curve_editor_point(graph_rect, display_easing.x1(), display_easing.y1());
            let second_handle =
                curve_editor_point(graph_rect, display_easing.x2(), display_easing.y2());

            let first_response = ui
                .interact(
                    curve_handle_hit_rect(first_handle),
                    egui::Id::new(("curve_handle", segment.from.id.get(), 0_u8)),
                    egui::Sense::click_and_drag(),
                )
                .on_hover_cursor(egui::CursorIcon::Grab);
            let second_response = ui
                .interact(
                    curve_handle_hit_rect(second_handle),
                    egui::Id::new(("curve_handle", segment.from.id.get(), 1_u8)),
                    egui::Sense::click_and_drag(),
                )
                .on_hover_cursor(egui::CursorIcon::Grab);

            if first_response.drag_started() {
                session.begin_curve_handle_drag(
                    segment.from.id,
                    CurveEditorHandle::First,
                    display_easing,
                );
            } else if second_response.drag_started() {
                session.begin_curve_handle_drag(
                    segment.from.id,
                    CurveEditorHandle::Second,
                    display_easing,
                );
            }

            let active_response = if first_response.dragged() {
                Some(&first_response)
            } else if second_response.dragged() {
                Some(&second_response)
            } else {
                None
            };
            if let Some(response) = active_response
                && let Some(pointer) = response.interact_pointer_pos()
            {
                let [x, y] = curve_editor_normalized_pointer(graph_rect, pointer);
                session.update_curve_handle_drag(x, y);
                display_easing = session
                    .curve_handle_drag_easing(segment.from.id)
                    .unwrap_or(display_easing);
            }

            if first_response.drag_stopped() || second_response.drag_stopped() {
                session.finish_curve_handle_drag(segment.from.id);
            }

            let mut points = Vec::with_capacity(CURVE_SAMPLE_COUNT + 1);
            for sample in 0..=CURVE_SAMPLE_COUNT {
                let x = sample as f32 / CURVE_SAMPLE_COUNT as f32;
                let y = evaluate_bezier_easing(display_easing, f64::from(x)) as f32;
                points.push(curve_editor_point(graph_rect, x, y));
            }
            painter.add(egui::Shape::line(points, curve_stroke));

            let first_handle =
                curve_editor_point(graph_rect, display_easing.x1(), display_easing.y1());
            let second_handle =
                curve_editor_point(graph_rect, display_easing.x2(), display_easing.y2());
            let guide_stroke = egui::Stroke::new(1.0, foreground.gamma_multiply(0.45));
            painter.line_segment([start, first_handle], guide_stroke);
            painter.line_segment([end, second_handle], guide_stroke);

            let first_radius = if first_response.hovered() || first_response.dragged() {
                6.0
            } else {
                5.0
            };
            let second_radius = if second_response.hovered() || second_response.dragged() {
                6.0
            } else {
                5.0
            };
            painter.circle_filled(first_handle, first_radius, visuals.selection.bg_fill);
            painter.circle_stroke(
                first_handle,
                first_radius,
                egui::Stroke::new(1.0, visuals.selection.stroke.color),
            );
            painter.circle_filled(second_handle, second_radius, visuals.selection.bg_fill);
            painter.circle_stroke(
                second_handle,
                second_radius,
                egui::Stroke::new(1.0, visuals.selection.stroke.color),
            );

            painter.text(
                egui::pos2(graph_rect.left() + 8.0, graph_rect.top() + 8.0),
                egui::Align2::LEFT_TOP,
                format!(
                    "P1 ({:.2}, {:.2})   P2 ({:.2}, {:.2})",
                    display_easing.x1(),
                    display_easing.y1(),
                    display_easing.x2(),
                    display_easing.y2()
                ),
                egui::FontId::monospace(11.0),
                visuals.weak_text_color(),
            );
        }
    }

    painter.circle_filled(start, 3.0, foreground);
    painter.circle_filled(end, 3.0, foreground);
}

pub fn draw_timeline(
    ui: &mut egui::Ui,
    session: &mut EditorSession,
    project: &Project,
    waveform: Option<&WaveformData>,
) {
    let tempo_map = &project.tempo_map;
    let project_duration = project.settings.duration;
    let empty_states = timeline_empty_states(project);
    let rows = build_timeline_rows(project);
    let (mut start_time, mut end_time) = session.timeline_range(project_duration);

    let (ruler_rect, ruler_response) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), RULER_ROW_HEIGHT),
        egui::Sense::click_and_drag(),
    );
    let (waveform_rect, waveform_response) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), WAVEFORM_ROW_HEIGHT),
        egui::Sense::click_and_drag(),
    );

    let grid_rect = egui::Rect::from_min_max(
        egui::pos2(ruler_rect.left(), ruler_rect.top()),
        egui::pos2(waveform_rect.right(), waveform_rect.bottom()),
    );

    let initial_transform = TimelineTransform::new(ruler_rect, start_time, end_time);
    let pointer = ui.input(|input| input.pointer.hover_pos());
    let pointer_over_timeline = pointer.is_some_and(|position| grid_rect.contains(position));

    if pointer_over_timeline {
        let (zoom_delta, scroll_delta, modifiers) = ui.input(|input| {
            (
                input.zoom_delta(),
                input.smooth_scroll_delta(),
                input.modifiers,
            )
        });

        if modifiers.ctrl && (zoom_delta - 1.0).abs() >= 0.000_1 {
            if let (Some(transform), Some(pointer)) = (initial_transform, pointer) {
                let anchor = transform.x_to_project_time(pointer.x);
                session.zoom_timeline(project_duration, anchor, zoom_delta);
            }
        } else if modifiers.shift {
            let horizontal_delta = if scroll_delta.x.abs() > f32::EPSILON {
                scroll_delta.x
            } else {
                scroll_delta.y
            };
            session.pan_timeline_points(project_duration, horizontal_delta, ruler_rect.width());
        }
    }

    let middle_drag_delta = if ruler_response.dragged_by(egui::PointerButton::Middle) {
        ruler_response.drag_delta().x
    } else if waveform_response.dragged_by(egui::PointerButton::Middle) {
        waveform_response.drag_delta().x
    } else {
        0.0
    };
    if middle_drag_delta.abs() > f32::EPSILON {
        session.pan_timeline_points(project_duration, middle_drag_delta, ruler_rect.width());
    }

    (start_time, end_time) = session.timeline_range(project_duration);
    let Some(ruler_transform) = TimelineTransform::new(ruler_rect, start_time, end_time) else {
        return;
    };
    let Some(waveform_transform) = TimelineTransform::new(waveform_rect, start_time, end_time)
    else {
        return;
    };

    if (ruler_response.clicked() || ruler_response.dragged_by(egui::PointerButton::Primary))
        && let Some(pointer) = ruler_response.interact_pointer_pos()
    {
        session.seek_paused(ruler_transform.x_to_project_time(pointer.x));
    }

    draw_waveform(ui, waveform_rect, waveform_transform, waveform);

    if empty_states.no_audio {
        draw_empty_state_label(ui, waveform_rect, "Import audio to show waveform");
    }

    draw_musical_grid(
        ui,
        grid_rect,
        ruler_transform,
        tempo_map,
        session.authoring_division(),
    );
    draw_time_ruler(ui, ruler_rect, ruler_transform);
    if empty_states.no_bpm {
        draw_empty_state_label(ui, ruler_rect, "Set BPM to enable rhythm grid");
    }
    draw_playhead(ui, grid_rect, ruler_transform, session.playhead());
    draw_curve_editor(ui, session, project, &rows);
    draw_timeline_rows(ui, session, &rows, project, tempo_map, ruler_transform);
}

fn draw_empty_state_label(ui: &egui::Ui, rect: egui::Rect, text: &str) {
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        text,
        egui::FontId::proportional(12.0),
        ui.visuals().weak_text_color(),
    );
}

fn collect_drag_members(
    project: &Project,
    rows: &[TimelineRow<'_>],
    keyframe_ids: &[KeyframeId],
) -> Vec<KeyframeDragMember> {
    let mut members = Vec::with_capacity(keyframe_ids.len());

    for keyframe_id in keyframe_ids {
        for row in rows {
            let TimelineRow::Property {
                object_id,
                property,
                ..
            } = row
            else {
                continue;
            };
            let animatable_property = property.animatable_property();
            let Ok(Some(keyframe)) =
                property_keyframe_by_id(project, *object_id, animatable_property, *keyframe_id)
            else {
                continue;
            };

            members.push(KeyframeDragMember {
                object_id: *object_id,
                property: animatable_property,
                keyframe_id: *keyframe_id,
                original_tick: keyframe.tick,
            });
            break;
        }
    }

    members
}

fn draw_timeline_rows(
    ui: &mut egui::Ui,
    session: &mut EditorSession,
    rows: &[TimelineRow<'_>],
    project: &Project,
    tempo_map: &TempoMap,
    transform: TimelineTransform,
) {
    let available_height = ui.available_height().max(0.0);
    if available_height <= 0.0 {
        return;
    }

    if rows.is_empty() {
        let (rect, _) = ui.allocate_exact_size(
            egui::vec2(ui.available_width(), available_height),
            egui::Sense::hover(),
        );
        draw_empty_state_label(ui, rect, "Add an object to animate");
        return;
    }

    let visible_tick_range = visible_tick_range(tempo_map, transform);
    let layout = TimelineRowLayout::new(rows);
    egui::ScrollArea::vertical()
        .id_salt("timeline_rows_scroll")
        .auto_shrink([false, false])
        .max_height(available_height)
        .show_viewport(ui, |ui, viewport| {
            let visible = layout.visible_range(viewport.top(), viewport.height());
            let selection_clip_rect = ui.clip_rect();
            let mut visible_key_hits = Vec::new();
            ui.add_space(layout.row_top(visible.start));

            for row in &rows[visible.clone()] {
                let height = row.height();
                let (rect, _) = ui.allocate_exact_size(
                    egui::vec2(ui.available_width(), height),
                    egui::Sense::hover(),
                );
                let color = ui.visuals().widgets.noninteractive.fg_stroke.color;
                match row {
                    TimelineRow::Object { name, .. } => {
                        ui.painter().text(
                            egui::pos2(rect.left() + 6.0, rect.center().y),
                            egui::Align2::LEFT_CENTER,
                            *name,
                            egui::FontId::proportional(13.0),
                            color,
                        );
                    }
                    TimelineRow::Property {
                        object_id,
                        property,
                        keyframe_count,
                    } => {
                        let animatable_property = property.animatable_property();
                        let row_response = ui.interact(
                            rect,
                            egui::Id::new(("timeline_property_row", object_id.get(), *property)),
                            egui::Sense::click(),
                        );
                        if row_response.clicked() {
                            session.focus_property(*object_id, animatable_property);
                        }
                        if session.focused_property().is_some_and(|focused| {
                            focused.object_id == *object_id
                                && focused.property == animatable_property
                        }) {
                            ui.painter().rect_filled(
                                rect,
                                0.0,
                                ui.visuals().selection.bg_fill.gamma_multiply(0.28),
                            );
                        }

                        ui.painter().text(
                            egui::pos2(rect.left() + 18.0, rect.center().y),
                            egui::Align2::LEFT_CENTER,
                            property.label(),
                            egui::FontId::proportional(12.0),
                            color.gamma_multiply(0.85),
                        );

                        if let Some((start_tick, end_tick)) = visible_tick_range {
                            for keyframe in query_visible_keyframes(
                                project, *object_id, *property, start_tick, end_tick,
                            ) {
                                let display_tick = session
                                    .keyframe_drag_preview_tick(keyframe.id)
                                    .unwrap_or(keyframe.tick);
                                let Ok(project_time) =
                                    tempo_map.project_time_for_tick(display_tick)
                                else {
                                    continue;
                                };
                                let center = egui::pos2(
                                    transform.project_time_to_x(project_time),
                                    rect.center().y,
                                );
                                let hit_rect = keyframe_hit_rect(center);
                                visible_key_hits.push(VisibleKeyHit {
                                    id: keyframe.id,
                                    rect: hit_rect,
                                });
                                let response = ui.interact(
                                    hit_rect,
                                    egui::Id::new(("timeline_keyframe", keyframe.id.get())),
                                    egui::Sense::click_and_drag(),
                                );
                                if response.clicked() {
                                    session.focus_property(*object_id, animatable_property);
                                    let ctrl = ui.input(|input| input.modifiers.ctrl);
                                    if ctrl {
                                        session.toggle_keyframe_selection(keyframe.id);
                                    } else {
                                        session.select_only_keyframe(keyframe.id);
                                    }
                                }
                                if response.secondary_clicked() {
                                    session.focus_property(*object_id, animatable_property);
                                    if !session.is_keyframe_selected(keyframe.id) {
                                        session.select_only_keyframe(keyframe.id);
                                    }
                                }
                                response.context_menu(|ui| {
                                    ui.label("Interpolation");
                                    for preset in KeyframeInterpolationPreset::ALL {
                                        if ui.button(preset.label()).clicked() {
                                            session.queue_selected_keyframe_interpolation(preset);
                                            ui.close();
                                        }
                                    }
                                });
                                if response.drag_started() {
                                    session.focus_property(*object_id, animatable_property);
                                    if !session.is_keyframe_selected(keyframe.id) {
                                        session.select_only_keyframe(keyframe.id);
                                    }
                                    let selected_ids = session.selected_keyframe_ids();
                                    let members =
                                        collect_drag_members(project, rows, &selected_ids);
                                    session.begin_keyframe_drag(keyframe.id, members);
                                }
                                if response.dragged()
                                    && let Some(pointer) = response.interact_pointer_pos()
                                {
                                    let pointer_time = transform.x_to_project_time(pointer.x);
                                    if let Ok(continuous_tick) =
                                        tempo_map.continuous_tick_position(pointer_time)
                                        && let Ok(target_tick) = snap_tick_position_to_grid(
                                            continuous_tick,
                                            session.authoring_division(),
                                        )
                                    {
                                        session.update_keyframe_drag(keyframe.id, target_tick);
                                    }
                                }
                                if response.drag_stopped() {
                                    session.finish_keyframe_drag(keyframe.id);
                                }
                                draw_keyframe_diamond(
                                    ui,
                                    center,
                                    session.is_keyframe_selected(keyframe.id),
                                    response.hovered(),
                                    color,
                                );
                            }
                        }

                        if *keyframe_count > 0 {
                            ui.painter().text(
                                egui::pos2(rect.right() - 6.0, rect.center().y),
                                egui::Align2::RIGHT_CENTER,
                                keyframe_count.to_string(),
                                egui::FontId::monospace(11.0),
                                color.gamma_multiply(0.65),
                            );
                        }
                    }
                }
            }

            let (primary_pressed, primary_down, primary_released, pointer_pos, press_origin, ctrl) =
                ui.input(|input| {
                    (
                        input.pointer.primary_pressed(),
                        input.pointer.primary_down(),
                        input.pointer.primary_released(),
                        input.pointer.interact_pos(),
                        input.pointer.press_origin(),
                        input.modifiers.ctrl,
                    )
                });

            if primary_pressed
                && let Some(origin) = press_origin
                && selection_clip_rect.contains(origin)
                && !visible_key_hits.iter().any(|hit| hit.rect.contains(origin))
            {
                session.begin_timeline_box_selection([origin.x, origin.y], ctrl);
            }

            if primary_down
                && let Some(pointer) = pointer_pos
                && session.timeline_box_selection().is_some()
            {
                session.update_timeline_box_selection([pointer.x, pointer.y]);
            }

            if let Some((start, current, _)) = session.timeline_box_selection() {
                let selection_rect = egui::Rect::from_two_pos(
                    egui::pos2(start[0], start[1]),
                    egui::pos2(current[0], current[1]),
                );
                let clipped = selection_rect.intersect(selection_clip_rect);
                if clipped.is_positive() {
                    ui.painter().rect_stroke(
                        clipped,
                        0.0,
                        egui::Stroke::new(1.0, ui.visuals().selection.stroke.color),
                        egui::StrokeKind::Inside,
                    );
                    ui.painter().rect_filled(
                        clipped,
                        0.0,
                        ui.visuals().selection.bg_fill.gamma_multiply(0.12),
                    );
                }
            }

            if primary_released
                && let Some((start, current, ctrl_toggle)) = session.take_timeline_box_selection()
            {
                let selection_rect = egui::Rect::from_two_pos(
                    egui::pos2(start[0], start[1]),
                    egui::pos2(current[0], current[1]),
                );
                let selected = keyframe_ids_in_box(selection_rect, &visible_key_hits);
                if ctrl_toggle {
                    session.toggle_keyframe_selection_many(selected);
                } else {
                    session.replace_keyframe_selection(selected);
                }
            }

            let rendered_bottom = layout.row_top(visible.end);
            ui.add_space((layout.total_height() - rendered_bottom).max(0.0));
        });
}

fn visible_tick_range(
    tempo_map: &TempoMap,
    transform: TimelineTransform,
) -> Option<(MusicalTick, MusicalTick)> {
    let start = tempo_map
        .continuous_tick_position(transform.start_time())
        .ok()?
        .floor();
    let end = tempo_map
        .continuous_tick_position(transform.end_time())
        .ok()?
        .ceil();
    if !start.is_finite()
        || !end.is_finite()
        || start < i64::MIN as f64
        || start > i64::MAX as f64
        || end < i64::MIN as f64
        || end > i64::MAX as f64
    {
        return None;
    }

    Some((MusicalTick::new(start as i64), MusicalTick::new(end as i64)))
}

#[derive(Debug, Clone, Copy)]
struct VisibleKeyHit {
    id: KeyframeId,
    rect: egui::Rect,
}

fn keyframe_ids_in_box(selection: egui::Rect, hits: &[VisibleKeyHit]) -> Vec<KeyframeId> {
    hits.iter()
        .filter(|hit| selection.contains(hit.rect.center()))
        .map(|hit| hit.id)
        .collect()
}

fn keyframe_hit_rect(center: egui::Pos2) -> egui::Rect {
    egui::Rect::from_center_size(center, egui::vec2(18.0, 18.0))
}

fn draw_keyframe_diamond(
    ui: &egui::Ui,
    center: egui::Pos2,
    selected: bool,
    hovered: bool,
    base_color: egui::Color32,
) {
    let radius = if selected || hovered { 5.5 } else { 4.5 };
    let color = if selected {
        ui.visuals().selection.bg_fill
    } else if hovered {
        ui.visuals().selection.stroke.color
    } else {
        base_color
    };
    let points = vec![
        egui::pos2(center.x, center.y - radius),
        egui::pos2(center.x + radius, center.y),
        egui::pos2(center.x, center.y + radius),
        egui::pos2(center.x - radius, center.y),
    ];
    ui.painter().add(egui::Shape::convex_polygon(
        points,
        color,
        egui::Stroke::NONE,
    ));
}

fn draw_playhead(
    ui: &egui::Ui,
    rect: egui::Rect,
    transform: TimelineTransform,
    playhead: ProjectTimeNs,
) {
    let x = transform.project_time_to_x(playhead);
    if x < rect.left() || x > rect.right() {
        return;
    }

    let color = ui.visuals().selection.stroke.color;
    ui.painter().line_segment(
        [egui::pos2(x, rect.top()), egui::pos2(x, rect.bottom())],
        egui::Stroke::new(2.0, color),
    );
    ui.painter()
        .circle_filled(egui::pos2(x, rect.top() + 4.0), 4.0, color);
}

fn draw_time_ruler(ui: &egui::Ui, rect: egui::Rect, transform: TimelineTransform) {
    let interval_ns = choose_ruler_interval_ns(transform);
    let first = transform.start_time().get().div_euclid(interval_ns) * interval_ns;
    let color = ui.visuals().widgets.noninteractive.fg_stroke.color;
    let subtle = color.gamma_multiply(0.45);
    let mut tick_time = first;
    let mut rendered = 0_usize;

    while tick_time <= transform.end_time().get() && rendered < 10_000 {
        if tick_time >= transform.start_time().get() {
            let project_time = ProjectTimeNs::new(tick_time);
            let x = transform.project_time_to_x(project_time);
            ui.painter().line_segment(
                [
                    egui::pos2(x, rect.bottom() - 7.0),
                    egui::pos2(x, rect.bottom()),
                ],
                egui::Stroke::new(1.0, subtle),
            );
            ui.painter().text(
                egui::pos2(x + 3.0, rect.top() + 2.0),
                egui::Align2::LEFT_TOP,
                format_ruler_label(project_time, interval_ns),
                egui::FontId::monospace(11.0),
                color,
            );
        }

        let Some(next) = tick_time.checked_add(interval_ns) else {
            break;
        };
        tick_time = next;
        rendered += 1;
    }
}

fn choose_ruler_interval_ns(transform: TimelineTransform) -> i64 {
    const INTERVALS: [i64; 14] = [
        100_000_000,
        250_000_000,
        500_000_000,
        1_000_000_000,
        2_000_000_000,
        5_000_000_000,
        10_000_000_000,
        30_000_000_000,
        60_000_000_000,
        120_000_000_000,
        300_000_000_000,
        600_000_000_000,
        1_800_000_000_000,
        3_600_000_000_000,
    ];

    for interval in INTERVALS {
        let x0 = transform.project_time_to_x(transform.start_time());
        let x1 = transform.project_time_to_x(ProjectTimeNs::new(
            transform.start_time().get().saturating_add(interval),
        ));
        if (x1 - x0).abs() >= MIN_RULER_LABEL_SPACING_PX {
            return interval;
        }
    }

    INTERVALS[INTERVALS.len() - 1]
}

fn format_ruler_label(project_time: ProjectTimeNs, interval_ns: i64) -> String {
    let total_ns = project_time.get().max(0);
    let total_seconds = total_ns as f64 / 1_000_000_000.0;

    if total_seconds >= 60.0 {
        let minutes = (total_seconds / 60.0).floor() as u64;
        let seconds = total_seconds - minutes as f64 * 60.0;
        format!("{minutes}:{seconds:04.1}")
    } else if interval_ns < 1_000_000_000 {
        format!("{total_seconds:.1}s")
    } else {
        format!("{total_seconds:.0}s")
    }
}

fn draw_musical_grid(
    ui: &egui::Ui,
    rect: egui::Rect,
    transform: TimelineTransform,
    tempo_map: &TempoMap,
    division: BeatDivision,
) {
    let Some(segment) = tempo_map.initial_segment() else {
        return;
    };
    let lines = collect_musical_grid_lines(tempo_map, division, transform);
    if lines.is_empty() {
        return;
    }

    let base_color = ui.visuals().widgets.noninteractive.fg_stroke.color;
    let subdivision_color = base_color.gamma_multiply(0.16);
    let beat_color = base_color.gamma_multiply(0.32);
    let bar_color = base_color.gamma_multiply(0.58);

    let subdivision_spacing = grid_spacing_px(tempo_map, division.ticks_per_step(), transform);
    let beat_spacing = grid_spacing_px(tempo_map, PPQ, transform);
    let ticks_per_bar = ticks_per_bar(segment.meter());
    let bar_spacing = grid_spacing_px(tempo_map, ticks_per_bar, transform);
    let draw_subdivisions = subdivision_spacing >= 4.0;
    let draw_beats = beat_spacing >= 5.0;
    let draw_bar_labels = bar_spacing >= MIN_RULER_LABEL_SPACING_PX;

    for line in lines {
        if line.kind == MusicalGridLineKind::Subdivision && !draw_subdivisions {
            continue;
        }
        if line.kind == MusicalGridLineKind::Beat && !draw_beats {
            continue;
        }

        let Ok(project_time) = tempo_map.project_time_for_tick(line.tick) else {
            continue;
        };
        let x = transform.project_time_to_x(project_time);
        if x < rect.left() || x > rect.right() {
            continue;
        }

        let (width, color) = match line.kind {
            MusicalGridLineKind::Bar => (1.5, bar_color),
            MusicalGridLineKind::Beat => (1.0, beat_color),
            MusicalGridLineKind::Subdivision => (0.5, subdivision_color),
        };
        ui.painter().line_segment(
            [egui::pos2(x, rect.top()), egui::pos2(x, rect.bottom())],
            egui::Stroke::new(width, color),
        );

        if line.kind == MusicalGridLineKind::Bar && draw_bar_labels && ticks_per_bar > 0 {
            let bar_index = line.tick.get().div_euclid(ticks_per_bar) + 1;
            ui.painter().text(
                egui::pos2(x + 3.0, rect.top() + 14.0),
                egui::Align2::LEFT_TOP,
                format!("B{bar_index}"),
                egui::FontId::monospace(10.0),
                bar_color,
            );
        }
    }
}

fn collect_musical_grid_lines(
    tempo_map: &TempoMap,
    division: BeatDivision,
    transform: TimelineTransform,
) -> Vec<MusicalGridLine> {
    let Some(segment) = tempo_map.initial_segment() else {
        return Vec::new();
    };
    let Ok(start_tick_position) = tempo_map.continuous_tick_position(transform.start_time()) else {
        return Vec::new();
    };
    let Ok(end_tick_position) = tempo_map.continuous_tick_position(transform.end_time()) else {
        return Vec::new();
    };

    let Ok(mut tick) = floor_tick_position_to_grid(start_tick_position, division) else {
        return Vec::new();
    };
    let step = division.ticks_per_step();
    let last_tick = end_tick_position.ceil() as i64;
    let bar_ticks = ticks_per_bar(segment.meter());
    if step <= 0 || bar_ticks <= 0 {
        return Vec::new();
    }

    let mut lines = Vec::new();
    for _ in 0..MAX_GRID_LINES_PER_FRAME {
        if tick.get() > last_tick {
            break;
        }

        let kind = if tick.get().rem_euclid(bar_ticks) == 0 {
            MusicalGridLineKind::Bar
        } else if tick.get().rem_euclid(PPQ) == 0 {
            MusicalGridLineKind::Beat
        } else {
            MusicalGridLineKind::Subdivision
        };
        lines.push(MusicalGridLine { tick, kind });

        let Some(next) = tick.get().checked_add(step) else {
            break;
        };
        tick = MusicalTick::new(next);
    }

    lines
}

fn ticks_per_bar(meter: rhythm_core::time::TimeSignature) -> i64 {
    let numerator = i64::from(meter.numerator());
    let denominator = i64::from(meter.denominator());
    PPQ.saturating_mul(4)
        .saturating_mul(numerator)
        .checked_div(denominator)
        .unwrap_or(0)
}

fn grid_spacing_px(tempo_map: &TempoMap, tick_delta: i64, transform: TimelineTransform) -> f32 {
    let Ok(a) = tempo_map.project_time_for_tick(MusicalTick::new(0)) else {
        return 0.0;
    };
    let Ok(b) = tempo_map.project_time_for_tick(MusicalTick::new(tick_delta)) else {
        return 0.0;
    };
    (transform.project_time_to_x(b) - transform.project_time_to_x(a)).abs()
}

fn draw_waveform(
    ui: &egui::Ui,
    rect: egui::Rect,
    transform: TimelineTransform,
    waveform: Option<&WaveformData>,
) {
    let Some(waveform) = waveform else {
        return;
    };
    let sample_rate = waveform.pyramid.source_sample_rate;
    let source_frames = waveform.pyramid.source_frame_count;
    if sample_rate == 0 || source_frames == 0 {
        return;
    }

    let visible_start_frame = source_frame_for_project_time(transform.start_time(), sample_rate);
    let visible_end_frame =
        source_frame_for_project_time(transform.end_time(), sample_rate).min(source_frames);
    if visible_end_frame <= visible_start_frame {
        return;
    }

    let level_index =
        waveform.choose_level_for_view(visible_start_frame, visible_end_frame, rect.width());
    let Some(slice) = waveform.visible_slice(level_index, visible_start_frame, visible_end_frame)
    else {
        return;
    };

    let color = ui.visuals().widgets.noninteractive.fg_stroke.color;
    let mesh = build_waveform_mesh(slice, sample_rate, transform, color);
    if !mesh.indices.is_empty() {
        ui.painter().add(egui::Shape::mesh(mesh));
    }
}

fn build_waveform_mesh(
    slice: WaveformSlice<'_>,
    source_sample_rate: u32,
    transform: TimelineTransform,
    color: egui::Color32,
) -> egui::epaint::Mesh {
    let mut mesh = egui::epaint::Mesh::default();
    let rect = transform.rect();
    if slice.peaks.is_empty()
        || source_sample_rate == 0
        || rect.width() <= 0.0
        || rect.height() <= 0.0
    {
        return mesh;
    }

    let center_y = rect.center().y;
    let half_height = rect.height() * 0.5;

    for (offset, peak) in slice.peaks.iter().enumerate() {
        let peak_index = slice.first_peak_index.saturating_add(offset);
        let Ok(peak_index) = u64::try_from(peak_index) else {
            break;
        };
        let peak_start = peak_index.saturating_mul(slice.frames_per_peak);
        let peak_end = peak_start.saturating_add(slice.frames_per_peak);

        let Some(peak_start_time) = project_time_for_source_frame(peak_start, source_sample_rate)
        else {
            continue;
        };
        let Some(peak_end_time) = project_time_for_source_frame(peak_end, source_sample_rate)
        else {
            continue;
        };

        let x0 = transform
            .project_time_to_x(peak_start_time)
            .max(rect.left());
        let mut x1 = transform.project_time_to_x(peak_end_time).min(rect.right());
        x1 = x1.max(x0 + 0.75).min(rect.right());
        if x1 <= x0 {
            continue;
        }

        let amplitude_min = peak.min.clamp(-1.0, 1.0);
        let amplitude_max = peak.max.clamp(-1.0, 1.0);
        let mut top = center_y - amplitude_max * half_height;
        let mut bottom = center_y - amplitude_min * half_height;

        if bottom - top < 1.0 {
            let middle = (top + bottom) * 0.5;
            top = (middle - 0.5).max(rect.top());
            bottom = (middle + 0.5).min(rect.bottom());
        }

        let peak_rect = egui::Rect::from_min_max(
            egui::pos2(x0, top.max(rect.top())),
            egui::pos2(x1, bottom.min(rect.bottom())),
        );
        mesh.add_colored_rect(peak_rect, color);
    }

    mesh
}

fn project_time_for_source_frame(frame: u64, sample_rate: u32) -> Option<ProjectTimeNs> {
    if sample_rate == 0 {
        return None;
    }

    let nanos = u128::from(frame)
        .checked_mul(1_000_000_000)?
        .checked_div(u128::from(sample_rate))?;
    Some(ProjectTimeNs::new(i64::try_from(nanos).ok()?))
}

fn source_frame_for_project_time(project_time: ProjectTimeNs, sample_rate: u32) -> u64 {
    if project_time.get() <= 0 || sample_rate == 0 {
        return 0;
    }

    let frames =
        (project_time.get() as u128).saturating_mul(u128::from(sample_rate)) / 1_000_000_000_u128;
    u64::try_from(frames).unwrap_or(u64::MAX)
}

#[cfg(test)]
mod tests {
    use super::{
        MusicalGridLineKind, TimelineRow, TimelineRowLayout, TimelineTransform,
        build_waveform_mesh, collect_musical_grid_lines, query_visible_keyframes,
        selected_curve_editor_segment, timeline_empty_states,
    };
    use rhythm_core::time::{
        BeatDivision, BpmMicros, GridOffsetNs, ProjectTimeNs, TempoMap, TimeSignature,
    };
    use rhythm_engine::waveform::{WavePeak, WaveformSlice};

    #[test]
    fn curve_editor_uses_one_selected_outgoing_segment() {
        use crate::editor_session::EditorSession;
        use rhythm_core::{
            animation::{Animated, BezierEasing, Interpolation, Keyframe},
            domain::{LinearRgba, Vec2},
            ids::{KeyframeId, ObjectId},
            project::{
                Object, ObjectContent, Project, ProjectSettings, RectangleObject,
                TransformAnimation,
            },
            time::{GridOffsetNs, MusicalTick, TempoMap},
        };

        let object_id = ObjectId::new(1).expect("object id");
        let first = KeyframeId::new(11).expect("first key id");
        let second = KeyframeId::new(12).expect("second key id");
        let terminal = KeyframeId::new(13).expect("terminal key id");
        let mut project = Project::new(
            "Curve",
            ProjectSettings::default(),
            TempoMap::unset(GridOffsetNs::new(0)),
        );
        project.composition.objects.push(Object {
            id: object_id,
            name: "Rect".to_owned(),
            visible: true,
            locked: false,
            transform: TransformAnimation::new(
                Animated::new_static(Vec2::new(0.0, 0.0).expect("position")),
                Animated::new_static(Vec2::new(1.0, 1.0).expect("scale")),
                Animated::new_static(0.0),
                Animated::new_static(Vec2::new(0.5, 0.5).expect("anchor")),
                Animated::with_keyframes(
                    1.0,
                    vec![
                        Keyframe::new(
                            first,
                            MusicalTick::new(0),
                            0.0,
                            Interpolation::CubicBezier(BezierEasing::EASE_IN),
                        ),
                        Keyframe::new(second, MusicalTick::new(240), 0.5, Interpolation::Linear),
                        Keyframe::new(terminal, MusicalTick::new(480), 1.0, Interpolation::Linear),
                    ],
                )
                .expect("unique opacity keys"),
            ),
            content: ObjectContent::Rectangle(RectangleObject {
                size: Animated::new_static(Vec2::new(100.0, 50.0).expect("size")),
                fill: Animated::new_static(LinearRgba::black_opaque()),
                corner_radius: Animated::new_static(0.0),
            }),
            effects: Vec::new(),
        });

        let mut session = EditorSession::default();
        session.select_only_keyframe(first);
        let segment = selected_curve_editor_segment(&session, &project).expect("outgoing segment");
        assert_eq!(segment.object_id, object_id);
        assert_eq!(segment.from.id, first);
        assert_eq!(segment.to.id, second);
        assert_eq!(
            segment.from.interpolation,
            Interpolation::CubicBezier(BezierEasing::EASE_IN)
        );

        session.select_only_keyframe(terminal);
        assert!(selected_curve_editor_segment(&session, &project).is_none());

        session.select_only_keyframe(first);
        session.toggle_keyframe_selection(second);
        assert!(selected_curve_editor_segment(&session, &project).is_none());
    }

    #[test]
    fn curve_editor_pointer_coordinates_clamp_to_unit_square() {
        let rect = egui::Rect::from_min_max(egui::pos2(10.0, 20.0), egui::pos2(110.0, 220.0));

        assert_eq!(
            super::curve_editor_normalized_pointer(rect, egui::pos2(-40.0, 260.0)),
            [0.0, 0.0]
        );
        assert_eq!(
            super::curve_editor_normalized_pointer(rect, egui::pos2(160.0, -30.0)),
            [1.0, 1.0]
        );
        assert_eq!(
            super::curve_editor_normalized_pointer(rect, egui::pos2(35.0, 70.0)),
            [0.25, 0.75]
        );
    }

    #[test]
    fn curve_handle_hit_area_is_larger_than_visible_point() {
        let rect = super::curve_handle_hit_rect(egui::pos2(40.0, 50.0));

        assert_eq!(rect.width(), 18.0);
        assert_eq!(rect.height(), 18.0);
        assert_eq!(rect.center(), egui::pos2(40.0, 50.0));
    }

    #[test]
    fn curve_editor_normalized_points_map_to_graph_corners() {
        let rect = egui::Rect::from_min_max(egui::pos2(10.0, 20.0), egui::pos2(110.0, 220.0));

        assert_eq!(
            super::curve_editor_point(rect, 0.0, 0.0),
            egui::pos2(10.0, 220.0)
        );
        assert_eq!(
            super::curve_editor_point(rect, 1.0, 1.0),
            egui::pos2(110.0, 20.0)
        );
        assert_eq!(
            super::curve_editor_point(rect, 0.25, 0.75),
            egui::pos2(35.0, 70.0)
        );
    }

    #[test]
    fn box_selection_collects_key_centers_inside_rect() {
        let first = rhythm_core::ids::KeyframeId::new(31).expect("key id");
        let second = rhythm_core::ids::KeyframeId::new(32).expect("key id");
        let hits = [
            super::VisibleKeyHit {
                id: first,
                rect: super::keyframe_hit_rect(egui::pos2(20.0, 20.0)),
            },
            super::VisibleKeyHit {
                id: second,
                rect: super::keyframe_hit_rect(egui::pos2(80.0, 80.0)),
            },
        ];
        let selected = super::keyframe_ids_in_box(
            egui::Rect::from_two_pos(egui::pos2(0.0, 0.0), egui::pos2(50.0, 50.0)),
            &hits,
        );

        assert_eq!(selected, vec![first]);
    }

    #[test]
    fn keyframe_hit_box_is_at_least_18_by_18() {
        let rect = super::keyframe_hit_rect(egui::pos2(100.0, 50.0));

        assert!(rect.width() >= 18.0);
        assert!(rect.height() >= 18.0);
        assert_eq!(rect.center(), egui::pos2(100.0, 50.0));
    }

    #[test]
    fn row_layout_virtualizes_mixed_height_rows() {
        let object_id = rhythm_core::ids::ObjectId::new(1).expect("object id");
        let rows = [
            TimelineRow::Object {
                object_id,
                name: "Object",
                visible: true,
                locked: false,
            },
            TimelineRow::Property {
                object_id,
                property: super::TimelineProperty::Position,
                keyframe_count: 0,
            },
            TimelineRow::Property {
                object_id,
                property: super::TimelineProperty::Opacity,
                keyframe_count: 0,
            },
        ];
        let layout = TimelineRowLayout::new(&rows);

        assert_eq!(layout.total_height(), 86.0);
        assert_eq!(layout.visible_range(0.0, 29.0), 0..1);
        assert_eq!(layout.visible_range(30.0, 28.0), 1..2);
        assert_eq!(layout.visible_range(57.0, 29.0), 1..3);
        assert_eq!(layout.visible_range(100.0, 20.0), 3..3);
    }

    #[test]
    fn timeline_rows_include_object_transform_content_and_effect_properties() {
        use rhythm_core::{
            animation::Animated,
            domain::{LinearRgba, Vec2},
            ids::{EffectId, ObjectId},
            project::{
                BlurEffect, Effect, Object, ObjectContent, Project, ProjectSettings,
                RectangleObject, TransformAnimation,
            },
            time::{GridOffsetNs, TempoMap},
        };

        let mut project = Project::new(
            "Rows",
            ProjectSettings::default(),
            TempoMap::unset(GridOffsetNs::new(0)),
        );
        project.composition.objects.push(Object {
            id: ObjectId::new(1).expect("object id"),
            name: "Rect".to_owned(),
            visible: true,
            locked: false,
            transform: TransformAnimation::new(
                Animated::new_static(Vec2::new(0.0, 0.0).expect("position")),
                Animated::new_static(Vec2::new(1.0, 1.0).expect("scale")),
                Animated::new_static(0.0),
                Animated::new_static(Vec2::new(0.5, 0.5).expect("anchor")),
                Animated::new_static(1.0),
            ),
            content: ObjectContent::Rectangle(RectangleObject {
                size: Animated::new_static(Vec2::new(100.0, 50.0).expect("size")),
                fill: Animated::new_static(LinearRgba::black_opaque()),
                corner_radius: Animated::new_static(0.0),
            }),
            effects: vec![Effect {
                id: EffectId::new(2).expect("effect id"),
                enabled: true,
                kind: rhythm_core::project::EffectKind::Blur(BlurEffect {
                    radius_px: Animated::new_static(4.0),
                }),
            }],
        });

        let rows = super::build_timeline_rows(&project);

        assert_eq!(rows.len(), 10);
        assert!(matches!(
            rows[0],
            super::TimelineRow::Object {
                object_id,
                name: "Rect",
                ..
            } if object_id.get() == 1
        ));
        assert!(matches!(
            rows[1],
            super::TimelineRow::Property {
                property: super::TimelineProperty::Position,
                ..
            }
        ));
        assert!(matches!(
            rows[9],
            super::TimelineRow::Property {
                property: super::TimelineProperty::Effect {
                    property: super::TimelineEffectProperty::BlurRadius,
                    ..
                },
                ..
            }
        ));
    }

    fn timeline_stress_project() -> rhythm_core::project::Project {
        use rhythm_core::{
            animation::{Animated, Interpolation, Keyframe},
            domain::{LinearRgba, Vec2},
            ids::{KeyframeId, ObjectId},
            project::{
                Object, ObjectContent, Project, ProjectSettings, RectangleObject,
                TransformAnimation,
            },
            time::{GridOffsetNs, MusicalTick, TempoMap},
        };

        const OBJECT_COUNT: usize = 500;
        const KEYS_PER_OBJECT: usize = 20;

        let mut project = Project::new(
            "Timeline Stress",
            ProjectSettings::default(),
            TempoMap::unset(GridOffsetNs::new(0)),
        );
        let mut next_keyframe_id = 1_001_u64;

        for object_index in 0..OBJECT_COUNT {
            let object_id = ObjectId::new((object_index + 1) as u64).expect("object id");
            let mut opacity_keys = Vec::with_capacity(KEYS_PER_OBJECT);
            for key_index in 0..KEYS_PER_OBJECT {
                opacity_keys.push(Keyframe::new(
                    KeyframeId::new(next_keyframe_id).expect("keyframe id"),
                    MusicalTick::new((key_index as i64) * 60),
                    key_index as f32 / (KEYS_PER_OBJECT - 1) as f32,
                    Interpolation::Linear,
                ));
                next_keyframe_id += 1;
            }

            project.composition.objects.push(Object {
                id: object_id,
                name: format!("Stress Rect {object_index}"),
                visible: true,
                locked: false,
                transform: TransformAnimation::new(
                    Animated::new_static(Vec2::new(0.0, 0.0).expect("position")),
                    Animated::new_static(Vec2::new(1.0, 1.0).expect("scale")),
                    Animated::new_static(0.0),
                    Animated::new_static(Vec2::new(0.5, 0.5).expect("anchor")),
                    Animated::with_keyframes(1.0, opacity_keys).expect("sorted opacity keys"),
                ),
                content: ObjectContent::Rectangle(RectangleObject {
                    size: Animated::new_static(Vec2::new(100.0, 50.0).expect("size")),
                    fill: Animated::new_static(LinearRgba::black_opaque()),
                    corner_radius: Animated::new_static(0.0),
                }),
                effects: Vec::new(),
            });
        }

        project.next_entity_id = next_keyframe_id;
        project
            .validate()
            .expect("timeline stress fixture is valid");
        project
    }

    #[test]
    fn timeline_stress_fixture_limits_row_and_key_work_to_visible_ranges() {
        let project = timeline_stress_project();
        let rows = super::build_timeline_rows(&project);
        let total_keyframes: usize = rows
            .iter()
            .filter_map(|row| match row {
                TimelineRow::Property { keyframe_count, .. } => Some(*keyframe_count),
                TimelineRow::Object { .. } => None,
            })
            .sum();

        assert_eq!(project.composition.objects.len(), 500);
        assert_eq!(total_keyframes, 10_000);

        let layout = TimelineRowLayout::new(&rows);
        let visible_rows = layout.visible_range(0.0, 300.0);
        assert!(visible_rows.len() < 20);
        assert!(visible_rows.len() < rows.len() / 100);

        let visible_keys = query_visible_keyframes(
            &project,
            rhythm_core::ids::ObjectId::new(1).expect("object id"),
            super::TimelineProperty::Opacity,
            rhythm_core::time::MusicalTick::new(300),
            rhythm_core::time::MusicalTick::new(420),
        );
        assert_eq!(visible_keys.len(), 3);
        assert_eq!(
            visible_keys
                .iter()
                .map(|keyframe| keyframe.tick.get())
                .collect::<Vec<_>>(),
            vec![300, 360, 420]
        );
    }

    #[test]
    fn visible_keyframe_query_binary_searches_musical_tick_range() {
        use rhythm_core::{
            animation::{Animated, Interpolation, Keyframe},
            domain::{LinearRgba, Vec2},
            ids::{KeyframeId, ObjectId},
            project::{
                Object, ObjectContent, Project, ProjectSettings, RectangleObject,
                TransformAnimation,
            },
            time::{GridOffsetNs, MusicalTick, TempoMap},
        };

        let object_id = ObjectId::new(1).expect("object id");
        let mut project = Project::new(
            "Keys",
            ProjectSettings::default(),
            TempoMap::unset(GridOffsetNs::new(0)),
        );
        project.composition.objects.push(Object {
            id: object_id,
            name: "Rect".to_owned(),
            visible: true,
            locked: false,
            transform: TransformAnimation::new(
                Animated::new_static(Vec2::new(0.0, 0.0).expect("position")),
                Animated::new_static(Vec2::new(1.0, 1.0).expect("scale")),
                Animated::new_static(0.0),
                Animated::new_static(Vec2::new(0.5, 0.5).expect("anchor")),
                Animated::with_keyframes(
                    1.0,
                    vec![
                        Keyframe::new(
                            KeyframeId::new(2).expect("key id"),
                            MusicalTick::new(0),
                            0.0,
                            Interpolation::Linear,
                        ),
                        Keyframe::new(
                            KeyframeId::new(3).expect("key id"),
                            MusicalTick::new(240),
                            0.5,
                            Interpolation::Linear,
                        ),
                        Keyframe::new(
                            KeyframeId::new(4).expect("key id"),
                            MusicalTick::new(480),
                            1.0,
                            Interpolation::Linear,
                        ),
                    ],
                )
                .expect("sorted unique keys"),
            ),
            content: ObjectContent::Rectangle(RectangleObject {
                size: Animated::new_static(Vec2::new(100.0, 50.0).expect("size")),
                fill: Animated::new_static(LinearRgba::black_opaque()),
                corner_radius: Animated::new_static(0.0),
            }),
            effects: Vec::new(),
        });

        let visible = query_visible_keyframes(
            &project,
            object_id,
            super::TimelineProperty::Opacity,
            MusicalTick::new(100),
            MusicalTick::new(300),
        );

        assert_eq!(visible.len(), 1);
        assert_eq!(visible[0].id.get(), 3);
        assert_eq!(visible[0].tick, MusicalTick::new(240));
    }

    #[test]
    fn default_project_reports_all_timeline_empty_states() {
        let project = rhythm_core::project::Project::new(
            "Empty",
            rhythm_core::project::ProjectSettings::default(),
            rhythm_core::time::TempoMap::unset(rhythm_core::time::GridOffsetNs::new(0)),
        );

        let states = timeline_empty_states(&project);
        assert!(states.no_audio);
        assert!(states.no_bpm);
        assert!(states.no_objects);
    }

    #[test]
    fn ruler_midpoint_maps_to_continuous_project_time() {
        let rect = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1_000.0, 28.0));
        let transform = TimelineTransform::new(
            rect,
            ProjectTimeNs::new(0),
            ProjectTimeNs::new(10_000_000_000),
        )
        .expect("transform");

        let mapped = transform.x_to_project_time(333.3).get();
        assert!((mapped - 3_333_000_000).abs() < 2_000);
    }

    #[test]
    fn timeline_transform_round_trips_project_time_and_x() {
        let rect = egui::Rect::from_min_size(egui::pos2(10.0, 20.0), egui::vec2(100.0, 64.0));
        let transform = TimelineTransform::new(
            rect,
            ProjectTimeNs::new(1_000_000_000),
            ProjectTimeNs::new(3_000_000_000),
        )
        .expect("valid transform");

        assert_eq!(
            transform.project_time_to_x(ProjectTimeNs::new(2_000_000_000)),
            60.0
        );
        assert_eq!(
            transform.x_to_project_time(60.0),
            ProjectTimeNs::new(2_000_000_000)
        );
        assert_eq!(
            transform.x_to_project_time(-100.0),
            ProjectTimeNs::new(1_000_000_000)
        );
        assert_eq!(
            transform.x_to_project_time(1_000.0),
            ProjectTimeNs::new(3_000_000_000)
        );
    }

    #[test]
    fn four_four_grid_classifies_bars_beats_and_subdivisions() {
        let tempo = TempoMap::with_initial_tempo(
            GridOffsetNs::new(0),
            BpmMicros::new(120_000_000).expect("BPM"),
            TimeSignature::default(),
        );
        let rect = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(800.0, 100.0));
        let transform = TimelineTransform::new(
            rect,
            ProjectTimeNs::new(0),
            ProjectTimeNs::new(2_000_000_000),
        )
        .expect("transform");
        let lines =
            collect_musical_grid_lines(&tempo, BeatDivision::new(4).expect("division"), transform);

        assert_eq!(
            lines
                .iter()
                .filter(|line| line.kind == MusicalGridLineKind::Bar)
                .count(),
            2
        );
        assert_eq!(
            lines
                .iter()
                .filter(|line| line.kind == MusicalGridLineKind::Beat)
                .count(),
            3
        );
        assert!(
            lines
                .iter()
                .filter(|line| line.kind == MusicalGridLineKind::Subdivision)
                .count()
                >= 12
        );
    }

    #[test]
    fn unset_tempo_has_no_musical_grid() {
        let tempo = TempoMap::unset(GridOffsetNs::new(0));
        let rect = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(800.0, 100.0));
        let transform = TimelineTransform::new(
            rect,
            ProjectTimeNs::new(0),
            ProjectTimeNs::new(10_000_000_000),
        )
        .expect("transform");

        assert!(
            collect_musical_grid_lines(&tempo, BeatDivision::new(4).expect("division"), transform,)
                .is_empty()
        );
    }

    #[test]
    fn waveform_uses_one_mesh_for_all_visible_peaks() {
        let peaks = [
            WavePeak {
                min: -1.0,
                max: 0.5,
            },
            WavePeak {
                min: -0.25,
                max: 0.75,
            },
        ];
        let slice = WaveformSlice {
            level_index: 0,
            first_peak_index: 0,
            frames_per_peak: 64,
            peaks: &peaks,
        };
        let rect = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(128.0, 64.0));
        let transform =
            TimelineTransform::new(rect, ProjectTimeNs::new(0), ProjectTimeNs::new(128_000_000))
                .expect("valid transform");

        let mesh = build_waveform_mesh(slice, 1_000, transform, egui::Color32::WHITE);

        assert_eq!(mesh.vertices.len(), 8);
        assert_eq!(mesh.indices.len(), 12);
    }

    #[test]
    fn waveform_mesh_maps_visible_range_to_panel_width() {
        let peaks = [WavePeak {
            min: -0.5,
            max: 0.5,
        }];
        let slice = WaveformSlice {
            level_index: 2,
            first_peak_index: 1,
            frames_per_peak: 256,
            peaks: &peaks,
        };
        let rect = egui::Rect::from_min_size(egui::pos2(10.0, 20.0), egui::vec2(100.0, 64.0));
        let transform = TimelineTransform::new(
            rect,
            ProjectTimeNs::new(256_000_000),
            ProjectTimeNs::new(512_000_000),
        )
        .expect("valid transform");

        let mesh = build_waveform_mesh(slice, 1_000, transform, egui::Color32::WHITE);
        let min_x = mesh
            .vertices
            .iter()
            .map(|vertex| vertex.pos.x)
            .fold(f32::INFINITY, f32::min);
        let max_x = mesh
            .vertices
            .iter()
            .map(|vertex| vertex.pos.x)
            .fold(f32::NEG_INFINITY, f32::max);

        assert_eq!(min_x, rect.left());
        assert_eq!(max_x, rect.right());
    }
}
