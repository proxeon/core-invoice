//! UBL 2.1 and UN/CEFACT CII on top of [`core_invoice`].
//!
//! Conversion goes through the semantic model, not tag-by-tag.

use core_invoice::{
    En16931Marker, Invoice, PeppolBis3Marker, PintMarker, PintMyMarker, Profile, ProfileMarker,
    Report, Validated,
};

pub mod cii;
pub mod ubl;
pub mod xml;

#[derive(Debug, thiserror::Error)]
pub enum FormatError {
    #[error("unsupported syntax {0}")]
    UnsupportedSyntax(String),
    #[error("parse error: {0}")]
    Parse(String),
    /// PINT-MY is UBL-only. EN/Peppol may emit D16B under the subset policy.
    ///
    /// PINT-MY Billing 1.3.0 has no CII binding. Returning this (instead of
    /// emitting a costume `CrossIndustryInvoice`) keeps convert honest.
    #[error("CII D16B is not a PINT-MY syntax; PINT-MY Billing 1.3.0 is UBL-only")]
    CiiNotForProfile,
    #[error(transparent)]
    Semantic(#[from] SemanticReject),
}

#[derive(Debug, thiserror::Error)]
#[error("invoice failed semantic validation:\n{0}")]
pub struct SemanticReject(pub Report);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Syntax {
    Ubl,
    Cii,
}

/// Children we saw and did not map. Convert must not claim lossless if this is non-empty.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Read {
    pub invoice: Invoice,
    pub unmapped: Vec<String>,
    pub malformed: Vec<String>,
}

impl Syntax {
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "ubl" | "ubl-2.1" | "xml" => Some(Self::Ubl),
            "cii" | "d16b" | "un/cefact" => Some(Self::Cii),
            _ => None,
        }
    }

    pub fn slug(self) -> &'static str {
        match self {
            Self::Ubl => "ubl",
            Self::Cii => "cii",
        }
    }
}

/// Unchecked serialisation. Does **not** prove the invoice.
///
/// CLI convert and any production write must use [`write_validated`].
pub fn write_unchecked(invoice: &Invoice, syntax: Syntax) -> Result<String, FormatError> {
    match syntax {
        Syntax::Ubl => Ok(ubl::write_unchecked(invoice)),
        Syntax::Cii => cii::write_unchecked(invoice),
    }
}

/// Production write. Stamps BT-24 / BT-23 from `P`, then serialises.
pub fn write_validated<P: ProfileMarker>(
    proof: &Validated<P>,
    syntax: Syntax,
) -> Result<String, FormatError> {
    let mut invoice = proof.invoice().clone();
    // BT-24 and BT-23 come from the proved profile, not leftover fields on Invoice.
    invoice.stamp_profile(P::profile());
    write_unchecked(&invoice, syntax)
}

/// Read, prove against the parsed profile (or `forced`), then [`write_validated`].
pub fn convert(xml: &str, to: Syntax) -> Result<String, FormatError> {
    convert_with_profile(xml, to, None)
}

/// Like [`convert`], but a named profile forces that rule set (“would this pass as Peppol?”).
pub fn convert_with_profile(
    xml: &str,
    to: Syntax,
    forced: Option<Profile>,
) -> Result<String, FormatError> {
    let mut invoice = read(xml)?;
    if let Some(profile) = forced {
        invoice.profile = profile;
    }
    convert_invoice(invoice, to)
}

fn convert_invoice(invoice: Invoice, to: Syntax) -> Result<String, FormatError> {
    // PINT-MY is UBL-only; refuse the syntax before proving so a broken MY
    // invoice is still “wrong syntax” (exit 2), not “invalid document” (exit 1).
    if to == Syntax::Cii && invoice.profile == Profile::PintMy {
        return Err(FormatError::CiiNotForProfile);
    }
    // Convert must not emit a document that would fail validate on the same profile.
    match invoice.profile {
        Profile::En16931 => prove_write::<En16931Marker>(invoice, to),
        Profile::PeppolBis3 => prove_write::<PeppolBis3Marker>(invoice, to),
        Profile::Pint => prove_write::<PintMarker>(invoice, to),
        Profile::PintMy => prove_write::<PintMyMarker>(invoice, to),
        Profile::Unknown => {
            let report = core_invoice::validate(&invoice);
            Err(FormatError::Semantic(SemanticReject(report)))
        }
    }
}

fn prove_write<P: ProfileMarker>(invoice: Invoice, syntax: Syntax) -> Result<String, FormatError> {
    match Validated::<P>::new(invoice) {
        Ok(proof) => write_validated(&proof, syntax),
        Err(rejected) => Err(FormatError::Semantic(SemanticReject(rejected.1))),
    }
}

pub fn read(xml: &str) -> Result<Invoice, FormatError> {
    let traced = read_with_trace(xml)?;
    if !traced.malformed.is_empty() {
        return Err(FormatError::Parse(format!(
            "malformed amounts: {}",
            traced.malformed.join("; ")
        )));
    }
    Ok(traced.invoice)
}

pub fn read_with_trace(xml: &str) -> Result<Read, FormatError> {
    xml::refuse_dtd(xml)?;
    xml::refuse_oversize(xml)?;
    xml::refuse_depth(xml)?;
    // Syntax from the document element after skipping comments/PIs, not from substring search.
    let local = xml::document_element_local(xml)
        .ok_or_else(|| FormatError::Parse("document is not well-formed (no element)".into()))?;
    match local {
        "Invoice" | "CreditNote" => ubl::read(xml),
        "CrossIndustryInvoice" => cii::read(xml),
        other => Err(FormatError::Parse(format!(
            "document element must be Invoice, CreditNote, or CrossIndustryInvoice, not {other}"
        ))),
    }
}

