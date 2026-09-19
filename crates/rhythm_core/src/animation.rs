use crate::{
    domain::{LinearRgba, Vec2},
    ids::KeyframeId,
    time::MusicalTick,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BezierEasing {
    pub(crate) x1: f32,
    pub(crate) y1: f32,
    pub(crate) x2: f32,
    pub(crate) y2: f32,
}

impl BezierEasing {
    pub(crate) const fn from_unchecked(x1: f32, y1: f32, x2: f32, y2: f32) -> Self {
        Self { x1, y1, x2, y2 }
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

#[must_use]
pub fn interpolate_rotation_degrees(from: f32, to: f32, progress: f64) -> f32 {
    interpolate_linear_f32(from, to, progress)
}

#[must_use]
pub fn interpolate_linear_rgba(
    from: LinearRgba,
    to: LinearRgba,
    progress: f64,
) -> LinearRgba {
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
