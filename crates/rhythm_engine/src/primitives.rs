//! GPU-ready Rectangle/Ellipse vertices derived from the evaluated project scene.
//! No editor-only state enters the creative geometry or painter-order semantics.

use rhythm_core::domain::{LinearRgba, Vec2};

use crate::scene_eval::{EvaluatedObject, EvaluatedObjectContent, EvaluatedScene};

pub(crate) const VERTEX_STRIDE: u64 = 48;
pub(crate) const VERTEX_ATTRIBUTES: [wgpu::VertexAttribute; 6] = wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x2, 2 => Float32x2, 3 => Float32x4, 4 => Float32, 5 => Float32];

pub(crate) const PRIMITIVE_SHADER: &str = r#"
struct VertexInput {
    @location(0) ndc: vec2<f32>,
    @location(1) local: vec2<f32>,
    @location(2) size: vec2<f32>,
    @location(3) premultiplied_linear: vec4<f32>,
    @location(4) radius: f32,
    @location(5) kind: f32,
};

struct FragmentInput {
    @builtin(position) position: vec4<f32>,
    @location(0) local: vec2<f32>,
    @location(1) size: vec2<f32>,
    @location(2) premultiplied_linear: vec4<f32>,
    @location(3) radius: f32,
    @location(4) kind: f32,
};

@vertex
fn vs_main(input: VertexInput) -> FragmentInput {
    var output: FragmentInput;
    output.position = vec4<f32>(input.ndc, 0.0, 1.0);
    output.local = input.local;
    output.size = input.size;
    output.premultiplied_linear = input.premultiplied_linear;
    output.radius = input.radius;
    output.kind = input.kind;
    return output;
}

@fragment
fn fs_main(input: FragmentInput) -> @location(0) vec4<f32> {
    var signed_distance: f32;
    if input.kind < 0.5 {
        let half_size = input.size * 0.5;
        let q = abs(input.local - half_size) -
                max(half_size - vec2<f32>(input.radius), vec2<f32>(0.0));
        signed_distance = length(max(q, vec2<f32>(0.0))) +
                          min(max(q.x, q.y), 0.0) - input.radius;
    } else {
        let normalized = (input.local / input.size - vec2<f32>(0.5, 0.5)) * 2.0;
        signed_distance = length(normalized) - 1.0;
    }

    let footprint = max(fwidth(signed_distance), 0.00001);
    let coverage = clamp(0.5 - signed_distance / footprint, 0.0, 1.0);
    return input.premultiplied_linear * coverage;
}
"#;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct PrimitiveVertex {
    pub ndc: [f32; 2],
    pub local: [f32; 2],
    pub size: [f32; 2],
    pub color: [f32; 4],
    pub radius: f32,
    pub kind: f32,
}

impl PrimitiveVertex {
    pub fn write_bytes(self, bytes: &mut Vec<u8>) {
        for value in self
            .ndc
            .into_iter()
            .chain(self.local)
            .chain(self.size)
            .chain(self.color)
            .chain([self.radius, self.kind])
        {
            bytes.extend_from_slice(&value.to_ne_bytes());
        }
    }
}

/// Six vertices per primitive, preserving original scene painter order.
/// Effect-bearing objects deliberately stay unsupported until object isolation
/// is implemented; rendering their unmodified pixels would be misleading.
pub(crate) fn scene_vertices(
    scene: &EvaluatedScene,
    composition_size: [u32; 2],
) -> Vec<PrimitiveVertex> {
    if composition_size.contains(&0) {
        return Vec::new();
    }

    let mut vertices = Vec::new();
    for object in &scene.objects {
        if !object.effects.is_empty() {
            continue;
        }
        append_object(&mut vertices, object, composition_size);
    }
    vertices
}

