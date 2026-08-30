use crate::FormatError;
use core_invoice::{Amount, Invoice, Line, Party, Profile, TaxCategory, TaxSystem};
use rust_decimal::Decimal;
use std::str::FromStr;

pub fn write(invoice: &Invoice) -> String {
    let mut lines = String::new();
    for line in &invoice.lines {
        lines.push_str(&format!(
            r#"    <cac:InvoiceLine>
      <cbc:ID>{}</cbc:ID>
      <cbc:LineExtensionAmount currencyID="{}">{}</cbc:LineExtensionAmount>
      <cac:Item>
        <cbc:Name>{}</cbc:Name>
        <cac:ClassifiedTaxCategory>
          <cbc:ID>{}</cbc:ID>
          <cbc:Percent>{}</cbc:Percent>
          <cac:TaxScheme><cbc:ID>{}</cbc:ID></cac:TaxScheme>
        </cac:ClassifiedTaxCategory>
      </cac:Item>
    </cac:InvoiceLine>
"#,
            escape(&line.id),
            escape(&invoice.currency),
            line.net,
            escape(&line.name),
            escape(&line.tax.code),
            line.tax.percent,
            line.tax.system.as_str(),
        ));
    }

    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<Invoice xmlns="urn:oasis:names:specification:ubl:schema:xsd:Invoice-2"
         xmlns:cac="urn:oasis:names:specification:ubl:schema:xsd:CommonAggregateComponents-2"
         xmlns:cbc="urn:oasis:names:specification:ubl:schema:xsd:CommonBasicComponents-2">
  <cbc:CustomizationID>{}</cbc:CustomizationID>
  <cbc:ID>{}</cbc:ID>
  <cbc:DocumentCurrencyCode>{}</cbc:DocumentCurrencyCode>
  <cac:AccountingSupplierParty>
    <cac:Party>
      <cac:PartyName><cbc:Name>{}</cbc:Name></cac:PartyName>
      <cac:PostalAddress><cac:Country><cbc:IdentificationCode>{}</cbc:IdentificationCode></cac:Country></cac:PostalAddress>
    </cac:Party>
  </cac:AccountingSupplierParty>
  <cac:AccountingCustomerParty>
    <cac:Party>
      <cac:PartyName><cbc:Name>{}</cbc:Name></cac:PartyName>
      <cac:PostalAddress><cac:Country><cbc:IdentificationCode>{}</cbc:IdentificationCode></cac:Country></cac:PostalAddress>
    </cac:Party>
  </cac:AccountingCustomerParty>
  <cac:TaxTotal>
    <cbc:TaxAmount currencyID="{}">{}</cbc:TaxAmount>
  </cac:TaxTotal>
  <cac:LegalMonetaryTotal>
    <cbc:PayableAmount currencyID="{}">{}</cbc:PayableAmount>
  </cac:LegalMonetaryTotal>
{lines}</Invoice>
"#,
        escape(
            invoice
                .specification_id
                .as_deref()
                .unwrap_or_else(|| invoice.profile.specification_id()),
        ),
        escape(&invoice.number),
        escape(&invoice.currency),
        escape(&invoice.seller.name),
        escape(&invoice.seller.country),
        escape(&invoice.buyer.name),
        escape(&invoice.buyer.country),
        escape(&invoice.currency),
        invoice.tax_total,
        escape(&invoice.currency),
        invoice.payable,
    )
}

