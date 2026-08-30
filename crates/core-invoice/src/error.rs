use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AmountError {
    TooManyDecimals,
    Overflow,
}

impl fmt::Display for AmountError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooManyDecimals => write!(f, "amount has more than two fraction digits"),
            Self::Overflow => write!(f, "amount overflow"),
        }
    }
}

impl std::error::Error for AmountError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DateError {
    Invalid,
}

impl fmt::Display for DateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "not an EN 16931 calendar date (YYYY-MM-DD, no time)")
    }
}

impl std::error::Error for DateError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AttachmentError {
    EmptyMime,
    EmptyFilename,
}

impl fmt::Display for AttachmentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyMime => write!(f, "attachment mime code shall be present"),
            Self::EmptyFilename => write!(f, "attachment filename shall be present"),
        }
    }
}

impl std::error::Error for AttachmentError {}
