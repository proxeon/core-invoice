//! Verbatim [`Code`]. List membership is a rule (BR-CL-*), not a constructor.

use std::fmt;

/// Verbatim code. Membership is a rule (BR-CL-*), not a constructor.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Code(String);

impl Code {
    /// Verbatim code as written. No list check.
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Code as written.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// True when empty or whitespace-only.
    pub fn is_empty(&self) -> bool {
        self.0.trim().is_empty()
    }
}

impl fmt::Display for Code {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl From<&str> for Code {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}
