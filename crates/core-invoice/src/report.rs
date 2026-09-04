//! Validation findings. [`Severity::Fatal`] fails [`Report::ok`]; [`Severity::Warning`] does not.

use crate::bt::Path;
use std::fmt;

/// Finding severity. Only [`Severity::Fatal`] fails [`Report::ok`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    /// Fails [`Report::ok`].
    Fatal,
    /// Does not fail [`Report::ok`].
    Warning,
    /// Does not fail [`Report::ok`].
    Info,
}

/// Provenance of a registered rule id.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    /// Standard text and artefacts agree.
    Both,
    /// Standard text only.
    StandardOnly,
    /// Artefact-only (eval may be a no-op).
    ArtefactOnly,
    /// Crate-owned id (`CORE-*`, `PINT-TAX`, `IBR-*-MY`).
    Crate,
}

/// One rule hit at a [`Path`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    /// Registered rule id (`BR-02`, `PINT-TAX`).
    pub id: &'static str,
    /// Fatal fails [`Report::ok`]; Warning and Info do not.
    pub severity: Severity,
    /// Finding location ([`Path`]; index is 0-based).
    pub path: Path,
    /// Authority wording. Arithmetic expected/actual belongs in [`Self::detail`].
    pub message: String,
    /// Optional expected/actual. Not appended to [`Self::message`] by constructors.
    pub detail: Option<String>,
    /// Optional hint. Not shown by `Display`.
    pub hint: Option<String>,
}

impl Finding {
    /// Fatal finding. Fails [`Report::ok`].
    pub fn fatal(id: &'static str, path: Path, message: impl Into<String>) -> Self {
        Self {
            id,
            severity: Severity::Fatal,
            path,
            message: message.into(),
            detail: None,
            hint: None,
        }
    }

    /// Warning finding. Does not fail [`Report::ok`].
    pub fn warning(id: &'static str, path: Path, message: impl Into<String>) -> Self {
        Self {
            id,
            severity: Severity::Warning,
            path,
            message: message.into(),
            detail: None,
            hint: None,
        }
    }
}

/// Validation result. [`Self::ok`] is “no Fatal”, not “no findings”.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Report {
    /// Collected hits, unsorted until [`Self::sort_stable`].
    pub findings: Vec<Finding>,
    /// Profile slug of the invoice that was checked.
    pub profile_slug: &'static str,
    /// Number of rule evals invoked (CORE + extras).
    pub rules_checked: usize,
}

impl Report {
    /// `true` when no finding is [`Severity::Fatal`]. Warning and Info do not fail.
    pub fn ok(&self) -> bool {
        !self.findings.iter().any(|f| f.severity == Severity::Fatal)
    }

    /// Append a finding. Does not sort.
    pub fn push(&mut self, finding: Finding) {
        self.findings.push(finding);
    }

    /// Sort by severity, then path display, then id.
    pub fn sort_stable(&mut self) {
        self.findings.sort_by(|a, b| {
            a.severity
                .cmp(&b.severity)
                .then_with(|| a.path.to_string().cmp(&b.path.to_string()))
                .then_with(|| a.id.cmp(b.id))
        });
    }
}

impl fmt::Display for Finding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {} — {}", self.id, self.path, self.message)?;
        if let Some(d) = &self.detail {
            write!(f, " ({d})")?;
        }
        Ok(())
    }
}

impl fmt::Display for Report {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.ok() {
            return write!(f, "valid");
        }
        for finding in &self.findings {
            writeln!(f, "{finding}")?;
        }
        Ok(())
    }
}
