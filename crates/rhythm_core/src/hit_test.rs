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

    rectangle_bounds_hit(local_x, local_y, size)
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

/// Tests image bounds using already prepared intrinsic pixel dimensions.
///
/// The image object stores only an AssetId; dimensions come from an external
/// decoded/prepared-image metadata cache. The supplied asset ID must match the
/// object's image source to avoid picking using stale dimensions after Relink.
/// Missing metadata, zero dimensions and non-image objects never hit.
///
/// This function performs no image decoding, I/O, allocation or GPU readback.
/// Size is an unscaled, uncropped local pixel rectangle; the shared inverse
/// transform applies Position, Rotation, Scale (including mirroring) and Anchor.
/// Hidden/locked filtering and viewport-camera conversion are separate concerns.
#[must_use]
pub fn hit_test_image(
    object: &Object,
    continuous_tick: f64,
    composition_point: Vec2,
    intrinsic_size: Option<(crate::ids::AssetId, [u32; 2])>,
) -> bool {
    let ObjectContent::Image(image) = &object.content else {
        return false;
    };
    if !continuous_tick.is_finite() {
        return false;
    }

    let Some((asset_id, [width, height])) = intrinsic_size else {
        return false;
    };
    if asset_id != image.asset || width == 0 || height == 0 {
        return false;
    }

    // All u32 image dimensions can be represented as finite f32 coordinates.
    // No local Size is added to the semantic Project schema.
    let size = Vec2::new(width as f32, height as f32).expect("u32 pixel sizes are finite");
    let Some((local_x, local_y)) =
        inverse_transform_point(object, size, continuous_tick, composition_point)
    else {
        return false;
    };

    rectangle_bounds_hit(local_x, local_y, size)
}

/// Rectangle returned by the composition text shaper in its *local* layout space.
///
/// Bounds can have a nonzero or negative origin (for example when text is
/// center/right aligned). Its extent controls normalized Anchor positioning;
/// glyph shaping, line-height and font fallback are the layout engine's work.
/// The viewport must supply bounds from the same cached layout used to render.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TextLayoutBounds {
    pub min: Vec2,
    pub max: Vec2,
}

/// Tests an already-shaped Text object's local layout rectangle.
///
/// Uses the exact inverse object transform shared by Rectangle, Ellipse and
/// Image. Text's normalized Anchor is relative to the measured bounding box,
/// not an estimated number of characters. Missing, empty or invalid layout
/// bounds cannot hit; the caller owns layout cache freshness after text, font,
/// size or alignment changes. This function never shapes text or opens fonts.
///
/// Camera conversion, hidden/locked filtering and painter-order selection
/// remain separate viewport responsibilities (AI-166).
#[must_use]
pub fn hit_test_text(
    object: &Object,
    continuous_tick: f64,
    composition_point: Vec2,
    layout_bounds: Option<TextLayoutBounds>,
) -> bool {
    if !matches!(&object.content, ObjectContent::Text(_)) || !continuous_tick.is_finite() {
        return false;
    }
    let Some(bounds) = layout_bounds else {
        return false;
    };

    let width = bounds.max.x() - bounds.min.x();
    let height = bounds.max.y() - bounds.min.y();
    if width <= 0.0 || height <= 0.0 {
        return false;
    }

    let Ok(size) = Vec2::new(width, height) else {
        return false;
    };
    let Some((local_x, local_y)) =
        inverse_transform_point(object, size, continuous_tick, composition_point)
    else {
        return false;
    };

    // Local coordinates are normalized to the measured bounds' minimum
    // before applying Anchor. Layout origin offsets thus cannot cause an
    // extra shift in the selection rectangle.
    rectangle_bounds_hit(local_x, local_y, size)
}

