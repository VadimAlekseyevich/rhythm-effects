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

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LocalBounds2d {
    pub min: Vec2,
    pub size: Vec2,
}

impl LocalBounds2d {
    #[must_use]
    pub const fn new(min: Vec2, size: Vec2) -> Self {
        Self { min, size }
    }

    #[must_use]
    pub fn from_size(size: Vec2) -> Self {
        Self {
            min: Vec2::new(0.0, 0.0).expect("zero local-bounds origin is finite"),
            size,
        }
    }

    #[must_use]
    pub fn contains(self, point: Vec2) -> bool {
        if self.size.x() < 0.0 || self.size.y() < 0.0 {
            return false;
        }

        let max_x = self.min.x() + self.size.x();
        let max_y = self.min.y() + self.size.y();
        (self.min.x()..=max_x).contains(&point.x()) && (self.min.y()..=max_y).contains(&point.y())
    }
}

#[must_use]
pub fn object_transform_point_in_bounds(
    local_point: Vec2,
    transform: ObjectTransform2d,
    bounds: LocalBounds2d,
) -> Option<Vec2> {
    let anchor_x = bounds.min.x() + transform.anchor.x() * bounds.size.x();
    let anchor_y = bounds.min.y() + transform.anchor.y() * bounds.size.y();

    let scaled_x = (local_point.x() - anchor_x) * transform.scale.x();
    let scaled_y = (local_point.y() - anchor_y) * transform.scale.y();

    let radians = transform.rotation_degrees.to_radians();
    let (sin, cos) = radians.sin_cos();
    let rotated_x = cos * scaled_x - sin * scaled_y;
    let rotated_y = sin * scaled_x + cos * scaled_y;

    Vec2::new(
        rotated_x + transform.position.x(),
        rotated_y + transform.position.y(),
    )
    .ok()
}

pub fn inverse_object_transform_point_in_bounds(
    composition_point: Vec2,
    transform: ObjectTransform2d,
    bounds: LocalBounds2d,
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

    let anchor_x = bounds.min.x() + transform.anchor.x() * bounds.size.x();
    let anchor_y = bounds.min.y() + transform.anchor.y() * bounds.size.y();

    Vec2::new(
        unrotated_x / scale_x + anchor_x,
        unrotated_y / scale_y + anchor_y,
    )
    .ok()
}

#[must_use]
pub fn inverse_object_transform_point(
    composition_point: Vec2,
    transform: ObjectTransform2d,
    bounds_size: Vec2,
) -> Option<Vec2> {
    inverse_object_transform_point_in_bounds(
        composition_point,
        transform,
        LocalBounds2d::from_size(bounds_size),
    )
}

#[must_use]
pub fn hit_test_text_layout_bounds(
    composition_point: Vec2,
    transform: ObjectTransform2d,
    layout_bounds: LocalBounds2d,
) -> bool {
    if layout_bounds.size.x() <= 0.0 || layout_bounds.size.y() <= 0.0 {
        return false;
    }

    let Some(local_point) =
        inverse_object_transform_point_in_bounds(composition_point, transform, layout_bounds)
    else {
        return false;
    };

    layout_bounds.contains(local_point)
}

#[must_use]
pub fn hit_test_image_bounds(
    composition_point: Vec2,
    transform: ObjectTransform2d,
    intrinsic_size: Vec2,
) -> bool {
    if intrinsic_size.x() <= 0.0 || intrinsic_size.y() <= 0.0 {
        return false;
    }

    hit_test_rectangle(composition_point, transform, intrinsic_size)
}