fn append_object(
    vertices: &mut Vec<PrimitiveVertex>,
    object: &EvaluatedObject,
    dimensions: [u32; 2],
) {
    let (size, fill, radius, kind) = match &object.content {
        EvaluatedObjectContent::Rectangle {
            size,
            fill,
            corner_radius,
        } => (*size, *fill, *corner_radius, 0.0),
        EvaluatedObjectContent::Ellipse { size, fill } => (*size, *fill, 0.0, 1.0),
        EvaluatedObjectContent::Image { .. } | EvaluatedObjectContent::Text { .. } => return,
    };

    if size.x() <= 0.0
        || size.y() <= 0.0
        || object.transform.scale.x() == 0.0
        || object.transform.scale.y() == 0.0
    {
        return;
    }
    let alpha = fill.a() * object.transform.opacity;
    if !alpha.is_finite() || alpha <= 0.0 {
        return;
    }
    let color = [fill.r() * alpha, fill.g() * alpha, fill.b() * alpha, alpha];
    if !color.iter().all(|v| v.is_finite()) {
        return;
    }
    let width = f64::from(size.x());
    let height = f64::from(size.y());
    let radians = f64::from(object.transform.rotation_degrees).to_radians();
    let (sin, cos) = radians.sin_cos();
    let mut corners = [[0.0; 2]; 4];
    let locals = [
        [0.0, 0.0],
        [size.x(), 0.0],
        [size.x(), size.y()],
        [0.0, size.y()],
    ];
    for (i, local) in locals.iter().enumerate() {
        let local_x = f64::from(local[0]) - f64::from(object.transform.anchor.x()) * width;
        let local_y = f64::from(local[1]) - f64::from(object.transform.anchor.y()) * height;
        let scaled_x = local_x * f64::from(object.transform.scale.x());
        let scaled_y = local_y * f64::from(object.transform.scale.y());
        let x = f64::from(object.transform.position.x()) + cos * scaled_x - sin * scaled_y;
        let y = f64::from(object.transform.position.y()) + sin * scaled_x + cos * scaled_y;
        let ndc_x = (2.0 * x / f64::from(dimensions[0])) - 1.0;
        let ndc_y = 1.0 - (2.0 * y / f64::from(dimensions[1]));
        if !ndc_x.is_finite()
            || !ndc_y.is_finite()
            || ndc_x.abs() > f64::from(f32::MAX)
            || ndc_y.abs() > f64::from(f32::MAX)
        {
            return;
        }
        corners[i] = [ndc_x as f32, ndc_y as f32];
    }

    let radius = if kind == 0.0 {
        radius.max(0.0).min(size.x().min(size.y()) * 0.5)
    } else {
        0.0
    };
    for index in [0_usize, 1, 2, 0, 2, 3] {
        vertices.push(PrimitiveVertex {
            ndc: corners[index],
            local: locals[index],
            size: [size.x(), size.y()],
            color,
            radius,
            kind,
        });
    }
}

pub(crate) fn vertices_as_bytes(vertices: &[PrimitiveVertex]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(vertices.len() * VERTEX_STRIDE as usize);
    for vertex in vertices {
        vertex.write_bytes(&mut bytes);
    }
    bytes
}

pub(crate) fn clear_color(background: LinearRgba) -> wgpu::Color {
    let alpha = f64::from(background.a());
    wgpu::Color {
        r: f64::from(background.r()) * alpha,
        g: f64::from(background.g()) * alpha,
        b: f64::from(background.b()) * alpha,
        a: alpha,
    }
}

#[cfg(test)]
mod tests {
    use super::{VERTEX_STRIDE, clear_color, scene_vertices, vertices_as_bytes};
    use crate::scene_eval::{
        EvaluatedObject, EvaluatedObjectContent, EvaluatedScene, EvaluatedTransform,
    };
    use rhythm_core::{
        domain::{LinearRgba, Vec2},
        ids::ObjectId,
    };

    fn v(x: f32, y: f32) -> Vec2 {
        Vec2::new(x, y).expect("finite")
    }
    fn rectangle(id: u64, position: Vec2, fill: LinearRgba) -> EvaluatedObject {
        EvaluatedObject {
            id: ObjectId::new(id).expect("ID"),
            transform: EvaluatedTransform {
                position,
                scale: v(1.0, 1.0),
                rotation_degrees: 0.0,
                anchor: v(0.5, 0.5),
                opacity: 1.0,
            },
            content: EvaluatedObjectContent::Rectangle {
                size: v(20.0, 10.0),
                fill,
                corner_radius: 0.0,
            },
            effects: Vec::new(),
        }
    }

    #[test]
    fn center_anchor_maps_to_correct_ndc_and_serialization_stride() {
        let scene = EvaluatedScene {
            objects: vec![rectangle(1, v(50.0, 50.0), LinearRgba::black_opaque())],
        };
        let vertices = scene_vertices(&scene, [100, 100]);
        assert_eq!(vertices.len(), 6);
        assert_eq!(vertices[0].local, [0.0, 0.0]);
        for (actual, expected) in vertices[0].ndc.into_iter().zip([-0.2, 0.1]) {
            assert!((actual - expected).abs() < 1e-6);
        }
        for (actual, expected) in vertices[2].ndc.into_iter().zip([0.2, -0.1]) {
            assert!((actual - expected).abs() < 1e-6);
        }
        assert_eq!(
            vertices_as_bytes(&vertices).len(),
            6 * VERTEX_STRIDE as usize
        );
    }

    #[test]
    fn rotation_and_negative_scale_use_clockwise_screen_convention() {
        let mut shape = rectangle(1, v(50.0, 50.0), LinearRgba::black_opaque());
        shape.transform.anchor = v(0.0, 0.0);
        shape.transform.scale = v(-1.0, 2.0);
        shape.transform.rotation_degrees = 90.0;
        let vertices = scene_vertices(
            &EvaluatedScene {
                objects: vec![shape],
            },
            [100, 100],
        );
        // Local (20, 0) mirrors left and rotates upward in screen coordinates.
        assert!((vertices[1].ndc[0]).abs() < 0.0001);
        assert!((vertices[1].ndc[1] - 0.4).abs() < 0.0001);
        assert_eq!(vertices.len(), 6);
    }

