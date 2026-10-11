//! CPU geometry for viewport picking in composition coordinates.
//!
//! Pointer/camera conversion is deliberately external: the eventual viewport
//! first converts screen coordinates into composition space, then invokes this
//! test with the same evaluated transform values used for rendering.

use crate::{
    domain::Vec2,
    project::{Object, ObjectContent},
    property::{evaluate_scalar, evaluate_vec2},
};

/// Tests the *unrounded* Rectangle geometry at a continuous musical position.
///
/// Transform convention: `position` is the position of the normalized local
/// anchor. The local point is shifted by `anchor * size`, scaled (negative
/// scales mirror), and then rotated around that anchor. The inverse order is
/// rotation -> inverse scale -> anchor translation.
///
/// This is a geometry test only. Hidden/locked filtering and z-order selection
/// belong to the viewport object-selection step (AI-166).
#[must_use]
pub fn hit_test_rectangle(object: &Object, continuous_tick: f64, composition_point: Vec2) -> bool {
    let ObjectContent::Rectangle(rectangle) = &object.content else {
        return false;
    };
    if !continuous_tick.is_finite() {
        return false;
    }
    let size = evaluate_vec2(&rectangle.size, continuous_tick);
    let Some((local_x, local_y)) =
        inverse_transform_point(object, size, continuous_tick, composition_point)
    else {
        return false;
    };

    // Floating-point trigonometry can differ by a few ULPs at edges.
    let epsilon_x = f64::from(size.x()).max(1.0) * 1e-9;
    let epsilon_y = f64::from(size.y()).max(1.0) * 1e-9;
    local_x >= -epsilon_x
        && local_x <= f64::from(size.x()) + epsilon_x
        && local_y >= -epsilon_y
        && local_y <= f64::from(size.y()) + epsilon_y
}

/// Tests an ellipse contained in the object's unrounded local Size rectangle.
///
/// Its normalized center is (0.5, 0.5), independent of the transform Anchor.
/// A point inside the bounding box but outside the elliptical curve must miss.
/// Selection visibility, locking and z-order are handled by the viewport layer.
#[must_use]
pub fn hit_test_ellipse(object: &Object, continuous_tick: f64, composition_point: Vec2) -> bool {
    let ObjectContent::Ellipse(ellipse) = &object.content else {
        return false;
    };
    if !continuous_tick.is_finite() {
        return false;
    }

    let size = evaluate_vec2(&ellipse.size, continuous_tick);
    let Some((local_x, local_y)) =
        inverse_transform_point(object, size, continuous_tick, composition_point)
    else {
        return false;
    };

    let norm_x = local_x / f64::from(size.x()) * 2.0 - 1.0;
    let norm_y = local_y / f64::from(size.y()) * 2.0 - 1.0;

    // Inclusive curve with tiny normalized tolerance for trig/rounding noise.
    norm_x * norm_x + norm_y * norm_y <= 1.0 + 1e-9
}

/// Inverts the exact same affine transform for every geometric object.
/// Keeps shape formulas separate so later Image/Text picking cannot diverge.
fn inverse_transform_point(
    object: &Object,
    size: Vec2,
    continuous_tick: f64,
    composition_point: Vec2,
) -> Option<(f64, f64)> {
    if !continuous_tick.is_finite() || size.x() <= 0.0 || size.y() <= 0.0 {
        return None;
    }

    let position = evaluate_vec2(&object.transform.position, continuous_tick);
    let scale = evaluate_vec2(&object.transform.scale, continuous_tick);
    let anchor = evaluate_vec2(&object.transform.anchor, continuous_tick);
    let degrees = evaluate_scalar(&object.transform.rotation_degrees, continuous_tick);

    if scale.x() == 0.0 || scale.y() == 0.0 || !degrees.is_finite() {
        return None;
    }

    // f64 intermediates reduce cancellation near rotated shape boundaries.
    let radians = f64::from(degrees).to_radians();
    let (sin, cos) = radians.sin_cos();
    let dx = f64::from(composition_point.x()) - f64::from(position.x());
    let dy = f64::from(composition_point.y()) - f64::from(position.y());

    let local_x =
        (cos * dx + sin * dy) / f64::from(scale.x()) + f64::from(anchor.x()) * f64::from(size.x());
    let local_y =
        (-sin * dx + cos * dy) / f64::from(scale.y()) + f64::from(anchor.y()) * f64::from(size.y());

    if !local_x.is_finite() || !local_y.is_finite() {
        return None;
    }

    Some((local_x, local_y))
}

