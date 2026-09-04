//! Syntax prohibitions extracted from CEN preprocessed Schematron.
//!
//! A writer built from the semantic model cannot emit most of these — the model
//! has no term for `cbc:UUID`. A reader reports leftovers via [`crate::Read::unmapped`].
//! Context is half the rule: `/ubl:Invoice` + `cbc:UUID` is not a ban on `cbc:UUID`
//! everywhere.
//!
//! [`scan_written`] reports hits on written bytes. It does not rewrite the tree.
//! Empty hits are not CEN Valid, not KoSIT Valid, and not OpenPEPPOL Valid.

#[path = "prohibitions_cii.rs"]
mod prohibitions_cii;
#[path = "prohibitions_ubl.rs"]
mod prohibitions_ubl;

pub mod ubl {
    pub use super::prohibitions_ubl::{
        FORBIDDEN_ATTRIBUTE_PATHS, FORBIDDEN_ATTRIBUTES, FORBIDDEN_PATHS, TOTAL_PARAMS,
        UNEXTRACTED, UNEXTRACTED_IDS,
    };
}

pub mod cii {
    pub use super::prohibitions_cii::{
        FORBIDDEN_ATTRIBUTE_PATHS, FORBIDDEN_ATTRIBUTES, FORBIDDEN_PATHS, TOTAL_PARAMS,
        UNEXTRACTED, UNEXTRACTED_IDS,
    };
}

/// Does `path` (document-element-relative, `/`-joined) match `(context, relative)`?
///
/// A context beginning with a single `/` anchors at the document element; `//`
/// or a bare name matches at any depth. The document element is compared by
/// local name (`Invoice` vs `ubl:Invoice`). An empty `relative` means the
/// context node itself (`//cac:FinancialInstitution`).
pub fn path_matches(path: &str, context: &str, relative: &str) -> bool {
    let floating = context.starts_with("//") || !context.starts_with('/');
    let ctx = context.trim_start_matches('/');
    let target = if relative.is_empty() {
        ctx.to_string()
    } else {
        format!("{ctx}/{relative}")
    };
    if floating {
        return path == target.as_str() || path.ends_with(&format!("/{target}"));
    }
    match_anchored(path, &target)
}

fn match_anchored(path: &str, target: &str) -> bool {
    let mut path_segs = path.split('/');
    let mut target_segs = target.split('/');
    let (Some(ph), Some(th)) = (path_segs.next(), target_segs.next()) else {
        return false;
    };
    local(ph) == local(th) && path_segs.eq(target_segs)
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

pub fn cii_forbidden_attribute(name: &str) -> Option<&'static str> {
    cii::FORBIDDEN_ATTRIBUTES
        .iter()
        .find(|(_, a)| *a == name)
        .map(|(id, _)| *id)
}

/// Contextual `elem/@attr` ban. Not a document-wide `//@attr`.
pub fn ubl_forbidden_attribute_path(path: &str, attr: &str) -> Option<&'static str> {
    ubl::FORBIDDEN_ATTRIBUTE_PATHS
        .iter()
        .find(|(_, ctx, a)| *a == attr && path_matches(path, ctx, ""))
        .map(|(id, _, _)| *id)
}

