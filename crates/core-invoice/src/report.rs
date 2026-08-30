use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Fatal,
    Warning,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    pub id: &'static str,
    pub severity: Severity,
    pub message: String,
}

impl Finding {
    pub fn fatal(id: &'static str, message: impl Into<String>) -> Self {
        Self {
            id,
            severity: Severity::Fatal,
            message: message.into(),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Report {
    pub findings: Vec<Finding>,
    /// Profile slug that was actually checked (`en16931`, `peppol`, `pint`, `pint-my`).
    pub profile_slug: &'static str,
}

impl Report {
    pub fn ok(&self) -> bool {
        !self.findings.iter().any(|f| f.severity == Severity::Fatal)
    }

    pub fn push(&mut self, finding: Finding) {
        self.findings.push(finding);
    }
}

impl fmt::Display for Report {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.ok() {
            return write!(f, "valid");
        }
        for finding in &self.findings {
            writeln!(f, "[{}] {}", finding.id, finding.message)?;
        }
        Ok(())
    }
}