#[cfg(test)]
mod tests {
    use super::{hit_test_ellipse, hit_test_rectangle};
    use crate::{
        animation::{Animated, Interpolation, Keyframe},
        domain::{LinearRgba, Vec2},
        ids::{KeyframeId, ObjectId},
        project::{EllipseObject, Object, ObjectContent, RectangleObject, TransformAnimation},
        time::MusicalTick,
    };

    fn vec2(x: f32, y: f32) -> Vec2 {
        Vec2::new(x, y).expect("finite test point")
    }

    fn rectangle() -> Object {
        Object {
            id: ObjectId::new(1).expect("valid id"),
            name: "hit target".to_owned(),
            visible: true,
            locked: false,
            transform: TransformAnimation::new(
                Animated::new_static(vec2(200.0, 100.0)),
                Animated::new_static(vec2(1.0, 1.0)),
                Animated::new_static(0.0),
                Animated::new_static(vec2(0.5, 0.5)),
                Animated::new_static(1.0),
            ),
            content: ObjectContent::Rectangle(RectangleObject {
                size: Animated::new_static(vec2(100.0, 40.0)),
                fill: Animated::new_static(LinearRgba::black_opaque()),
                corner_radius: Animated::new_static(0.0),
            }),
            effects: Vec::new(),
        }
    }

    fn ellipse() -> Object {
        let mut object = rectangle();
        object.content = ObjectContent::Ellipse(EllipseObject {
            size: Animated::new_static(vec2(100.0, 40.0)),
            fill: Animated::new_static(LinearRgba::black_opaque()),
        });
        object
    }

    #[test]
    fn ellipse_hits_center_and_curve_but_not_bounding_box_corners() {
        let object = ellipse();
        assert!(hit_test_ellipse(&object, 0.0, vec2(200.0, 100.0)));
        assert!(hit_test_ellipse(&object, 0.0, vec2(150.0, 100.0)));
        assert!(hit_test_ellipse(&object, 0.0, vec2(250.0, 100.0)));
        assert!(hit_test_ellipse(&object, 0.0, vec2(200.0, 80.0)));
        assert!(!hit_test_ellipse(&object, 0.0, vec2(250.0, 120.0)));
        assert!(!hit_test_ellipse(&object, 0.0, vec2(251.0, 100.0)));
        assert!(!hit_test_ellipse(&object, 0.0, vec2(200.0, 121.0)));
    }

    #[test]
    fn ellipse_anchor_and_rotation_follow_shared_inverse_transform() {
        let mut object = ellipse();
        object.transform.anchor = Animated::new_static(vec2(0.0, 0.0));
        assert!(hit_test_ellipse(&object, 0.0, vec2(250.0, 120.0)));
        assert!(!hit_test_ellipse(&object, 0.0, vec2(200.0, 100.0)));

        object.transform.anchor = Animated::new_static(vec2(0.5, 0.5));
        object.transform.rotation_degrees = Animated::new_static(90.0);
        assert!(hit_test_ellipse(&object, 0.0, vec2(200.0, 149.0)));
        assert!(!hit_test_ellipse(&object, 0.0, vec2(200.0, 151.0)));
        assert!(!hit_test_ellipse(&object, 0.0, vec2(220.0, 150.0)));
    }

