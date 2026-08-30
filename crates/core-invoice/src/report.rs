use crate::bt::Path;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Severity {
    Fatal,
    Warning,
    Info,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    Both,
    StandardOnly,
    ArtefactOnly,
    Crate,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    pub id: &'static str,
    pub severity: Severity,
    pub path: Path,
    pub message: String,
    pub detail: Option<String>,
    pub hint: Option<String>,
}

impl Finding {
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

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Report {
    pub findings: Vec<Finding>,
    pub profile_slug: &'static str,
    pub rules_checked: usize,
}

impl Report {
    pub fn ok(&self) -> bool {
        !self.findings.iter().any(|f| f.severity == Severity::Fatal)
    }

    pub fn push(&mut self, finding: Finding) {
        self.findings.push(finding);
    }

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
