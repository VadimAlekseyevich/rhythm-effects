pub const PPQ: i64 = 960;

pub const MVP_BEAT_DIVISIONS: [u16; 10] = [1, 2, 3, 4, 6, 8, 12, 16, 24, 32];

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BeatDivision {
    parts_per_beat: u16,
}

impl BeatDivision {
    #[must_use]
    pub const fn new(parts_per_beat: u16) -> Option<Self> {
        match parts_per_beat {
            1 | 2 | 3 | 4 | 6 | 8 | 12 | 16 | 24 | 32 => Some(Self { parts_per_beat }),
            _ => None,
        }
    }

    #[must_use]
    pub const fn parts_per_beat(self) -> u16 {
        self.parts_per_beat
    }

    #[must_use]
    pub const fn ticks_per_step(self) -> i64 {
        PPQ / self.parts_per_beat as i64
    }
}

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeConversionError {
    TempoUnavailable,
    NonFiniteTickPosition,
    Overflow,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TempoSegment {
    start_tick: MusicalTick,
    bpm: BpmMicros,
    meter: TimeSignature,
}

impl TempoSegment {
    #[must_use]
    pub const fn start_tick(self) -> MusicalTick {
        self.start_tick
    }

    #[must_use]
    pub const fn bpm(self) -> BpmMicros {
        self.bpm
    }

    #[must_use]
    pub const fn meter(self) -> TimeSignature {
        self.meter
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TempoMap {
    grid_offset: GridOffsetNs,
    segments: Vec<TempoSegment>,
}

impl TempoMap {
    #[must_use]
    pub fn unset(grid_offset: GridOffsetNs) -> Self {
        Self {
            grid_offset,
            segments: Vec::new(),
        }
    }

    #[must_use]
    pub fn with_initial_tempo(
        grid_offset: GridOffsetNs,
        bpm: BpmMicros,
        meter: TimeSignature,
    ) -> Self {
        Self {
            grid_offset,
            segments: vec![TempoSegment {
                start_tick: MusicalTick::new(0),
                bpm,
                meter,
            }],
        }
    }

    #[must_use]
    pub const fn grid_offset(&self) -> GridOffsetNs {
        self.grid_offset
    }

    #[must_use]
    pub fn segments(&self) -> &[TempoSegment] {
        &self.segments
    }

    #[must_use]
    pub fn initial_segment(&self) -> Option<TempoSegment> {
        self.segments.first().copied()
    }

    pub fn continuous_tick_position(
        &self,
        project_time: ProjectTimeNs,
    ) -> Result<f64, TimeConversionError> {
        let segment = self
            .initial_segment()
            .ok_or(TimeConversionError::TempoUnavailable)?;

        let delta_ns = i128::from(project_time.get()) - i128::from(self.grid_offset.get());
        let numerator = delta_ns * i128::from(segment.bpm().get()) * i128::from(PPQ);
        let denominator = 60_000_000_000_i128 * 1_000_000_i128;

        Ok(numerator as f64 / denominator as f64)
    }

    pub fn project_time_for_tick(
        &self,
        tick: MusicalTick,
    ) -> Result<ProjectTimeNs, TimeConversionError> {
        let segment = self
            .initial_segment()
            .ok_or(TimeConversionError::TempoUnavailable)?;

        let numerator = i128::from(tick.get()) * 60_000_000_000_i128 * 1_000_000_i128;
        let denominator = i128::from(segment.bpm().get()) * i128::from(PPQ);
        let delta_ns = div_round_nearest_ties_away_from_zero(numerator, denominator);
        let project_ns = i128::from(self.grid_offset.get()) + delta_ns;
        let project_ns = i64::try_from(project_ns).map_err(|_| TimeConversionError::Overflow)?;

        Ok(ProjectTimeNs::new(project_ns))
    }
}

pub fn floor_tick_position_to_grid(
    tick_position: f64,
    division: BeatDivision,
) -> Result<MusicalTick, TimeConversionError> {
    grid_bound_tick_position(tick_position, division, false)
}

pub fn ceil_tick_position_to_grid(
    tick_position: f64,
    division: BeatDivision,
) -> Result<MusicalTick, TimeConversionError> {
    grid_bound_tick_position(tick_position, division, true)
}

fn grid_bound_tick_position(
    tick_position: f64,
    division: BeatDivision,
    ceil: bool,
) -> Result<MusicalTick, TimeConversionError> {
    if !tick_position.is_finite() {
        return Err(TimeConversionError::NonFiniteTickPosition);
    }

    let step = division.ticks_per_step() as f64;
    let scaled = tick_position / step;
    let snapped = if ceil {
        scaled.ceil() * step
    } else {
        scaled.floor() * step
    };

    if snapped < i64::MIN as f64 || snapped > i64::MAX as f64 {
        return Err(TimeConversionError::Overflow);
    }

    Ok(MusicalTick::new(snapped as i64))
}

pub fn snap_tick_position_to_grid(
    tick_position: f64,
    division: BeatDivision,
) -> Result<MusicalTick, TimeConversionError> {
    if !tick_position.is_finite() {
        return Err(TimeConversionError::NonFiniteTickPosition);
    }

    let step = division.ticks_per_step() as f64;
    let lower = (tick_position / step).floor() * step;
    let upper = lower + step;
    let lower_distance = tick_position - lower;
    let upper_distance = upper - tick_position;
    let snapped = if lower_distance < upper_distance {
        lower
    } else {
        upper
    };

    if snapped < i64::MIN as f64 || snapped > i64::MAX as f64 {
        return Err(TimeConversionError::Overflow);
    }

    Ok(MusicalTick::new(snapped as i64))
}

fn div_round_nearest_ties_away_from_zero(numerator: i128, denominator: i128) -> i128 {
    debug_assert!(denominator > 0);

    let quotient = numerator / denominator;
    let remainder = numerator % denominator;
    let twice_abs_remainder = remainder.abs() * 2;

    if twice_abs_remainder < denominator {
        quotient
    } else if numerator >= 0 {
        quotient + 1
    } else {
        quotient - 1
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimeSignatureError {
    ZeroNumerator,
    ZeroDenominator,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TimeSignature {
    numerator: u8,
    denominator: u8,
}

impl TimeSignature {
    pub const fn new(numerator: u8, denominator: u8) -> Result<Self, TimeSignatureError> {
        if numerator == 0 {
            return Err(TimeSignatureError::ZeroNumerator);
        }
        if denominator == 0 {
            return Err(TimeSignatureError::ZeroDenominator);
        }

        Ok(Self {
            numerator,
            denominator,
        })
    }

    #[must_use]
    pub const fn numerator(self) -> u8 {
        self.numerator
    }

    #[must_use]
    pub const fn denominator(self) -> u8 {
        self.denominator
    }
}

impl Default for TimeSignature {
    fn default() -> Self {
        Self {
            numerator: 4,
            denominator: 4,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameRateError {
    ZeroNumerator,
    ZeroDenominator,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FrameRate {
    numerator: u32,
    denominator: u32,
}

impl FrameRate {
    pub fn new(numerator: u32, denominator: u32) -> Result<Self, FrameRateError> {
        if numerator == 0 {
            return Err(FrameRateError::ZeroNumerator);
        }
        if denominator == 0 {
            return Err(FrameRateError::ZeroDenominator);
        }

        let divisor = gcd_u32(numerator, denominator);
        Ok(Self {
            numerator: numerator / divisor,
            denominator: denominator / divisor,
        })
    }

    #[must_use]
    pub const fn numerator(self) -> u32 {
        self.numerator
    }

    #[must_use]
    pub const fn denominator(self) -> u32 {
        self.denominator
    }
}

pub fn project_time_for_frame(
    frame_index: u64,
    frame_rate: FrameRate,
) -> Result<ProjectTimeNs, TimeConversionError> {
    let numerator =
        i128::from(frame_index) * i128::from(frame_rate.denominator()) * 1_000_000_000_i128;
    let denominator = i128::from(frame_rate.numerator());
    let time_ns = div_round_nearest_ties_away_from_zero(numerator, denominator);
    let time_ns = i64::try_from(time_ns).map_err(|_| TimeConversionError::Overflow)?;

    Ok(ProjectTimeNs::new(time_ns))
}

const fn gcd_u32(mut a: u32, mut b: u32) -> u32 {
    while b != 0 {
        let remainder = a % b;
        a = b;
        b = remainder;
    }
    a
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
        if parts.next().is_some() || whole.is_empty() || !whole.bytes().all(|b| b.is_ascii_digit())
        {
            return Err(BpmError::InvalidFormat);
        }

        let whole_value = whole.parse::<u64>().map_err(|_| BpmError::InvalidFormat)?;
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
    fn grid_floor_and_ceil_handle_negative_positions() {
        let division = super::BeatDivision::new(4).expect("1/4 grid");

        assert_eq!(
            super::floor_tick_position_to_grid(-1.0, division)
                .expect("finite floor")
                .get(),
            -240
        );
        assert_eq!(
            super::ceil_tick_position_to_grid(-1.0, division)
                .expect("finite ceil")
                .get(),
            0
        );
        assert_eq!(
            super::floor_tick_position_to_grid(-240.0, division)
                .expect("exact floor")
                .get(),
            -240
        );
        assert_eq!(
            super::ceil_tick_position_to_grid(-240.0, division)
                .expect("exact ceil")
                .get(),
            -240
        );
        assert_eq!(
            super::floor_tick_position_to_grid(241.0, division)
                .expect("finite floor")
                .get(),
            240
        );
        assert_eq!(
            super::ceil_tick_position_to_grid(241.0, division)
                .expect("finite ceil")
                .get(),
            480
        );
    }

    #[test]
    fn grid_snap_half_ties_choose_later_tick() {
        let division = super::BeatDivision::new(4).expect("1/4 grid");

        assert_eq!(
            super::snap_tick_position_to_grid(120.0, division)
                .expect("finite snap")
                .get(),
            240
        );
        assert_eq!(
            super::snap_tick_position_to_grid(-120.0, division)
                .expect("finite snap")
                .get(),
            0
        );
        assert_eq!(
            super::snap_tick_position_to_grid(119.999, division)
                .expect("finite snap")
                .get(),
            0
        );
        assert_eq!(
            super::snap_tick_position_to_grid(120.001, division)
                .expect("finite snap")
                .get(),
            240
        );
    }

    #[test]
    fn grid_snap_rejects_non_finite_positions() {
        let division = super::BeatDivision::new(4).expect("1/4 grid");
        assert_eq!(
            super::snap_tick_position_to_grid(f64::NAN, division),
            Err(super::TimeConversionError::NonFiniteTickPosition)
        );
        assert_eq!(
            super::snap_tick_position_to_grid(f64::INFINITY, division),
            Err(super::TimeConversionError::NonFiniteTickPosition)
        );
    }

    #[test]
    fn project_time_maps_to_continuous_tick_position_without_rounding() {
        let bpm = BpmMicros::new(120_000_000).expect("valid BPM");
        let map = super::TempoMap::with_initial_tempo(
            GridOffsetNs::new(350_000_000),
            bpm,
            super::TimeSignature::default(),
        );

        let zero = map
            .continuous_tick_position(ProjectTimeNs::new(350_000_000))
            .expect("tempo available");
        let beat = map
            .continuous_tick_position(ProjectTimeNs::new(850_000_000))
            .expect("tempo available");
        let half_before = map
            .continuous_tick_position(ProjectTimeNs::new(100_000_000))
            .expect("tempo available");

        assert_eq!(zero, 0.0);
        assert_eq!(beat, 960.0);
        assert_eq!(half_before, -480.0);
    }

    #[test]
    fn tick_to_project_time_uses_exact_integer_math() {
        let bpm = BpmMicros::new(120_000_000).expect("valid BPM");
        let map = super::TempoMap::with_initial_tempo(
            GridOffsetNs::new(350_000_000),
            bpm,
            super::TimeSignature::default(),
        );

        assert_eq!(
            map.project_time_for_tick(MusicalTick::new(0))
                .expect("tempo available")
                .get(),
            350_000_000
        );
        assert_eq!(
            map.project_time_for_tick(MusicalTick::new(960))
                .expect("tempo available")
                .get(),
            850_000_000
        );
        assert_eq!(
            map.project_time_for_tick(MusicalTick::new(-960))
                .expect("tempo available")
                .get(),
            -150_000_000
        );
    }

    #[test]
    fn tick_to_project_time_reports_unset_tempo() {
        let map = super::TempoMap::unset(GridOffsetNs::new(0));
        assert_eq!(
            map.project_time_for_tick(MusicalTick::new(0)),
            Err(super::TimeConversionError::TempoUnavailable)
        );
    }

    #[test]
    fn tempo_map_initial_segment_starts_at_tick_zero() {
        let bpm = BpmMicros::new(120_000_000).expect("valid BPM");
        let map = super::TempoMap::with_initial_tempo(
            GridOffsetNs::new(350_000_000),
            bpm,
            super::TimeSignature::default(),
        );

        let segment = map.initial_segment().expect("initial tempo");
        assert_eq!(map.grid_offset().get(), 350_000_000);
        assert_eq!(segment.start_tick().get(), 0);
        assert_eq!(segment.bpm(), bpm);
        assert_eq!(segment.meter(), super::TimeSignature::default());

        let unset = super::TempoMap::unset(GridOffsetNs::new(0));
        assert!(unset.initial_segment().is_none());
    }

    #[test]
    fn frame_timestamp_is_derived_directly_from_frame_index() {
        let sixty = super::FrameRate::new(60, 1).expect("valid frame rate");
        assert_eq!(
            super::project_time_for_frame(0, sixty)
                .expect("frame time")
                .get(),
            0
        );
        assert_eq!(
            super::project_time_for_frame(60, sixty)
                .expect("frame time")
                .get(),
            1_000_000_000
        );

        let ntsc = super::FrameRate::new(60_000, 1_001).expect("valid frame rate");
        assert_eq!(
            super::project_time_for_frame(60_000, ntsc)
                .expect("frame time")
                .get(),
            1_001_000_000_000
        );
    }

    #[test]
    fn time_signature_defaults_to_four_four() {
        let meter = super::TimeSignature::default();
        assert_eq!(meter.numerator(), 4);
        assert_eq!(meter.denominator(), 4);
        assert_eq!(
            super::TimeSignature::new(0, 4),
            Err(super::TimeSignatureError::ZeroNumerator)
        );
        assert_eq!(
            super::TimeSignature::new(4, 0),
            Err(super::TimeSignatureError::ZeroDenominator)
        );
    }

    #[test]
    fn frame_rate_normalizes_positive_rationals() {
        let sixty = super::FrameRate::new(60, 1).expect("valid frame rate");
        assert_eq!(sixty.numerator(), 60);
        assert_eq!(sixty.denominator(), 1);

        let ntsc = super::FrameRate::new(60_000, 1_001).expect("valid frame rate");
        assert_eq!(ntsc.numerator(), 60_000);
        assert_eq!(ntsc.denominator(), 1_001);

        let reduced = super::FrameRate::new(120, 2).expect("valid frame rate");
        assert_eq!(reduced.numerator(), 60);
        assert_eq!(reduced.denominator(), 1);

        assert_eq!(
            super::FrameRate::new(0, 1),
            Err(super::FrameRateError::ZeroNumerator)
        );
        assert_eq!(
            super::FrameRate::new(60, 0),
            Err(super::FrameRateError::ZeroDenominator)
        );
    }

    #[test]
    fn every_mvp_division_has_exact_tick_step() {
        let cases = [
            (1, 960),
            (2, 480),
            (3, 320),
            (4, 240),
            (6, 160),
            (8, 120),
            (12, 80),
            (16, 60),
            (24, 40),
            (32, 30),
        ];

        for (parts, expected_ticks) in cases {
            let division = super::BeatDivision::new(parts).expect("MVP beat division");
            assert_eq!(division.ticks_per_step(), expected_ticks);
        }
    }

    #[test]
    fn beat_division_accepts_exactly_the_mvp_set() {
        for parts in super::MVP_BEAT_DIVISIONS {
            let division = super::BeatDivision::new(parts).expect("MVP beat division");
            assert_eq!(division.parts_per_beat(), parts);
        }

        for invalid in [0, 5, 7, 10, 48, u16::MAX] {
            assert!(super::BeatDivision::new(invalid).is_none());
        }
    }

    #[test]
    fn ppq_is_schema_constant() {
        assert_eq!(super::PPQ, 960);
    }

    #[test]
    fn bpm_range_is_inclusive_and_validated() {
        assert_eq!(BpmMicros::new(1_000_000).map(BpmMicros::get), Ok(1_000_000));
        assert_eq!(
            BpmMicros::new(1_000_000_000).map(BpmMicros::get),
            Ok(1_000_000_000)
        );
        assert_eq!(BpmMicros::new(999_999), Err(super::BpmError::OutOfRange));
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
