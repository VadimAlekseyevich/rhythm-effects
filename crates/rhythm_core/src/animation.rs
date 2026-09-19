use crate::{
    domain::{LinearRgba, Vec2},
    ids::KeyframeId,
    time::MusicalTick,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BezierEasingError {
    NonFinite,
    ControlPointOutOfRange,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BezierEasing {
    x1: f32,
    y1: f32,
    x2: f32,
    y2: f32,
}

impl BezierEasing {
    pub fn new(x1: f32, y1: f32, x2: f32, y2: f32) -> Result<Self, BezierEasingError> {
        if !x1.is_finite() || !y1.is_finite() || !x2.is_finite() || !y2.is_finite() {
            return Err(BezierEasingError::NonFinite);
        }
        if ![x1, y1, x2, y2]
            .into_iter()
            .all(|value| (0.0..=1.0).contains(&value))
        {
            return Err(BezierEasingError::ControlPointOutOfRange);
        }

        Ok(Self { x1, y1, x2, y2 })
    }

    #[must_use]
    pub const fn x1(self) -> f32 {
        self.x1
    }

    #[must_use]
    pub const fn y1(self) -> f32 {
        self.y1
    }

    #[must_use]
    pub const fn x2(self) -> f32 {
        self.x2
    }

    #[must_use]
    pub const fn y2(self) -> f32 {
        self.y2
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Interpolation {
    Hold,
    Linear,
    CubicBezier(BezierEasing),
}

impl Default for Interpolation {
    fn default() -> Self {
        Self::Linear
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Keyframe<T> {
    pub id: KeyframeId,
    pub tick: MusicalTick,
    pub value: T,
    pub interpolation: Interpolation,
}

impl<T> Keyframe<T> {
    #[must_use]
    pub const fn new(
        id: KeyframeId,
        tick: MusicalTick,
        value: T,
        interpolation: Interpolation,
    ) -> Self {
        Self {
            id,
            tick,
            value,
            interpolation,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnimationInvariantError {
    DuplicateTick(MusicalTick),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Animated<T> {
    base_value: T,
    keyframes: Vec<Keyframe<T>>,
}

impl<T> Animated<T> {
    #[must_use]
    pub const fn new_static(base_value: T) -> Self {
        Self {
            base_value,
            keyframes: Vec::new(),
        }
    }

    pub fn with_keyframes(
        base_value: T,
        mut keyframes: Vec<Keyframe<T>>,
    ) -> Result<Self, AnimationInvariantError> {
        keyframes.sort_by_key(|keyframe| keyframe.tick);

        for pair in keyframes.windows(2) {
            if pair[0].tick == pair[1].tick {
                return Err(AnimationInvariantError::DuplicateTick(pair[0].tick));
            }
        }

        Ok(Self {
            base_value,
            keyframes,
        })
    }

    #[must_use]
    pub const fn base_value(&self) -> &T {
        &self.base_value
    }

    pub fn base_value_mut(&mut self) -> &mut T {
        &mut self.base_value
    }

    #[must_use]
    pub fn keyframes(&self) -> &[Keyframe<T>] {
        &self.keyframes
    }

    pub fn insert_keyframe(
        &mut self,
        keyframe: Keyframe<T>,
    ) -> Result<usize, AnimationInvariantError> {
        match self
            .keyframes
            .binary_search_by_key(&keyframe.tick, |existing| existing.tick)
        {
            Ok(_) => Err(AnimationInvariantError::DuplicateTick(keyframe.tick)),
            Err(index) => {
                self.keyframes.insert(index, keyframe);
                Ok(index)
            }
        }
    }

    #[must_use]
    pub fn range_value(&self, continuous_tick: f64) -> Option<&T> {
        let first = match self.keyframes.first() {
            Some(first) => first,
            None => return Some(&self.base_value),
        };
        let last = self.keyframes.last().expect("first keyframe exists");

        if continuous_tick <= first.tick.get() as f64 {
            return Some(&first.value);
        }
        if continuous_tick >= last.tick.get() as f64 {
            return Some(&last.value);
        }

        self.keyframes
            .binary_search_by(|keyframe| {
                (keyframe.tick.get() as f64).total_cmp(&continuous_tick)
            })
            .ok()
            .map(|index| &self.keyframes[index].value)
    }
}

fn clamp_progress(progress: f64) -> f64 {
    progress.clamp(0.0, 1.0)
}

#[must_use]
pub fn interpolate_linear_f32(from: f32, to: f32, progress: f64) -> f32 {
    let progress = clamp_progress(progress);
    (f64::from(from) + (f64::from(to) - f64::from(from)) * progress) as f32
}

#[must_use]
pub fn interpolate_linear_vec2(from: Vec2, to: Vec2, progress: f64) -> Vec2 {
    Vec2::new(
        interpolate_linear_f32(from.x(), to.x(), progress),
        interpolate_linear_f32(from.y(), to.y(), progress),
    )
    .expect("linear interpolation of finite Vec2 endpoints stays finite")
}

fn cubic_bezier_coordinate(t: f64, p1: f64, p2: f64) -> f64 {
    let one_minus_t = 1.0 - t;
    3.0 * one_minus_t * one_minus_t * t * p1
        + 3.0 * one_minus_t * t * t * p2
        + t * t * t
}

#[must_use]
pub fn evaluate_bezier_easing(easing: BezierEasing, progress: f64) -> f64 {
    let progress = clamp_progress(progress);
    if progress <= 0.0 {
        return 0.0;
    }
    if progress >= 1.0 {
        return 1.0;
    }

    let mut low = 0.0_f64;
    let mut high = 1.0_f64;

    for _ in 0..32 {
        let t = (low + high) * 0.5;
        let x = cubic_bezier_coordinate(t, f64::from(easing.x1), f64::from(easing.x2));
        if x < progress {
            low = t;
        } else {
            high = t;
        }
    }

    let t = (low + high) * 0.5;
    cubic_bezier_coordinate(t, f64::from(easing.y1), f64::from(easing.y2))
        .clamp(0.0, 1.0)
}

#[must_use]
pub fn interpolate_rotation_degrees(from: f32, to: f32, progress: f64) -> f32 {
    interpolate_linear_f32(from, to, progress)
}

#[must_use]
pub fn interpolate_linear_rgba(from: LinearRgba, to: LinearRgba, progress: f64) -> LinearRgba {
    LinearRgba::new(
        interpolate_linear_f32(from.r(), to.r(), progress),
        interpolate_linear_f32(from.g(), to.g(), progress),
        interpolate_linear_f32(from.b(), to.b(), progress),
        interpolate_linear_f32(from.a(), to.a(), progress),
    )
    .expect("linear interpolation of valid LinearRgba endpoints stays valid")
}

#[must_use]
pub fn interpolate_hold<T: Clone>(from: &T, to: &T, progress: f64) -> T {
    if progress >= 1.0 {
        to.clone()
    } else {
        from.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::{Animated, AnimationInvariantError, Interpolation, Keyframe};
    use crate::{ids::KeyframeId, time::MusicalTick};

    fn keyframe(id: u64, tick: i64, value: f32) -> Keyframe<f32> {
        Keyframe::new(
            KeyframeId::new(id).expect("nonzero keyframe id"),
            MusicalTick::new(tick),
            value,
            Interpolation::Linear,
        )
    }

    #[test]
    fn range_behavior_uses_base_and_endpoint_key_values() {
        let static_value = Animated::new_static(7.0);
        assert_eq!(static_value.range_value(-100.0), Some(&7.0));
        assert_eq!(static_value.range_value(100.0), Some(&7.0));

        let animated = Animated::with_keyframes(
            0.0,
            vec![
                keyframe(1, 0, 10.0),
                keyframe(2, 240, 20.0),
                keyframe(3, 480, 30.0),
            ],
        )
        .expect("unique ticks");

        assert_eq!(animated.range_value(-1.0), Some(&10.0));
        assert_eq!(animated.range_value(0.0), Some(&10.0));
        assert_eq!(animated.range_value(240.0), Some(&20.0));
        assert_eq!(animated.range_value(480.0), Some(&30.0));
        assert_eq!(animated.range_value(999.0), Some(&30.0));
        assert_eq!(animated.range_value(120.0), None);
    }

    #[test]
    fn bezier_easing_is_deterministic_and_preserves_endpoints() {
        let linear = super::BezierEasing::new(0.0, 0.0, 1.0, 1.0).expect("valid easing");
        assert_eq!(super::evaluate_bezier_easing(linear, 0.0), 0.0);
        assert_eq!(super::evaluate_bezier_easing(linear, 1.0), 1.0);
        assert!((super::evaluate_bezier_easing(linear, 0.5) - 0.5).abs() < 1e-9);

        let ease = super::BezierEasing::new(0.25, 0.1, 0.25, 1.0).expect("valid easing");
        let first = super::evaluate_bezier_easing(ease, 0.5);
        let second = super::evaluate_bezier_easing(ease, 0.5);
        assert_eq!(first, second);
        assert!((first - 0.802_403_4).abs() < 0.000_01);
    }

    #[test]
    fn bezier_easing_constrains_all_handles_to_unit_square() {
        let easing = super::BezierEasing::new(0.25, 0.1, 0.25, 1.0).expect("valid easing");
        assert_eq!(easing.x1(), 0.25);
        assert_eq!(easing.y1(), 0.1);
        assert_eq!(easing.x2(), 0.25);
        assert_eq!(easing.y2(), 1.0);

        assert_eq!(
            super::BezierEasing::new(-0.01, 0.0, 1.0, 1.0),
            Err(super::BezierEasingError::ControlPointOutOfRange)
        );
        assert_eq!(
            super::BezierEasing::new(0.0, 0.0, 1.01, 1.0),
            Err(super::BezierEasingError::ControlPointOutOfRange)
        );
        assert_eq!(
            super::BezierEasing::new(0.0, f32::NAN, 1.0, 1.0),
            Err(super::BezierEasingError::NonFinite)
        );
    }

    #[test]
    fn rotation_interpolation_uses_direct_numeric_degrees() {
        assert_eq!(super::interpolate_rotation_degrees(0.0, 360.0, 0.5), 180.0);
        assert_eq!(
            super::interpolate_rotation_degrees(350.0, 370.0, 0.5),
            360.0
        );
        assert_eq!(
            super::interpolate_rotation_degrees(10.0, -350.0, 0.5),
            -170.0
        );
    }

    #[test]
    fn linear_rgba_interpolation_operates_in_linear_light_values() {
        let from = crate::domain::LinearRgba::new(0.0, 0.0, 0.0, 0.0).expect("valid color");
        let to = crate::domain::LinearRgba::new(1.0, 0.5, 0.25, 1.0).expect("valid color");
        let midpoint = super::interpolate_linear_rgba(from, to, 0.5);

        assert_eq!(midpoint.r(), 0.5);
        assert_eq!(midpoint.g(), 0.25);
        assert_eq!(midpoint.b(), 0.125);
        assert_eq!(midpoint.a(), 0.5);
    }

    #[test]
    fn linear_interpolation_supports_scalar_and_vec2() {
        assert_eq!(super::interpolate_linear_f32(0.0, 10.0, 0.5), 5.0);
        assert_eq!(super::interpolate_linear_f32(0.0, 10.0, -1.0), 0.0);
        assert_eq!(super::interpolate_linear_f32(0.0, 10.0, 2.0), 10.0);

        let from = crate::domain::Vec2::new(-10.0, 20.0).expect("finite vector");
        let to = crate::domain::Vec2::new(10.0, -20.0).expect("finite vector");
        let midpoint = super::interpolate_linear_vec2(from, to, 0.5);

        assert_eq!(midpoint.x(), 0.0);
        assert_eq!(midpoint.y(), 0.0);
    }

    #[test]
    fn hold_interpolation_switches_only_at_segment_end() {
        assert_eq!(super::interpolate_hold(&10, &20, -1.0), 10);
        assert_eq!(super::interpolate_hold(&10, &20, 0.0), 10);
        assert_eq!(super::interpolate_hold(&10, &20, 0.999_999), 10);
        assert_eq!(super::interpolate_hold(&10, &20, 1.0), 20);
        assert_eq!(super::interpolate_hold(&10, &20, 2.0), 20);
    }

    #[test]
    fn with_keyframes_sorts_by_tick() {
        let animated = Animated::with_keyframes(
            0.0,
            vec![
                keyframe(1, 960, 1.0),
                keyframe(2, -240, 2.0),
                keyframe(3, 0, 3.0),
            ],
        )
        .expect("unique ticks");

        let ticks: Vec<_> = animated
            .keyframes()
            .iter()
            .map(|keyframe| keyframe.tick.get())
            .collect();

        assert_eq!(ticks, vec![-240, 0, 960]);
    }

    #[test]
    fn duplicate_tick_is_rejected() {
        let result =
            Animated::with_keyframes(0.0, vec![keyframe(1, 240, 1.0), keyframe(2, 240, 2.0)]);

        assert_eq!(
            result,
            Err(AnimationInvariantError::DuplicateTick(MusicalTick::new(
                240
            )))
        );
    }

    #[test]
    fn insertion_preserves_sorted_unique_invariant() {
        let mut animated = Animated::new_static(0.0);

        assert_eq!(animated.insert_keyframe(keyframe(1, 480, 1.0)), Ok(0));
        assert_eq!(animated.insert_keyframe(keyframe(2, 0, 2.0)), Ok(0));
        assert_eq!(animated.insert_keyframe(keyframe(3, 240, 3.0)), Ok(1));
        assert_eq!(
            animated.insert_keyframe(keyframe(4, 240, 4.0)),
            Err(AnimationInvariantError::DuplicateTick(MusicalTick::new(
                240
            )))
        );

        let ticks: Vec<_> = animated
            .keyframes()
            .iter()
            .map(|keyframe| keyframe.tick.get())
            .collect();
        assert_eq!(ticks, vec![0, 240, 480]);
    }
}
