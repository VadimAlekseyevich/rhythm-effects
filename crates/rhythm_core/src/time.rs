pub const PPQ: i64 = 960;

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

pub const MIN_BPM_MICROS: u64 = 1_000_000;
pub const MAX_BPM_MICROS: u64 = 1_000_000_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BpmError {
    OutOfRange,
    InvalidFormat,
    TooManyFractionDigits,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BpmMicros(u64);

impl BpmMicros {
    pub const fn new(value: u64) -> Result<Self, BpmError> {
        if value < MIN_BPM_MICROS || value > MAX_BPM_MICROS {
            return Err(BpmError::OutOfRange);
        }

        Ok(Self(value))
    }

    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }

    pub fn parse_decimal(input: &str) -> Result<Self, BpmError> {
        let input = input.trim();
        if input.is_empty() || input.starts_with('+') || input.starts_with('-') {
            return Err(BpmError::InvalidFormat);
        }

        let mut parts = input.split('.');
        let whole = parts.next().ok_or(BpmError::InvalidFormat)?;
        let fraction = parts.next();
        if parts.next().is_some() || whole.is_empty() || !whole.bytes().all(|b| b.is_ascii_digit()) {
            return Err(BpmError::InvalidFormat);
        }

        let whole_value = whole
            .parse::<u64>()
            .map_err(|_| BpmError::InvalidFormat)?;
        let mut micros = whole_value
            .checked_mul(1_000_000)
            .ok_or(BpmError::OutOfRange)?;

        if let Some(fraction) = fraction {
            if fraction.is_empty() || !fraction.bytes().all(|b| b.is_ascii_digit()) {
                return Err(BpmError::InvalidFormat);
            }
            if fraction.len() > 6 {
                return Err(BpmError::TooManyFractionDigits);
            }

            let fractional_value = fraction
                .parse::<u64>()
                .map_err(|_| BpmError::InvalidFormat)?;
            let scale = 10_u64.pow((6 - fraction.len()) as u32);
            micros = micros
                .checked_add(fractional_value * scale)
                .ok_or(BpmError::OutOfRange)?;
        }

        Self::new(micros)
    }

    #[must_use]
    pub fn format_decimal(self) -> String {
        let whole = self.0 / 1_000_000;
        let fraction = self.0 % 1_000_000;
        if fraction == 0 {
            return whole.to_string();
        }

        let mut fractional = format!("{fraction:06}");
        while fractional.ends_with('0') {
            fractional.pop();
        }

        format!("{whole}.{fractional}")
    }
}

#[cfg(test)]
mod tests {
    use super::{
        AudioFramePosition, BpmMicros, DurationNs, GridOffsetNs, MusicalTick, ProjectTimeNs,
        SampleRate,
    };

    #[test]
    fn ppq_is_schema_constant() {
        assert_eq!(super::PPQ, 960);
    }

    #[test]
    fn bpm_range_is_inclusive_and_validated() {
        assert_eq!(
            BpmMicros::new(1_000_000).map(BpmMicros::get),
            Ok(1_000_000)
        );
        assert_eq!(
            BpmMicros::new(1_000_000_000).map(BpmMicros::get),
            Ok(1_000_000_000)
        );
        assert_eq!(
            BpmMicros::new(999_999),
            Err(super::BpmError::OutOfRange)
        );
        assert_eq!(
            BpmMicros::new(1_000_000_001),
            Err(super::BpmError::OutOfRange)
        );
    }

    #[test]
    fn bpm_decimal_parse_and_format_is_exact() {
        let cases = [
            ("120", 120_000_000, "120"),
            ("128.5", 128_500_000, "128.5"),
            ("174.123456", 174_123_456, "174.123456"),
            ("1.000001", 1_000_001, "1.000001"),
            ("1000.000000", 1_000_000_000, "1000"),
        ];

        for (input, expected_micros, expected_text) in cases {
            let bpm = BpmMicros::parse_decimal(input).expect("valid BPM text");
            assert_eq!(bpm.get(), expected_micros);
            assert_eq!(bpm.format_decimal(), expected_text);
        }
    }

    #[test]
    fn bpm_decimal_parser_rejects_invalid_precision_and_range() {
        assert_eq!(
            BpmMicros::parse_decimal("120.1234567"),
            Err(super::BpmError::TooManyFractionDigits)
        );
        assert_eq!(
            BpmMicros::parse_decimal("0.5"),
            Err(super::BpmError::OutOfRange)
        );
        assert_eq!(
            BpmMicros::parse_decimal("1000.000001"),
            Err(super::BpmError::OutOfRange)
        );
        assert_eq!(
            BpmMicros::parse_decimal("12e1"),
            Err(super::BpmError::InvalidFormat)
        );
        assert_eq!(
            BpmMicros::parse_decimal("120."),
            Err(super::BpmError::InvalidFormat)
        );
    }

    #[test]
    fn time_units_keep_raw_values_without_cross_unit_conversion() {
        assert_eq!(MusicalTick::new(-960).get(), -960);
        assert_eq!(ProjectTimeNs::new(-1).get(), -1);
        assert_eq!(DurationNs::new(10).get(), 10);
        assert_eq!(GridOffsetNs::new(350_000_000).get(), 350_000_000);
        assert_eq!(SampleRate::new(48_000).get(), 48_000);
        assert_eq!(AudioFramePosition::new(12_345).get(), 12_345);
        assert_eq!(
            BpmMicros::new(120_000_000).expect("valid BPM").get(),
            120_000_000
        );
    }
}
