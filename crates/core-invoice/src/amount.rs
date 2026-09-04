//! [`InvoiceAmount`] (≤2 fraction digits, refuse excess) and [`UnitPriceAmount`] (uncapped).

use crate::error::AmountError;
use rust_decimal::Decimal;
use std::fmt;
use std::str::FromStr;

/// EN 16931 Amount.Type: at most two fraction digits. Never `f64`. Never rounds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct InvoiceAmount(Decimal);

/// 0.1.x alias. Prefer [`InvoiceAmount`].
pub type Amount = InvoiceAmount;

impl InvoiceAmount {
    /// Zero amount.
    pub const ZERO: Self = Self(Decimal::ZERO);

    /// Integer minor units (cents) as a two-decimal amount.
    pub fn from_minor(cents: i64) -> Self {
        Self(Decimal::new(cents, 2))
    }

    /// Accepts at most two fraction digits. Refuses excess; never rounds.
    pub fn try_new(value: Decimal) -> Result<Self, AmountError> {
        if value.scale() > 2 {
            return Err(AmountError::TooManyDecimals);
        }
        Ok(Self(value))
    }

    /// Parse a decimal string. Excess fraction digits are refused; never rounds.
    pub fn parse(s: &str) -> Result<Self, AmountError> {
        let d = Decimal::from_str(s.trim()).map_err(|_| AmountError::TooManyDecimals)?;
        Self::try_new(d)
    }

    /// Underlying decimal.
    pub fn raw(self) -> Decimal {
        self.0
    }

    /// Whether the value is zero.
    pub fn is_zero(self) -> bool {
        self.0.is_zero()
    }

    /// Sum, or `None` on overflow. Does not saturate.
    pub fn checked_add(self, other: Self) -> Option<Self> {
        self.0
            .checked_add(other.0)
            .and_then(|d| Self::try_new(d).ok())
    }

    /// Difference, or `None` on overflow. Does not saturate.
    pub fn checked_sub(self, other: Self) -> Option<Self> {
        self.0
            .checked_sub(other.0)
            .and_then(|d| Self::try_new(d).ok())
    }

    /// Absolute value.
    pub fn abs(self) -> Self {
        Self(self.0.abs())
    }

    /// Sum of amounts, or `None` on overflow. Does not saturate.
    pub fn checked_sum(amounts: impl IntoIterator<Item = Self>) -> Option<Self> {
        let mut acc = Self::ZERO;
        for a in amounts {
            acc = acc.checked_add(a)?;
        }
        Some(acc)
    }

    /// Commercial rounding (half away from zero) to two decimals — producer
    /// presentation. Validators use [`crate::arith::xpath_round`].
    pub fn from_decimal_rounded(value: Decimal) -> Result<Self, AmountError> {
        use rust_decimal::RoundingStrategy;
        let rounded = value.round_dp_with_strategy(2, RoundingStrategy::MidpointAwayFromZero);
        Self::try_new(rounded)
    }
}

impl fmt::Display for InvoiceAmount {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0.round_dp(2))
    }
}

impl FromStr for InvoiceAmount {
    type Err = AmountError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

/// EN 16931 Unit Price Amount.Type — no 2-dp cap (example `10000.1234`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UnitPriceAmount(Decimal);

impl UnitPriceAmount {
    /// Zero unit price.
    pub const ZERO: Self = Self(Decimal::ZERO);

    /// Uncapped fraction digits (BT-146, BT-147, BT-148).
    pub fn new(value: Decimal) -> Self {
        Self(value)
    }

    /// Parse a decimal string. No scale cap.
    pub fn parse(s: &str) -> Result<Self, rust_decimal::Error> {
        Ok(Self(Decimal::from_str(s.trim())?))
    }

    /// Underlying decimal.
    pub fn raw(self) -> Decimal {
        self.0
    }
}

impl fmt::Display for UnitPriceAmount {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refuses_third_decimal() {
        assert!(InvoiceAmount::parse("0.005").is_err());
        assert!(InvoiceAmount::try_new(Decimal::new(5, 3)).is_err());
    }

    #[test]
    fn accepts_two_or_fewer() {
        assert!(InvoiceAmount::parse("100.00").is_ok());
        assert!(InvoiceAmount::parse("100").is_ok());
        assert!(InvoiceAmount::parse("100.1").is_ok());
        assert_eq!(
            InvoiceAmount::from_minor(10000).raw(),
            Decimal::new(10000, 2)
        );
    }

    #[test]
    fn unit_price_keeps_four_decimals() {
        let p = UnitPriceAmount::parse("10000.1234").unwrap();
        assert_eq!(p.to_string(), "10000.1234");
    }

    #[test]
    fn checked_add_no_saturate() {
        let a = InvoiceAmount::parse("1.00").unwrap();
        let b = InvoiceAmount::parse("2.50").unwrap();
        assert_eq!(a.checked_add(b).unwrap().to_string(), "3.50");
    }
}