    #[test]
    fn order_premultiplied_opacity_and_unsupported_objects() {
        let fill = LinearRgba::new(0.8, 0.4, 0.2, 0.5).expect("color");
        let first = rectangle(1, v(10.0, 10.0), fill);
        let mut second = rectangle(2, v(20.0, 20.0), fill);
        second.transform.opacity = 0.5;
        second.content = EvaluatedObjectContent::Ellipse {
            size: v(8.0, 6.0),
            fill,
        };
        let mut ignored = rectangle(3, v(30.0, 30.0), fill);
        ignored.effects.push(crate::scene_eval::EvaluatedEffect {
            id: rhythm_core::ids::EffectId::new(4).expect("id"),
            kind: crate::scene_eval::EvaluatedEffectKind::Blur { radius_px: 8.0 },
        });
        let scene = EvaluatedScene {
            objects: vec![first, ignored, second],
        };
        let vertices = scene_vertices(&scene, [100, 100]);
        assert_eq!(vertices.len(), 12);
        assert_eq!(vertices[0].color, [0.4, 0.2, 0.1, 0.5]);
        assert_eq!(vertices[6].color, [0.2, 0.1, 0.05, 0.25]);
        assert_eq!(vertices[0].kind, 0.0);
        assert_eq!(vertices[6].kind, 1.0);
    }

    #[test]
    fn degenerate_or_unrepresentable_shapes_are_ignored() {
        let mut shape = rectangle(1, v(50.0, 50.0), LinearRgba::black_opaque());
        shape.transform.scale = v(0.0, 1.0);
        assert!(
            scene_vertices(
                &EvaluatedScene {
                    objects: vec![shape]
                },
                [100, 100]
            )
            .is_empty()
        );
        assert!(scene_vertices(&EvaluatedScene { objects: vec![] }, [0, 100]).is_empty());
        let mut huge = rectangle(1, v(f32::MAX, f32::MAX), LinearRgba::black_opaque());
        huge.transform.scale = v(f32::MAX, f32::MAX);
        assert!(
            scene_vertices(
                &EvaluatedScene {
                    objects: vec![huge]
                },
                [100, 100]
            )
            .is_empty()
        );
    }

    #[test]
    fn project_editor_position_drag_changes_creative_geometry_and_undo_restores_it() {
        use crate::scene_eval::evaluate_scene;
        use rhythm_core::{
            animation::Animated,
            editor::ProjectEditor,
            project::{
                Object, ObjectContent, Project, ProjectSettings, RectangleObject,
                TransformAnimation,
            },
            time::{GridOffsetNs, ProjectTimeNs, TempoMap},
        };

        let mut project = Project::new(
            "Live preview integration",
            ProjectSettings::default(),
            TempoMap::unset(GridOffsetNs::new(0)),
        );
        let id = ObjectId::new(1).expect("id");
        project.composition.objects.push(Object {
            id,
            name: "Visible rectangle".to_owned(),
            visible: true,
            locked: false,
            transform: TransformAnimation::new(
                Animated::new_static(v(100.0, 200.0)),
                Animated::new_static(v(1.0, 1.0)),
                Animated::new_static(0.0),
                Animated::new_static(v(0.5, 0.5)),
                Animated::new_static(1.0),
            ),
            content: ObjectContent::Rectangle(RectangleObject {
                size: Animated::new_static(v(20.0, 10.0)),
                fill: Animated::new_static(LinearRgba::black_opaque()),
                corner_radius: Animated::new_static(0.0),
            }),
            effects: Vec::new(),
        });
        project.next_entity_id = 2;
        let mut editor = ProjectEditor::new(project).expect("valid project");
        let displayed = |editor: &ProjectEditor| {
            let evaluated =
                evaluate_scene(editor.project(), ProjectTimeNs::new(0)).expect("static scene");
            scene_vertices(&evaluated, [1920, 1080])
        };
        let original = displayed(&editor);
        assert_eq!(original.len(), 6);
        editor.begin_position_transaction(id).expect("begin");
        editor
            .update_position_transaction(v(300.0, 400.0))
            .expect("live move");
        let moved = displayed(&editor);
        assert_ne!(original[0].ndc, moved[0].ndc);
        assert_eq!(editor.history_len(), 0);
        assert!(editor.commit_transaction().expect("commit"));
        assert_eq!(editor.history_len(), 1);
        assert!(editor.undo().expect("undo"));
        assert_eq!(displayed(&editor), original);
        assert!(editor.redo().expect("redo"));
        assert_eq!(displayed(&editor), moved);
    }

    #[test]
    fn background_alpha_is_premultiplied_in_linear_light() {
        let c = clear_color(LinearRgba::new(0.8, 0.2, 0.0, 0.5).expect("color"));
        assert!((c.r - 0.4).abs() < 1e-6);
        assert!((c.g - 0.1).abs() < 1e-6);
        assert_eq!(c.a, 0.5);
    }
}
