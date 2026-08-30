use rust_decimal::Decimal;
use std::fmt;
use std::str::FromStr;

/// Monetary amount. Never `f64`. Scale is two decimals unless noted (BR-DEC).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Amount(Decimal);

impl Amount {
    pub const ZERO: Self = Self(Decimal::ZERO);

    pub fn new(value: Decimal) -> Self {
        Self(value.round_dp(2))
    }

    pub fn from_minor(cents: i64) -> Self {
        Self(Decimal::new(cents, 2))
    }

    pub fn parse(s: &str) -> Result<Self, rust_decimal::Error> {
        Ok(Self::new(Decimal::from_str(s)?))
    }

    pub fn raw(self) -> Decimal {
        self.0
    }

    pub fn is_zero(self) -> bool {
        self.0.is_zero()
    }

    pub fn saturating_add(self, other: Self) -> Self {
        Self::new(self.0 + other.0)
    }

    pub fn saturating_sub(self, other: Self) -> Self {
        Self::new(self.0 - other.0)
    }
}

impl fmt::Display for Amount {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0.round_dp(2))
    }
}

impl From<Decimal> for Amount {
    fn from(value: Decimal) -> Self {
        Self::new(value)
    }
}
