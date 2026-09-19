use crate::{ids::KeyframeId, time::MusicalTick};

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
            Err(AnimationInvariantError::DuplicateTick(MusicalTick::new(240)))
        );

        let ticks: Vec<_> = animated
            .keyframes()
            .iter()
            .map(|keyframe| keyframe.tick.get())
            .collect();
        assert_eq!(ticks, vec![0, 240, 480]);
    }
}
