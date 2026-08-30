//! UBL 2.1 and UN/CEFACT CII on top of [`core_invoice`].
//!
//! Conversion goes through the semantic model, not tag-by-tag.

use core_invoice::{Invoice, Profile, Report};

pub mod cii;
pub mod ubl;

#[derive(Debug, thiserror::Error)]
pub enum FormatError {
    #[error("unsupported syntax {0}")]
    UnsupportedSyntax(String),
    #[error("parse error: {0}")]
    Parse(String),
    #[error(
        "CII D16B is not implemented; convert --to cii is refused until a real UN/CEFACT mapping exists"
    )]
    CiiNotImplemented,
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

pub fn write(invoice: &Invoice, syntax: Syntax) -> Result<String, FormatError> {
    match syntax {
        Syntax::Ubl => Ok(ubl::write(invoice)),
        Syntax::Cii => cii::write(invoice),
    }
}

pub fn write_validated<P: core_invoice::ProfileMarker>(
    proof: &core_invoice::Validated<P>,
    syntax: Syntax,
) -> Result<String, FormatError> {
    write(proof.invoice(), syntax)
}

pub fn convert(xml: &str, to: Syntax) -> Result<String, FormatError> {
    let invoice = read(xml)?;
    write(&invoice, to)
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
    fn cii_is_refused() {
        let xml = r#"<rsm:CrossIndustryInvoice xmlns:rsm="urn:un:unece:uncefact:data:standard:CrossIndustryInvoice:100"/>"#;
        assert!(matches!(read(xml), Err(FormatError::CiiNotImplemented)));
        let ubl = r#"<?xml version="1.0"?><Invoice xmlns="urn:oasis:names:specification:ubl:schema:xsd:Invoice-2" xmlns:cbc="urn:oasis:names:specification:ubl:schema:xsd:CommonBasicComponents-2" xmlns:cac="urn:oasis:names:specification:ubl:schema:xsd:CommonAggregateComponents-2"><cbc:CustomizationID>urn:peppol:pint:billing-1</cbc:CustomizationID><cbc:ID>1</cbc:ID><cbc:DocumentCurrencyCode>EUR</cbc:DocumentCurrencyCode><cac:LegalMonetaryTotal><cbc:PayableAmount>0</cbc:PayableAmount></cac:LegalMonetaryTotal></Invoice>"#;
        assert!(matches!(
            convert(ubl, Syntax::Cii),
            Err(FormatError::CiiNotImplemented)
        ));
    }
}
