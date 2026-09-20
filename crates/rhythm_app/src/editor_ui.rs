use crate::{
    editor_session::{
        EditorSession, InspectorMultiNumericTarget, InspectorNumericComponent,
        InspectorNumericTarget, InspectorTextField, InspectorTextTarget, PreviewQuality,
        ViewportCameraAction,
    },
    timeline::draw_timeline,
    viewport::{
        objects_intersecting_box, pick_topmost_object, selected_objects_bounds,
        selection_overlay_geometry,
    },
};
use rhythm_core::{
    domain::Vec2,
    geometry::LocalBounds2d,
    ids::{AssetId, ObjectId},
    project::{
        AssetSource, FontStyle, FontWeight, ObjectContent, Project, TextAlignment, TextObject,
    },
    property::{
        AnimatableProperty, PropertyValue, evaluate_property_at_tick, property_base_value,
        property_keyframe_at_tick, property_keyframe_count,
    },
    time::{ProjectTimeNs, snap_tick_position_to_grid},
};

const FONT_WEIGHTS: [FontWeight; 9] = [
    FontWeight::Thin,
    FontWeight::ExtraLight,
    FontWeight::Light,
    FontWeight::Normal,
    FontWeight::Medium,
    FontWeight::SemiBold,
    FontWeight::Bold,
    FontWeight::ExtraBold,
    FontWeight::Black,
];

fn font_weight_label(weight: FontWeight) -> &'static str {
    match weight {
        FontWeight::Thin => "Thin",
        FontWeight::ExtraLight => "Extra Light",
        FontWeight::Light => "Light",
        FontWeight::Normal => "Normal",
        FontWeight::Medium => "Medium",
        FontWeight::SemiBold => "Semi Bold",
        FontWeight::Bold => "Bold",
        FontWeight::ExtraBold => "Extra Bold",
        FontWeight::Black => "Black",
    }
}

fn font_style_label(style: FontStyle) -> &'static str {
    match style {
        FontStyle::Normal => "Normal",
        FontStyle::Italic => "Italic",
    }
}

fn text_alignment_label(alignment: TextAlignment) -> &'static str {
    match alignment {
        TextAlignment::Left => "Left",
        TextAlignment::Center => "Center",
        TextAlignment::Right => "Right",
    }
}

fn draw_text_inspector(
    ui: &mut egui::Ui,
    session: &mut EditorSession,
    object_id: ObjectId,
    text: &TextObject,
    project: &Project,
) {
    ui.add_space(8.0);
    ui.separator();
    ui.heading("Text");

    let content_target = InspectorTextTarget {
        object_id,
        field: InspectorTextField::Content,
    };
    ui.label("Content");
    if let Some(mut buffer) = session
        .inspector_text_edit_buffer(content_target)
        .map(str::to_owned)
    {
        let response = ui.add(
            egui::TextEdit::multiline(&mut buffer)
                .desired_rows(4)
                .desired_width(f32::INFINITY),
        );
        session.update_inspector_text_edit_buffer(content_target, buffer);
        let escape = response.has_focus() && ui.input(|input| input.key_pressed(egui::Key::Escape));
        let ctrl_enter = response.has_focus()
            && ui.input(|input| input.modifiers.ctrl && input.key_pressed(egui::Key::Enter));
        if escape {
            session.cancel_inspector_text_edit(content_target);
            response.surrender_focus();
        } else if ctrl_enter || response.lost_focus() {
            session.commit_inspector_text_edit(content_target);
        }
    } else {
        ui.label(if text.text.is_empty() {
            "(empty)"
        } else {
            text.text.as_str()
        });
        if ui.button("Edit content").clicked() {
            session.begin_inspector_text_edit(content_target, text.text.clone());
        }
    }

    let family_target = InspectorTextTarget {
        object_id,
        field: InspectorTextField::FontFamily,
    };
    ui.horizontal(|ui| {
        ui.label("Font");
        if let Some(mut buffer) = session
            .inspector_text_edit_buffer(family_target)
            .map(str::to_owned)
        {
            let response = ui.add_sized([150.0, 24.0], egui::TextEdit::singleline(&mut buffer));
            session.update_inspector_text_edit_buffer(family_target, buffer);
            let escape =
                response.has_focus() && ui.input(|input| input.key_pressed(egui::Key::Escape));
            let enter =
                response.has_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter));
            if escape {
                session.cancel_inspector_text_edit(family_target);
                response.surrender_focus();
            } else if enter || response.lost_focus() {
                session.commit_inspector_text_edit(family_target);
            }
        } else if ui.button(&text.font.family).clicked() {
            session.begin_inspector_text_edit(family_target, text.font.family.clone());
        }
    });

    ui.horizontal(|ui| {
        ui.label("Weight");
        let mut weight = text.font.weight;
        egui::ComboBox::from_id_salt(("text_weight", object_id.get()))
            .selected_text(font_weight_label(weight))
            .show_ui(ui, |ui| {
                for candidate in FONT_WEIGHTS {
                    ui.selectable_value(&mut weight, candidate, font_weight_label(candidate));
                }
            });
        if weight != text.font.weight {
            session.queue_text_font_weight(object_id, weight);
        }
    });

    ui.horizontal(|ui| {
        ui.label("Style");
        let mut style = text.font.style;
        egui::ComboBox::from_id_salt(("text_style", object_id.get()))
            .selected_text(font_style_label(style))
            .show_ui(ui, |ui| {
                for candidate in [FontStyle::Normal, FontStyle::Italic] {
                    ui.selectable_value(&mut style, candidate, font_style_label(candidate));
                }
            });
        if style != text.font.style {
            session.queue_text_font_style(object_id, style);
        }
    });

    let size_target = InspectorTextTarget {
        object_id,
        field: InspectorTextField::FontSize,
    };
    ui.horizontal(|ui| {
        ui.label("Size");
        if let Some(mut buffer) = session
            .inspector_text_edit_buffer(size_target)
            .map(str::to_owned)
        {
            let response = ui.add_sized(
                [80.0, 24.0],
                egui::TextEdit::singleline(&mut buffer).horizontal_align(egui::Align::RIGHT),
            );
            session.update_inspector_text_edit_buffer(size_target, buffer);
            let escape =
                response.has_focus() && ui.input(|input| input.key_pressed(egui::Key::Escape));
            let enter =
                response.has_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter));
            if escape {
                session.cancel_inspector_text_edit(size_target);
                response.surrender_focus();
            } else if enter || response.lost_focus() {
                session.commit_inspector_text_edit(size_target);
            }
        } else if ui.button(format!("{:.1} px", text.font_size)).clicked() {
            session.begin_inspector_text_edit(size_target, format!("{:.1}", text.font_size));
        }
    });

    ui.horizontal(|ui| {
        ui.label("Alignment");
        let mut alignment = text.alignment;
        egui::ComboBox::from_id_salt(("text_alignment", object_id.get()))
            .selected_text(text_alignment_label(alignment))
            .show_ui(ui, |ui| {
                for candidate in [
                    TextAlignment::Left,
                    TextAlignment::Center,
                    TextAlignment::Right,
                ] {
                    ui.selectable_value(&mut alignment, candidate, text_alignment_label(candidate));
                }
            });
        if alignment != text.alignment {
            session.queue_text_alignment(object_id, alignment);
        }
    });

    draw_animatable_property_row(
        ui,
        session,
        project,
        object_id,
        AnimatableProperty::TextColor,
        "Color",
    );
}