/// Returns the frontmost selectable object. Composition objects are stored in
/// painter order, so the last one is topmost. Disabled objects are skipped.
///
/// Image and Text require prepared metadata from runtime caches. Missing data
/// means no hit: this function never guesses bounds or invokes a decoder/shaper.
/// A plain click's empty-space behavior belongs to EditorSession.
#[must_use]
pub fn pick_topmost_object(
    objects: &[Object],
    continuous_tick: f64,
    composition_point: Vec2,
    image_dimensions: impl Fn(crate::ids::AssetId) -> Option<[u32; 2]>,
    text_layout: impl Fn(crate::ids::ObjectId) -> Option<TextLayoutBounds>,
) -> Option<crate::ids::ObjectId> {
    if !continuous_tick.is_finite() {
        return None;
    }
    objects.iter().rev().find_map(|object| {
        if !object.visible || object.locked {
            return None;
        }
        let hit = match &object.content {
            ObjectContent::Rectangle(_) => hit_test_rectangle(object, continuous_tick, composition_point),
            ObjectContent::Ellipse(_) => hit_test_ellipse(object, continuous_tick, composition_point),
            ObjectContent::Image(image) => hit_test_image(
                object,
                continuous_tick,
                composition_point,
                image_dimensions(image.asset).map(|dimensions| (image.asset, dimensions)),
            ),
            ObjectContent::Text(_) => hit_test_text(
                object,
                continuous_tick,
                composition_point,
                text_layout(object.id),
            ),
        };
        hit.then_some(object.id)
    })
}

/// Editor-only wireframe kind. The actual composition renderer is separate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutlineKind {
    Rectangle,
    Ellipse,
}

/// Evaluated object geometry reused for selection debug overlays. Positions are
/// composition pixels, not egui coordinates. Does not draw project content.
#[derive(Debug, Clone, Copy)]
pub struct OutlineGeometry {
    pub kind: OutlineKind,
    position: Vec2,
    size: Vec2,
    scale: Vec2,
    anchor: Vec2,
    sin: f64,
    cos: f64,
}

impl OutlineGeometry {
    /// Maps a point expressed as fractions of the local shape size to the
    /// composition, honoring the same anchor/scale/rotation as the hit test.
    #[must_use]
    pub fn point(self, u: f64, v: f64) -> Option<Vec2> {
        if !u.is_finite() || !v.is_finite() {
            return None;
        }
        let x = (u - f64::from(self.anchor.x()))
            * f64::from(self.size.x())
            * f64::from(self.scale.x());
        let y = (v - f64::from(self.anchor.y()))
            * f64::from(self.size.y())
            * f64::from(self.scale.y());
        let px = f64::from(self.position.x()) + self.cos * x - self.sin * y;
        let py = f64::from(self.position.y()) + self.sin * x + self.cos * y;
        if !px.is_finite() || !py.is_finite() {
            return None;
        }
        Vec2::new(px as f32, py as f32).ok()
    }
}

/// Geometry for transparent editor-only Rectangle/Ellipse guides until the
/// actual GPU composition pass is wired. Image/Text need runtime media/layout
/// metadata, so no false proxy outlines are fabricated for those objects.
#[must_use]
pub fn object_outline(object: &Object, continuous_tick: f64) -> Option<OutlineGeometry> {
    if !continuous_tick.is_finite() {
        return None;
    }
    let (kind, size) = match &object.content {
        ObjectContent::Rectangle(shape) => (
            OutlineKind::Rectangle,
            evaluate_vec2(&shape.size, continuous_tick),
        ),
        ObjectContent::Ellipse(shape) => (
            OutlineKind::Ellipse,
            evaluate_vec2(&shape.size, continuous_tick),
        ),
        _ => return None,
    };
    if size.x() <= 0.0 || size.y() <= 0.0 {
        return None;
    }
    let position = evaluate_vec2(&object.transform.position, continuous_tick);
    let scale = evaluate_vec2(&object.transform.scale, continuous_tick);
    let anchor = evaluate_vec2(&object.transform.anchor, continuous_tick);
    let degrees = evaluate_scalar(&object.transform.rotation_degrees, continuous_tick);
    if scale.x() == 0.0 || scale.y() == 0.0 || !degrees.is_finite() {
        return None;
    }
    let (sin, cos) = f64::from(degrees).to_radians().sin_cos();
    Some(OutlineGeometry {
        kind,
        position,
        size,
        scale,
        anchor,
        sin,
        cos,
    })
}

