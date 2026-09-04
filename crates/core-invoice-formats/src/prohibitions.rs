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

const NS_INV: &str = "urn:oasis:names:specification:ubl:schema:xsd:Invoice-2";
const NS_CN: &str = "urn:oasis:names:specification:ubl:schema:xsd:CreditNote-2";
const NS_CAC: &str = "urn:oasis:names:specification:ubl:schema:xsd:CommonAggregateComponents-2";
const NS_CBC: &str = "urn:oasis:names:specification:ubl:schema:xsd:CommonBasicComponents-2";
const NS_EXT: &str = "urn:oasis:names:specification:ubl:schema:xsd:CommonExtensionComponents-2";
const NS_RSM: &str = "urn:un:unece:uncefact:data:standard:CrossIndustryInvoice:100";
const NS_RAM: &str =
    "urn:un:unece:uncefact:data:standard:ReusableAggregateBusinessInformationEntity:100";
const NS_UDT: &str = "urn:un:unece:uncefact:data:standard:UnqualifiedDataType:100";

fn qualified(node: roxmltree::Node<'_, '_>) -> String {
    let local_name = node.tag_name().name();
    match node.tag_name().namespace() {
        Some(NS_CAC) => format!("cac:{local_name}"),
        Some(NS_CBC) => format!("cbc:{local_name}"),
        Some(NS_EXT) => format!("ext:{local_name}"),
        Some(NS_RAM) => format!("ram:{local_name}"),
        Some(NS_RSM) => format!("rsm:{local_name}"),
        Some(NS_UDT) => format!("udt:{local_name}"),
        Some(NS_INV) | Some(NS_CN) => local_name.to_owned(),
        _ => local_name.to_owned(),
    }
}

/// Walk a written document and list prohibition hits. Does not strip.
///
/// The semantic writer should produce none of these. Hits mean a write call
/// site emitted a CEN-forbidden child.
pub fn scan_written(xml: &str, syntax: crate::Syntax) -> Vec<String> {
    let Ok(doc) = roxmltree::Document::parse(xml) else {
        return Vec::new();
    };
    let root = doc.root_element();
    let mut hits = Vec::new();
    walk(root, &qualified(root), syntax, &mut hits);
    hits
}

fn walk(node: roxmltree::Node<'_, '_>, path: &str, syntax: crate::Syntax, hits: &mut Vec<String>) {
    let hit = match syntax {
        crate::Syntax::Ubl => ubl_forbidden_path(path),
        crate::Syntax::Cii => cii_forbidden_path(path),
    };
    if let Some(id) = hit {
        hits.push(format!("{path} ({id})"));
    }
    if syntax == crate::Syntax::Ubl {
        for a in node.attributes() {
            if let Some(id) = ubl_forbidden_attribute(a.name()) {
                hits.push(format!("{path}/@{} ({id})", a.name()));
            }
        }
    }
    for child in node.children().filter(|n| n.is_element()) {
        walk(child, &format!("{path}/{}", qualified(child)), syntax, hits);
    }
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