#[must_use]
pub fn hit_test_ellipse(composition_point: Vec2, transform: ObjectTransform2d, size: Vec2) -> bool {
    if size.x() <= 0.0 || size.y() <= 0.0 {
        return false;
    }

    let Some(local_point) = inverse_object_transform_point(composition_point, transform, size)
    else {
        return false;
    };

    let radius_x = size.x() * 0.5;
    let radius_y = size.y() * 0.5;
    let normalized_x = (local_point.x() - radius_x) / radius_x;
    let normalized_y = (local_point.y() - radius_y) / radius_y;

    normalized_x * normalized_x + normalized_y * normalized_y <= 1.0
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
    use super::{
        LocalBounds2d, ObjectTransform2d, hit_test_ellipse, hit_test_image_bounds,
        hit_test_rectangle, hit_test_text_layout_bounds, inverse_object_transform_point,
        object_transform_point_in_bounds,
    };
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
    fn text_hit_test_uses_resolved_layout_bounds_with_nonzero_origin() {
        let transform = transform((200.0, 100.0), (1.0, 1.0), 0.0, (0.5, 0.5));
        let layout_bounds = LocalBounds2d::new(vec2(-10.0, -20.0), vec2(100.0, 40.0));

        assert!(hit_test_text_layout_bounds(
            vec2(200.0, 100.0),
            transform,
            layout_bounds
        ));
        assert!(hit_test_text_layout_bounds(
            vec2(150.0, 80.0),
            transform,
            layout_bounds
        ));
        assert!(hit_test_text_layout_bounds(
            vec2(250.0, 120.0),
            transform,
            layout_bounds
        ));
        assert!(!hit_test_text_layout_bounds(
            vec2(250.1, 100.0),
            transform,
            layout_bounds
        ));
    }

    #[test]
    fn text_hit_test_hook_respects_transform_and_empty_layout() {
        let transform = transform((300.0, 200.0), (-2.0, 0.5), 90.0, (0.5, 0.5));
        let layout_bounds = LocalBounds2d::new(vec2(-20.0, 10.0), vec2(80.0, 40.0));

        assert!(hit_test_text_layout_bounds(
            vec2(300.0, 200.0),
            transform,
            layout_bounds
        ));
        assert!(!hit_test_text_layout_bounds(
            vec2(300.0, 200.0),
            transform,
            LocalBounds2d::new(vec2(0.0, 0.0), vec2(0.0, 40.0))
        ));
    }

    #[test]
    fn image_hit_test_uses_intrinsic_local_bounds() {
        let transform = transform((300.0, 200.0), (2.0, -1.0), 90.0, (0.5, 0.5));
        let intrinsic_size = vec2(200.0, 100.0);

        assert!(hit_test_image_bounds(
            vec2(300.0, 200.0),
            transform,
            intrinsic_size
        ));
        assert!(hit_test_image_bounds(
            vec2(350.0, 400.0),
            transform,
            intrinsic_size
        ));
        assert!(!hit_test_image_bounds(
            vec2(350.1, 400.0),
            transform,
            intrinsic_size
        ));
    }

    #[test]
    fn image_hit_test_rejects_invalid_intrinsic_bounds() {
        let transform = transform((300.0, 200.0), (1.0, 1.0), 0.0, (0.5, 0.5));

        assert!(!hit_test_image_bounds(
            vec2(300.0, 200.0),
            transform,
            vec2(0.0, 100.0)
        ));
    }

    #[test]
    fn ellipse_hit_test_uses_local_ellipse_geometry() {
        let transform = transform((200.0, 100.0), (1.0, 1.0), 0.0, (0.5, 0.5));
        let size = vec2(100.0, 50.0);

        assert!(hit_test_ellipse(vec2(200.0, 100.0), transform, size));
        assert!(hit_test_ellipse(vec2(250.0, 100.0), transform, size));
        assert!(hit_test_ellipse(vec2(200.0, 125.0), transform, size));
        assert!(!hit_test_ellipse(vec2(250.1, 100.0), transform, size));
        assert!(!hit_test_ellipse(vec2(250.0, 125.0), transform, size));
    }

    #[test]
    fn ellipse_hit_test_respects_rotation_and_negative_scale() {
        let size = vec2(100.0, 50.0);
        let rotated = transform((200.0, 100.0), (-2.0, 1.0), 90.0, (0.5, 0.5));

        assert!(hit_test_ellipse(vec2(200.0, 200.0), rotated, size));
        assert!(hit_test_ellipse(vec2(225.0, 100.0), rotated, size));
        assert!(!hit_test_ellipse(vec2(225.1, 100.0), rotated, size));
    }

    #[test]
    fn ellipse_hit_test_rejects_degenerate_size() {
        let transform = transform((200.0, 100.0), (1.0, 1.0), 0.0, (0.5, 0.5));

        assert!(!hit_test_ellipse(
            vec2(200.0, 100.0),
            transform,
            vec2(0.0, 50.0)
        ));
        assert!(!hit_test_ellipse(
            vec2(200.0, 100.0),
            transform,
            vec2(100.0, -50.0)
        ));
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
    fn forward_and_inverse_object_transform_round_trip() {
        let bounds = LocalBounds2d::new(vec2(-20.0, 10.0), vec2(100.0, 50.0));
        let transform = transform((300.0, 200.0), (-2.0, 0.5), 90.0, (0.25, 0.75));
        let local = vec2(40.0, 30.0);

        let composition =
            object_transform_point_in_bounds(local, transform, bounds).expect("forward transform");
        let restored =
            super::inverse_object_transform_point_in_bounds(composition, transform, bounds)
                .expect("inverse transform");

        assert!((restored.x() - local.x()).abs() < 0.0001);
        assert!((restored.y() - local.y()).abs() < 0.0001);
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