fn draw_image_inspector(
    ui: &mut egui::Ui,
    session: &mut EditorSession,
    project: &Project,
    asset_id: AssetId,
) {
    ui.add_space(8.0);
    ui.separator();
    ui.heading("Image");
    ui.label(format!("AssetId {}", asset_id.get()));

    if let Some(asset) = project.assets.iter().find(|asset| asset.id == asset_id) {
        match &asset.source {
            AssetSource::File {
                path,
                relative_to_project,
            } => {
                ui.label("Source");
                ui.monospace(path);
                ui.small(if *relative_to_project {
                    "Project-relative path"
                } else {
                    "External absolute path"
                });
            }
        }
    } else {
        ui.label("Source: missing AssetRecord");
    }

    ui.label("Intrinsic dimensions: unavailable until image decode");
    if session.image_relink_requested(asset_id) {
        ui.add_enabled(false, egui::Button::new("Relink queued"))
            .on_hover_text("Waiting for native file-dialog / asset relink backend");
    } else if ui
        .button("Relink")
        .on_hover_text("Choose a replacement source while keeping this AssetId stable")
        .clicked()
    {
        session.request_image_relink(asset_id);
    }
}

fn rotation_handle_points(corners: [egui::Pos2; 4]) -> (egui::Pos2, egui::Pos2) {
    let center = egui::pos2(
        corners.iter().map(|point| point.x).sum::<f32>() * 0.25,
        corners.iter().map(|point| point.y).sum::<f32>() * 0.25,
    );
    let top_midpoint = egui::pos2(
        (corners[0].x + corners[1].x) * 0.5,
        (corners[0].y + corners[1].y) * 0.5,
    );
    let outward = top_midpoint - center;
    let direction = if outward.length_sq() > f32::EPSILON {
        outward / outward.length()
    } else {
        egui::vec2(0.0, -1.0)
    };

    (top_midpoint, top_midpoint + direction * 28.0)
}

fn fit_composition_preview(available: egui::Vec2, composition: egui::Vec2) -> egui::Vec2 {
    if available.x <= 0.0 || available.y <= 0.0 || composition.x <= 0.0 || composition.y <= 0.0 {
        return egui::Vec2::ZERO;
    }

    let composition_aspect = composition.x / composition.y;
    let available_aspect = available.x / available.y;
    if available_aspect > composition_aspect {
        egui::vec2(available.y * composition_aspect, available.y)
    } else {
        egui::vec2(available.x, available.x / composition_aspect)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InspectorKeyframeState {
    Static,
    AnimatedOffKey,
    KeyAtGrid,
}

fn inspector_property_value_and_state(
    session: &EditorSession,
    project: &Project,
    object_id: ObjectId,
    property: AnimatableProperty,
) -> Option<(PropertyValue, InspectorKeyframeState)> {
    let keyframe_count = property_keyframe_count(project, object_id, property).ok()?;
    if keyframe_count == 0 {
        return Some((
            property_base_value(project, object_id, property).ok()?,
            InspectorKeyframeState::Static,
        ));
    }

    let continuous_tick = project
        .tempo_map
        .continuous_tick_position(session.playhead())
        .ok()?;
    let value = evaluate_property_at_tick(project, object_id, property, continuous_tick).ok()?;
    let state = snap_tick_position_to_grid(continuous_tick, session.authoring_division())
        .ok()
        .and_then(|tick| property_keyframe_at_tick(project, object_id, property, tick).ok())
        .flatten()
        .map_or(InspectorKeyframeState::AnimatedOffKey, |_| {
            InspectorKeyframeState::KeyAtGrid
        });

    Some((value, state))
}

fn draw_inspector_numeric_field(
    ui: &mut egui::Ui,
    session: &mut EditorSession,
    target: InspectorNumericTarget,
    value: f32,
    display_scale: f32,
    suffix: &str,
) {
    let display_value = value * display_scale;
    let editing_buffer = session
        .inspector_numeric_edit_buffer(target)
        .map(str::to_owned);

    if let Some(mut buffer) = editing_buffer {
        let response = ui.add_sized(
            [66.0, 24.0],
            egui::TextEdit::singleline(&mut buffer).horizontal_align(egui::Align::RIGHT),
        );
        session.update_inspector_numeric_edit_buffer(target, buffer);

        let escape = response.has_focus() && ui.input(|input| input.key_pressed(egui::Key::Escape));
        let enter = response.has_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter));
        if escape {
            session.cancel_inspector_numeric_edit(target);
            response.surrender_focus();
        } else if enter || response.lost_focus() {
            session.commit_inspector_numeric_edit(target);
        }
    } else {
        let label = if suffix.is_empty() {
            format!("{display_value:.1}")
        } else {
            format!("{display_value:.1}{suffix}")
        };
        if ui
            .add_sized([66.0, 24.0], egui::Button::new(label))
            .clicked()
        {
            session.begin_inspector_numeric_edit(target, value, format!("{display_value:.1}"));
        }
    }
}

fn draw_inspector_color_channel_field(
    ui: &mut egui::Ui,
    session: &mut EditorSession,
    target: InspectorNumericTarget,
    channel_label: &str,
    value: f32,
) {
    let display_value = value * 100.0;
    let editing_buffer = session
        .inspector_numeric_edit_buffer(target)
        .map(str::to_owned);

    if let Some(mut buffer) = editing_buffer {
        let response = ui.add_sized(
            [40.0, 24.0],
            egui::TextEdit::singleline(&mut buffer).horizontal_align(egui::Align::RIGHT),
        );
        session.update_inspector_numeric_edit_buffer(target, buffer);

        let escape = response.has_focus() && ui.input(|input| input.key_pressed(egui::Key::Escape));
        let enter = response.has_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter));
        if escape {
            session.cancel_inspector_numeric_edit(target);
            response.surrender_focus();
        } else if enter || response.lost_focus() {
            session.commit_inspector_numeric_edit(target);
        }
    } else if ui
        .add_sized(
            [40.0, 24.0],
            egui::Button::new(format!("{channel_label}{display_value:.0}")),
        )
        .on_hover_text(format!("{channel_label} channel: {display_value:.1}%"))
        .clicked()
    {
        session.begin_inspector_numeric_edit(target, value, format!("{display_value:.1}"));
    }
}

