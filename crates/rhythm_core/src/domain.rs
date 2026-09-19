#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DomainValueError {
    NonFinite,
    AlphaOutOfRange,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LinearRgba {
    r: f32,
    g: f32,
    b: f32,
    a: f32,
}

impl LinearRgba {
    pub fn new(r: f32, g: f32, b: f32, a: f32) -> Result<Self, DomainValueError> {
        if !r.is_finite() || !g.is_finite() || !b.is_finite() || !a.is_finite() {
            return Err(DomainValueError::NonFinite);
        }
        if !(0.0..=1.0).contains(&a) {
            return Err(DomainValueError::AlphaOutOfRange);
        }

        Ok(Self { r, g, b, a })
    }

    #[must_use]
    pub const fn black_opaque() -> Self {
        Self {
            r: 0.0,
            g: 0.0,
            b: 0.0,
            a: 1.0,
        }
    }

    #[must_use]
    pub const fn r(self) -> f32 {
        self.r
    }

    #[must_use]
    pub const fn g(self) -> f32 {
        self.g
    }

    #[must_use]
    pub const fn b(self) -> f32 {
        self.b
    }

    #[must_use]
    pub const fn a(self) -> f32 {
        self.a
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Vec2 {
    x: f32,
    y: f32,
}

impl Vec2 {
    pub fn new(x: f32, y: f32) -> Result<Self, DomainValueError> {
        if !x.is_finite() || !y.is_finite() {
            return Err(DomainValueError::NonFinite);
        }

        Ok(Self { x, y })
    }

    #[must_use]
    pub const fn x(self) -> f32 {
        self.x
    }

    #[must_use]
    pub const fn y(self) -> f32 {
        self.y
    }
}

#[cfg(test)]
mod tests {
    use super::{DomainValueError, LinearRgba, Vec2};

    #[test]
    fn linear_rgba_requires_finite_channels_and_normalized_alpha() {
        let color = LinearRgba::new(1.5, -0.25, 0.5, 0.75).expect("valid linear color");
        assert_eq!(color.r(), 1.5);
        assert_eq!(color.g(), -0.25);
        assert_eq!(color.b(), 0.5);
        assert_eq!(color.a(), 0.75);

        assert_eq!(
            LinearRgba::new(f32::NAN, 0.0, 0.0, 1.0),
            Err(DomainValueError::NonFinite)
        );
        assert_eq!(
            LinearRgba::new(0.0, 0.0, 0.0, 1.01),
            Err(DomainValueError::AlphaOutOfRange)
        );
        assert_eq!(
            LinearRgba::new(0.0, 0.0, 0.0, -0.01),
            Err(DomainValueError::AlphaOutOfRange)
        );
    }

    #[test]
    fn vec2_requires_finite_coordinates() {
        let value = Vec2::new(-100.0, 250.5).expect("finite vector");
        assert_eq!(value.x(), -100.0);
        assert_eq!(value.y(), 250.5);

        assert_eq!(
            Vec2::new(f32::INFINITY, 0.0),
            Err(DomainValueError::NonFinite)
        );
        assert_eq!(Vec2::new(0.0, f32::NAN), Err(DomainValueError::NonFinite));
    }
}