pub fn read(xml: &str) -> Result<Invoice, FormatError> {
    let customization = tag(xml, "CustomizationID");
    let profile = match customization.as_deref() {
        Some(id) => match Profile::for_specification_id(id) {
            core_invoice::ProfileLookup::Profile(p) => p,
            core_invoice::ProfileLookup::WrongProcess | core_invoice::ProfileLookup::Unknown => {
                Profile::En16931
            }
        },
        None => Profile::En16931,
    };

    let number = tag(xml, "ID").ok_or_else(|| FormatError::Parse("missing cbc:ID".into()))?;
    let currency = tag(xml, "DocumentCurrencyCode").unwrap_or_else(|| "EUR".into());
    let payable = amount_from(xml, "PayableAmount")?;
    let tax_total = amount_from(xml, "TaxAmount").unwrap_or(Amount::ZERO);

    let names: Vec<String> = all_tags(xml, "Name");
    let seller_name = names.first().cloned().unwrap_or_default();
    let buyer_name = names.get(1).cloned().unwrap_or_default();
    let countries = all_tags(xml, "IdentificationCode");
    let seller_country = countries.first().cloned().unwrap_or_else(|| "XX".into());
    let buyer_country = countries.get(1).cloned().unwrap_or_else(|| "XX".into());

    let mut lines = Vec::new();
    for (i, chunk) in xml.split("<cac:InvoiceLine").skip(1).enumerate() {
        let net = amount_from(chunk, "LineExtensionAmount").unwrap_or(Amount::ZERO);
        let name = tag(chunk, "Name").unwrap_or_else(|| format!("Line {}", i + 1));
        let code = tag(chunk, "ID").unwrap_or_else(|| "S".into());
        let percent = tag(chunk, "Percent")
            .and_then(|s| Decimal::from_str(&s).ok())
            .map(core_invoice::Percentage::new)
            .unwrap_or(core_invoice::Percentage::ZERO);
        let system = tag(chunk, "TaxScheme")
            .and_then(|s| extract_inner(&s).or(Some(s)))
            .and_then(|s| TaxSystem::parse(&s))
            .or_else(|| tag(chunk, "cbc:ID").and_then(|id| TaxSystem::parse(&id)))
            .unwrap_or_else(|| default_tax(profile));
        // Prefer TaxScheme/ID if present
        let system = inner_tag(chunk, "TaxScheme", "ID")
            .and_then(|s| TaxSystem::parse(&s))
            .unwrap_or(system);
        lines.push(Line {
            id: format!("{}", i + 1),
            name,
            net,
            tax: TaxCategory {
                system,
                code,
                percent,
            },
        });
    }

    Ok(Invoice {
        profile,
        specification_id: customization,
        kind: core_invoice::DocumentKind::Invoice,
        number,
        currency,
        seller: Party::new(seller_name, seller_country),
        buyer: Party::new(buyer_name, buyer_country),
        lines,
        tax_total,
        payable,
    })
}

fn default_tax(profile: Profile) -> TaxSystem {
    match profile {
        Profile::PintMy => TaxSystem::Sst,
        Profile::Pint => TaxSystem::Gst,
        _ => TaxSystem::Vat,
    }
}

fn amount_from(xml: &str, name: &str) -> Result<Amount, FormatError> {
    match tag(xml, name) {
        Some(s) => Amount::parse(&s).map_err(|e| FormatError::Parse(e.to_string())),
        None => Err(FormatError::Parse(format!("missing {name}"))),
    }
}

fn tag(xml: &str, local: &str) -> Option<String> {
    first_tagged_text(xml, local)
}

fn first_tagged_text(xml: &str, local: &str) -> Option<String> {
    for prefix in ["cbc:", ""] {
        let needle = format!("<{prefix}{local}");
        let mut rest = xml;
        while let Some(start) = rest.find(&needle) {
            let after = &rest[start + needle.len()..];
            // `<cbc:Foo>` or `<cbc:Foo currencyID="MYR">`
            if after.starts_with('>') || after.starts_with(' ') || after.starts_with('\n') {
                let gt = after.find('>')?;
                let inner = &after[gt + 1..];
                let close = format!("</{prefix}{local}>");
                let end = inner.find(&close)?;
                return Some(inner[..end].trim().to_string());
            }
            rest = &rest[start + needle.len()..];
        }
    }
    None
}

fn all_tags(xml: &str, local: &str) -> Vec<String> {
    let mut out = Vec::new();
    let needle = format!("<cbc:{local}");
    let close = format!("</cbc:{local}>");
    let mut rest = xml;
    while let Some(start) = rest.find(&needle) {
        let after = &rest[start + needle.len()..];
        if let Some(gt) = after.find('>') {
            let inner = &after[gt + 1..];
            if let Some(end) = inner.find(&close) {
                out.push(inner[..end].trim().to_string());
                rest = &inner[end + close.len()..];
                continue;
            }
        }
        break;
    }
    out
}

fn inner_tag(xml: &str, parent: &str, child: &str) -> Option<String> {
    let open = format!("<cac:{parent}>");
    let start = xml.find(&open)?;
    let rest = &xml[start..];
    tag(rest, child)
}

fn extract_inner(s: &str) -> Option<String> {
    let t = s.trim();
    if t.contains('<') {
        None
    } else {
        Some(t.to_string())
    }
}

fn escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