pub fn validate_xml(xml: &str, profile: Option<Profile>) -> Result<Report, FormatError> {
    let mut invoice = read(xml)?;
    if let Some(profile) = profile {
        invoice.profile = profile;
    }
    let mut report = core_invoice::validate(&invoice);
    apply_wire_currency_lists(xml, &invoice, &mut report);
    report.profile_slug = invoice.profile.slug();
    Ok(report)
}

/// BR-CL-03: @currencyID ∈ ISO 4217. Peppol R051: @currencyID = BT-5 except BT-111.
fn apply_wire_currency_lists(xml: &str, invoice: &Invoice, report: &mut Report) {
    let Ok(doc) = roxmltree::Document::parse(xml) else {
        return;
    };
    walk_currency(doc.root_element(), invoice, report);
}

fn walk_currency(node: roxmltree::Node<'_, '_>, invoice: &Invoice, report: &mut Report) {
    if node.is_element()
        && let Some(cid) = node.attribute("currencyID")
    {
        let cid = cid.trim();
        if !cid.is_empty() && !core_invoice::is_currency(cid) {
            report.push(core_invoice::Finding::fatal(
                "BR-CL-03",
                core_invoice::Path::term(core_invoice::BtId(5)),
                format!("currencyID {cid} is not an ISO 4217 alpha-3 code"),
            ));
        }
        if invoice.profile == Profile::PeppolBis3
            && !cid.is_empty()
            && cid != invoice.currency
            && !is_bt111_tax_amount(node)
        {
            report.push(core_invoice::Finding::fatal(
                "PEPPOL-EN16931-R051",
                core_invoice::Path::term(core_invoice::BtId(5)),
                format!(
                    "currencyID {cid} must equal invoice currency {}",
                    invoice.currency
                ),
            ));
        }
    }
    for child in node.children() {
        walk_currency(child, invoice, report);
    }
}

fn is_bt111_tax_amount(node: roxmltree::Node<'_, '_>) -> bool {
    if node.tag_name().name() != "TaxAmount" {
        return false;
    }
    let Some(parent) = node.parent() else {
        return false;
    };
    parent.tag_name().name() == "TaxTotal"
        && !parent
            .children()
            .any(|n| n.is_element() && n.tag_name().name() == "TaxSubtotal")
}

pub fn diff(left_xml: &str, right_xml: &str) -> Result<String, FormatError> {
    let left = read(left_xml)?;
    let right = read(right_xml)?;
    Ok(diff_invoices(&left, &right))
}

/// Model paths allowed to differ after UBL↔CII round-trip (D16B subset). Shrink when mapping grows.
pub const CII_DROPPED: &[&str] = &[
    "business_process",
    "seller.endpoint",
    "buyer.endpoint",
    "seller.identifiers",
    "buyer.identifiers",
    "seller.contact",
    "buyer.contact",
    "seller.address",
    "buyer.address",
    "notes",
    "notes.subject",
    "payee",
    "tax_representative",
    "delivery",
    "payment",
    "document_allowances",
    "document_charges",
    "supporting_documents",
    "preceding",
    "purchase_order",
    "sales_order",
    "contract",
    "project",
    "despatch",
    "receiving_advice",
    "tender",
    "invoiced_object",
    "buyer_accounting",
    "buyer_reference",
    "payment_terms",
    "tax_point",
    "due_date",
    "lines.extra_tax",
    "lines.tax_total",
    "lines.allowances",
    "lines.charges",
    "lines.attributes",
    "lines.standard_id",
    "lines.item_id",
    "lines.buyer_id",
    "lines.order_line",
    "lines.accounting_reference",
    "lines.origin_country",
    "lines.classifications",
    "lines.invoiced_object",
    "lines.note",
    "lines.description",
    "lines.period",
    "price.discount",
    "price.gross",
];

/// Syntax terms the writer will omit. CreditNote DueDate is stored as BT-9 and dropped on UBL write.
pub fn write_drops(invoice: &Invoice, syntax: Syntax) -> Vec<String> {
    match syntax {
        Syntax::Ubl => ubl::write_drops(invoice),
        Syntax::Cii => Vec::new(),
    }
}

fn opt_amt(a: Option<core_invoice::Amount>) -> String {
    a.map(|x| x.to_string()).unwrap_or_else(|| "absent".into())
}

