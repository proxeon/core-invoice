//! [`Quantity`] (signed, not money) and [`Percentage`] (per cent, not a fraction).

use rust_decimal::Decimal;
use std::fmt;
use std::str::FromStr;

/// Signed quantity (BT-129, BT-149). May be negative. Not money.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Quantity(Decimal);

impl Quantity {
    /// Zero quantity.
    pub const ZERO: Self = Self(Decimal::ZERO);
    /// Quantity one.
    pub const ONE: Self = Self(Decimal::ONE);

    /// Signed quantity. May be negative. Not money.
    pub fn new(value: Decimal) -> Self {
        Self(value)
    }

    /// Parse a decimal string. Negatives allowed.
    pub fn parse(s: &str) -> Result<Self, rust_decimal::Error> {
        Ok(Self(Decimal::from_str(s.trim())?))
    }

    /// Underlying decimal.
    pub fn raw(self) -> Decimal {
        self.0
    }

    /// True when the value is strictly negative.
    pub fn is_negative(self) -> bool {
        self.0.is_sign_negative() && !self.0.is_zero()
    }
}

impl fmt::Display for Quantity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Per cent (`19` means 19%), not a fraction. `19` and `19.00` compare equal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Percentage(Decimal);

impl Percentage {
    /// Zero per cent.
    pub const ZERO: Self = Self(Decimal::ZERO);

    /// Per cent (`19` means 19%), not a fraction.
    pub fn new(percent: Decimal) -> Self {
        Self(percent)
    }

    /// Convert a fraction (`0.19` → 19%). `None` on overflow.
    pub fn from_fraction(fraction: Decimal) -> Option<Self> {
        fraction
            .checked_mul(Decimal::ONE_HUNDRED)
            .map(|d| Self(d.normalize()))
    }

    /// Stored per-cent value (`19` for 19%).
    pub fn as_percent(self) -> Decimal {
        self.0
    }

    /// Fraction (`0.19` for 19%).
    pub fn as_fraction(self) -> Decimal {
        self.0 / Decimal::ONE_HUNDRED
    }

    /// Whether the rate is 0%.
    pub fn is_zero(self) -> bool {
        self.0.is_zero()
    }

    /// Whether the rate is strictly greater than 0%.
    pub fn is_positive(self) -> bool {
        self.0 > Decimal::ZERO
    }

    /// Whether the rate is strictly less than 0%.
    pub fn is_negative(self) -> bool {
        self.0 < Decimal::ZERO
    }
}

impl fmt::Display for Percentage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0.normalize())
    }
}

impl From<Decimal> for Percentage {
    fn from(value: Decimal) -> Self {
        Self::new(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quantity_may_be_negative() {
        let q = Quantity::new(Decimal::NEGATIVE_ONE);
        assert!(q.is_negative());
        assert_eq!(q.to_string(), "-1");
    }

    #[test]
    fn ten_percent_is_not_point_one() {
        let p = Percentage::new(Decimal::from(10));
        assert_eq!(p.as_percent(), Decimal::from(10));
        assert_eq!(p.as_fraction(), Decimal::new(10, 2));
        assert_eq!(
            Percentage::new(Decimal::from(19)),
            Percentage::new(Decimal::new(1900, 2))
        );
    }
}
