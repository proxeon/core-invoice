//! Syntax prohibitions extracted from CEN preprocessed Schematron.
//!
//! A writer built from the semantic model cannot emit most of these — the model
//! has no term for `cbc:UUID`. A reader reports leftovers via [`crate::Read::unmapped`].
//! Context is half the rule: `/ubl:Invoice` + `cbc:UUID` is not a ban on `cbc:UUID`
//! everywhere.

#[path = "prohibitions_cii.rs"]
mod prohibitions_cii;
#[path = "prohibitions_ubl.rs"]
mod prohibitions_ubl;

pub mod ubl {
    pub use super::prohibitions_ubl::{
        FORBIDDEN_ATTRIBUTES, FORBIDDEN_PATHS, TOTAL_PARAMS, UNEXTRACTED,
    };
}

pub mod cii {
    pub use super::prohibitions_cii::{
        FORBIDDEN_ATTRIBUTES, FORBIDDEN_PATHS, TOTAL_PARAMS, UNEXTRACTED,
    };
}

/// Does `path` (document-element-relative, `/`-joined) match `(context, relative)`?
///
/// A context beginning with a single `/` anchors at the document element; `//`
/// or a bare name matches at any depth. The document element is compared by
/// local name (`Invoice` vs `ubl:Invoice`).
pub fn path_matches(path: &str, context: &str, relative: &str) -> bool {
    let floating = context.starts_with("//") || !context.starts_with('/');
    let ctx = context.trim_start_matches('/');
    if floating {
        let needle = format!("{ctx}/{relative}");
        return path == needle || path.ends_with(&format!("/{needle}"));
    }
    let Some((head, rest)) = path.split_once('/') else {
        return false;
    };
    local(head) == local(ctx) && rest == relative
}

fn local(name: &str) -> &str {
    name.rsplit(':').next().unwrap_or(name)
}

pub fn ubl_forbidden_path(path: &str) -> Option<&'static str> {
    ubl::FORBIDDEN_PATHS
        .iter()
        .find(|(_, ctx, rel)| path_matches(path, ctx, rel))
        .map(|(id, _, _)| *id)
}

pub fn cii_forbidden_path(path: &str) -> Option<&'static str> {
    cii::FORBIDDEN_PATHS
        .iter()
        .find(|(_, ctx, rel)| path_matches(path, ctx, rel))
        .map(|(id, _, _)| *id)
}

pub fn ubl_forbidden_attribute(name: &str) -> Option<&'static str> {
    ubl::FORBIDDEN_ATTRIBUTES
        .iter()
        .find(|(_, a)| *a == name)
        .map(|(id, _)| *id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ubl_tables_are_populated() {
        assert!(
            ubl::FORBIDDEN_PATHS.len() > 1_000,
            "{}",
            ubl::FORBIDDEN_PATHS.len()
        );
        assert!(ubl::FORBIDDEN_ATTRIBUTES.len() > 10);
    }

    #[test]
    fn cii_tables_are_populated() {
        assert!(
            cii::FORBIDDEN_PATHS.len() > 400,
            "{}",
            cii::FORBIDDEN_PATHS.len()
        );
    }

    #[test]
    fn uuid_is_forbidden_on_invoice_root_only() {
        assert_eq!(ubl_forbidden_path("Invoice/cbc:UUID"), Some("UBL-CR-005"));
        assert_eq!(
            ubl_forbidden_path("Invoice/cac:AccountingCustomerParty/cbc:UUID"),
            None
        );
    }
}
