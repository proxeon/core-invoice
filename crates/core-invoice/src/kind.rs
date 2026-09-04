//! Syntax root [`DocumentKind`]. Not derived from BT-3.

/// Syntax root. Not derived from BT-3. Self-billing is a profile + type code, not a third kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum DocumentKind {
    /// UBL `Invoice` / CII billed as invoice. Default.
    #[default]
    Invoice,
    /// UBL `CreditNote` root. Not inferred from BT-3 `381`.
    CreditNote,
}