    #[test]
    fn ellipse_negative_and_nonuniform_scale_are_invertible() {
        let mut object = ellipse();
        object.transform.scale = Animated::new_static(vec2(-2.0, 0.5));
        assert!(hit_test_ellipse(&object, 0.0, vec2(100.0, 100.0)));
        assert!(hit_test_ellipse(&object, 0.0, vec2(200.0, 110.0)));
        assert!(!hit_test_ellipse(&object, 0.0, vec2(99.0, 100.0)));
        assert!(!hit_test_ellipse(&object, 0.0, vec2(200.0, 111.0)));
    }

    #[test]
    fn ellipse_rejects_singular_invalid_and_nonellipse_inputs() {
        let mut object = ellipse();
        object.transform.scale = Animated::new_static(vec2(0.0, 1.0));
        assert!(!hit_test_ellipse(&object, 0.0, vec2(200.0, 100.0)));
        object.transform.scale = Animated::new_static(vec2(1.0, 1.0));
        object.transform.rotation_degrees = Animated::new_static(f32::NAN);
        assert!(!hit_test_ellipse(&object, 0.0, vec2(200.0, 100.0)));
        object.transform.rotation_degrees = Animated::new_static(0.0);
        assert!(!hit_test_ellipse(&object, f64::NAN, vec2(200.0, 100.0)));
        if let ObjectContent::Ellipse(ellipse) = &mut object.content {
            ellipse.size = Animated::new_static(vec2(100.0, 0.0));
        }
        assert!(!hit_test_ellipse(&object, 0.0, vec2(200.0, 100.0)));
        assert!(!hit_test_ellipse(&rectangle(), 0.0, vec2(200.0, 100.0)));
        assert!(!hit_test_rectangle(&ellipse(), 0.0, vec2(200.0, 100.0)));
    }

    #[test]
    fn ellipse_size_and_position_are_evaluated_at_continuous_tick() {
        let mut object = ellipse();
        object.transform.position = Animated::with_keyframes(
            vec2(200.0, 100.0),
            vec![
                Keyframe::new(
                    KeyframeId::new(2).expect("id"),
                    MusicalTick::new(0),
                    vec2(200.0, 100.0),
                    Interpolation::Linear,
                ),
                Keyframe::new(
                    KeyframeId::new(3).expect("id"),
                    MusicalTick::new(960),
                    vec2(400.0, 100.0),
                    Interpolation::Linear,
                ),
            ],
        )
        .expect("unique position keys");
        if let ObjectContent::Ellipse(ellipse) = &mut object.content {
            ellipse.size = Animated::with_keyframes(
                vec2(100.0, 40.0),
                vec![
                    Keyframe::new(
                        KeyframeId::new(4).expect("id"),
                        MusicalTick::new(0),
                        vec2(100.0, 40.0),
                        Interpolation::Linear,
                    ),
                    Keyframe::new(
                        KeyframeId::new(5).expect("id"),
                        MusicalTick::new(960),
                        vec2(200.0, 80.0),
                        Interpolation::Linear,
                    ),
                ],
            )
            .expect("unique size keys");
        }
        assert!(hit_test_ellipse(&object, 480.0, vec2(375.0, 100.0)));
        assert!(!hit_test_ellipse(&object, 480.0, vec2(376.0, 100.0)));
        assert!(!hit_test_ellipse(&object, 480.0, vec2(200.0, 100.0)));
        assert!(hit_test_ellipse(&object, 960.0, vec2(500.0, 100.0)));
    }

    #[test]
    fn centered_anchor_hit_includes_boundaries_and_excludes_outside() {
        let object = rectangle();
        assert!(hit_test_rectangle(&object, 0.0, vec2(200.0, 100.0)));
        assert!(hit_test_rectangle(&object, 0.0, vec2(150.0, 80.0)));
        assert!(hit_test_rectangle(&object, 0.0, vec2(250.0, 120.0)));
        assert!(!hit_test_rectangle(&object, 0.0, vec2(149.0, 100.0)));
        assert!(!hit_test_rectangle(&object, 0.0, vec2(200.0, 121.0)));
    }

