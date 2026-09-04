//! Construction errors for amounts, dates, and attachments.

use std::fmt;

/// Error constructing an [`crate::InvoiceAmount`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AmountError {
    /// More than two fraction digits. The type never rounds.
    TooManyDecimals,
    /// Decimal overflow.
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

/// Error constructing a [`crate::Date`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DateError {
    /// Not a calendar day `YYYY-MM-DD` (no time, no zone).
    Invalid,
}

impl fmt::Display for DateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "not an EN 16931 calendar date (YYYY-MM-DD, no time)")
    }
}

impl std::error::Error for DateError {}

/// Error constructing an [`crate::Attachment`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AttachmentError {
    /// MIME code is empty or whitespace.
    EmptyMime,
    /// Filename is empty or whitespace.
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