fn diff_invoices(left: &Invoice, right: &Invoice) -> String {
    let mut lines = Vec::new();
    if left.number != right.number {
        lines.push(format!("number: {} → {}", left.number, right.number));
    }
    if left.payable() != right.payable() {
        lines.push(format!(
            "payable: {} → {}",
            opt_amt(left.payable()),
            opt_amt(right.payable())
        ));
    }
    if left.currency != right.currency {
        lines.push(format!("currency: {} → {}", left.currency, right.currency));
    }
    if left.profile != right.profile {
        lines.push(format!(
            "profile: {} → {}",
            left.profile.slug(),
            right.profile.slug()
        ));
    }
    if left.issue_date != right.issue_date {
        lines.push(format!(
            "issue_date: {:?} → {:?}",
            left.issue_date, right.issue_date
        ));
    }
    if left.due_date != right.due_date {
        lines.push(format!(
            "due_date: {:?} → {:?}",
            left.due_date, right.due_date
        ));
    }
    if left.type_code != right.type_code {
        lines.push(format!(
            "type_code: {:?} → {:?}",
            left.type_code, right.type_code
        ));
    }
    if left.kind != right.kind {
        lines.push(format!("kind: {:?} → {:?}", left.kind, right.kind));
    }
    diff_party(&mut lines, "seller", &left.seller, &right.seller);
    diff_party(&mut lines, "buyer", &left.buyer, &right.buyer);
    if left.notes != right.notes {
        lines.push(format!(
            "notes: {} → {}",
            left.notes.len(),
            right.notes.len()
        ));
        for (i, (a, b)) in left.notes.iter().zip(right.notes.iter()).enumerate() {
            if a.text != b.text {
                lines.push(format!("notes[{i}].text: differ"));
            }
            if a.subject != b.subject {
                lines.push("notes.subject: differ".into());
            }
        }
    }
    if left.specification_id != right.specification_id {
        lines.push("specification_id: differ".into());
    }
    if left.business_process != right.business_process {
        lines.push("business_process: differ".into());
    }
    if left.payee != right.payee {
        lines.push("payee: differ".into());
    }
    if left.tax_representative != right.tax_representative {
        lines.push("tax_representative: differ".into());
    }
    if left.delivery != right.delivery {
        lines.push("delivery: differ".into());
    }
    if left.supporting_documents != right.supporting_documents {
        lines.push("supporting_documents: differ".into());
    }
    if left.document_allowances != right.document_allowances {
        lines.push("document_allowances: differ".into());
    }
    if left.document_charges != right.document_charges {
        lines.push("document_charges: differ".into());
    }
    if left.preceding != right.preceding {
        lines.push("preceding: differ".into());
    }
    if left.payment != right.payment {
        lines.push("payment: differ".into());
    }
    if left.tax_breakdown != right.tax_breakdown {
        lines.push(format!(
            "tax_breakdown: {} → {}",
            left.tax_breakdown.len(),
            right.tax_breakdown.len()
        ));
    }
    if left.totals != right.totals {
        lines.push("totals: differ".into());
    }
    if left.lines.len() != right.lines.len() {
        lines.push(format!(
            "lines: {} → {}",
            left.lines.len(),
            right.lines.len()
        ));
    }
    for (i, (a, b)) in left.lines.iter().zip(right.lines.iter()).enumerate() {
        if a.id != b.id {
            lines.push(format!("lines[{i}].id: {} → {}", a.id, b.id));
        }
        if a.name != b.name {
            lines.push(format!("lines[{i}].name: {} → {}", a.name, b.name));
        }
        if a.net != b.net {
            lines.push(format!("lines[{i}].net: {} → {}", a.net, b.net));
        }
        if a.tax != b.tax {
            lines.push(format!("lines[{i}].tax: {} → {}", a.tax.code, b.tax.code));
        }
        if a.quantity != b.quantity {
            lines.push(format!(
                "lines[{i}].quantity: {:?} → {:?}",
                a.quantity, b.quantity
            ));
        }
        if a.price != b.price {
            lines.push(format!("lines[{i}].price: differ"));
        }
        if a.extra_tax != b.extra_tax {
            lines.push("lines.extra_tax: differ".into());
        }
        if a.tax_total != b.tax_total {
            lines.push("lines.tax_total: differ".into());
        }
        if a.allowances != b.allowances || a.charges != b.charges {
            lines.push(format!("lines[{i}].allowances: differ"));
        }
        if a.classifications != b.classifications {
            lines.push(format!("lines[{i}].classifications: differ"));
        }
        if a.period != b.period {
            lines.push("lines.period: differ".into());
        }
        if a.origin_country != b.origin_country {
            lines.push("lines.origin_country: differ".into());
        }
    }
    if lines.is_empty() {
        "no semantic difference".into()
    } else {
        lines.join("\n")
    }
}