    #[test]
    fn noncentral_anchor_changes_origin_without_changing_dimensions() {
        let mut object = rectangle();
        object.transform.anchor = Animated::new_static(vec2(0.0, 0.0));
        assert!(hit_test_rectangle(&object, 0.0, vec2(200.0, 100.0)));
        assert!(hit_test_rectangle(&object, 0.0, vec2(300.0, 140.0)));
        assert!(!hit_test_rectangle(&object, 0.0, vec2(199.0, 100.0)));
        assert!(!hit_test_rectangle(&object, 0.0, vec2(250.0, 141.0)));
    }

    #[test]
    fn rotation_is_inverted_before_local_rectangle_test() {
        let mut object = rectangle();
        object.transform.rotation_degrees = Animated::new_static(90.0);
        assert!(hit_test_rectangle(&object, 0.0, vec2(200.0, 149.0)));
        assert!(hit_test_rectangle(&object, 0.0, vec2(180.0, 100.0)));
        assert!(!hit_test_rectangle(&object, 0.0, vec2(200.0, 151.0)));
        assert!(!hit_test_rectangle(&object, 0.0, vec2(230.0, 100.0)));
    }

    #[test]
    fn inverse_handles_nonuniform_and_negative_scale() {
        let mut object = rectangle();
        object.transform.scale = Animated::new_static(vec2(2.0, 0.5));
        assert!(hit_test_rectangle(&object, 0.0, vec2(299.0, 100.0)));
        assert!(!hit_test_rectangle(&object, 0.0, vec2(301.0, 100.0)));
        assert!(!hit_test_rectangle(&object, 0.0, vec2(200.0, 111.0)));

        object.transform.anchor = Animated::new_static(vec2(0.0, 0.0));
        object.transform.scale = Animated::new_static(vec2(-1.0, 1.0));
        assert!(hit_test_rectangle(&object, 0.0, vec2(150.0, 110.0)));
        assert!(!hit_test_rectangle(&object, 0.0, vec2(220.0, 110.0)));
    }

    #[test]
    fn singular_or_invalid_geometry_does_not_hit() {
        let mut object = rectangle();
        object.transform.scale = Animated::new_static(vec2(0.0, 1.0));
        assert!(!hit_test_rectangle(&object, 0.0, vec2(200.0, 100.0)));

        object.transform.scale = Animated::new_static(vec2(1.0, 1.0));
        object.transform.rotation_degrees = Animated::new_static(f32::NAN);
        assert!(!hit_test_rectangle(&object, 0.0, vec2(200.0, 100.0)));
        object.transform.rotation_degrees = Animated::new_static(0.0);
        assert!(!hit_test_rectangle(&object, f64::NAN, vec2(200.0, 100.0)));
        if let ObjectContent::Rectangle(rectangle) = &mut object.content {
            rectangle.size = Animated::new_static(vec2(-10.0, 40.0));
        }
        assert!(!hit_test_rectangle(&object, 0.0, vec2(200.0, 100.0)));
    }

    #[test]
    fn animated_position_is_evaluated_at_current_continuous_tick() {
        let mut object = rectangle();
        object.transform.position = Animated::with_keyframes(
            vec2(200.0, 100.0),
            vec![
                Keyframe::new(
                    KeyframeId::new(2).expect("id"),
                    MusicalTick::new(0),
                    vec2(200.0, 100.0),
                    Interpolation::Linear,
                ),
                Keyframe::new(
                    KeyframeId::new(3).expect("id"),
                    MusicalTick::new(960),
                    vec2(400.0, 100.0),
                    Interpolation::Linear,
                ),
            ],
        )
        .expect("unique keys");
        assert!(hit_test_rectangle(&object, 480.0, vec2(300.0, 100.0)));
        assert!(!hit_test_rectangle(&object, 480.0, vec2(200.0, 100.0)));
        assert!(hit_test_rectangle(&object, 960.0, vec2(400.0, 100.0)));
    }
}
