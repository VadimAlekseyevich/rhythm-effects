macro_rules! signed_time_type {
    ($name:ident) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
        pub struct $name(i64);

        impl $name {
            #[must_use]
            pub const fn new(value: i64) -> Self {
                Self(value)
            }

            #[must_use]
            pub const fn get(self) -> i64 {
                self.0
            }
        }
    };
}

macro_rules! unsigned_time_type {
    ($name:ident, $inner:ty) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
        pub struct $name($inner);

        impl $name {
            #[must_use]
            pub const fn new(value: $inner) -> Self {
                Self(value)
            }

            #[must_use]
            pub const fn get(self) -> $inner {
                self.0
            }
        }
    };
}

signed_time_type!(MusicalTick);
signed_time_type!(ProjectTimeNs);
signed_time_type!(GridOffsetNs);
unsigned_time_type!(DurationNs, u64);
unsigned_time_type!(SampleRate, u32);
unsigned_time_type!(AudioFramePosition, u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BpmMicros(u64);

impl BpmMicros {
    pub(crate) const fn from_micros_unchecked(value: u64) -> Self {
        Self(value)
    }

    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

#[cfg(test)]
mod tests {
    use super::{
        AudioFramePosition, BpmMicros, DurationNs, GridOffsetNs, MusicalTick, ProjectTimeNs,
        SampleRate,
    };

    #[test]
    fn time_units_keep_raw_values_without_cross_unit_conversion() {
        assert_eq!(MusicalTick::new(-960).get(), -960);
        assert_eq!(ProjectTimeNs::new(-1).get(), -1);
        assert_eq!(DurationNs::new(10).get(), 10);
        assert_eq!(GridOffsetNs::new(350_000_000).get(), 350_000_000);
        assert_eq!(SampleRate::new(48_000).get(), 48_000);
        assert_eq!(AudioFramePosition::new(12_345).get(), 12_345);
        assert_eq!(BpmMicros::from_micros_unchecked(120_000_000).get(), 120_000_000);
    }
}
