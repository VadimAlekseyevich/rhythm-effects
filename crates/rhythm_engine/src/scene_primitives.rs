//! CPU tessellation for evaluated primitive scene content.

use rhythm_core::{
    domain::{LinearRgba, Vec2},
    geometry::{LocalBounds2d, ObjectTransform2d, object_transform_point_in_bounds},
};

use crate::scene_eval::{EvaluatedObject, EvaluatedObjectContent, EvaluatedScene};

const ELLIPSE_SEGMENTS: usize = 48;
const ROUNDED_CORNER_SEGMENTS: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PrimitiveVertex {
    pub position: [f32; 2],
    pub color: [f32; 4],
}

#[derive(Debug, Clone, PartialEq)]
pub struct PrimitiveBatch {
    pub vertices: Vec<PrimitiveVertex>,
    pub skipped_effect_objects: usize,
    pub skipped_non_primitive_objects: usize,
}

#[must_use]
pub fn tessellate_scene_primitives(
    scene: &EvaluatedScene,
    composition_size: [u32; 2],
) -> PrimitiveBatch {
    let mut batch = PrimitiveBatch {
        vertices: Vec::new(),
        skipped_effect_objects: 0,
        skipped_non_primitive_objects: 0,
    };
    for object in &scene.objects {
        append_object_primitives(&mut batch, object, composition_size);
    }
    batch
}

#[must_use]
pub fn tessellate_object_primitives(
    object: &EvaluatedObject,
    composition_size: [u32; 2],
) -> PrimitiveBatch {
    let mut batch = PrimitiveBatch {
        vertices: Vec::new(),
        skipped_effect_objects: 0,
        skipped_non_primitive_objects: 0,
    };
    append_object_primitives(&mut batch, object, composition_size);
    batch
}

fn append_object_primitives(
    batch: &mut PrimitiveBatch,
    object: &EvaluatedObject,
    composition_size: [u32; 2],
) {
    if composition_size[0] == 0 || composition_size[1] == 0 {
        return;
    }
    if !object.effects.is_empty() {
        batch.skipped_effect_objects += 1;
        return;
    }

    match object.content {
        EvaluatedObjectContent::Rectangle {
            size,
            fill,
            corner_radius,
        } => {
            if size.x() <= 0.0 || size.y() <= 0.0 {
                return;
            }
            let radius = corner_radius
                .max(0.0)
                .min(size.x() * 0.5)
                .min(size.y() * 0.5);
            let polygon = rounded_rectangle_polygon(size.x(), size.y(), radius);
            append_fan(
                &mut batch.vertices,
                object,
                fill,
                &polygon,
                composition_size,
            );
        }
        EvaluatedObjectContent::Ellipse { size, fill } => {
            if size.x() <= 0.0 || size.y() <= 0.0 {
                return;
            }
            let mut polygon = Vec::with_capacity(ELLIPSE_SEGMENTS);
            for segment in 0..ELLIPSE_SEGMENTS {
                let angle = std::f32::consts::TAU * segment as f32 / ELLIPSE_SEGMENTS as f32;
                polygon.push([
                    size.x() * 0.5 + angle.cos() * size.x() * 0.5,
                    size.y() * 0.5 + angle.sin() * size.y() * 0.5,
                ]);
            }
            append_fan(
                &mut batch.vertices,
                object,
                fill,
                &polygon,
                composition_size,
            );
        }
        EvaluatedObjectContent::Image { .. } | EvaluatedObjectContent::Text { .. } => {
            batch.skipped_non_primitive_objects += 1;
        }
    }
}

fn rounded_rectangle_polygon(width: f32, height: f32, radius: f32) -> Vec<[f32; 2]> {
    if radius == 0.0 {
        return vec![[0.0, 0.0], [width, 0.0], [width, height], [0.0, height]];
    }
    let corners = [
        ([width - radius, radius], -std::f32::consts::FRAC_PI_2),
        ([width - radius, height - radius], 0.0),
        ([radius, height - radius], std::f32::consts::FRAC_PI_2),
        ([radius, radius], std::f32::consts::PI),
    ];
    let mut polygon = Vec::with_capacity(corners.len() * (ROUNDED_CORNER_SEGMENTS + 1));
    for (center, start) in corners {
        for step in 0..=ROUNDED_CORNER_SEGMENTS {
            let angle =
                start + std::f32::consts::FRAC_PI_2 * step as f32 / ROUNDED_CORNER_SEGMENTS as f32;
            polygon.push([
                center[0] + angle.cos() * radius,
                center[1] + angle.sin() * radius,
            ]);
        }
    }
    polygon
}