pub fn cii_forbidden_attribute_path(path: &str, attr: &str) -> Option<&'static str> {
    cii::FORBIDDEN_ATTRIBUTE_PATHS
        .iter()
        .find(|(_, ctx, a)| *a == attr && path_matches(path, ctx, ""))
        .map(|(id, _, _)| *id)
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
/// site emitted a CEN-forbidden child or attribute. Ill-formed written XML is
/// itself a hit — a successful empty list must mean a parseable document with
/// no table matches, not a scanner that gave up.
pub fn scan_written(xml: &str, syntax: crate::Syntax) -> Vec<String> {
    let Ok(doc) = roxmltree::Document::parse(xml) else {
        return vec!["<unparseable written XML>".into()];
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
    for a in node.attributes() {
        let name = a.name();
        let doc_hit = match syntax {
            crate::Syntax::Ubl => ubl_forbidden_attribute(name),
            crate::Syntax::Cii => cii_forbidden_attribute(name),
        };
        if let Some(id) = doc_hit {
            hits.push(format!("{path}/@{name} ({id})"));
        }
        let ctx_hit = match syntax {
            crate::Syntax::Ubl => ubl_forbidden_attribute_path(path, name),
            crate::Syntax::Cii => cii_forbidden_attribute_path(path, name),
        };
        if let Some(id) = ctx_hit {
            hits.push(format!("{path}/@{name} ({id})"));
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
            ubl::FORBIDDEN_PATHS.len() > 1_500,
            "{}",
            ubl::FORBIDDEN_PATHS.len()
        );
        assert!(ubl::FORBIDDEN_ATTRIBUTES.len() > 10);
        assert!(
            ubl::FORBIDDEN_ATTRIBUTE_PATHS.len() > 30,
            "{}",
            ubl::FORBIDDEN_ATTRIBUTE_PATHS.len()
        );
        assert_eq!(ubl::TOTAL_PARAMS, 696);
        assert_eq!(ubl::UNEXTRACTED, 3);
        assert_eq!(ubl::UNEXTRACTED_IDS.len(), ubl::UNEXTRACTED);
        assert!(ubl::UNEXTRACTED_IDS.contains(&"UBL-CR-665"));
        assert!(ubl::UNEXTRACTED_IDS.contains(&"UBL-CR-666"));
        assert!(ubl::UNEXTRACTED_IDS.contains(&"UBL-CR-673"));
        assert!(
            !ubl::FORBIDDEN_ATTRIBUTES
                .iter()
                .any(|(_, a)| *a == "schemeID")
        );
        assert!(
            !ubl::FORBIDDEN_ATTRIBUTES
                .iter()
                .any(|(_, a)| *a == "listID")
        );
    }

    #[test]
    fn cii_tables_are_populated() {
        assert!(
            cii::FORBIDDEN_PATHS.len() > 400,
            "{}",
            cii::FORBIDDEN_PATHS.len()
        );
        assert!(
            cii::FORBIDDEN_ATTRIBUTE_PATHS.len() > 30,
            "{}",
            cii::FORBIDDEN_ATTRIBUTE_PATHS.len()
        );
        assert!(cii::FORBIDDEN_ATTRIBUTES.is_empty());
        assert_eq!(cii::TOTAL_PARAMS, 511);
        assert_eq!(cii::UNEXTRACTED, 45);
        assert_eq!(cii::UNEXTRACTED_IDS.len(), cii::UNEXTRACTED);
        assert!(cii::UNEXTRACTED_IDS.contains(&"CII-SR-465"));
        assert!(cii::UNEXTRACTED_IDS.contains(&"CII-DT-031"));
        assert!(!cii::UNEXTRACTED_IDS.contains(&"CII-SR-046"));
        assert!(
            !cii::FORBIDDEN_ATTRIBUTE_PATHS
                .iter()
                .any(|(_, ctx, a)| ctx.starts_with("//@") || *a == "@schemeID")
        );
    }

    #[test]
    fn uuid_is_forbidden_on_invoice_root_only() {
        assert_eq!(ubl_forbidden_path("Invoice/cbc:UUID"), Some("UBL-CR-005"));
        assert_eq!(
            ubl_forbidden_path("Invoice/cac:AccountingCustomerParty/cbc:UUID"),
            None
        );
        assert_eq!(
            ubl_forbidden_path("CreditNote/cbc:UUID"),
            Some("UBL-CR-005")
        );
    }

    #[test]
    fn customization_scheme_id_is_contextual() {
        assert_eq!(
            ubl_forbidden_attribute_path("Invoice/cbc:CustomizationID", "schemeID"),
            Some("UBL-CR-648")
        );
        assert_eq!(
            ubl_forbidden_attribute_path(
                "Invoice/cac:AccountingSupplierParty/cac:Party/cbc:EndpointID",
                "schemeID"
            ),
            None
        );
        assert_eq!(ubl_forbidden_attribute("schemeID"), None);
    }

    #[test]
    fn payer_financial_account_is_direct_child_of_payment_means() {
        assert_eq!(
            ubl_forbidden_path("Invoice/cac:PaymentMeans/cac:PayerFinancialAccount"),
            Some("UBL-CR-680")
        );
        assert_eq!(
            ubl_forbidden_path(
                "Invoice/cac:PaymentMeans/cac:PaymentMandate/cac:PayerFinancialAccount"
            ),
            None
        );
    }

    #[test]
    fn financial_institution_matches_self() {
        assert_eq!(
            ubl_forbidden_path(
                "Invoice/cac:PaymentMeans/cac:PayeeFinancialAccount/cac:FinancialInstitution"
            ),
            Some("UBL-CR-664")
        );
        assert_eq!(
            ubl_forbidden_path(
                "Invoice/cac:PaymentMeans/cac:PayeeFinancialAccount/cac:FinancialInstitutionBranch"
            ),
            None
        );
    }

    #[test]
    fn company_id_scheme_is_party_tax_scheme_only() {
        assert_eq!(
            ubl_forbidden_attribute_path(
                "Invoice/cac:AccountingSupplierParty/cac:Party/cac:PartyTaxScheme/cbc:CompanyID",
                "schemeID"
            ),
            Some("UBL-CR-652")
        );
        assert_eq!(
            ubl_forbidden_attribute_path(
                "Invoice/cac:AccountingSupplierParty/cac:Party/cac:PartyLegalEntity/cbc:CompanyID",
                "schemeID"
            ),
            None
        );
    }

    #[test]
    fn cii_scheme_name_is_not_a_blanket_ban() {
        assert_eq!(
            cii_forbidden_attribute_path(
                "rsm:CrossIndustryInvoice/rsm:ExchangedDocument/ram:ID",
                "schemeName"
            ),
            Some("CII-DT-001")
        );
        assert_eq!(
            cii_forbidden_attribute_path(
                "rsm:CrossIndustryInvoice/rsm:SupplyChainTradeTransaction/ram:ApplicableHeaderTradeAgreement/ram:SellerTradeParty/ram:SpecifiedTaxRegistration/ram:ID",
                "schemeID"
            ),
            None
        );
        assert_eq!(cii_forbidden_attribute("schemeID"), None);
        assert_eq!(cii_forbidden_attribute("currencyID"), None);
    }

    #[test]
    fn unparseable_written_xml_is_a_hit() {
        let hits = scan_written("<not xml", crate::Syntax::Ubl);
        assert_eq!(hits, ["<unparseable written XML>"]);
    }
}
