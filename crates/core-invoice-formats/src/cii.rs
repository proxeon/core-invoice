use crate::FormatError;
use crate::ubl;
use core_invoice::Invoice;

/// CII is emitted as a CrossIndustryInvoice wrapper; read maps back through the
/// same fields we write. Full D16B coverage is later work.
pub fn write(invoice: &Invoice) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<rsm:CrossIndustryInvoice xmlns:rsm="urn:un:unece:uncefact:data:standard:CrossIndustryInvoice:100">
  <rsm:ExchangedDocument>
    <ram:ID xmlns:ram="urn:un:unece:uncefact:data:standard:ReusableAggregateBusinessInformationEntity:100">{}</ram:ID>
  </rsm:ExchangedDocument>
  <rsm:SupplyChainTradeTransaction>
    <!-- UBL sibling used as the semantic payload until a full CII mapping lands. -->
    <ubl:payload xmlns:ubl="urn:oasis:names:specification:ubl:schema:xsd:Invoice-2">
{}
    </ubl:payload>
  </rsm:SupplyChainTradeTransaction>
</rsm:CrossIndustryInvoice>
"#,
        xml_escape(&invoice.number),
        ubl::write(invoice)
            .lines()
            .filter(|l| !l.starts_with("<?xml"))
            .collect::<Vec<_>>()
            .join("\n"),
    )
}

pub fn read(xml: &str) -> Result<Invoice, FormatError> {
    if let Some(start) = xml.find("<Invoice") {
        ubl::read(&xml[start..])
    } else {
        Err(FormatError::Parse(
            "CII document without an embedded Invoice payload".into(),
        ))
    }
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}
