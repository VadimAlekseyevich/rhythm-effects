use crate::domain::Vec2;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ObjectTransform2d {
    pub position: Vec2,
    pub scale: Vec2,
    pub rotation_degrees: f32,
    pub anchor: Vec2,
}

impl ObjectTransform2d {
    #[must_use]
    pub const fn new(position: Vec2, scale: Vec2, rotation_degrees: f32, anchor: Vec2) -> Self {
        Self {
            position,
            scale,
            rotation_degrees,
            anchor,
        }
    }
}

#[must_use]
pub fn inverse_object_transform_point(
    composition_point: Vec2,
    transform: ObjectTransform2d,
    bounds_size: Vec2,
) -> Option<Vec2> {
    let scale_x = transform.scale.x();
    let scale_y = transform.scale.y();
    if scale_x == 0.0 || scale_y == 0.0 {
        return None;
    }

    let translated_x = composition_point.x() - transform.position.x();
    let translated_y = composition_point.y() - transform.position.y();

    let radians = transform.rotation_degrees.to_radians();
    let (sin, cos) = radians.sin_cos();

    let unrotated_x = cos * translated_x + sin * translated_y;
    let unrotated_y = -sin * translated_x + cos * translated_y;

    let anchor_x = transform.anchor.x() * bounds_size.x();
    let anchor_y = transform.anchor.y() * bounds_size.y();

    Vec2::new(
        unrotated_x / scale_x + anchor_x,
        unrotated_y / scale_y + anchor_y,
    )
    .ok()
}

#[must_use]
pub fn hit_test_rectangle(
    composition_point: Vec2,
    transform: ObjectTransform2d,
    size: Vec2,
) -> bool {
    if size.x() < 0.0 || size.y() < 0.0 {
        return false;
    }

    let Some(local_point) = inverse_object_transform_point(composition_point, transform, size)
    else {
        return false;
    };

    (0.0..=size.x()).contains(&local_point.x()) && (0.0..=size.y()).contains(&local_point.y())
}

#[cfg(test)]
mod tests {
    use super::{ObjectTransform2d, hit_test_rectangle, inverse_object_transform_point};
    use crate::domain::Vec2;

    fn vec2(x: f32, y: f32) -> Vec2 {
        Vec2::new(x, y).expect("finite vector")
    }

    fn transform(
        position: (f32, f32),
        scale: (f32, f32),
        rotation_degrees: f32,
        anchor: (f32, f32),
    ) -> ObjectTransform2d {
        ObjectTransform2d::new(
            vec2(position.0, position.1),
            vec2(scale.0, scale.1),
            rotation_degrees,
            vec2(anchor.0, anchor.1),
        )
    }

    #[test]
    fn rectangle_hit_test_uses_center_anchor() {
        let transform = transform((200.0, 100.0), (1.0, 1.0), 0.0, (0.5, 0.5));
        let size = vec2(100.0, 50.0);

        assert!(hit_test_rectangle(vec2(200.0, 100.0), transform, size));
        assert!(hit_test_rectangle(vec2(150.0, 75.0), transform, size));
        assert!(hit_test_rectangle(vec2(250.0, 125.0), transform, size));
        assert!(!hit_test_rectangle(vec2(149.9, 100.0), transform, size));
    }

    #[test]
    fn rectangle_hit_test_respects_top_left_and_outside_anchor() {
        let size = vec2(100.0, 50.0);
        let top_left = transform((20.0, 30.0), (1.0, 1.0), 0.0, (0.0, 0.0));
        assert!(hit_test_rectangle(vec2(20.0, 30.0), top_left, size));
        assert!(hit_test_rectangle(vec2(120.0, 80.0), top_left, size));

        let outside = transform((200.0, 100.0), (1.0, 1.0), 0.0, (1.5, 0.5));
        assert!(hit_test_rectangle(vec2(50.0, 75.0), outside, size));
        assert!(hit_test_rectangle(vec2(150.0, 125.0), outside, size));
        assert!(!hit_test_rectangle(vec2(151.0, 100.0), outside, size));
    }

    #[test]
    fn rectangle_hit_test_respects_scale_and_negative_scale() {
        let size = vec2(100.0, 50.0);
        let doubled = transform((200.0, 100.0), (2.0, 2.0), 0.0, (0.5, 0.5));
        assert!(hit_test_rectangle(vec2(100.0, 50.0), doubled, size));
        assert!(hit_test_rectangle(vec2(300.0, 150.0), doubled, size));
        assert!(!hit_test_rectangle(vec2(300.1, 100.0), doubled, size));

        let mirrored = transform((200.0, 100.0), (-1.0, 1.0), 0.0, (0.5, 0.5));
        assert!(hit_test_rectangle(vec2(150.0, 100.0), mirrored, size));
        assert!(hit_test_rectangle(vec2(250.0, 100.0), mirrored, size));
        assert!(!hit_test_rectangle(vec2(250.1, 100.0), mirrored, size));
    }

    #[test]
    fn rectangle_hit_test_respects_clockwise_rotation() {
        let size = vec2(100.0, 50.0);
        let transform = transform((200.0, 100.0), (1.0, 1.0), 90.0, (0.5, 0.5));

        assert!(hit_test_rectangle(vec2(200.0, 150.0), transform, size));
        assert!(hit_test_rectangle(vec2(225.0, 100.0), transform, size));
        assert!(!hit_test_rectangle(vec2(200.0, 150.1), transform, size));
    }

    #[test]
    fn inverse_transform_round_trips_expected_local_points() {
        let size = vec2(100.0, 50.0);
        let transform = transform((300.0, 200.0), (-2.0, 0.5), 90.0, (0.25, 0.75));

        let local = inverse_object_transform_point(vec2(318.75, 150.0), transform, size)
            .expect("invertible transform");

        assert!((local.x() - 50.0).abs() < 0.0001);
        assert!((local.y() - 0.0).abs() < 0.0001);
    }

    #[test]
    fn singular_transform_and_negative_size_do_not_hit() {
        let size = vec2(100.0, 50.0);
        let zero_scale = transform((200.0, 100.0), (0.0, 1.0), 0.0, (0.5, 0.5));

        assert!(!hit_test_rectangle(vec2(200.0, 100.0), zero_scale, size));
        assert!(!hit_test_rectangle(
            vec2(200.0, 100.0),
            transform((200.0, 100.0), (1.0, 1.0), 0.0, (0.5, 0.5)),
            vec2(-100.0, 50.0),
        ));
    }
}