fn draw_animatable_property_row(
    ui: &mut egui::Ui,
    session: &mut EditorSession,
    project: &Project,
    object_id: ObjectId,
    property: AnimatableProperty,
    label: &str,
) {
    let Some((value, keyframe_state)) =
        inspector_property_value_and_state(session, project, object_id, property)
    else {
        ui.horizontal(|ui| {
            ui.label(label);
            ui.label("—");
        });
        return;
    };

    let (key_label, key_tooltip) = match keyframe_state {
        InspectorKeyframeState::Static => ("K+", "Static — add first keyframe"),
        InspectorKeyframeState::AnimatedOffKey => ("K", "Animated — no key at current grid"),
        InspectorKeyframeState::KeyAtGrid => ("K*", "Keyframe at current grid — remove keyframe"),
    };

    ui.horizontal(|ui| {
        ui.add_sized([72.0, 24.0], egui::Label::new(label));
        match value {
            PropertyValue::Vec2(value) => {
                let display_scale = match property {
                    AnimatableProperty::Scale | AnimatableProperty::Anchor => 100.0,
                    _ => 1.0,
                };
                let suffix = match property {
                    AnimatableProperty::Position
                    | AnimatableProperty::RectangleSize
                    | AnimatableProperty::EllipseSize => " px",
                    AnimatableProperty::Scale | AnimatableProperty::Anchor => "%",
                    _ => "",
                };
                draw_inspector_numeric_field(
                    ui,
                    session,
                    InspectorNumericTarget {
                        object_id,
                        property,
                        component: InspectorNumericComponent::X,
                    },
                    value.x(),
                    display_scale,
                    suffix,
                );
                draw_inspector_numeric_field(
                    ui,
                    session,
                    InspectorNumericTarget {
                        object_id,
                        property,
                        component: InspectorNumericComponent::Y,
                    },
                    value.y(),
                    display_scale,
                    suffix,
                );
            }
            PropertyValue::Scalar(value) => {
                let (display_scale, suffix) = match property {
                    AnimatableProperty::Opacity => (100.0, "%"),
                    AnimatableProperty::Rotation => (1.0, "°"),
                    AnimatableProperty::RectangleCornerRadius => (1.0, " px"),
                    _ => (1.0, ""),
                };
                draw_inspector_numeric_field(
                    ui,
                    session,
                    InspectorNumericTarget {
                        object_id,
                        property,
                        component: InspectorNumericComponent::Scalar,
                    },
                    value,
                    display_scale,
                    suffix,
                );
            }
            PropertyValue::Color(value) => {
                for (component, channel_label, channel_value) in [
                    (InspectorNumericComponent::R, "R", value.r()),
                    (InspectorNumericComponent::G, "G", value.g()),
                    (InspectorNumericComponent::B, "B", value.b()),
                    (InspectorNumericComponent::A, "A", value.a()),
                ] {
                    draw_inspector_color_channel_field(
                        ui,
                        session,
                        InspectorNumericTarget {
                            object_id,
                            property,
                            component,
                        },
                        channel_label,
                        channel_value,
                    );
                }
            }
        }
        if ui.button(key_label).on_hover_text(key_tooltip).clicked() {
            session.request_focused_keyframe_action(object_id, property);
        }
    });
}

fn inspector_common_numeric_component(
    values: &[(PropertyValue, InspectorKeyframeState)],
    component: InspectorNumericComponent,
) -> Option<Option<f32>> {
    let mut components = values.iter().map(|(value, _)| match (value, component) {
        (PropertyValue::Scalar(value), InspectorNumericComponent::Scalar) => Some(*value),
        (PropertyValue::Vec2(value), InspectorNumericComponent::X) => Some(value.x()),
        (PropertyValue::Vec2(value), InspectorNumericComponent::Y) => Some(value.y()),
        (PropertyValue::Color(value), InspectorNumericComponent::R) => Some(value.r()),
        (PropertyValue::Color(value), InspectorNumericComponent::G) => Some(value.g()),
        (PropertyValue::Color(value), InspectorNumericComponent::B) => Some(value.b()),
        (PropertyValue::Color(value), InspectorNumericComponent::A) => Some(value.a()),
        _ => None,
    });
    let first = components.next()??;
    let mut common = true;
    for value in components {
        if value? != first {
            common = false;
        }
    }
    Some(common.then_some(first))
}

fn draw_inspector_multi_numeric_field(
    ui: &mut egui::Ui,
    session: &mut EditorSession,
    target: InspectorMultiNumericTarget,
    object_ids: &[ObjectId],
    value: Option<f32>,
    display_scale: f32,
    suffix: &str,
) {
    let editing_buffer = session
        .inspector_multi_numeric_edit_buffer(target, object_ids)
        .map(str::to_owned);

    if let Some(mut buffer) = editing_buffer {
        let response = ui.add_sized(
            [66.0, 24.0],
            egui::TextEdit::singleline(&mut buffer).horizontal_align(egui::Align::RIGHT),
        );
        session.update_inspector_multi_numeric_edit_buffer(target, buffer);

        let escape = response.has_focus() && ui.input(|input| input.key_pressed(egui::Key::Escape));
        let enter = response.has_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter));
        if escape {
            session.cancel_inspector_multi_numeric_edit(target);
            response.surrender_focus();
        } else if enter || response.lost_focus() {
            session.commit_inspector_multi_numeric_edit(target);
        }
    } else {
        let label = value.map_or_else(
            || "Mixed".to_owned(),
            |value| {
                let display_value = value * display_scale;
                if suffix.is_empty() {
                    format!("{display_value:.1}")
                } else {
                    format!("{display_value:.1}{suffix}")
                }
            },
        );
        if ui
            .add_sized([66.0, 24.0], egui::Button::new(label))
            .clicked()
        {
            let buffer = value
                .map(|value| format!("{:.1}", value * display_scale))
                .unwrap_or_default();
            session.begin_inspector_multi_numeric_edit(target, object_ids.to_vec(), buffer);
        }
    }
}

fn draw_inspector_multi_color_channel_field(
    ui: &mut egui::Ui,
    session: &mut EditorSession,
    target: InspectorMultiNumericTarget,
    object_ids: &[ObjectId],
    channel_label: &str,
    value: Option<f32>,
) {
    let editing_buffer = session
        .inspector_multi_numeric_edit_buffer(target, object_ids)
        .map(str::to_owned);

    if let Some(mut buffer) = editing_buffer {
        let response = ui.add_sized(
            [40.0, 24.0],
            egui::TextEdit::singleline(&mut buffer).horizontal_align(egui::Align::RIGHT),
        );
        session.update_inspector_multi_numeric_edit_buffer(target, buffer);

        let escape = response.has_focus() && ui.input(|input| input.key_pressed(egui::Key::Escape));
        let enter = response.has_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter));
        if escape {
            session.cancel_inspector_multi_numeric_edit(target);
            response.surrender_focus();
        } else if enter || response.lost_focus() {
            session.commit_inspector_multi_numeric_edit(target);
        }
    } else {
        let label = value.map_or_else(
            || format!("{channel_label}Mix"),
            |value| format!("{channel_label}{:.0}", value * 100.0),
        );
        if ui
            .add_sized([40.0, 24.0], egui::Button::new(label))
            .clicked()
        {
            let buffer = value
                .map(|value| format!("{:.1}", value * 100.0))
                .unwrap_or_default();
            session.begin_inspector_multi_numeric_edit(target, object_ids.to_vec(), buffer);
        }
    }
}

