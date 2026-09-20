use std::collections::HashMap;

use rhythm_core::{
    domain::Vec2,
    geometry::{
        LocalBounds2d, ObjectTransform2d, hit_test_ellipse, hit_test_image_bounds,
        hit_test_rectangle, hit_test_text_layout_bounds, object_transform_point_in_bounds,
    },
    ids::ObjectId,
    project::Project,
};
use rhythm_engine::scene_eval::{EvaluatedObject, EvaluatedObjectContent, EvaluatedScene};

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RuntimeHitBounds {
    ImageIntrinsicSize(Vec2),
    TextLayout(LocalBounds2d),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SelectionOverlayGeometry {
    pub corners: [Vec2; 4],
    pub anchor: Vec2,
}

fn transformed_corners(
    transform: ObjectTransform2d,
    local_bounds: LocalBounds2d,
) -> Option<[Vec2; 4]> {
    if local_bounds.size.x() < 0.0 || local_bounds.size.y() < 0.0 {
        return None;
    }

    let min_x = local_bounds.min.x();
    let min_y = local_bounds.min.y();
    let max_x = min_x + local_bounds.size.x();
    let max_y = min_y + local_bounds.size.y();
    let local_corners = [
        Vec2::new(min_x, min_y).ok()?,
        Vec2::new(max_x, min_y).ok()?,
        Vec2::new(max_x, max_y).ok()?,
        Vec2::new(min_x, max_y).ok()?,
    ];

    Some([
        object_transform_point_in_bounds(local_corners[0], transform, local_bounds)?,
        object_transform_point_in_bounds(local_corners[1], transform, local_bounds)?,
        object_transform_point_in_bounds(local_corners[2], transform, local_bounds)?,
        object_transform_point_in_bounds(local_corners[3], transform, local_bounds)?,
    ])
}

fn bounds_corners(bounds: LocalBounds2d) -> Option<[Vec2; 4]> {
    if bounds.size.x() < 0.0 || bounds.size.y() < 0.0 {
        return None;
    }

    let min_x = bounds.min.x();
    let min_y = bounds.min.y();
    let max_x = min_x + bounds.size.x();
    let max_y = min_y + bounds.size.y();
    Some([
        Vec2::new(min_x, min_y).ok()?,
        Vec2::new(max_x, min_y).ok()?,
        Vec2::new(max_x, max_y).ok()?,
        Vec2::new(min_x, max_y).ok()?,
    ])
}

fn transformed_bounds(
    transform: ObjectTransform2d,
    local_bounds: LocalBounds2d,
) -> Option<LocalBounds2d> {
    if local_bounds.size.x() < 0.0 || local_bounds.size.y() < 0.0 {
        return None;
    }

    let corners = transformed_corners(transform, local_bounds)?;
    let mut transformed = corners.into_iter();
    let first = transformed.next()?;
    let mut min_x = first.x();
    let mut min_y = first.y();
    let mut max_x = first.x();
    let mut max_y = first.y();

    for point in transformed {
        let point = point?;
        min_x = min_x.min(point.x());
        min_y = min_y.min(point.y());
        max_x = max_x.max(point.x());
        max_y = max_y.max(point.y());
    }

    Some(LocalBounds2d::new(
        Vec2::new(min_x, min_y).ok()?,
        Vec2::new(max_x - min_x, max_y - min_y).ok()?,
    ))
}

fn bounds_intersect(a: LocalBounds2d, b: LocalBounds2d) -> bool {
    if a.size.x() < 0.0 || a.size.y() < 0.0 || b.size.x() < 0.0 || b.size.y() < 0.0 {
        return false;
    }

    let a_max_x = a.min.x() + a.size.x();
    let a_max_y = a.min.y() + a.size.y();
    let b_max_x = b.min.x() + b.size.x();
    let b_max_y = b.min.y() + b.size.y();

    a.min.x() <= b_max_x && a_max_x >= b.min.x() && a.min.y() <= b_max_y && a_max_y >= b.min.y()
}

fn evaluated_local_bounds<F>(
    object_id: ObjectId,
    content: &EvaluatedObjectContent,
    runtime_bounds: &mut F,
) -> Option<LocalBounds2d>
where
    F: FnMut(ObjectId, &EvaluatedObjectContent) -> Option<RuntimeHitBounds>,
{
    match content {
        EvaluatedObjectContent::Rectangle { size, .. }
        | EvaluatedObjectContent::Ellipse { size, .. } => Some(LocalBounds2d::from_size(*size)),
        EvaluatedObjectContent::Image { .. } => match runtime_bounds(object_id, content) {
            Some(RuntimeHitBounds::ImageIntrinsicSize(size)) => {
                Some(LocalBounds2d::from_size(size))
            }
            _ => None,
        },
        EvaluatedObjectContent::Text { .. } => match runtime_bounds(object_id, content) {
            Some(RuntimeHitBounds::TextLayout(bounds)) => Some(bounds),
            _ => None,
        },
    }
}

#[must_use]
pub fn selected_objects_bounds<F>(
    scene: &EvaluatedScene,
    selected_object_ids: &[ObjectId],
    mut runtime_bounds: F,
) -> Option<LocalBounds2d>
where
    F: FnMut(ObjectId, &EvaluatedObjectContent) -> Option<RuntimeHitBounds>,
{
    let mut combined: Option<LocalBounds2d> = None;

    for evaluated in &scene.objects {
        if !selected_object_ids.contains(&evaluated.id) {
            continue;
        }

        let Some(local_bounds) =
            evaluated_local_bounds(evaluated.id, &evaluated.content, &mut runtime_bounds)
        else {
            continue;
        };
        let transform = ObjectTransform2d::new(
            evaluated.transform.position,
            evaluated.transform.scale,
            evaluated.transform.rotation_degrees,
            evaluated.transform.anchor,
        );
        let Some(bounds) = transformed_bounds(transform, local_bounds) else {
            continue;
        };

        combined = Some(match combined {
            None => bounds,
            Some(current) => {
                let min_x = current.min.x().min(bounds.min.x());
                let min_y = current.min.y().min(bounds.min.y());
                let max_x =
                    (current.min.x() + current.size.x()).max(bounds.min.x() + bounds.size.x());
                let max_y =
                    (current.min.y() + current.size.y()).max(bounds.min.y() + bounds.size.y());
                LocalBounds2d::new(
                    Vec2::new(min_x, min_y).ok()?,
                    Vec2::new(max_x - min_x, max_y - min_y).ok()?,
                )
            }
        });
    }

    combined
}

#[must_use]
pub fn selection_overlay_geometry<F>(
    scene: &EvaluatedScene,
    selected_object_ids: &[ObjectId],
    mut runtime_bounds: F,
) -> Option<SelectionOverlayGeometry>
where
    F: FnMut(ObjectId, &EvaluatedObjectContent) -> Option<RuntimeHitBounds>,
{
    if selected_object_ids.is_empty() {
        return None;
    }

    if selected_object_ids.len() == 1 {
        let object_id = selected_object_ids[0];
        let evaluated = scene.objects.iter().find(|object| object.id == object_id)?;
        let local_bounds =
            evaluated_local_bounds(evaluated.id, &evaluated.content, &mut runtime_bounds)?;
        let transform = ObjectTransform2d::new(
            evaluated.transform.position,
            evaluated.transform.scale,
            evaluated.transform.rotation_degrees,
            evaluated.transform.anchor,
        );
        return Some(SelectionOverlayGeometry {
            corners: transformed_corners(transform, local_bounds)?,
            anchor: evaluated.transform.position,
        });
    }

    let bounds = selected_objects_bounds(scene, selected_object_ids, |object_id, content| {
        runtime_bounds(object_id, content)
    })?;
    Some(SelectionOverlayGeometry {
        corners: bounds_corners(bounds)?,
        anchor: Vec2::new(
            bounds.min.x() + bounds.size.x() * 0.5,
            bounds.min.y() + bounds.size.y() * 0.5,
        )
        .ok()?,
    })
}

#[must_use]
pub fn objects_intersecting_box<F>(
    project: &Project,
    scene: &EvaluatedScene,
    selection_bounds: LocalBounds2d,
    mut runtime_bounds: F,
) -> Vec<ObjectId>
where
    F: FnMut(ObjectId, &EvaluatedObjectContent) -> Option<RuntimeHitBounds>,
{
    let evaluated_by_id: HashMap<ObjectId, &EvaluatedObject> = scene
        .objects
        .iter()
        .map(|object| (object.id, object))
        .collect();
    let mut selected = Vec::new();

    for object in &project.composition.objects {
        if !object.visible || object.locked {
            continue;
        }

        let Some(evaluated) = evaluated_by_id.get(&object.id).copied() else {
            continue;
        };
        let Some(local_bounds) =
            evaluated_local_bounds(evaluated.id, &evaluated.content, &mut runtime_bounds)
        else {
            continue;
        };

        let transform = ObjectTransform2d::new(
            evaluated.transform.position,
            evaluated.transform.scale,
            evaluated.transform.rotation_degrees,
            evaluated.transform.anchor,
        );
        let Some(composition_bounds) = transformed_bounds(transform, local_bounds) else {
            continue;
        };

        if bounds_intersect(selection_bounds, composition_bounds) {
            selected.push(evaluated.id);
        }
    }

    selected
}

#[must_use]
pub fn pick_topmost_object<F>(
    project: &Project,
    scene: &EvaluatedScene,
    composition_point: Vec2,
    mut runtime_bounds: F,
) -> Option<ObjectId>
where
    F: FnMut(ObjectId, &EvaluatedObjectContent) -> Option<RuntimeHitBounds>,
{
    let evaluated_by_id: HashMap<ObjectId, &EvaluatedObject> = scene
        .objects
        .iter()
        .map(|object| (object.id, object))
        .collect();

    for object in project.composition.objects.iter().rev() {
        if !object.visible || object.locked {
            continue;
        }

        let Some(evaluated) = evaluated_by_id.get(&object.id).copied() else {
            continue;
        };
        let transform = ObjectTransform2d::new(
            evaluated.transform.position,
            evaluated.transform.scale,
            evaluated.transform.rotation_degrees,
            evaluated.transform.anchor,
        );

        let hit = match &evaluated.content {
            EvaluatedObjectContent::Rectangle { size, .. } => {
                hit_test_rectangle(composition_point, transform, *size)
            }
            EvaluatedObjectContent::Ellipse { size, .. } => {
                hit_test_ellipse(composition_point, transform, *size)
            }
            EvaluatedObjectContent::Image { .. } => matches!(
                runtime_bounds(evaluated.id, &evaluated.content),
                Some(RuntimeHitBounds::ImageIntrinsicSize(size))
                    if hit_test_image_bounds(composition_point, transform, size)
            ),
            EvaluatedObjectContent::Text { .. } => matches!(
                runtime_bounds(evaluated.id, &evaluated.content),
                Some(RuntimeHitBounds::TextLayout(bounds))
                    if hit_test_text_layout_bounds(composition_point, transform, bounds)
            ),
        };

        if hit {
            return Some(evaluated.id);
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::{
        objects_intersecting_box, pick_topmost_object, selected_objects_bounds,
        selection_overlay_geometry,
    };
    use rhythm_core::{
        animation::Animated,
        domain::{LinearRgba, Vec2},
        ids::ObjectId,
        project::{
            Object, ObjectContent, Project, ProjectSettings, RectangleObject, TransformAnimation,
        },
        time::{GridOffsetNs, ProjectTimeNs, TempoMap},
    };
    use rhythm_engine::scene_eval::evaluate_scene;

    fn rectangle(id: u64, name: &str, visible: bool, locked: bool) -> Object {
        Object {
            id: ObjectId::new(id).expect("object id"),
            name: name.to_owned(),
            visible,
            locked,
            transform: TransformAnimation::new(
                Animated::new_static(Vec2::new(200.0, 100.0).expect("position")),
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
            effects: Vec::new(),
        }
    }

    fn project_with_overlapping_rectangles() -> Project {
        let mut project = Project::new(
            "Viewport Pick",
            ProjectSettings::default(),
            TempoMap::unset(GridOffsetNs::new(0)),
        );
        project
            .composition
            .objects
            .push(rectangle(1, "Back", true, false));
        project
            .composition
            .objects
            .push(rectangle(2, "Front", true, false));
        project.next_entity_id = 3;
        project
    }

    #[test]
    fn single_selection_overlay_preserves_rotated_bounds_and_anchor() {
        let mut project = project_with_overlapping_rectangles();
        project.composition.objects[0].transform.position =
            Animated::new_static(Vec2::new(300.0, 200.0).expect("position"));
        project.composition.objects[0].transform.rotation_degrees = Animated::new_static(90.0);
        let scene = evaluate_scene(&project, ProjectTimeNs::new(0)).expect("scene");

        let overlay = selection_overlay_geometry(
            &scene,
            &[ObjectId::new(1).expect("object id")],
            |_, _| None,
        )
        .expect("selection overlay");

        assert_eq!(
            overlay.corners,
            [
                Vec2::new(325.0, 150.0).expect("corner"),
                Vec2::new(325.0, 250.0).expect("corner"),
                Vec2::new(275.0, 250.0).expect("corner"),
                Vec2::new(275.0, 150.0).expect("corner"),
            ]
        );
        assert_eq!(overlay.anchor, Vec2::new(300.0, 200.0).expect("anchor"));
    }

    #[test]
    fn multi_selection_overlay_uses_combined_bounds_center() {
        let mut project = project_with_overlapping_rectangles();
        project.composition.objects[1].transform.position =
            Animated::new_static(Vec2::new(400.0, 200.0).expect("position"));
        let scene = evaluate_scene(&project, ProjectTimeNs::new(0)).expect("scene");

        let overlay = selection_overlay_geometry(
            &scene,
            &[
                ObjectId::new(1).expect("object id"),
                ObjectId::new(2).expect("object id"),
            ],
            |_, _| None,
        )
        .expect("selection overlay");

        assert_eq!(
            overlay.corners,
            [
                Vec2::new(150.0, 75.0).expect("corner"),
                Vec2::new(450.0, 75.0).expect("corner"),
                Vec2::new(450.0, 225.0).expect("corner"),
                Vec2::new(150.0, 225.0).expect("corner"),
            ]
        );
        assert_eq!(overlay.anchor, Vec2::new(300.0, 150.0).expect("anchor"));
    }

    #[test]
    fn selected_object_bounds_combine_transformed_selection() {
        let mut project = project_with_overlapping_rectangles();
        project.composition.objects[1].transform.position =
            Animated::new_static(Vec2::new(400.0, 200.0).expect("position"));
        let scene = evaluate_scene(&project, ProjectTimeNs::new(0)).expect("scene");

        let bounds = selected_objects_bounds(
            &scene,
            &[
                ObjectId::new(1).expect("object id"),
                ObjectId::new(2).expect("object id"),
            ],
            |_, _| None,
        )
        .expect("combined bounds");

        assert_eq!(bounds.min, Vec2::new(150.0, 75.0).expect("bounds min"));
        assert_eq!(bounds.size, Vec2::new(300.0, 150.0).expect("bounds size"));
    }

    #[test]
    fn box_selection_returns_intersecting_visible_unlocked_objects() {
        let mut project = project_with_overlapping_rectangles();
        project.composition.objects[1].transform.position =
            Animated::new_static(Vec2::new(400.0, 100.0).expect("position"));
        project
            .composition
            .objects
            .push(rectangle(3, "Locked", true, true));
        project.next_entity_id = 4;
        let scene = evaluate_scene(&project, ProjectTimeNs::new(0)).expect("scene");

        let selected = objects_intersecting_box(
            &project,
            &scene,
            rhythm_core::geometry::LocalBounds2d::new(
                Vec2::new(140.0, 60.0).expect("box min"),
                Vec2::new(140.0, 100.0).expect("box size"),
            ),
            |_, _| None,
        );

        assert_eq!(selected, vec![ObjectId::new(1).expect("object id")]);
    }

    #[test]
    fn picker_returns_topmost_hit_in_painter_order() {
        let project = project_with_overlapping_rectangles();
        let scene = evaluate_scene(&project, ProjectTimeNs::new(0)).expect("scene");

        let picked = pick_topmost_object(
            &project,
            &scene,
            Vec2::new(200.0, 100.0).expect("point"),
            |_, _| None,
        );

        assert_eq!(picked, ObjectId::new(2));
    }

    #[test]
    fn picker_skips_locked_and_hidden_objects() {
        let mut project = project_with_overlapping_rectangles();
        project.composition.objects[1].locked = true;
        let scene = evaluate_scene(&project, ProjectTimeNs::new(0)).expect("scene");

        assert_eq!(
            pick_topmost_object(
                &project,
                &scene,
                Vec2::new(200.0, 100.0).expect("point"),
                |_, _| None,
            ),
            ObjectId::new(1)
        );

        project.composition.objects[1].locked = false;
        project.composition.objects[1].visible = false;
        let scene = evaluate_scene(&project, ProjectTimeNs::new(0)).expect("scene");
        assert_eq!(
            pick_topmost_object(
                &project,
                &scene,
                Vec2::new(200.0, 100.0).expect("point"),
                |_, _| None,
            ),
            ObjectId::new(1)
        );
    }

    #[test]
    fn picker_returns_none_for_empty_space() {
        let project = project_with_overlapping_rectangles();
        let scene = evaluate_scene(&project, ProjectTimeNs::new(0)).expect("scene");

        assert_eq!(
            pick_topmost_object(
                &project,
                &scene,
                Vec2::new(20.0, 20.0).expect("point"),
                |_, _| None,
            ),
            None
        );
    }
}
