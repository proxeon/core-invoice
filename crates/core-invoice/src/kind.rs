/// Syntax root. Not derived from BT-3. Self-billing is a profile + type code, not a third kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum DocumentKind {
    #[default]
    Invoice,
    CreditNote,
}