fn draw_multi_animatable_property_row(
    ui: &mut egui::Ui,
    session: &mut EditorSession,
    project: &Project,
    object_ids: &[ObjectId],
    property: AnimatableProperty,
    label: &str,
) {
    let values: Option<Vec<_>> = object_ids
        .iter()
        .map(|object_id| inspector_property_value_and_state(session, project, *object_id, property))
        .collect();
    let Some(values) = values else {
        ui.horizontal(|ui| {
            ui.label(label);
            ui.label("—");
        });
        return;
    };
    let Some((first_value, first_state)) = values.first().copied() else {
        return;
    };

    let common_state = values
        .iter()
        .all(|(_, state)| *state == first_state)
        .then_some(first_state);
    let (key_label, key_tooltip) = match common_state {
        Some(InspectorKeyframeState::Static) => ("K+", "All selected properties are static"),
        Some(InspectorKeyframeState::AnimatedOffKey) => {
            ("K", "All selected properties are animated off-key")
        }
        Some(InspectorKeyframeState::KeyAtGrid) => (
            "K*",
            "All selected properties have a key at the current grid",
        ),
        None => ("K±", "Selected properties have mixed animation states"),
    };

    ui.horizontal(|ui| {
        ui.add_sized([72.0, 24.0], egui::Label::new(label));
        match first_value {
            PropertyValue::Vec2(_) => {
                let display_scale = match property {
                    AnimatableProperty::Scale | AnimatableProperty::Anchor => 100.0,
                    _ => 1.0,
                };
                let suffix = match property {
                    AnimatableProperty::Position
                    | AnimatableProperty::RectangleSize
                    | AnimatableProperty::EllipseSize => " px",
                    AnimatableProperty::Scale | AnimatableProperty::Anchor => "%",
                    _ => "",
                };
                for component in [InspectorNumericComponent::X, InspectorNumericComponent::Y] {
                    let value = inspector_common_numeric_component(&values, component).flatten();
                    draw_inspector_multi_numeric_field(
                        ui,
                        session,
                        InspectorMultiNumericTarget {
                            property,
                            component,
                        },
                        object_ids,
                        value,
                        display_scale,
                        suffix,
                    );
                }
            }
            PropertyValue::Scalar(_) => {
                let (display_scale, suffix) = match property {
                    AnimatableProperty::Opacity => (100.0, "%"),
                    AnimatableProperty::Rotation => (1.0, "°"),
                    AnimatableProperty::RectangleCornerRadius => (1.0, " px"),
                    _ => (1.0, ""),
                };
                let component = InspectorNumericComponent::Scalar;
                let value = inspector_common_numeric_component(&values, component).flatten();
                draw_inspector_multi_numeric_field(
                    ui,
                    session,
                    InspectorMultiNumericTarget {
                        property,
                        component,
                    },
                    object_ids,
                    value,
                    display_scale,
                    suffix,
                );
            }
            PropertyValue::Color(_) => {
                for (component, channel_label) in [
                    (InspectorNumericComponent::R, "R"),
                    (InspectorNumericComponent::G, "G"),
                    (InspectorNumericComponent::B, "B"),
                    (InspectorNumericComponent::A, "A"),
                ] {
                    let value = inspector_common_numeric_component(&values, component).flatten();
                    draw_inspector_multi_color_channel_field(
                        ui,
                        session,
                        InspectorMultiNumericTarget {
                            property,
                            component,
                        },
                        object_ids,
                        channel_label,
                        value,
                    );
                }
            }
        }
        ui.add_enabled(false, egui::Button::new(key_label))
            .on_hover_text(key_tooltip);
    });
}

#[derive(Debug, Clone)]
pub struct DiagnosticsView {
    pub adapter_name: String,
    pub backend: String,
    pub window_size: [u32; 2],
    pub frame_time_ms: f32,
}

pub fn configure_theme(context: &egui::Context) {
    context.set_theme(egui::Theme::Dark);
    context.set_visuals(egui::Visuals::dark());

    context.global_style_mut(|style| {
        style.text_styles.insert(
            egui::TextStyle::Body,
            egui::FontId::new(15.0, egui::FontFamily::Proportional),
        );
        style.text_styles.insert(
            egui::TextStyle::Button,
            egui::FontId::new(15.0, egui::FontFamily::Proportional),
        );
        style.text_styles.insert(
            egui::TextStyle::Heading,
            egui::FontId::new(16.0, egui::FontFamily::Proportional),
        );
        style.text_styles.insert(
            egui::TextStyle::Small,
            egui::FontId::new(13.0, egui::FontFamily::Proportional),
        );
        style.spacing.item_spacing = egui::vec2(8.0, 8.0);
        style.spacing.interact_size = egui::vec2(32.0, 34.0);
        style.spacing.button_padding = egui::vec2(12.0, 8.0);
    });
}