fn diff_party(
    lines: &mut Vec<String>,
    label: &str,
    left: &core_invoice::Party,
    right: &core_invoice::Party,
) {
    if left.name != right.name {
        lines.push(format!("{label}.name: {} → {}", left.name, right.name));
    }
    if left.country() != right.country() {
        lines.push(format!(
            "{label}.country: {} → {}",
            left.country(),
            right.country()
        ));
    }
    if left.electronic_address != right.electronic_address {
        lines.push(format!("{label}.endpoint: differ"));
    }
    if left.vat_identifier != right.vat_identifier {
        lines.push(format!("{label}.vat: differ"));
    }
    if left.tax_registration != right.tax_registration {
        lines.push(format!("{label}.tax_registration: differ"));
    }
    if left.legal_registration != right.legal_registration {
        lines.push(format!("{label}.legal_registration: differ"));
    }
    if left.identifiers != right.identifiers {
        lines.push(format!("{label}.identifiers: differ"));
    }
    if left.contact != right.contact {
        lines.push(format!("{label}.contact: differ"));
    }
    if left.address != right.address {
        lines.push(format!("{label}.address: differ"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ubl_converts_to_real_cii() {
        let ubl = r#"<?xml version="1.0"?><Invoice xmlns="urn:oasis:names:specification:ubl:schema:xsd:Invoice-2" xmlns:cbc="urn:oasis:names:specification:ubl:schema:xsd:CommonBasicComponents-2" xmlns:cac="urn:oasis:names:specification:ubl:schema:xsd:CommonAggregateComponents-2"><cbc:CustomizationID>urn:cen.eu:en16931:2017</cbc:CustomizationID><cbc:ID>1</cbc:ID><cbc:IssueDate>2026-01-15</cbc:IssueDate><cbc:InvoiceTypeCode>380</cbc:InvoiceTypeCode><cbc:DocumentCurrencyCode>EUR</cbc:DocumentCurrencyCode><cac:LegalMonetaryTotal><cbc:PayableAmount currencyID="EUR">0</cbc:PayableAmount></cac:LegalMonetaryTotal></Invoice>"#;
        // Unchecked path: this skeleton would fail prove (missing parties, lines).
        let invoice = read(ubl).unwrap();
        let cii = write_unchecked(&invoice, Syntax::Cii).unwrap();
        assert!(cii.contains("CrossIndustryInvoice"));
        assert!(cii.contains("SupplyChainTradeTransaction"));
        assert!(!cii.contains("<Invoice "));
    }

    #[test]
    fn pint_my_convert_to_cii_is_cii_not_for_profile() {
        // BT-24 prefix `urn:peppol:pint:billing-1@my-1` selects Profile::PintMy.
        let ubl = r#"<?xml version="1.0"?><Invoice xmlns="urn:oasis:names:specification:ubl:schema:xsd:Invoice-2" xmlns:cbc="urn:oasis:names:specification:ubl:schema:xsd:CommonBasicComponents-2" xmlns:cac="urn:oasis:names:specification:ubl:schema:xsd:CommonAggregateComponents-2"><cbc:CustomizationID>urn:peppol:pint:billing-1@my-1</cbc:CustomizationID><cbc:ID>1</cbc:ID><cbc:IssueDate>2026-01-15</cbc:IssueDate><cbc:InvoiceTypeCode>380</cbc:InvoiceTypeCode><cbc:DocumentCurrencyCode>MYR</cbc:DocumentCurrencyCode><cac:LegalMonetaryTotal><cbc:PayableAmount currencyID="MYR">0</cbc:PayableAmount></cac:LegalMonetaryTotal></Invoice>"#;
        let err = convert(ubl, Syntax::Cii).unwrap_err();
        assert!(matches!(err, FormatError::CiiNotForProfile), "{err:?}");
    }

    #[test]
    fn comment_cross_industry_does_not_dispatch_as_cii() {
        let xml = r#"<?xml version="1.0"?><!-- CrossIndustryInvoice --><Invoice xmlns="urn:oasis:names:specification:ubl:schema:xsd:Invoice-2" xmlns:cbc="urn:oasis:names:specification:ubl:schema:xsd:CommonBasicComponents-2"><cbc:ID>1</cbc:ID></Invoice>"#;
        let inv = read(xml).unwrap();
        assert_eq!(inv.kind, core_invoice::DocumentKind::Invoice);
    }

    #[test]
    fn random_bytes_do_not_panic() {
        let _ = read("\0\0<?xml");
        let _ = read("<");
        let _ = read("<<<<<<<<");
        let _ = read(&"a".repeat(100));
        let _ = read("<!DOCTYPE foo [<!ENTITY x SYSTEM 'file:///etc/passwd'>]><Invoice/>");
        let _ = read(&format!("<Invoice>{}</Invoice>", "x".repeat(200_000)));
        let _ = read(&"<a>".repeat(80));
        let _ = read("<?xml version='1.0'?><Invoice>");
        let _ = read(&format!("<Invoice>{}</Invoice>", "\u{0}".repeat(1000)));
        let _ =
            read("<Invoice xmlns='x'><InvoiceLine><Item><Name/></Item></InvoiceLine></Invoice>");
    }

    #[test]
    fn neither_root_is_parse_error() {
        let xml = r#"<NotAnInvoice/>"#;
        let err = read(xml).unwrap_err();
        assert!(err.to_string().contains("document element"), "{err}");
    }

    #[test]
    fn missing_line_tax_is_not_s() {
        let xml = r#"<?xml version="1.0"?><Invoice xmlns="urn:oasis:names:specification:ubl:schema:xsd:Invoice-2" xmlns:cbc="urn:oasis:names:specification:ubl:schema:xsd:CommonBasicComponents-2" xmlns:cac="urn:oasis:names:specification:ubl:schema:xsd:CommonAggregateComponents-2"><cbc:CustomizationID>urn:cen.eu:en16931:2017</cbc:CustomizationID><cbc:ID>1</cbc:ID><cbc:IssueDate>2026-01-15</cbc:IssueDate><cbc:InvoiceTypeCode>380</cbc:InvoiceTypeCode><cbc:DocumentCurrencyCode>EUR</cbc:DocumentCurrencyCode><cac:InvoiceLine><cbc:ID>1</cbc:ID><cbc:LineExtensionAmount currencyID="EUR">10.00</cbc:LineExtensionAmount><cac:Item><cbc:Name>A</cbc:Name></cac:Item></cac:InvoiceLine></Invoice>"#;
        let inv = read(xml).unwrap();
        assert_ne!(inv.lines[0].tax.code, "S");
        assert!(inv.lines[0].tax.code.is_empty());
        let report = core_invoice::validate(&inv);
        assert!(
            report.findings.iter().any(|f| f.id == "BR-CO-04"),
            "{report}"
        );
    }

    #[test]
    fn unknown_bt24_is_core_spec_01_not_en16931() {
        let xml = r#"<?xml version="1.0"?><Invoice xmlns="urn:oasis:names:specification:ubl:schema:xsd:Invoice-2" xmlns:cbc="urn:oasis:names:specification:ubl:schema:xsd:CommonBasicComponents-2"><cbc:CustomizationID>urn:example:painting</cbc:CustomizationID><cbc:ID>1</cbc:ID></Invoice>"#;
        let inv = read(xml).unwrap();
        assert_eq!(inv.profile, Profile::Unknown);
        assert_eq!(
            inv.specification_id.as_deref(),
            Some("urn:example:painting")
        );
        let report = core_invoice::validate(&inv);
        assert!(
            report.findings.iter().any(|f| f.id == "CORE-SPEC-01"),
            "{report}"
        );
        assert!(!report.findings.iter().any(|f| f.id.starts_with("BR-S-")));
    }

    #[test]
    fn third_decimal_is_malformed_not_zero() {
        let xml = r#"<?xml version="1.0"?><Invoice xmlns="urn:oasis:names:specification:ubl:schema:xsd:Invoice-2" xmlns:cbc="urn:oasis:names:specification:ubl:schema:xsd:CommonBasicComponents-2" xmlns:cac="urn:oasis:names:specification:ubl:schema:xsd:CommonAggregateComponents-2"><cbc:ID>1</cbc:ID><cac:InvoiceLine><cbc:ID>1</cbc:ID><cbc:LineExtensionAmount currencyID="EUR">0.001</cbc:LineExtensionAmount></cac:InvoiceLine></Invoice>"#;
        let traced = read_with_trace(xml).unwrap();
        assert!(
            traced.malformed.iter().any(|m| m.contains("0.001")),
            "{:?}",
            traced.malformed
        );
        assert!(read(xml).is_err());
    }

    #[test]
    fn unit_price_keeps_four_decimals() {
        let xml = r#"<?xml version="1.0"?><Invoice xmlns="urn:oasis:names:specification:ubl:schema:xsd:Invoice-2" xmlns:cbc="urn:oasis:names:specification:ubl:schema:xsd:CommonBasicComponents-2" xmlns:cac="urn:oasis:names:specification:ubl:schema:xsd:CommonAggregateComponents-2"><cbc:ID>1</cbc:ID><cac:InvoiceLine><cbc:ID>1</cbc:ID><cac:Price><cbc:PriceAmount currencyID="EUR">10000.1234</cbc:PriceAmount></cac:Price></cac:InvoiceLine></Invoice>"#;
        let inv = read(xml).unwrap();
        assert_eq!(
            inv.lines[0].price.as_ref().unwrap().net.to_string(),
            "10000.1234"
        );
    }

    #[test]
    fn wire_currencyid_not_iso4217_is_br_cl_03() {
        let xml = r#"<?xml version="1.0"?><Invoice xmlns="urn:oasis:names:specification:ubl:schema:xsd:Invoice-2" xmlns:cbc="urn:oasis:names:specification:ubl:schema:xsd:CommonBasicComponents-2" xmlns:cac="urn:oasis:names:specification:ubl:schema:xsd:CommonAggregateComponents-2"><cbc:CustomizationID>urn:cen.eu:en16931:2017</cbc:CustomizationID><cbc:ID>1</cbc:ID><cbc:InvoiceTypeCode>380</cbc:InvoiceTypeCode><cbc:DocumentCurrencyCode>EUR</cbc:DocumentCurrencyCode><cac:LegalMonetaryTotal><cbc:PayableAmount currencyID="US$">1.00</cbc:PayableAmount></cac:LegalMonetaryTotal></Invoice>"#;
        let report = validate_xml(xml, Some(Profile::En16931)).unwrap();
        assert!(
            report.findings.iter().any(|f| f.id == "BR-CL-03"),
            "{report}"
        );
    }

    #[test]
    fn peppol_mixed_currencyid_is_r051() {
        let xml = r#"<?xml version="1.0"?><Invoice xmlns="urn:oasis:names:specification:ubl:schema:xsd:Invoice-2" xmlns:cbc="urn:oasis:names:specification:ubl:schema:xsd:CommonBasicComponents-2" xmlns:cac="urn:oasis:names:specification:ubl:schema:xsd:CommonAggregateComponents-2"><cbc:CustomizationID>urn:cen.eu:en16931:2017#compliant#urn:fdc:peppol.eu:2017:poacc:billing:3.0</cbc:CustomizationID><cbc:ID>1</cbc:ID><cbc:InvoiceTypeCode>380</cbc:InvoiceTypeCode><cbc:DocumentCurrencyCode>EUR</cbc:DocumentCurrencyCode><cac:LegalMonetaryTotal><cbc:PayableAmount currencyID="USD">1.00</cbc:PayableAmount></cac:LegalMonetaryTotal></Invoice>"#;
        let report = validate_xml(xml, Some(Profile::PeppolBis3)).unwrap();
        assert!(
            report
                .findings
                .iter()
                .any(|f| f.id == "PEPPOL-EN16931-R051"),
            "{report}"
        );
    }

    #[test]
    fn unknown_direct_child_is_unmapped() {
        let xml = r#"<?xml version="1.0"?><Invoice xmlns="urn:oasis:names:specification:ubl:schema:xsd:Invoice-2" xmlns:cbc="urn:oasis:names:specification:ubl:schema:xsd:CommonBasicComponents-2"><cbc:ID>1</cbc:ID><cbc:Foo>bar</cbc:Foo></Invoice>"#;
        let traced = read_with_trace(xml).unwrap();
        assert!(
            traced.unmapped.iter().any(|u| u.contains("Foo")),
            "{:?}",
            traced.unmapped
        );
    }

    #[test]
    fn diff_sees_issue_date() {
        let a = r#"<?xml version="1.0"?><Invoice xmlns="urn:oasis:names:specification:ubl:schema:xsd:Invoice-2" xmlns:cbc="urn:oasis:names:specification:ubl:schema:xsd:CommonBasicComponents-2"><cbc:ID>1</cbc:ID><cbc:IssueDate>2026-01-15</cbc:IssueDate><cbc:DocumentCurrencyCode>EUR</cbc:DocumentCurrencyCode></Invoice>"#;
        let b = r#"<?xml version="1.0"?><Invoice xmlns="urn:oasis:names:specification:ubl:schema:xsd:Invoice-2" xmlns:cbc="urn:oasis:names:specification:ubl:schema:xsd:CommonBasicComponents-2"><cbc:ID>1</cbc:ID><cbc:IssueDate>2026-01-16</cbc:IssueDate><cbc:DocumentCurrencyCode>EUR</cbc:DocumentCurrencyCode></Invoice>"#;
        let out = diff(a, b).unwrap();
        assert!(out.contains("issue_date"), "{out}");
        assert_ne!(out, "no semantic difference");
    }

    #[test]
    fn xs_boolean_accepts_one_and_zero() {
        assert_eq!(xml::parse_xs_boolean("1"), Some(true));
        assert_eq!(xml::parse_xs_boolean("0"), Some(false));
        assert_eq!(xml::parse_xs_boolean(" true "), Some(true));
        assert_eq!(xml::parse_xs_boolean("false"), Some(false));
    }

    #[test]
    fn cii_dtd_is_refused() {
        let xml = r#"<!DOCTYPE CrossIndustryInvoice [<!ENTITY x "a">]><rsm:CrossIndustryInvoice xmlns:rsm="urn:un:unece:uncefact:data:standard:CrossIndustryInvoice:100"/>"#;
        assert!(read(xml).is_err());
    }

    #[test]
    fn ubl_to_cii_to_ubl_keeps_qty_price() {
        let ubl = r#"<?xml version="1.0"?><Invoice xmlns="urn:oasis:names:specification:ubl:schema:xsd:Invoice-2" xmlns:cbc="urn:oasis:names:specification:ubl:schema:xsd:CommonBasicComponents-2" xmlns:cac="urn:oasis:names:specification:ubl:schema:xsd:CommonAggregateComponents-2"><cbc:CustomizationID>urn:cen.eu:en16931:2017</cbc:CustomizationID><cbc:ID>1</cbc:ID><cbc:IssueDate>2026-01-15</cbc:IssueDate><cbc:InvoiceTypeCode>380</cbc:InvoiceTypeCode><cbc:DocumentCurrencyCode>EUR</cbc:DocumentCurrencyCode><cac:InvoiceLine><cbc:ID>1</cbc:ID><cbc:InvoicedQuantity unitCode="C62">2</cbc:InvoicedQuantity><cbc:LineExtensionAmount currencyID="EUR">10.00</cbc:LineExtensionAmount><cac:Item><cbc:Name>A</cbc:Name><cac:ClassifiedTaxCategory><cbc:ID>S</cbc:ID><cbc:Percent>19</cbc:Percent><cac:TaxScheme><cbc:ID>VAT</cbc:ID></cac:TaxScheme></cac:ClassifiedTaxCategory></cac:Item><cac:Price><cbc:PriceAmount currencyID="EUR">5.00</cbc:PriceAmount></cac:Price></cac:InvoiceLine></Invoice>"#;
        let inv = read(ubl).unwrap();
        assert!(inv.lines[0].quantity.is_some());
        let cii = write_unchecked(&inv, Syntax::Cii).unwrap();
        let back = read(&cii).unwrap();
        assert_eq!(back.lines[0].quantity, inv.lines[0].quantity);
        assert_eq!(
            back.lines[0].price.as_ref().map(|p| p.net.to_string()),
            Some("5.00".into())
        );
        let ubl2 = write_unchecked(&back, Syntax::Ubl).unwrap();
        let out = diff(ubl, &ubl2).unwrap();
        assert!(!out.contains("quantity"), "qty is mapped on CII: {out}");
        for line in out.lines() {
            if line == "no semantic difference" {
                continue;
            }
            assert!(
                CII_DROPPED.iter().any(|p| line.contains(p)),
                "unexpected CII drop {line:?} in {out}"
            );
        }
    }

    #[test]
    fn cii_to_ubl_round_trip_is_named_drops() {
        let inv = read(
            r#"<?xml version="1.0"?><Invoice xmlns="urn:oasis:names:specification:ubl:schema:xsd:Invoice-2" xmlns:cbc="urn:oasis:names:specification:ubl:schema:xsd:CommonBasicComponents-2" xmlns:cac="urn:oasis:names:specification:ubl:schema:xsd:CommonAggregateComponents-2"><cbc:CustomizationID>urn:cen.eu:en16931:2017</cbc:CustomizationID><cbc:ID>1</cbc:ID><cbc:IssueDate>2026-01-15</cbc:IssueDate><cbc:InvoiceTypeCode>380</cbc:InvoiceTypeCode><cbc:DocumentCurrencyCode>EUR</cbc:DocumentCurrencyCode><cac:InvoiceLine><cbc:ID>1</cbc:ID><cbc:InvoicedQuantity unitCode="C62">2</cbc:InvoicedQuantity><cbc:LineExtensionAmount currencyID="EUR">10.00</cbc:LineExtensionAmount><cac:Item><cbc:Name>A</cbc:Name><cac:ClassifiedTaxCategory><cbc:ID>S</cbc:ID><cbc:Percent>19</cbc:Percent><cac:TaxScheme><cbc:ID>VAT</cbc:ID></cac:TaxScheme></cac:ClassifiedTaxCategory></cac:Item><cac:Price><cbc:PriceAmount currencyID="EUR">5.00</cbc:PriceAmount></cac:Price></cac:InvoiceLine></Invoice>"#,
        )
        .unwrap();
        let cii = write_unchecked(&inv, Syntax::Cii).unwrap();
        let from_cii = read(&cii).unwrap();
        let ubl = write_unchecked(&from_cii, Syntax::Ubl).unwrap();
        let back = read(&ubl).unwrap();
        let out = diff_invoices(&inv, &back);
        for line in out.lines() {
            if line == "no semantic difference" {
                continue;
            }
            assert!(
                CII_DROPPED.iter().any(|p| line.contains(p)),
                "unexpected CII drop {line:?} in {out}"
            );
        }
        assert!(!cii.contains("<Invoice "));
    }

    #[test]
    fn diff_sees_payee_and_attachment() {
        let a = r#"<?xml version="1.0"?><Invoice xmlns="urn:oasis:names:specification:ubl:schema:xsd:Invoice-2" xmlns:cbc="urn:oasis:names:specification:ubl:schema:xsd:CommonBasicComponents-2" xmlns:cac="urn:oasis:names:specification:ubl:schema:xsd:CommonAggregateComponents-2"><cbc:ID>1</cbc:ID></Invoice>"#;
        let b = r#"<?xml version="1.0"?><Invoice xmlns="urn:oasis:names:specification:ubl:schema:xsd:Invoice-2" xmlns:cbc="urn:oasis:names:specification:ubl:schema:xsd:CommonBasicComponents-2" xmlns:cac="urn:oasis:names:specification:ubl:schema:xsd:CommonAggregateComponents-2"><cbc:ID>1</cbc:ID><cac:PayeeParty><cac:PartyName><cbc:Name>Payee AG</cbc:Name></cac:PartyName></cac:PayeeParty></Invoice>"#;
        let out = diff(a, b).unwrap();
        assert!(out.contains("payee"), "{out}");
        let c = r#"<?xml version="1.0"?><Invoice xmlns="urn:oasis:names:specification:ubl:schema:xsd:Invoice-2" xmlns:cbc="urn:oasis:names:specification:ubl:schema:xsd:CommonBasicComponents-2" xmlns:cac="urn:oasis:names:specification:ubl:schema:xsd:CommonAggregateComponents-2"><cbc:ID>1</cbc:ID><cac:AdditionalDocumentReference><cbc:ID>ATT</cbc:ID><cac:Attachment><cbc:EmbeddedDocumentBinaryObject mimeCode="application/pdf" filename="a.pdf">YQ==</cbc:EmbeddedDocumentBinaryObject></cac:Attachment></cac:AdditionalDocumentReference></Invoice>"#;
        let out = diff(a, c).unwrap();
        assert!(out.contains("supporting_documents"), "{out}");
    }

    #[test]
    fn cii_missing_line_tax_is_not_sst() {
        let xml = r#"<?xml version="1.0"?><rsm:CrossIndustryInvoice xmlns:rsm="urn:un:unece:uncefact:data:standard:CrossIndustryInvoice:100" xmlns:ram="urn:un:unece:uncefact:data:standard:ReusableAggregateBusinessInformationEntity:100" xmlns:udt="urn:un:unece:uncefact:data:standard:UnqualifiedDataType:100"><rsm:ExchangedDocumentContext><ram:GuidelineSpecifiedDocumentContextParameter><ram:ID>urn:cen.eu:en16931:2017</ram:ID></ram:GuidelineSpecifiedDocumentContextParameter></rsm:ExchangedDocumentContext><rsm:ExchangedDocument><ram:ID>1</ram:ID><ram:TypeCode>380</ram:TypeCode></rsm:ExchangedDocument><rsm:SupplyChainTradeTransaction><ram:IncludedSupplyChainTradeLineItem><ram:AssociatedDocumentLineDocument><ram:LineID>1</ram:LineID></ram:AssociatedDocumentLineDocument><ram:SpecifiedTradeProduct><ram:Name>A</ram:Name></ram:SpecifiedTradeProduct></ram:IncludedSupplyChainTradeLineItem><ram:ApplicableHeaderTradeAgreement/><ram:ApplicableHeaderTradeDelivery/><ram:ApplicableHeaderTradeSettlement/></rsm:SupplyChainTradeTransaction></rsm:CrossIndustryInvoice>"#;
        let inv = read(xml).unwrap();
        assert_ne!(inv.lines[0].tax.system, core_invoice::TaxSystem::Sst);
        assert!(inv.lines[0].tax.code.is_empty());
    }

    #[test]
    fn cii_kind_from_credit_note_type_not_only_381() {
        let xml = r#"<?xml version="1.0"?><rsm:CrossIndustryInvoice xmlns:rsm="urn:un:unece:uncefact:data:standard:CrossIndustryInvoice:100" xmlns:ram="urn:un:unece:uncefact:data:standard:ReusableAggregateBusinessInformationEntity:100"><rsm:ExchangedDocumentContext><ram:GuidelineSpecifiedDocumentContextParameter><ram:ID>urn:cen.eu:en16931:2017</ram:ID></ram:GuidelineSpecifiedDocumentContextParameter></rsm:ExchangedDocumentContext><rsm:ExchangedDocument><ram:ID>1</ram:ID><ram:TypeCode>396</ram:TypeCode></rsm:ExchangedDocument><rsm:SupplyChainTradeTransaction><ram:ApplicableHeaderTradeAgreement/><ram:ApplicableHeaderTradeDelivery/><ram:ApplicableHeaderTradeSettlement/></rsm:SupplyChainTradeTransaction></rsm:CrossIndustryInvoice>"#;
        let inv = read(xml).unwrap();
        assert_eq!(inv.kind, core_invoice::DocumentKind::CreditNote);
    }

    #[test]
    fn cii_non_102_date_is_malformed() {
        let xml = r#"<?xml version="1.0"?><rsm:CrossIndustryInvoice xmlns:rsm="urn:un:unece:uncefact:data:standard:CrossIndustryInvoice:100" xmlns:ram="urn:un:unece:uncefact:data:standard:ReusableAggregateBusinessInformationEntity:100" xmlns:udt="urn:un:unece:uncefact:data:standard:UnqualifiedDataType:100"><rsm:ExchangedDocumentContext><ram:GuidelineSpecifiedDocumentContextParameter><ram:ID>urn:cen.eu:en16931:2017</ram:ID></ram:GuidelineSpecifiedDocumentContextParameter></rsm:ExchangedDocumentContext><rsm:ExchangedDocument><ram:ID>1</ram:ID><ram:TypeCode>380</ram:TypeCode><ram:IssueDateTime><udt:DateTimeString format="616">2026</udt:DateTimeString></ram:IssueDateTime></rsm:ExchangedDocument><rsm:SupplyChainTradeTransaction><ram:ApplicableHeaderTradeAgreement/><ram:ApplicableHeaderTradeDelivery/><ram:ApplicableHeaderTradeSettlement/></rsm:SupplyChainTradeTransaction></rsm:CrossIndustryInvoice>"#;
        let traced = read_with_trace(xml).unwrap();
        assert!(
            traced.malformed.iter().any(|m| m.contains("format=616")),
            "{:?}",
            traced.malformed
        );
        assert!(traced.invoice.issue_date.is_none());
    }

    #[test]
    fn cii_two_ibans_round_trip() {
        let mut inv = read(
            r#"<?xml version="1.0"?><Invoice xmlns="urn:oasis:names:specification:ubl:schema:xsd:Invoice-2" xmlns:cbc="urn:oasis:names:specification:ubl:schema:xsd:CommonBasicComponents-2"><cbc:CustomizationID>urn:cen.eu:en16931:2017</cbc:CustomizationID><cbc:ID>1</cbc:ID><cbc:InvoiceTypeCode>380</cbc:InvoiceTypeCode><cbc:DocumentCurrencyCode>EUR</cbc:DocumentCurrencyCode></Invoice>"#,
        )
        .unwrap();
        inv.payment = Some(core_invoice::PaymentInstructions {
            means_code: Some(core_invoice::Code::new("30")),
            means_text: None,
            remittance: None,
            means: Some(core_invoice::PaymentMeans::CreditTransfer(vec![
                core_invoice::CreditTransfer {
                    account_id: core_invoice::Identifier::new("DE89370400440532013000"),
                    account_name: None,
                    provider: None,
                },
                core_invoice::CreditTransfer {
                    account_id: core_invoice::Identifier::new("FR1420041010050500013M02606"),
                    account_name: None,
                    provider: None,
                },
            ])),
        });
        let cii = write_unchecked(&inv, Syntax::Cii).unwrap();
        assert_eq!(
            cii.matches("<ram:SpecifiedTradeSettlementPaymentMeans>")
                .count(),
            2
        );
        let back = read(&cii).unwrap();
        match back.payment.unwrap().means.unwrap() {
            core_invoice::PaymentMeans::CreditTransfer(a) => {
                assert_eq!(a.len(), 2);
                assert_eq!(a[1].account_id.value, "FR1420041010050500013M02606");
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn cii_direct_read_refuses_oversize() {
        let xml = format!(
            r#"<rsm:CrossIndustryInvoice xmlns:rsm="urn:un:unece:uncefact:data:standard:CrossIndustryInvoice:100">{}</rsm:CrossIndustryInvoice>"#,
            "x".repeat(10 * 1024 * 1024 + 1)
        );
        assert!(cii::read(&xml).is_err());
    }

    #[test]
    fn write_drops_credit_note_duedate() {
        let mut inv = read(
            r#"<?xml version="1.0"?><Invoice xmlns="urn:oasis:names:specification:ubl:schema:xsd:Invoice-2" xmlns:cbc="urn:oasis:names:specification:ubl:schema:xsd:CommonBasicComponents-2"><cbc:ID>1</cbc:ID><cbc:DueDate>2026-02-01</cbc:DueDate></Invoice>"#,
        )
        .unwrap();
        inv.kind = core_invoice::DocumentKind::CreditNote;
        let drops = write_drops(&inv, Syntax::Ubl);
        assert!(drops.iter().any(|d| d == "CreditNote/DueDate"), "{drops:?}");
    }
}
