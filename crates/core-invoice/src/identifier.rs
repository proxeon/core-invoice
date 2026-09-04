//! [`Identifier`] (value + optional scheme) and [`DocumentReference`] (content only).

use std::fmt;

/// Identifier.Type: content + optional scheme + optional scheme version.
/// Lists are profile-scoped; this type does not require EAS.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Identifier {
    /// Identifier content.
    pub value: String,
    /// Optional scheme identifier. Not required by this type.
    pub scheme: Option<String>,
    /// Optional scheme version identifier.
    pub scheme_version: Option<String>,
}

impl Identifier {
    /// Unschemed identifier.
    pub fn new(value: impl Into<String>) -> Self {
        Self {
            value: value.into(),
            scheme: None,
            scheme_version: None,
        }
    }

    /// Identifier with scheme, no version.
    pub fn schemed(value: impl Into<String>, scheme: impl Into<String>) -> Self {
        Self {
            value: value.into(),
            scheme: Some(scheme.into()),
            scheme_version: None,
        }
    }

    /// Identifier with scheme and scheme version.
    pub fn with_version(
        value: impl Into<String>,
        scheme: impl Into<String>,
        version: impl Into<String>,
    ) -> Self {
        Self {
            value: value.into(),
            scheme: Some(scheme.into()),
            scheme_version: Some(version.into()),
        }
    }
}

/// Document reference (PO, contract, preceding invoice id). No scheme.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DocumentReference(
    /// Reference content. No scheme.
    pub String,
);

impl DocumentReference {
    /// Content-only document reference.
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Reference as written.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for DocumentReference {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}