pub fn draw_editor_shell(
    ui: &mut egui::Ui,
    session: &mut EditorSession,
    project: &Project,
    diagnostics: &DiagnosticsView,
    composition_texture_id: Option<egui::TextureId>,
    waveform: Option<&rhythm_engine::waveform::WaveformData>,
) {
    egui::Panel::top("transport_rhythm")
        .exact_size(54.0)
        .show(ui, |ui| {
            ui.horizontal_centered(|ui| {
                ui.heading("Rhythm Effects");
                ui.separator();
                ui.label("Transport");
                if ui.button("Start").clicked() {
                    session.seek_paused(ProjectTimeNs::new(0));
                }
                let playhead_seconds = session.playhead().get() as f64 / 1_000_000_000.0;
                ui.label(format!("Playhead {playhead_seconds:.3}s"));
                ui.separator();
                if let Some(segment) = project.tempo_map.initial_segment() {
                    ui.label(format!("BPM {}", segment.bpm().format_decimal()));
                    ui.label(format!(
                        "{}/{}",
                        segment.meter().numerator(),
                        segment.meter().denominator()
                    ));
                } else {
                    ui.label("BPM —");
                    ui.label("4/4");
                }
                ui.label(format!(
                    "Grid 1/{}",
                    session.authoring_division().parts_per_beat()
                ));
                let mut follow_playhead = session.follow_playhead();
                if ui
                    .toggle_value(&mut follow_playhead, "Follow Playhead")
                    .changed()
                {
                    session.set_follow_playhead(follow_playhead);
                }
                ui.separator();
                egui::ComboBox::from_id_salt("preview_quality")
                    .selected_text(session.preview_quality.label())
                    .show_ui(ui, |ui| {
                        for quality in PreviewQuality::ALL {
                            ui.selectable_value(
                                &mut session.preview_quality,
                                quality,
                                quality.label(),
                            );
                        }
                    });
            });
        });

    egui::Panel::bottom("timeline")
        .resizable(true)
        .default_size(300.0)
        .show(ui, |ui| {
            ui.heading("Timeline");
            ui.separator();
            draw_timeline(ui, session, project, waveform);
            ui.take_available_space();
        });

    egui::Panel::left("objects")
        .resizable(true)
        .default_size(240.0)
        .show(ui, |ui| {
            ui.heading("Objects");
            ui.separator();

            if project.composition.objects.is_empty() {
                ui.label("No objects");
            } else {
                egui::ScrollArea::vertical()
                    .id_salt("object_list_scroll")
                    .show(ui, |ui| {
                        for object in project.composition.objects.iter().rev() {
                            ui.push_id(object.id.get(), |ui| {
                                ui.horizontal(|ui| {
                                    let mut visible = object.visible;
                                    if ui
                                        .toggle_value(&mut visible, "V")
                                        .on_hover_text(if visible { "Visible" } else { "Hidden" })
                                        .changed()
                                    {
                                        session.queue_object_visibility(object.id, visible);
                                    }

                                    let mut locked = object.locked;
                                    if ui
                                        .toggle_value(&mut locked, "L")
                                        .on_hover_text(if locked { "Locked" } else { "Unlocked" })
                                        .changed()
                                    {
                                        session.queue_object_locked(object.id, locked);
                                    }

                                    let selected = session.is_object_selected(object.id);
                                    let response = ui.selectable_label(selected, &object.name);
                                    if response.clicked() {
                                        let ctrl = ui.input(|input| input.modifiers.ctrl);
                                        if ctrl {
                                            session.toggle_object_selection(object.id);
                                        } else {
                                            session.replace_object_selection(Some(object.id));
                                        }
                                    }
                                });
                            });
                        }
                    });
            }

            ui.take_available_space();
        });

    egui::Panel::right("inspector")
        .resizable(true)
        .default_size(320.0)
        .show(ui, |ui| {
            ui.heading("Inspector");
            ui.separator();

            let mut selected = session.selected_object_ids();
            selected.sort_by_key(|object_id| object_id.get());
            match selected.as_slice() {
                [] => {
                    ui.label("No object selected");
                    ui.label("Select an object in the viewport or Object list.");
                }
                [object_id] => {
                    if let Some(object) = project
                        .composition
                        .objects
                        .iter()
                        .find(|object| object.id == *object_id)
                    {
                        ui.heading(&object.name);
                        let visibility = if object.visible { "Visible" } else { "Hidden" };
                        let lock = if object.locked { "Locked" } else { "Unlocked" };
                        ui.label(format!("{visibility} · {lock}"));
                        ui.add_space(8.0);
                        ui.separator();
                        ui.heading("Transform");
                        draw_animatable_property_row(
                            ui,
                            session,
                            project,
                            object.id,
                            AnimatableProperty::Position,
                            "Position",
                        );
                        draw_animatable_property_row(
                            ui,
                            session,
                            project,
                            object.id,
                            AnimatableProperty::Scale,
                            "Scale",
                        );
                        draw_animatable_property_row(
                            ui,
                            session,
                            project,
                            object.id,
                            AnimatableProperty::Rotation,
                            "Rotation",
                        );
                        draw_animatable_property_row(
                            ui,
                            session,
                            project,
                            object.id,
                            AnimatableProperty::Anchor,
                            "Anchor",
                        );
                        draw_animatable_property_row(
                            ui,
                            session,
                            project,
                            object.id,
                            AnimatableProperty::Opacity,
                            "Opacity",
                        );

                        match &object.content {
                            ObjectContent::Rectangle(_) => {
                                ui.add_space(8.0);
                                ui.separator();
                                ui.heading("Rectangle");
                                draw_animatable_property_row(
                                    ui,
                                    session,
                                    project,
                                    object.id,
                                    AnimatableProperty::RectangleSize,
                                    "Size",
                                );
                                draw_animatable_property_row(
                                    ui,
                                    session,
                                    project,
                                    object.id,
                                    AnimatableProperty::RectangleFill,
                                    "Fill",
                                );
                                draw_animatable_property_row(
                                    ui,
                                    session,
                                    project,
                                    object.id,
                                    AnimatableProperty::RectangleCornerRadius,
                                    "Corner Radius",
                                );
                            }
                            ObjectContent::Ellipse(_) => {
                                ui.add_space(8.0);
                                ui.separator();
                                ui.heading("Ellipse");
                                draw_animatable_property_row(
                                    ui,
                                    session,
                                    project,
                                    object.id,
                                    AnimatableProperty::EllipseSize,
                                    "Size",
                                );
                                draw_animatable_property_row(
                                    ui,
                                    session,
                                    project,
                                    object.id,
                                    AnimatableProperty::EllipseFill,
                                    "Fill",
                                );
                            }
                            ObjectContent::Image(image) => {
                                draw_image_inspector(ui, session, project, image.asset);
                            }
                            ObjectContent::Text(text) => {
                                draw_text_inspector(ui, session, object.id, text, project);
                            }
                        }
                    } else {
                        ui.label("Selected object is unavailable");
                    }
                }
                _ => {
                    ui.heading(format!("{} objects selected", selected.len()));
                    ui.add_space(8.0);
                    ui.separator();
                    ui.heading("Transform");
                    draw_multi_animatable_property_row(
                        ui,
                        session,
                        project,
                        &selected,
                        AnimatableProperty::Position,
                        "Position",
                    );
                    draw_multi_animatable_property_row(
                        ui,
                        session,
                        project,
                        &selected,
                        AnimatableProperty::Scale,
                        "Scale",
                    );
                    draw_multi_animatable_property_row(
                        ui,
                        session,
                        project,
                        &selected,
                        AnimatableProperty::Rotation,
                        "Rotation",
                    );
                    draw_multi_animatable_property_row(
                        ui,
                        session,
                        project,
                        &selected,
                        AnimatableProperty::Anchor,
                        "Anchor",
                    );
                    draw_multi_animatable_property_row(
                        ui,
                        session,
                        project,
                        &selected,
                        AnimatableProperty::Opacity,
                        "Opacity",
                    );

                    let all_rectangles = selected.iter().all(|object_id| {
                        project
                            .composition
                            .objects
                            .iter()
                            .find(|object| object.id == *object_id)
                            .is_some_and(|object| {
                                matches!(&object.content, ObjectContent::Rectangle(_))
                            })
                    });
                    if all_rectangles {
                        ui.add_space(8.0);
                        ui.separator();
                        ui.heading("Rectangle");
                        draw_multi_animatable_property_row(
                            ui,
                            session,
                            project,
                            &selected,
                            AnimatableProperty::RectangleSize,
                            "Size",
                        );
                        draw_multi_animatable_property_row(
                            ui,
                            session,
                            project,
                            &selected,
                            AnimatableProperty::RectangleFill,
                            "Fill",
                        );
                        draw_multi_animatable_property_row(
                            ui,
                            session,
                            project,
                            &selected,
                            AnimatableProperty::RectangleCornerRadius,
                            "Corner Radius",
                        );
                    }

                    let all_images = selected.iter().all(|object_id| {
                        project
                            .composition
                            .objects
                            .iter()
                            .find(|object| object.id == *object_id)
                            .is_some_and(|object| matches!(&object.content, ObjectContent::Image(_)))
                    });
                    if all_images {
                        ui.add_space(8.0);
                        ui.separator();
                        ui.heading("Image");
                        ui.label("Multiple image sources selected");
                        ui.label("Intrinsic dimensions are shown per image when runtime decode is available.");
                        ui.add_enabled(false, egui::Button::new("Relink"))
                            .on_hover_text("Relink is a single-asset action");
                    }

                    let all_text = selected.iter().all(|object_id| {
                        project
                            .composition
                            .objects
                            .iter()
                            .find(|object| object.id == *object_id)
                            .is_some_and(|object| matches!(&object.content, ObjectContent::Text(_)))
                    });
                    if all_text {
                        ui.add_space(8.0);
                        ui.separator();
                        ui.heading("Text");
                        ui.label("Content and font controls require single selection.");
                        draw_multi_animatable_property_row(
                            ui,
                            session,
                            project,
                            &selected,
                            AnimatableProperty::TextColor,
                            "Color",
                        );
                    }

                    let all_ellipses = selected.iter().all(|object_id| {
                        project
                            .composition
                            .objects
                            .iter()
                            .find(|object| object.id == *object_id)
                            .is_some_and(|object| {
                                matches!(&object.content, ObjectContent::Ellipse(_))
                            })
                    });
                    if all_ellipses {
                        ui.add_space(8.0);
                        ui.separator();
                        ui.heading("Ellipse");
                        draw_multi_animatable_property_row(
                            ui,
                            session,
                            project,
                            &selected,
                            AnimatableProperty::EllipseSize,
                            "Size",
                        );
                        draw_multi_animatable_property_row(
                            ui,
                            session,
                            project,
                            &selected,
                            AnimatableProperty::EllipseFill,
                            "Fill",
                        );
                    }
                }
            }

            ui.add_space(16.0);
            ui.separator();
            ui.heading("Diagnostics");
            ui.label(format!("GPU: {}", diagnostics.adapter_name));
            ui.label(format!("Backend: {}", diagnostics.backend));
            ui.label(format!(
                "Window: {} × {}",
                diagnostics.window_size[0], diagnostics.window_size[1]
            ));
            ui.label(format!("Frame: {:.2} ms", diagnostics.frame_time_ms));
            ui.take_available_space();
        });

    egui::CentralPanel::default().show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.heading("Viewport");
            ui.separator();
            if ui
                .button("Fit Composition")
                .on_hover_text("Shift+F")
                .clicked()
            {
                session.request_viewport_camera_action(ViewportCameraAction::FitComposition);
            }
            if ui.button("Frame Selection").on_hover_text("F").clicked() {
                session.request_viewport_camera_action(ViewportCameraAction::FrameSelection);
            }
        });
        ui.separator();

        let viewport_rect = ui.available_rect_before_wrap();
        let composition_size = egui::vec2(
            project.settings.composition_width as f32,
            project.settings.composition_height as f32,
        );
        let fitted_preview_size = fit_composition_preview(viewport_rect.size(), composition_size);

        if let Some(action) = session.take_viewport_camera_action() {
            match action {
                ViewportCameraAction::FitComposition => {
                    session.fit_viewport_composition();
                }
                ViewportCameraAction::FrameSelection => {
                    if let Ok(scene) =
                        rhythm_engine::scene_eval::evaluate_scene(project, session.playhead())
                    {
                        let selected = session.selected_object_ids();
                        if let Some(bounds) =
                            selected_objects_bounds(&scene, &selected, |_, _| None)
                        {
                            session.frame_viewport_bounds(
                                bounds,
                                [composition_size.x, composition_size.y],
                                [viewport_rect.width(), viewport_rect.height()],
                                [fitted_preview_size.x, fitted_preview_size.y],
                            );
                        }
                    }
                }
            }
        }
        let pan_response = ui.interact(
            viewport_rect,
            ui.id().with("viewport_middle_pan"),
            egui::Sense::drag(),
        );
        if pan_response.dragged_by(egui::PointerButton::Middle) {
            let delta = ui.input(|input| input.pointer.delta());
            session.pan_viewport_points([delta.x, delta.y]);
        }

        if let Some(pointer) = ui.input(|input| input.pointer.hover_pos())
            && viewport_rect.contains(pointer)
        {
            let scroll_y = ui.input(|input| input.smooth_scroll_delta().y);
            if scroll_y.abs() > f32::EPSILON {
                let zoom_factor = (scroll_y * 0.002).exp();
                let anchor = [
                    pointer.x - viewport_rect.center().x,
                    pointer.y - viewport_rect.center().y,
                ];
                session.zoom_viewport_around_anchor(zoom_factor, anchor);
            }
        }

        if let Some(texture_id) = composition_texture_id {
            let preview_size = fitted_preview_size * session.viewport_zoom();
            let pan = session.viewport_pan_points();
            let preview_rect = egui::Rect::from_center_size(
                viewport_rect.center() + egui::vec2(pan[0], pan[1]),
                preview_size,
            );
            let response = ui.put(
                preview_rect,
                egui::Image::from_texture(egui::load::SizedTexture::new(
                    texture_id,
                    egui::vec2(1920.0, 1080.0),
                ))
                .fit_to_exact_size(preview_size)
                .sense(egui::Sense::click()),
            );

            let screen_to_composition_unclamped = |point: egui::Pos2| -> Option<Vec2> {
                if response.rect.width() <= 0.0 || response.rect.height() <= 0.0 {
                    return None;
                }

                Vec2::new(
                    (point.x - response.rect.left()) / response.rect.width()
                        * project.settings.composition_width as f32,
                    (point.y - response.rect.top()) / response.rect.height()
                        * project.settings.composition_height as f32,
                )
                .ok()
            };
            let screen_to_composition = |point: egui::Pos2| -> Option<Vec2> {
                response
                    .rect
                    .contains(point)
                    .then(|| screen_to_composition_unclamped(point))
                    .flatten()
            };
            let composition_to_screen = |point: Vec2| {
                egui::pos2(
                    response.rect.left() + point.x() / composition_size.x * response.rect.width(),
                    response.rect.top() + point.y() / composition_size.y * response.rect.height(),
                )
            };

            if response.clicked()
                && !session.viewport_multi_position_drag_active()
                && let Some(pointer) = response.interact_pointer_pos()
                && let Some(composition_point) = screen_to_composition(pointer)
                && let Ok(scene) =
                    rhythm_engine::scene_eval::evaluate_scene(project, session.playhead())
            {
                let picked = pick_topmost_object(project, &scene, composition_point, |_, _| None);
                let ctrl = ui.input(|input| input.modifiers.ctrl);
                if ctrl {
                    if let Some(object_id) = picked {
                        session.toggle_object_selection(object_id);
                    }
                } else {
                    session.replace_object_selection(picked);
                }
            }

            let (primary_pressed, primary_down, primary_released, pointer_pos, press_origin) = ui
                .input(|input| {
                    (
                        input.pointer.primary_pressed(),
                        input.pointer.primary_down(),
                        input.pointer.primary_released(),
                        input.pointer.interact_pos(),
                        input.pointer.press_origin(),
                    )
                });

            if primary_pressed
                && let Some(origin) = press_origin
                && let Ok(scene) =
                    rhythm_engine::scene_eval::evaluate_scene(project, session.playhead())
            {
                let ctrl = ui.input(|input| input.modifiers.ctrl);
                let mut selected = session.selected_object_ids();
                selected.sort_by_key(|object_id| object_id.get());
                let overlay = if selected.len() == 1 {
                    selection_overlay_geometry(&scene, &selected, |_, _| None)
                } else {
                    None
                };
                const SCALE_HANDLE_HIT_RADIUS: f32 = 9.0;
                const ROTATION_HANDLE_HIT_RADIUS: f32 = 10.0;

                let rotation_drag_started = if !ctrl && selected.len() == 1 {
                    let object_id = selected[0];
                    project
                        .composition
                        .objects
                        .iter()
                        .find(|object| object.id == object_id)
                        .filter(|object| object.visible && !object.locked)
                        .and_then(|_| {
                            let overlay = overlay?;
                            let corners = overlay.corners.map(composition_to_screen);
                            let (_, handle) = rotation_handle_points(corners);
                            if handle.distance(origin) > ROTATION_HANDLE_HIT_RADIUS {
                                return None;
                            }
                            let composition_pointer = screen_to_composition_unclamped(origin)?;
                            let evaluated = scene
                                .objects
                                .iter()
                                .find(|evaluated| evaluated.id == object_id)?;
                            Some(session.begin_viewport_rotation_drag(
                                object_id,
                                overlay.anchor,
                                composition_pointer,
                                evaluated.transform.rotation_degrees,
                            ))
                        })
                        .unwrap_or(false)
                } else {
                    false
                };

                let scale_drag_started = if !rotation_drag_started && !ctrl && selected.len() == 1 {
                    let object_id = selected[0];
                    project
                        .composition
                        .objects
                        .iter()
                        .find(|object| object.id == object_id)
                        .filter(|object| object.visible && !object.locked)
                        .and_then(|_| {
                            let overlay = overlay?;
                            let handle_start = overlay.corners.iter().copied().find(|corner| {
                                composition_to_screen(*corner).distance(origin)
                                    <= SCALE_HANDLE_HIT_RADIUS
                            })?;
                            let evaluated = scene
                                .objects
                                .iter()
                                .find(|evaluated| evaluated.id == object_id)?;
                            Some(session.begin_viewport_scale_drag(
                                object_id,
                                overlay.anchor,
                                handle_start,
                                evaluated.transform.scale,
                                evaluated.transform.rotation_degrees,
                            ))
                        })
                        .unwrap_or(false)
                } else {
                    false
                };

                if !rotation_drag_started
                    && !scale_drag_started
                    && let Some(composition_origin) = screen_to_composition(origin)
                {
                    let picked =
                        pick_topmost_object(project, &scene, composition_origin, |_, _| None);
                    let multi_move_started = if !ctrl
                        && selected.len() > 1
                        && picked.is_some_and(|object_id| selected.contains(&object_id))
                        && selected.iter().all(|selected_id| {
                            project
                                .composition
                                .objects
                                .iter()
                                .find(|object| object.id == *selected_id)
                                .is_some_and(|object| object.visible && !object.locked)
                        }) {
                        session.begin_viewport_multi_position_drag(
                            selected.clone(),
                            [origin.x, origin.y],
                        )
                    } else {
                        false
                    };

                    if !multi_move_started
                        && !ctrl
                        && selected.len() == 1
                        && picked == selected.first().copied()
                        && let Some(object_id) = picked
                        && project
                            .composition
                            .objects
                            .iter()
                            .find(|object| object.id == object_id)
                            .is_some_and(|object| object.visible && !object.locked)
                        && let Some(evaluated) = scene
                            .objects
                            .iter()
                            .find(|evaluated| evaluated.id == object_id)
                    {
                        session.begin_viewport_position_drag(
                            object_id,
                            [origin.x, origin.y],
                            evaluated.transform.position,
                        );
                    } else if !multi_move_started && picked.is_none() {
                        session.begin_viewport_box_selection([origin.x, origin.y]);
                    }
                }
            }

            if primary_down
                && let Some(pointer) = pointer_pos
                && session.viewport_rotation_drag_active()
            {
                if let Some(composition_pointer) = screen_to_composition_unclamped(pointer) {
                    session.update_viewport_rotation_drag(
                        composition_pointer,
                        ui.input(|input| input.modifiers.shift),
                    );
                }
            } else if primary_down
                && let Some(pointer) = pointer_pos
                && session.viewport_scale_drag_active()
            {
                if let Some(composition_pointer) = screen_to_composition_unclamped(pointer) {
                    session.update_viewport_scale_drag(
                        composition_pointer,
                        ui.input(|input| input.modifiers.shift),
                    );
                }
            } else if primary_down
                && let Some(pointer) = pointer_pos
                && session.viewport_multi_position_drag_active()
            {
                session.update_viewport_multi_position_drag(
                    [pointer.x, pointer.y],
                    [
                        composition_size.x / response.rect.width(),
                        composition_size.y / response.rect.height(),
                    ],
                    ui.input(|input| input.modifiers.shift),
                );
            } else if primary_down
                && let Some(pointer) = pointer_pos
                && session.viewport_position_drag_active()
            {
                session.update_viewport_position_drag(
                    [pointer.x, pointer.y],
                    [
                        composition_size.x / response.rect.width(),
                        composition_size.y / response.rect.height(),
                    ],
                    ui.input(|input| input.modifiers.shift),
                );
            } else if primary_down
                && let Some(pointer) = pointer_pos
                && session.viewport_box_selection().is_some()
            {
                let clamped = egui::pos2(
                    pointer.x.clamp(response.rect.left(), response.rect.right()),
                    pointer.y.clamp(response.rect.top(), response.rect.bottom()),
                );
                session.update_viewport_box_selection([clamped.x, clamped.y]);
            }

            if let Some((start, current)) = session.viewport_box_selection() {
                let selection_rect = egui::Rect::from_two_pos(
                    egui::pos2(start[0], start[1]),
                    egui::pos2(current[0], current[1]),
                )
                .intersect(response.rect);
                if selection_rect.is_positive() {
                    ui.painter().rect_stroke(
                        selection_rect,
                        0.0,
                        egui::Stroke::new(1.0, ui.visuals().selection.stroke.color),
                        egui::StrokeKind::Inside,
                    );
                    ui.painter().rect_filled(
                        selection_rect,
                        0.0,
                        ui.visuals().selection.bg_fill.gamma_multiply(0.12),
                    );
                }
            }

            let rotation_drag_released =
                primary_released && session.viewport_rotation_drag_active();
            if rotation_drag_released {
                if let Some(pointer) = pointer_pos
                    && let Some(composition_pointer) = screen_to_composition_unclamped(pointer)
                {
                    session.update_viewport_rotation_drag(
                        composition_pointer,
                        ui.input(|input| input.modifiers.shift),
                    );
                }
                session.finish_viewport_rotation_drag();
            }

            let scale_drag_released =
                primary_released && !rotation_drag_released && session.viewport_scale_drag_active();
            if scale_drag_released {
                if let Some(pointer) = pointer_pos
                    && let Some(composition_pointer) = screen_to_composition_unclamped(pointer)
                {
                    session.update_viewport_scale_drag(
                        composition_pointer,
                        ui.input(|input| input.modifiers.shift),
                    );
                }
                session.finish_viewport_scale_drag();
            }

            let multi_position_drag_released = primary_released
                && !rotation_drag_released
                && !scale_drag_released
                && session.viewport_multi_position_drag_active();
            if multi_position_drag_released {
                if let Some(pointer) = pointer_pos {
                    session.update_viewport_multi_position_drag(
                        [pointer.x, pointer.y],
                        [
                            composition_size.x / response.rect.width(),
                            composition_size.y / response.rect.height(),
                        ],
                        ui.input(|input| input.modifiers.shift),
                    );
                }
                session.finish_viewport_multi_position_drag();
            }

            let position_drag_released = primary_released
                && !rotation_drag_released
                && !scale_drag_released
                && !multi_position_drag_released
                && session.viewport_position_drag_active();
            if position_drag_released {
                if let Some(pointer) = pointer_pos {
                    session.update_viewport_position_drag(
                        [pointer.x, pointer.y],
                        [
                            composition_size.x / response.rect.width(),
                            composition_size.y / response.rect.height(),
                        ],
                        ui.input(|input| input.modifiers.shift),
                    );
                }
                session.finish_viewport_position_drag();
            }

            if primary_released
                && !rotation_drag_released
                && !scale_drag_released
                && !multi_position_drag_released
                && !position_drag_released
                && let Some((start, current)) = session.take_viewport_box_selection()
            {
                let screen_rect = egui::Rect::from_two_pos(
                    egui::pos2(start[0], start[1]),
                    egui::pos2(current[0], current[1]),
                )
                .intersect(response.rect);
                if let (Some(min), Some(max)) = (
                    screen_to_composition(screen_rect.min),
                    screen_to_composition(screen_rect.max),
                ) && let Ok(scene) =
                    rhythm_engine::scene_eval::evaluate_scene(project, session.playhead())
                {
                    let selection_bounds = LocalBounds2d::new(
                        min,
                        Vec2::new(max.x() - min.x(), max.y() - min.y())
                            .expect("viewport selection bounds are finite"),
                    );
                    let selected =
                        objects_intersecting_box(project, &scene, selection_bounds, |_, _| None);
                    session.replace_object_selection_many(selected);
                }
            }

            if let Ok(scene) =
                rhythm_engine::scene_eval::evaluate_scene(project, session.playhead())
            {
                let selected = session.selected_object_ids();
                if let Some(overlay) = selection_overlay_geometry(&scene, &selected, |_, _| None) {
                    let corners = overlay.corners.map(composition_to_screen);
                    let anchor = composition_to_screen(overlay.anchor);
                    let stroke = egui::Stroke::new(1.5, ui.visuals().selection.stroke.color);

                    for (start, end) in [(0, 1), (1, 2), (2, 3), (3, 0)] {
                        ui.painter()
                            .line_segment([corners[start], corners[end]], stroke);
                    }

                    if selected.len() == 1 {
                        const SCALE_HANDLE_SIZE: f32 = 8.0;
                        for corner in corners {
                            ui.painter().rect_filled(
                                egui::Rect::from_center_size(
                                    corner,
                                    egui::vec2(SCALE_HANDLE_SIZE, SCALE_HANDLE_SIZE),
                                ),
                                1.0,
                                ui.visuals().window_fill(),
                            );
                            ui.painter().rect_stroke(
                                egui::Rect::from_center_size(
                                    corner,
                                    egui::vec2(SCALE_HANDLE_SIZE, SCALE_HANDLE_SIZE),
                                ),
                                1.0,
                                stroke,
                                egui::StrokeKind::Inside,
                            );
                        }

                        let (rotation_stem, rotation_handle) = rotation_handle_points(corners);
                        ui.painter()
                            .line_segment([rotation_stem, rotation_handle], stroke);
                        ui.painter().circle_filled(
                            rotation_handle,
                            5.0,
                            ui.visuals().window_fill(),
                        );
                        ui.painter().circle_stroke(rotation_handle, 5.0, stroke);
                    }

                    const ANCHOR_RADIUS: f32 = 5.0;
                    const ANCHOR_ARM: f32 = 7.0;
                    ui.painter().circle_stroke(anchor, ANCHOR_RADIUS, stroke);
                    ui.painter().line_segment(
                        [
                            egui::pos2(anchor.x - ANCHOR_ARM, anchor.y),
                            egui::pos2(anchor.x + ANCHOR_ARM, anchor.y),
                        ],
                        stroke,
                    );
                    ui.painter().line_segment(
                        [
                            egui::pos2(anchor.x, anchor.y - ANCHOR_ARM),
                            egui::pos2(anchor.x, anchor.y + ANCHOR_ARM),
                        ],
                        stroke,
                    );
                }
            }
        } else {
            ui.painter().text(
                viewport_rect.center(),
                egui::Align2::CENTER_CENTER,
                "Composition preview unavailable",
                egui::FontId::proportional(15.0),
                ui.visuals().text_color(),
            );
        }
    });
}

