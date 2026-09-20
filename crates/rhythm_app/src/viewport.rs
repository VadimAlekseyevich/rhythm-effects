use std::collections::HashMap;

use rhythm_core::{
    domain::Vec2,
    geometry::{
        LocalBounds2d, ObjectTransform2d, hit_test_ellipse, hit_test_image_bounds,
        hit_test_rectangle, hit_test_text_layout_bounds,
    },
    ids::ObjectId,
    project::Project,
};
use rhythm_engine::scene_eval::{EvaluatedObject, EvaluatedObjectContent, EvaluatedScene};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RuntimeHitBounds {
    ImageIntrinsicSize(Vec2),
    TextLayout(LocalBounds2d),
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
    let evaluated_by_id: HashMap<ObjectId, &EvaluatedObject> =
        scene.objects.iter().map(|object| (object.id, object)).collect();

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
    use super::pick_topmost_object;
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