fn rectangle_bounds_hit(local_x: f64, local_y: f64, size: Vec2) -> bool {
    // Inclusive edges with minimal tolerance for f64 trigonometric rounding.
    let epsilon_x = f64::from(size.x()).max(1.0) * 1e-9;
    let epsilon_y = f64::from(size.y()).max(1.0) * 1e-9;
    local_x >= -epsilon_x
        && local_x <= f64::from(size.x()) + epsilon_x
        && local_y >= -epsilon_y
        && local_y <= f64::from(size.y()) + epsilon_y
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
    use super::{
        OutlineKind, TextLayoutBounds, hit_test_ellipse, hit_test_image, hit_test_rectangle,
        hit_test_text, object_outline, pick_topmost_object,
    };
    use crate::{
        animation::{Animated, Interpolation, Keyframe},
        domain::{LinearRgba, Vec2},
        ids::{AssetId, KeyframeId, ObjectId},
        project::{
            EllipseObject, FontReference, FontStyle, FontWeight, ImageObject, Object,
            ObjectContent, RectangleObject, TextAlignment, TextObject, TransformAnimation,
        },
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

    fn image() -> Object {
        let mut object = rectangle();
        object.content = ObjectContent::Image(ImageObject {
            asset: AssetId::new(2).expect("asset id"),
        });
        object
    }

    fn intrinsic(width: u32, height: u32) -> Option<(AssetId, [u32; 2])> {
        Some((AssetId::new(2).expect("asset id"), [width, height]))
    }

    fn text_object() -> Object {
        let mut object = rectangle();
        object.content = ObjectContent::Text(TextObject {
            text: "Привет\nHello".to_owned(),
            font: FontReference {
                family: "Inter".to_owned(),
                weight: FontWeight::Normal,
                style: FontStyle::Normal,
            },
            font_size: 24.0,
            color: Animated::new_static(LinearRgba::black_opaque()),
            alignment: TextAlignment::Center,
        });
        object
    }

    fn layout(min_x: f32, min_y: f32, max_x: f32, max_y: f32) -> Option<TextLayoutBounds> {
        Some(TextLayoutBounds {
            min: vec2(min_x, min_y),
            max: vec2(max_x, max_y),
        })
    }

    #[test]
    fn frontmost_picking_respects_order_visibility_and_locks() {
        let mut bottom = rectangle();
        bottom.id = ObjectId::new(1).expect("id");
        let mut top = ellipse();
        top.id = ObjectId::new(2).expect("id");
        let mut objects = vec![bottom, top];
        let at = vec2(200.0, 100.0);
        let pick = |objects: &[Object], point| {
            pick_topmost_object(objects, 0.0, point, |_| None, |_| None)
        };
        assert_eq!(pick(&objects, at), Some(objects[1].id));
        objects[1].locked = true;
        assert_eq!(pick(&objects, at), Some(objects[0].id));
        objects[0].visible = false;
        assert_eq!(pick(&objects, at), None);
        objects[1].locked = false;
        objects[1].visible = false;
        assert_eq!(pick(&objects, at), None);
    }

    #[test]
    fn picker_rejects_ellipse_box_corner_and_selects_rectangle_below() {
        let bottom = rectangle();
        let mut top = ellipse();
        top.id = ObjectId::new(2).expect("id");
        let point = vec2(249.0, 119.0);
        assert_eq!(
            pick_topmost_object(&[bottom.clone(), top], 0.0, point, |_| None, |_| None),
            Some(bottom.id),
        );
        assert_eq!(
            pick_topmost_object(&[bottom], f64::NAN, point, |_| None, |_| None),
            None,
        );
    }

    #[test]
    fn picker_requires_prepared_image_and_text_bounds() {
        let mut image_object = image();
        image_object.id = ObjectId::new(3).expect("id");
        let mut text = text_object();
        text.id = ObjectId::new(4).expect("id");
        let objects = [image_object, text];
        let center = vec2(200.0, 100.0);
        assert_eq!(
            pick_topmost_object(&objects, 0.0, center, |_| None, |_| None),
            None,
        );
        assert_eq!(
            pick_topmost_object(&objects, 0.0, center, |_| Some([100, 40]), |_| None),
            Some(objects[0].id),
        );
        assert_eq!(
            pick_topmost_object(
                &objects,
                0.0,
                center,
                |_| Some([100, 40]),
                |_| layout(0.0, 0.0, 100.0, 40.0),
            ),
            Some(objects[1].id),
        );
    }

    #[test]
    fn editor_outline_and_cpu_hit_test_share_rotated_mirrored_transform() {
        let mut rectangle = rectangle();
        rectangle.transform.scale = Animated::new_static(vec2(-2.0, 0.5));
        rectangle.transform.rotation_degrees = Animated::new_static(45.0);
        let geometry = object_outline(&rectangle, 0.0).expect("valid outline");
        assert_eq!(geometry.kind, OutlineKind::Rectangle);
        for (u, v) in [(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)] {
            let point = geometry.point(u, v).expect("finite projected corner");
            assert!(hit_test_rectangle(&rectangle, 0.0, point));
        }
        assert!(geometry.point(f64::NAN, 0.0).is_none());
        assert!(object_outline(&image(), 0.0).is_none());
        assert!(object_outline(&ellipse(), f64::NAN).is_none());
    }

    #[test]
    fn text_layout_bounds_use_measured_extent_not_origin_or_string_length() {
        let object = text_object();
        let shape = layout(-70.0, 12.0, 30.0, 52.0);
        assert!(hit_test_text(&object, 0.0, vec2(200.0, 100.0), shape));
        assert!(hit_test_text(&object, 0.0, vec2(150.0, 80.0), shape));
        assert!(hit_test_text(&object, 0.0, vec2(250.0, 120.0), shape));
        assert!(!hit_test_text(&object, 0.0, vec2(149.0, 100.0), shape));
        assert!(!hit_test_text(&object, 0.0, vec2(200.0, 121.0), shape));
    }

    #[test]
    fn text_hit_supports_noncentral_anchor_and_rotation() {
        let mut object = text_object();
        object.transform.anchor = Animated::new_static(vec2(0.0, 0.0));
        assert!(hit_test_text(
            &object,
            0.0,
            vec2(300.0, 140.0),
            layout(-70.0, 12.0, 30.0, 52.0),
        ));
        assert!(!hit_test_text(
            &object,
            0.0,
            vec2(199.0, 100.0),
            layout(-70.0, 12.0, 30.0, 52.0),
        ));

        object.transform.anchor = Animated::new_static(vec2(0.5, 0.5));
        object.transform.rotation_degrees = Animated::new_static(90.0);
        assert!(hit_test_text(
            &object,
            0.0,
            vec2(200.0, 149.0),
            layout(-70.0, 12.0, 30.0, 52.0),
        ));
        assert!(!hit_test_text(
            &object,
            0.0,
            vec2(200.0, 151.0),
            layout(-70.0, 12.0, 30.0, 52.0),
        ));
    }

    #[test]
    fn text_hit_handles_mirrored_nonuniform_scale() {
        let mut object = text_object();
        object.transform.anchor = Animated::new_static(vec2(0.0, 0.0));
        object.transform.scale = Animated::new_static(vec2(-2.0, 0.5));
        let shape = layout(-70.0, 12.0, 30.0, 52.0);
        assert!(hit_test_text(&object, 0.0, vec2(0.0, 120.0), shape));
        assert!(hit_test_text(&object, 0.0, vec2(150.0, 110.0), shape));
        assert!(!hit_test_text(&object, 0.0, vec2(201.0, 110.0), shape));
        assert!(!hit_test_text(&object, 0.0, vec2(150.0, 121.0), shape));
    }

    #[test]
    fn text_hit_rejects_missing_degenerate_and_wrong_kind_layouts() {
        let object = text_object();
        assert!(!hit_test_text(&object, 0.0, vec2(200.0, 100.0), None));
        for bounds in [
            layout(0.0, 0.0, 0.0, 40.0),
            layout(0.0, 0.0, 100.0, 0.0),
            layout(100.0, 0.0, 0.0, 40.0),
            layout(0.0, 40.0, 100.0, 0.0),
        ] {
            assert!(!hit_test_text(&object, 0.0, vec2(200.0, 100.0), bounds));
        }
        assert!(!hit_test_text(
            &rectangle(),
            0.0,
            vec2(200.0, 100.0),
            layout(0.0, 0.0, 100.0, 40.0),
        ));
    }

    #[test]
    fn text_hit_rejects_singular_transform_and_invalid_tick() {
        let mut object = text_object();
        let shape = layout(0.0, 0.0, 100.0, 40.0);
        object.transform.scale = Animated::new_static(vec2(0.0, 1.0));
        assert!(!hit_test_text(&object, 0.0, vec2(200.0, 100.0), shape));
        object.transform.scale = Animated::new_static(vec2(1.0, 1.0));
        object.transform.rotation_degrees = Animated::new_static(f32::NAN);
        assert!(!hit_test_text(&object, 0.0, vec2(200.0, 100.0), shape));
        object.transform.rotation_degrees = Animated::new_static(0.0);
        assert!(!hit_test_text(&object, f64::NAN, vec2(200.0, 100.0), shape));
    }

    #[test]
    fn text_hit_evaluates_animated_transform_at_tick() {
        let mut object = text_object();
        object.transform.position = Animated::with_keyframes(
            vec2(200.0, 100.0),
            vec![
                Keyframe::new(
                    KeyframeId::new(3).expect("key id"),
                    MusicalTick::new(0),
                    vec2(200.0, 100.0),
                    Interpolation::Linear,
                ),
                Keyframe::new(
                    KeyframeId::new(4).expect("key id"),
                    MusicalTick::new(960),
                    vec2(400.0, 100.0),
                    Interpolation::Linear,
                ),
            ],
        )
        .expect("distinct key IDs");
        let shape = layout(0.0, 0.0, 100.0, 40.0);
        assert!(hit_test_text(&object, 480.0, vec2(300.0, 100.0), shape));
        assert!(!hit_test_text(&object, 480.0, vec2(200.0, 100.0), shape));
        assert!(hit_test_text(&object, 960.0, vec2(400.0, 100.0), shape));
        assert!(!hit_test_text(&object, f64::NAN, vec2(400.0, 100.0), shape));
    }

    #[test]
    fn image_uses_intrinsic_pixel_bounds_with_inclusive_edges() {
        let object = image();
        assert!(hit_test_image(
            &object,
            0.0,
            vec2(200.0, 100.0),
            intrinsic(100, 40)
        ));
        assert!(hit_test_image(
            &object,
            0.0,
            vec2(150.0, 80.0),
            intrinsic(100, 40)
        ));
        assert!(hit_test_image(
            &object,
            0.0,
            vec2(250.0, 120.0),
            intrinsic(100, 40)
        ));
        assert!(!hit_test_image(
            &object,
            0.0,
            vec2(149.0, 100.0),
            intrinsic(100, 40)
        ));
        assert!(!hit_test_image(
            &object,
            0.0,
            vec2(200.0, 121.0),
            intrinsic(100, 40)
        ));
    }

    #[test]
    fn image_picking_inverts_rotation_and_noncentral_anchor() {
        let mut object = image();
        object.transform.anchor = Animated::new_static(vec2(0.0, 0.0));
        assert!(hit_test_image(
            &object,
            0.0,
            vec2(300.0, 140.0),
            intrinsic(100, 40)
        ));
        assert!(!hit_test_image(
            &object,
            0.0,
            vec2(199.0, 100.0),
            intrinsic(100, 40)
        ));

        object.transform.anchor = Animated::new_static(vec2(0.5, 0.5));
        object.transform.rotation_degrees = Animated::new_static(90.0);
        assert!(hit_test_image(
            &object,
            0.0,
            vec2(200.0, 149.0),
            intrinsic(100, 40)
        ));
        assert!(!hit_test_image(
            &object,
            0.0,
            vec2(230.0, 100.0),
            intrinsic(100, 40)
        ));
        assert!(!hit_test_image(
            &object,
            0.0,
            vec2(200.0, 151.0),
            intrinsic(100, 40)
        ));
    }

    #[test]
    fn image_hit_supports_mirrored_nonuniform_scale() {
        let mut object = image();
        object.transform.anchor = Animated::new_static(vec2(0.0, 0.0));
        object.transform.scale = Animated::new_static(vec2(-2.0, 0.5));
        assert!(hit_test_image(
            &object,
            0.0,
            vec2(0.0, 120.0),
            intrinsic(100, 40)
        ));
        assert!(hit_test_image(
            &object,
            0.0,
            vec2(150.0, 110.0),
            intrinsic(100, 40)
        ));
        assert!(!hit_test_image(
            &object,
            0.0,
            vec2(201.0, 110.0),
            intrinsic(100, 40)
        ));
        assert!(!hit_test_image(
            &object,
            0.0,
            vec2(150.0, 121.0),
            intrinsic(100, 40)
        ));
    }

    #[test]
    fn image_hit_rejects_missing_stale_or_zero_asset_dimensions() {
        let object = image();
        assert!(!hit_test_image(&object, 0.0, vec2(200.0, 100.0), None));
        assert!(!hit_test_image(
            &object,
            0.0,
            vec2(200.0, 100.0),
            intrinsic(0, 40)
        ));
        assert!(!hit_test_image(
            &object,
            0.0,
            vec2(200.0, 100.0),
            intrinsic(100, 0)
        ));
        assert!(!hit_test_image(
            &object,
            0.0,
            vec2(200.0, 100.0),
            Some((AssetId::new(3).expect("other asset"), [100, 40])),
        ));
        assert!(!hit_test_image(
            &rectangle(),
            0.0,
            vec2(200.0, 100.0),
            intrinsic(100, 40)
        ));
        assert!(!hit_test_rectangle(&object, 0.0, vec2(200.0, 100.0)));
    }

    #[test]
    fn image_hit_rejects_singular_transform_and_invalid_time() {
        let mut object = image();
        object.transform.scale = Animated::new_static(vec2(1.0, 0.0));
        assert!(!hit_test_image(
            &object,
            0.0,
            vec2(200.0, 100.0),
            intrinsic(100, 40)
        ));
        object.transform.scale = Animated::new_static(vec2(1.0, 1.0));
        object.transform.rotation_degrees = Animated::new_static(f32::NAN);
        assert!(!hit_test_image(
            &object,
            0.0,
            vec2(200.0, 100.0),
            intrinsic(100, 40)
        ));
        object.transform.rotation_degrees = Animated::new_static(0.0);
        assert!(!hit_test_image(
            &object,
            f64::NAN,
            vec2(200.0, 100.0),
            intrinsic(100, 40)
        ));
    }

    #[test]
    fn image_hit_evaluates_animated_position_at_the_requested_tick() {
        let mut object = image();
        object.transform.position = Animated::with_keyframes(
            vec2(200.0, 100.0),
            vec![
                Keyframe::new(
                    KeyframeId::new(3).expect("key id"),
                    MusicalTick::new(0),
                    vec2(200.0, 100.0),
                    Interpolation::Linear,
                ),
                Keyframe::new(
                    KeyframeId::new(4).expect("key id"),
                    MusicalTick::new(960),
                    vec2(400.0, 100.0),
                    Interpolation::Linear,
                ),
            ],
        )
        .expect("unique keys");
        assert!(hit_test_image(
            &object,
            480.0,
            vec2(300.0, 100.0),
            intrinsic(100, 40)
        ));
        assert!(!hit_test_image(
            &object,
            480.0,
            vec2(200.0, 100.0),
            intrinsic(100, 40)
        ));
        assert!(hit_test_image(
            &object,
            960.0,
            vec2(400.0, 100.0),
            intrinsic(100, 40)
        ));
        assert!(!hit_test_image(
            &object,
            f64::NAN,
            vec2(400.0, 100.0),
            intrinsic(100, 40)
        ));
    }

    #[test]
    fn ellipse_hits_center_and_curve_but_not_bounding_box_corners() {
        let object = ellipse();
        assert!(hit_test_ellipse(&object, 0.0, vec2(200.0, 100.0)));
        assert!(hit_test_ellipse(&object, 0.0, vec2(150.0, 100.0)));
        assert!(hit_test_ellipse(&object, 0.0, vec2(250.0, 100.0)));
        assert!(hit_test_ellipse(&object, 0.0, vec2(200.0, 80.0)));
        assert!(!hit_test_ellipse(&object, 0.0, vec2(250.0, 120.0)));
        assert!(hit_test_ellipse(&object, 0.0, vec2(235.0, 114.0)));
        assert!(!hit_test_ellipse(&object, 0.0, vec2(236.0, 115.0)));
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
        assert!(!hit_test_ellipse(&object, f64::NAN, vec2(300.0, 100.0)));
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