#[cfg(test)]
mod tests {
    use super::{fit_composition_preview, rotation_handle_points};

    #[test]
    fn rotation_handle_extends_outward_from_top_edge() {
        let (stem, handle) = rotation_handle_points([
            egui::pos2(0.0, 0.0),
            egui::pos2(100.0, 0.0),
            egui::pos2(100.0, 100.0),
            egui::pos2(0.0, 100.0),
        ]);

        assert_eq!(stem, egui::pos2(50.0, 0.0));
        assert!((handle.x - 50.0).abs() < 0.0001);
        assert!((handle.y + 28.0).abs() < 0.0001);
    }

    #[test]
    fn preview_fit_preserves_composition_aspect() {
        let composition = egui::vec2(1920.0, 1080.0);
        let wide = fit_composition_preview(egui::vec2(1000.0, 400.0), composition);
        let tall = fit_composition_preview(egui::vec2(400.0, 1000.0), composition);

        assert!((wide.x / wide.y - 16.0 / 9.0).abs() < 0.0001);
        assert!((tall.x / tall.y - 16.0 / 9.0).abs() < 0.0001);
        assert!(wide.x <= 1000.0 && wide.y <= 400.0);
        assert!(tall.x <= 400.0 && tall.y <= 1000.0);
    }
}
