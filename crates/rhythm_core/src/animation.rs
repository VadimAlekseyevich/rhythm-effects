#[derive(Debug, Clone, PartialEq)]
pub struct Animated<T> {
    base_value: T,
}

impl<T> Animated<T> {
    #[must_use]
    pub const fn new_static(base_value: T) -> Self {
        Self { base_value }
    }

    #[must_use]
    pub const fn base_value(&self) -> &T {
        &self.base_value
    }

    pub fn base_value_mut(&mut self) -> &mut T {
        &mut self.base_value
    }
}
