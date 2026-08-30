//! UBL 2.1 and UN/CEFACT CII on top of [`core_invoice`].
//!
//! Conversion goes through the semantic model, not tag-by-tag.

use core_invoice::{
    En16931Marker, Invoice, PeppolBis3Marker, PintMarker, PintMyMarker, Profile, ProfileMarker,
    Report, Validated,
};

pub mod cii;
pub mod ubl;

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
    }
}

fn prove_write<P: ProfileMarker>(invoice: Invoice, syntax: Syntax) -> Result<String, FormatError> {
    match Validated::<P>::new(invoice) {
        Ok(proof) => write_validated(&proof, syntax),
        Err(rejected) => Err(FormatError::Semantic(SemanticReject(rejected.1))),
    }
}

pub fn read(xml: &str) -> Result<Invoice, FormatError> {
    if xml.to_ascii_lowercase().contains("<!doctype") {
        return Err(FormatError::Parse("DTD is refused".into()));
    }
    let rest = xml.trim_start();
    if rest.contains("CrossIndustryInvoice") {
        return cii::read(xml);
    }
    match ubl::sniff(xml) {
        Ok(_) => ubl::read(xml),
        Err(_) if rest.contains("urn:oasis:names:specification:ubl") => ubl::read(xml),
        Err(e) => Err(e),
    }
}

pub fn validate_xml(xml: &str, profile: Option<Profile>) -> Result<Report, FormatError> {
    let mut invoice = read(xml)?;
    if let Some(profile) = profile {
        invoice.profile = profile;
    }
    let mut report = core_invoice::validate(&invoice);
    report.profile_slug = invoice.profile.slug();
    Ok(report)
}

pub fn diff(left_xml: &str, right_xml: &str) -> Result<String, FormatError> {
    let left = read(left_xml)?;
    let right = read(right_xml)?;
    let mut lines = Vec::new();
    if left.number != right.number {
        lines.push(format!("number: {} → {}", left.number, right.number));
    }
    if left.payable != right.payable {
        lines.push(format!("payable: {} → {}", left.payable, right.payable));
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
    if lines.is_empty() {
        Ok("no semantic difference".into())
    } else {
        Ok(lines.join("\n"))
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
}