fn append_fan(
    vertices: &mut Vec<PrimitiveVertex>,
    object: &EvaluatedObject,
    fill: LinearRgba,
    polygon: &[[f32; 2]],
    composition_size: [u32; 2],
) {
    if polygon.len() < 3 {
        return;
    }
    let alpha = (fill.a() * object.transform.opacity).clamp(0.0, 1.0);
    if alpha == 0.0 {
        return;
    }
    let color = [fill.r() * alpha, fill.g() * alpha, fill.b() * alpha, alpha];
    let transform = ObjectTransform2d::new(
        object.transform.position,
        object.transform.scale,
        object.transform.rotation_degrees,
        object.transform.anchor,
    );
    let bounds = match object.content {
        EvaluatedObjectContent::Rectangle { size, .. }
        | EvaluatedObjectContent::Ellipse { size, .. } => LocalBounds2d::from_size(size),
        _ => return,
    };
    let sum = polygon.iter().fold([0.0_f32; 2], |sum, point| {
        [sum[0] + point[0], sum[1] + point[1]]
    });
    let count = polygon.len() as f32;
    let center = [sum[0] / count, sum[1] / count];

    for index in 0..polygon.len() {
        for local in [center, polygon[index], polygon[(index + 1) % polygon.len()]] {
            let Ok(local) = Vec2::new(local[0], local[1]) else {
                return;
            };
            let Some(point) = object_transform_point_in_bounds(local, transform, bounds) else {
                return;
            };
            vertices.push(PrimitiveVertex {
                position: composition_to_clip(point.x(), point.y(), composition_size),
                color,
            });
        }
    }
}

fn composition_to_clip(x: f32, y: f32, size: [u32; 2]) -> [f32; 2] {
    [
        x * 2.0 / size[0] as f32 - 1.0,
        1.0 - y * 2.0 / size[1] as f32,
    ]
}

#[cfg(test)]
mod tests {
    use super::{tessellate_object_primitives, tessellate_scene_primitives};
    use crate::scene_eval::{
        EvaluatedObject, EvaluatedObjectContent, EvaluatedScene, EvaluatedTransform,
    };
    use rhythm_core::{
        domain::{LinearRgba, Vec2},
        ids::ObjectId,
    };

    fn vec2(x: f32, y: f32) -> Vec2 {
        Vec2::new(x, y).expect("finite")
    }

    fn object(content: EvaluatedObjectContent) -> EvaluatedObject {
        EvaluatedObject {
            id: ObjectId::new(1).expect("id"),
            transform: EvaluatedTransform {
                position: vec2(50.0, 25.0),
                scale: vec2(1.0, 1.0),
                rotation_degrees: 0.0,
                anchor: vec2(0.5, 0.5),
                opacity: 0.5,
            },
            content,
            effects: Vec::new(),
        }
    }

    #[test]
    fn rectangle_maps_to_clip_space_and_premultiplies_alpha() {
        let scene = EvaluatedScene {
            objects: vec![object(EvaluatedObjectContent::Rectangle {
                size: vec2(100.0, 50.0),
                fill: LinearRgba::new(0.8, 0.4, 0.2, 0.5).expect("color"),
                corner_radius: 0.0,
            })],
        };
        let batch = tessellate_scene_primitives(&scene, [100, 50]);
        assert_eq!(batch.vertices.len(), 12);
        assert_eq!(batch.vertices[0].position, [0.0, 0.0]);
        assert_eq!(batch.vertices[0].color, [0.2, 0.1, 0.05, 0.25]);
    }


    #[test]
    fn single_object_tessellation_does_not_reorder_scene_content() {
        let rectangle = object(EvaluatedObjectContent::Rectangle {
            size: vec2(100.0, 50.0),
            fill: LinearRgba::black_opaque(),
            corner_radius: 0.0,
        });
        let batch = tessellate_object_primitives(&rectangle, [100, 50]);
        assert_eq!(batch.vertices.len(), 12);
        assert_eq!(batch.skipped_effect_objects, 0);
        assert_eq!(batch.skipped_non_primitive_objects, 0);
    }

    #[test]
    fn rounded_rectangle_and_ellipse_generate_geometry() {
        let scene = EvaluatedScene {
            objects: vec![
                object(EvaluatedObjectContent::Rectangle {
                    size: vec2(100.0, 50.0),
                    fill: LinearRgba::black_opaque(),
                    corner_radius: 12.0,
                }),
                object(EvaluatedObjectContent::Ellipse {
                    size: vec2(100.0, 50.0),
                    fill: LinearRgba::black_opaque(),
                }),
            ],
        };
        let batch = tessellate_scene_primitives(&scene, [100, 50]);
        assert!(batch.vertices.len() > 12 + 48 * 3);
        assert_eq!(batch.skipped_effect_objects, 0);
        assert_eq!(batch.skipped_non_primitive_objects, 0);
    }
}
