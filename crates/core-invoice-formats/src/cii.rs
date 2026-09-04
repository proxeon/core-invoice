//! UN/CEFACT CII D16B. Three-part envelope; line items **before** header
//! agreement/delivery/settlement. Dates are format `102` (YYYYMMDD).
//! Not a UBL wrapper.
//!
//! Subset for EN 16931 and Peppol BIS. `Profile::Pint` (international) may emit
//! the same envelope. **PINT-MY is UBL-only.**

use crate::xml;
use crate::{FormatError, Read};
use core_invoice::kind::DocumentKind;
use core_invoice::numeric::Percentage;
use core_invoice::tax::{TaxCategory, TaxSystem, wire_scheme};
use core_invoice::{
    Amount, Code, Date, DocumentReference, DocumentTotals, Identifier, Invoice, InvoiceAmount,
    InvoiceNote, Line, Party, Payee, Profile, ProfileLookup, SupportingDocument, TaxBreakdown,
    TaxRepresentative,
};
use rust_decimal::Decimal;
use std::str::FromStr;

const NS_RSM: &str = "urn:un:unece:uncefact:data:standard:CrossIndustryInvoice:100";
const NS_RAM: &str =
    "urn:un:unece:uncefact:data:standard:ReusableAggregateBusinessInformationEntity:100";
const NS_UDT: &str = "urn:un:unece:uncefact:data:standard:UnqualifiedDataType:100";

/// Unchecked CII serialisation. Does not prove. Production write is crate
/// [`write_validated`](crate::write_validated).
///
/// `Profile::Pint` (international) is allowed to emit this subset. PINT-MY is not.
pub fn write_unchecked(invoice: &Invoice) -> Result<String, FormatError> {
    // PINT-MY is UBL-only; CII is EN/Peppol (and later ZUGFeRD extract).
    if invoice.profile == Profile::PintMy {
        return Err(FormatError::CiiNotForProfile);
    }
    let spec = invoice
        .specification_id
        .as_deref()
        .unwrap_or_else(|| invoice.profile.specification_id());
    let type_code = invoice.type_code.as_ref().map(Code::as_str);
    let mut s = String::new();
    s.push_str(r#"<?xml version="1.0" encoding="UTF-8"?>"#);
    s.push('\n');
    s.push_str(&format!(
        r#"<rsm:CrossIndustryInvoice xmlns:rsm="{NS_RSM}" xmlns:ram="{NS_RAM}" xmlns:udt="{NS_UDT}">"#
    ));
    s.push('\n');
    s.push_str("  <rsm:ExchangedDocumentContext>\n");
    if let Some(bp) = invoice.business_process.as_deref() {
        s.push_str("    <ram:BusinessProcessSpecifiedDocumentContextParameter>\n");
        leaf_ram(&mut s, 3, "ID", bp, None);
        s.push_str("    </ram:BusinessProcessSpecifiedDocumentContextParameter>\n");
    }
    s.push_str("    <ram:GuidelineSpecifiedDocumentContextParameter>\n");
    leaf_ram(&mut s, 3, "ID", spec, None);
    s.push_str("    </ram:GuidelineSpecifiedDocumentContextParameter>\n");
    s.push_str("  </rsm:ExchangedDocumentContext>\n");
    s.push_str("  <rsm:ExchangedDocument>\n");
    leaf_ram(&mut s, 2, "ID", &invoice.number, None);
    // Emit BT-3. Do not invent 380/381 when the type code is missing.
    if let Some(code) = type_code {
        leaf_ram(&mut s, 2, "TypeCode", code, None);
    }
    if let Some(d) = invoice.issue_date {
        s.push_str("    <ram:IssueDateTime>\n");
        s.push_str(&format!(
            "      <udt:DateTimeString format=\"102\">{}</udt:DateTimeString>\n",
            to_102(d)
        ));
        s.push_str("    </ram:IssueDateTime>\n");
    }
    for n in &invoice.notes {
        s.push_str("    <ram:IncludedNote>\n");
        leaf_ram(&mut s, 3, "Content", &n.text, None);
        if let Some(subj) = n.subject.as_ref() {
            leaf_ram(&mut s, 3, "SubjectCode", subj.as_str(), None);
        }
        s.push_str("    </ram:IncludedNote>\n");
    }
    s.push_str("  </rsm:ExchangedDocument>\n");
    s.push_str("  <rsm:SupplyChainTradeTransaction>\n");
    for line in &invoice.lines {
        write_line(&mut s, line, invoice);
    }
    s.push_str("    <ram:ApplicableHeaderTradeAgreement>\n");
    if let Some(br) = invoice.buyer_reference.as_ref() {
        leaf_ram(&mut s, 3, "BuyerReference", br.as_str(), None);
    }
    write_trade_party(&mut s, "SellerTradeParty", &invoice.seller, true);
    write_trade_party(&mut s, "BuyerTradeParty", &invoice.buyer, false);
    write_tax_rep(&mut s, invoice.tax_representative.as_ref());
    write_doc_ref(
        &mut s,
        "SellerOrderReferencedDocument",
        invoice.sales_order.as_ref(),
    );
    write_doc_ref(
        &mut s,
        "BuyerOrderReferencedDocument",
        invoice.purchase_order.as_ref(),
    );
    write_doc_ref(
        &mut s,
        "ContractReferencedDocument",
        invoice.contract.as_ref(),
    );
    for d in &invoice.supporting_documents {
        write_additional_doc(&mut s, d, "916");
    }
    if let Some(t) = invoice.tender.as_ref() {
        s.push_str("      <ram:AdditionalReferencedDocument>\n");
        leaf_ram(&mut s, 4, "IssuerAssignedID", t.as_str(), None);
        leaf_ram(&mut s, 4, "TypeCode", "50", None);
        s.push_str("      </ram:AdditionalReferencedDocument>\n");
    }
    if let Some(o) = invoice.invoiced_object.as_ref() {
        s.push_str("      <ram:AdditionalReferencedDocument>\n");
        leaf_ram(&mut s, 4, "IssuerAssignedID", &o.value, None);
        leaf_ram(&mut s, 4, "TypeCode", "130", None);
        if let Some(sc) = o.scheme.as_deref() {
            leaf_ram(&mut s, 4, "ReferenceTypeCode", sc, None);
        }
        s.push_str("      </ram:AdditionalReferencedDocument>\n");
    }
    if let Some(p) = invoice.project.as_ref() {
        s.push_str("      <ram:SpecifiedProcuringProject>\n");
        leaf_ram(&mut s, 4, "ID", p.as_str(), None);
        leaf_ram(&mut s, 4, "Name", p.as_str(), None);
        s.push_str("      </ram:SpecifiedProcuringProject>\n");
    }
    s.push_str("    </ram:ApplicableHeaderTradeAgreement>\n");
    write_delivery(&mut s, invoice);
    s.push_str("    <ram:ApplicableHeaderTradeSettlement>\n");
    if let Some(r) = invoice
        .payment
        .as_ref()
        .and_then(|p| p.remittance.as_deref())
    {
        leaf_ram(&mut s, 3, "PaymentReference", r, None);
    }
    if !invoice.currency.is_empty() {
        leaf_ram(&mut s, 3, "InvoiceCurrencyCode", &invoice.currency, None);
    }
    write_payee(&mut s, invoice.payee.as_ref());
    write_payment(&mut s, invoice);
    write_tax(&mut s, invoice);
    write_period(&mut s, invoice);
    write_doc_ac(&mut s, invoice);
    write_payment_terms(&mut s, invoice);
    write_totals(&mut s, invoice);
    write_preceding(&mut s, invoice);
    s.push_str("    </ram:ApplicableHeaderTradeSettlement>\n");
    s.push_str("  </rsm:SupplyChainTradeTransaction>\n");
    s.push_str("</rsm:CrossIndustryInvoice>\n");
    Ok(s)
}

pub fn read(xml: &str) -> Result<Read, FormatError> {
    xml::refuse_dtd(xml)?;
    xml::refuse_oversize(xml)?;
    xml::refuse_depth(xml)?;
    let doc = roxmltree::Document::parse(xml)
        .map_err(|e| FormatError::Parse(format!("not well-formed CII: {e}")))?;
    let root = doc.root_element();
    if local(root) != "CrossIndustryInvoice" {
        return Err(FormatError::Parse(format!(
            "CII document element must be CrossIndustryInvoice, not {}",
            local(root)
        )));
    }
    let ctx = child(root, "ExchangedDocumentContext");
    let spec = ctx
        .and_then(|n| child(n, "GuidelineSpecifiedDocumentContextParameter"))
        .and_then(|n| child_text(n, "ID"));
    // Unknown BT-24 stays unknown; CORE-SPEC-01 is Fatal. Do not silently select En16931.
    let profile = match spec.as_deref() {
        Some(id) => match Profile::for_specification_id(id) {
            ProfileLookup::Profile(p) => p,
            ProfileLookup::WrongProcess | ProfileLookup::Unknown => Profile::Unknown,
        },
        None => Profile::Unknown,
    };
    let mut malformed = Vec::new();
    let doc_el = child(root, "ExchangedDocument");
    let number = doc_el.and_then(|n| child_text(n, "ID")).unwrap_or_default();
    let type_code = doc_el.and_then(|n| child_text(n, "TypeCode"));
    let issue_date = doc_el
        .and_then(|n| child(n, "IssueDateTime"))
        .and_then(|n| child(n, "DateTimeString"))
        .and_then(|n| from_102_node(n, &mut malformed));
    let tx = child(root, "SupplyChainTradeTransaction")
        .ok_or_else(|| FormatError::Parse("missing SupplyChainTradeTransaction".into()))?;
    let agreement = child(tx, "ApplicableHeaderTradeAgreement");
    let seller = agreement
        .and_then(|n| child(n, "SellerTradeParty"))
        .map(|n| read_party(n, profile))
        .unwrap_or_else(|| Party::new("", ""));
    let buyer = agreement
        .and_then(|n| child(n, "BuyerTradeParty"))
        .map(|n| read_party(n, profile))
        .unwrap_or_else(|| Party::new("", ""));
    let settlement = child(tx, "ApplicableHeaderTradeSettlement");
    let currency = settlement
        .and_then(|n| child_text(n, "InvoiceCurrencyCode"))
        .unwrap_or_default();
    let mut invoice = Invoice::blank(profile, number, currency, seller, buyer);
    invoice.specification_id = spec;
    invoice.issue_date = issue_date;
    invoice.type_code = type_code.clone().map(Code::new);
    match type_code.as_deref() {
        Some(c) if core_invoice::codes::credit_note_type(c) => {
            invoice.kind = DocumentKind::CreditNote;
        }
        Some(c) if core_invoice::codes::invoice_type(c) => {
            invoice.kind = DocumentKind::Invoice;
        }
        Some(c) => malformed.push(format!("ExchangedDocument/TypeCode: {c}")),
        None => malformed.push("ExchangedDocument/TypeCode missing".into()),
    }
    if let Some(doc_el) = doc_el {
        invoice.notes = children(doc_el, "IncludedNote")
            .filter_map(|n| {
                Some(InvoiceNote {
                    subject: child_text(n, "SubjectCode").map(Code::new),
                    text: child_text(n, "Content")?,
                })
            })
            .collect();
    }
    invoice.lines = children(tx, "IncludedSupplyChainTradeLineItem")
        .filter_map(|n| read_line(n, profile, &mut malformed))
        .collect();
    if let Some(bp) = ctx
        .and_then(|n| child(n, "BusinessProcessSpecifiedDocumentContextParameter"))
        .and_then(|n| child_text(n, "ID"))
    {
        invoice.business_process = Some(bp);
    }
    if let Some(ag) = agreement {
        invoice.buyer_reference = child_text(ag, "BuyerReference").map(DocumentReference::new);
        invoice.tax_representative =
            child(ag, "SellerTaxRepresentativeTradeParty").map(|n| TaxRepresentative {
                name: child_text(n, "Name").unwrap_or_default(),
                vat_identifier: child(n, "SpecifiedTaxRegistration")
                    .and_then(|t| child(t, "ID"))
                    .map(|id| Identifier {
                        value: text(id).unwrap_or_default(),
                        scheme: id.attribute("schemeID").map(str::to_owned),
                        scheme_version: None,
                    }),
                address: child(n, "PostalTradeAddress").map(read_postal),
            });
        invoice.sales_order = child(ag, "SellerOrderReferencedDocument")
            .and_then(|n| child_text(n, "IssuerAssignedID"))
            .map(DocumentReference::new);
        invoice.purchase_order = child(ag, "BuyerOrderReferencedDocument")
            .and_then(|n| child_text(n, "IssuerAssignedID"))
            .map(DocumentReference::new);
        invoice.contract = child(ag, "ContractReferencedDocument")
            .and_then(|n| child_text(n, "IssuerAssignedID"))
            .map(DocumentReference::new);
        invoice.project = child(ag, "SpecifiedProcuringProject")
            .and_then(|n| child_text(n, "ID"))
            .map(DocumentReference::new);
        for adr in children(ag, "AdditionalReferencedDocument") {
            let dtype = child_text(adr, "TypeCode");
            let id = child_text(adr, "IssuerAssignedID").unwrap_or_default();
            match dtype.as_deref() {
                Some("130") => {
                    invoice.invoiced_object = Some(Identifier {
                        value: id,
                        scheme: child_text(adr, "ReferenceTypeCode"),
                        scheme_version: None,
                    });
                }
                Some("50") => invoice.tender = Some(DocumentReference::new(id)),
                _ => invoice.supporting_documents.push(SupportingDocument {
                    id: DocumentReference::new(id),
                    description: child_text(adr, "Name"),
                    uri: child_text(adr, "URIID"),
                    attachment: None,
                }),
            }
        }
    }
    if let Some(del) = child(tx, "ApplicableHeaderTradeDelivery") {
        let date = child(del, "ActualDeliverySupplyChainEvent")
            .and_then(|n| child(n, "OccurrenceDateTime"))
            .and_then(|n| child(n, "DateTimeString"))
            .and_then(|n| from_102_node(n, &mut malformed));
        let ship = child(del, "ShipToTradeParty");
        if date.is_some() || ship.is_some() {
            invoice.delivery = Some(core_invoice::Delivery {
                name: ship.and_then(|n| child_text(n, "Name")),
                location_id: ship
                    .and_then(|n| child(n, "ID").or_else(|| child(n, "GlobalID")))
                    .map(|id| Identifier {
                        value: text(id).unwrap_or_default(),
                        scheme: id.attribute("schemeID").map(str::to_owned),
                        scheme_version: None,
                    }),
                date,
                address: ship.and_then(|n| child(n, "PostalTradeAddress").map(read_postal)),
            });
        }
    }
    if let Some(st) = settlement {
        invoice.tax_breakdown = children(st, "ApplicableTradeTax")
            .filter_map(|n| read_tax(n, profile, &mut malformed))
            .collect();
        if let Some(ms) = child(st, "SpecifiedTradeSettlementHeaderMonetarySummation") {
            invoice.totals = Some(read_totals(ms, &mut malformed));
        }
        let pms: Vec<_> = children(st, "SpecifiedTradeSettlementPaymentMeans").collect();
        if let Some(first) = pms.first().copied() {
            let mut accts = Vec::new();
            for pm in pms {
                for n in children(pm, "PayeePartyCreditorFinancialAccount") {
                    let id = child_text(n, "IBANID").or_else(|| child_text(n, "ProprietaryID"));
                    if let Some(id) = id {
                        accts.push(core_invoice::CreditTransfer {
                            account_id: Identifier::new(id),
                            account_name: child_text(n, "AccountName"),
                            provider: None,
                        });
                    }
                }
            }
            invoice.payment = Some(core_invoice::PaymentInstructions {
                means_code: child_text(first, "TypeCode").map(Code::new),
                means_text: child_text(first, "Information"),
                remittance: child_text(st, "PaymentReference"),
                means: if accts.is_empty() {
                    None
                } else {
                    Some(core_invoice::PaymentMeans::CreditTransfer(accts))
                },
            });
        } else if let Some(r) = child_text(st, "PaymentReference") {
            invoice.payment = Some(core_invoice::PaymentInstructions {
                means_code: None,
                means_text: None,
                remittance: Some(r),
                means: None,
            });
        }
        if let Some(p) = child(st, "PayeeTradeParty") {
            invoice.payee = Some(Payee {
                name: child_text(p, "Name").unwrap_or_default(),
                identifier: child(p, "ID")
                    .or_else(|| child(p, "GlobalID"))
                    .map(|id| Identifier {
                        value: text(id).unwrap_or_default(),
                        scheme: id.attribute("schemeID").map(str::to_owned),
                        scheme_version: None,
                    }),
                legal_registration: child(p, "SpecifiedLegalOrganization")
                    .and_then(|n| child(n, "ID"))
                    .map(|id| Identifier {
                        value: text(id).unwrap_or_default(),
                        scheme: id.attribute("schemeID").map(str::to_owned),
                        scheme_version: None,
                    }),
            });
        }
        if let Some(per) = child(st, "BillingSpecifiedPeriod") {
            invoice.period = Some(core_invoice::Period {
                start: child(per, "StartDateTime")
                    .and_then(|n| child(n, "DateTimeString"))
                    .and_then(|n| from_102_node(n, &mut malformed)),
                end: child(per, "EndDateTime")
                    .and_then(|n| child(n, "DateTimeString"))
                    .and_then(|n| from_102_node(n, &mut malformed)),
            });
        }
        if let Some(pt) = child(st, "SpecifiedTradePaymentTerms") {
            invoice.payment_terms = child_text(pt, "Description");
            invoice.due_date = child(pt, "DueDateDateTime")
                .and_then(|n| child(n, "DateTimeString"))
                .and_then(|n| from_102_node(n, &mut malformed));
        }
        for p in children(st, "InvoiceReferencedDocument") {
            if let Some(id) = child_text(p, "IssuerAssignedID") {
                invoice.preceding.push(core_invoice::PrecedingInvoice {
                    reference: DocumentReference::new(id),
                    issue_date: None,
                });
            }
        }
        for ac in children(st, "SpecifiedTradeAllowanceCharge") {
            let charge = child(ac, "ChargeIndicator")
                .and_then(|n| child_text(n, "Indicator"))
                .is_some_and(|s| s.eq_ignore_ascii_case("true"));
            let Some(amount) = child_amount(ac, "ActualAmount", &mut malformed, "CII-ac") else {
                continue;
            };
            let row = core_invoice::AllowanceCharge {
                amount,
                base: None,
                percent: None,
                reason: child_text(ac, "Reason"),
                reason_code: None,
                tax: None,
            };
            if charge {
                invoice.document_charges.push(row);
            } else {
                invoice.document_allowances.push(row);
            }
        }
    }
    const MAPPED_TX: &[&str] = &[
        "IncludedSupplyChainTradeLineItem",
        "ApplicableHeaderTradeAgreement",
        "ApplicableHeaderTradeDelivery",
        "ApplicableHeaderTradeSettlement",
    ];
    let unmapped = tx
        .children()
        .filter(|n| n.is_element())
        .map(local)
        .filter(|name| !MAPPED_TX.contains(name))
        .map(|name| format!("SupplyChainTradeTransaction/{name}"))
        .collect();
    Ok(Read {
        invoice,
        unmapped,
        malformed,
    })
}

fn write_line(s: &mut String, line: &Line, invoice: &Invoice) {
    s.push_str("    <ram:IncludedSupplyChainTradeLineItem>\n");
    s.push_str("      <ram:AssociatedDocumentLineDocument>\n");
    leaf_ram(s, 4, "LineID", &line.id, None);
    if let Some(note) = line.note.as_deref() {
        s.push_str("        <ram:IncludedNote>\n");
        leaf_ram(s, 5, "Content", note, None);
        s.push_str("        </ram:IncludedNote>\n");
    }
    s.push_str("      </ram:AssociatedDocumentLineDocument>\n");
    s.push_str("      <ram:SpecifiedTradeProduct>\n");
    leaf_ram(s, 4, "Name", &line.name, None);
    if let Some(d) = line.description.as_deref() {
        leaf_ram(s, 4, "Description", d, None);
    }
    s.push_str("      </ram:SpecifiedTradeProduct>\n");
    s.push_str("      <ram:SpecifiedLineTradeAgreement>\n");
    if let Some(price) = line.price.as_ref() {
        if let Some(g) = price.gross {
            s.push_str("        <ram:GrossPriceProductTradePrice>\n");
            leaf_ram(
                s,
                5,
                "ChargeAmount",
                &g.to_string(),
                Some(("currencyID", invoice.currency.as_str())),
            );
            s.push_str("        </ram:GrossPriceProductTradePrice>\n");
        }
        s.push_str("        <ram:NetPriceProductTradePrice>\n");
        leaf_ram(
            s,
            5,
            "ChargeAmount",
            &price.net.to_string(),
            Some(("currencyID", invoice.currency.as_str())),
        );
        s.push_str("        </ram:NetPriceProductTradePrice>\n");
    }
    s.push_str("      </ram:SpecifiedLineTradeAgreement>\n");
    s.push_str("      <ram:SpecifiedLineTradeDelivery>\n");
    if let Some(q) = line.quantity {
        let unit = line.unit.as_ref().map(|c| ("unitCode", c.as_str()));
        leaf_ram(s, 5, "BilledQuantity", &q.to_string(), unit);
    }
    s.push_str("      </ram:SpecifiedLineTradeDelivery>\n");
    s.push_str("      <ram:SpecifiedLineTradeSettlement>\n");
    s.push_str("        <ram:ApplicableTradeTax>\n");
    leaf_ram(
        s,
        5,
        "TypeCode",
        wire_scheme(invoice.profile, line.tax.system, &line.tax.code),
        None,
    );
    leaf_ram(s, 5, "CategoryCode", &line.tax.code, None);
    if let Some(p) = line.tax.percent {
        leaf_ram(s, 5, "RateApplicablePercent", &p.to_string(), None);
    }
    s.push_str("        </ram:ApplicableTradeTax>\n");
    s.push_str("        <ram:SpecifiedTradeSettlementLineMonetarySummation>\n");
    amount_ram(s, 5, "LineTotalAmount", line.net, &invoice.currency);
    s.push_str("        </ram:SpecifiedTradeSettlementLineMonetarySummation>\n");
    s.push_str("      </ram:SpecifiedLineTradeSettlement>\n");
    s.push_str("    </ram:IncludedSupplyChainTradeLineItem>\n");
}

fn write_ident(s: &mut String, indent: usize, tag: &str, id: &Identifier) {
    leaf_ram(
        s,
        indent,
        tag,
        &id.value,
        id.scheme.as_deref().map(|sc| ("schemeID", sc)),
    );
}

fn write_doc_ref(s: &mut String, tag: &str, r: Option<&DocumentReference>) {
    let Some(r) = r else {
        return;
    };
    s.push_str(&format!("      <ram:{tag}>\n"));
    leaf_ram(s, 4, "IssuerAssignedID", r.as_str(), None);
    s.push_str(&format!("      </ram:{tag}>\n"));
}

fn write_additional_doc(s: &mut String, d: &SupportingDocument, type_code: &str) {
    s.push_str("      <ram:AdditionalReferencedDocument>\n");
    leaf_ram(s, 4, "IssuerAssignedID", d.id.as_str(), None);
    if let Some(u) = d.uri.as_deref() {
        leaf_ram(s, 4, "URIID", u, None);
    }
    leaf_ram(s, 4, "TypeCode", type_code, None);
    if let Some(desc) = d.description.as_deref() {
        leaf_ram(s, 4, "Name", desc, None);
    }
    s.push_str("      </ram:AdditionalReferencedDocument>\n");
}

fn write_postal(s: &mut String, indent: usize, addr: &core_invoice::PostalAddress) {
    s.push_str(&format!(
        "{}<ram:PostalTradeAddress>\n",
        "  ".repeat(indent)
    ));
    if let Some(pc) = addr.post_code.as_deref() {
        leaf_ram(s, indent + 1, "PostcodeCode", pc, None);
    }
    if let Some(l) = addr.line1.as_deref() {
        leaf_ram(s, indent + 1, "LineOne", l, None);
    }
    if let Some(l) = addr.line2.as_deref() {
        leaf_ram(s, indent + 1, "LineTwo", l, None);
    }
    if let Some(l) = addr.line3.as_deref() {
        leaf_ram(s, indent + 1, "LineThree", l, None);
    }
    if let Some(city) = addr.city.as_deref() {
        leaf_ram(s, indent + 1, "CityName", city, None);
    }
    if let Some(c) = addr.country.as_ref() {
        leaf_ram(s, indent + 1, "CountryID", c.as_str(), None);
    }
    if let Some(sub) = addr.subdivision.as_deref() {
        leaf_ram(s, indent + 1, "CountrySubDivisionName", sub, None);
    }
    s.push_str(&format!(
        "{}</ram:PostalTradeAddress>\n",
        "  ".repeat(indent)
    ));
}

fn write_trade_party(s: &mut String, tag: &str, party: &Party, seller: bool) {
    s.push_str(&format!("      <ram:{tag}>\n"));
    for id in &party.identifiers {
        if id.scheme.is_some() {
            write_ident(s, 4, "GlobalID", id);
        } else {
            leaf_ram(s, 4, "ID", &id.value, None);
        }
    }
    leaf_ram(s, 4, "Name", &party.name, None);
    if seller && let Some(a) = party.additional_legal.as_deref() {
        leaf_ram(s, 4, "Description", a, None);
    }
    if party.legal_registration.is_some() || party.trading_name.is_some() {
        s.push_str("        <ram:SpecifiedLegalOrganization>\n");
        if let Some(legal) = party.legal_registration.as_ref() {
            write_ident(s, 5, "ID", legal);
        }
        if let Some(t) = party.trading_name.as_deref() {
            leaf_ram(s, 5, "TradingBusinessName", t, None);
        }
        s.push_str("        </ram:SpecifiedLegalOrganization>\n");
    }
    if let Some(c) = party.contact.as_ref() {
        s.push_str("        <ram:DefinedTradeContact>\n");
        if let Some(p) = c.point.as_deref() {
            leaf_ram(s, 5, "PersonName", p, None);
        }
        if let Some(ph) = c.phone.as_deref() {
            s.push_str("          <ram:TelephoneUniversalCommunication>\n");
            leaf_ram(s, 6, "CompleteNumber", ph, None);
            s.push_str("          </ram:TelephoneUniversalCommunication>\n");
        }
        if let Some(em) = c.email.as_deref() {
            s.push_str("          <ram:EmailURIUniversalCommunication>\n");
            leaf_ram(s, 6, "URIID", em, None);
            s.push_str("          </ram:EmailURIUniversalCommunication>\n");
        }
        s.push_str("        </ram:DefinedTradeContact>\n");
    }
    if let Some(addr) = party.address.as_ref() {
        write_postal(s, 4, addr);
    } else if !party.country().is_empty() {
        s.push_str("        <ram:PostalTradeAddress>\n");
        leaf_ram(s, 5, "CountryID", party.country(), None);
        s.push_str("        </ram:PostalTradeAddress>\n");
    }
    if let Some(ep) = party.electronic_address.as_ref() {
        s.push_str("        <ram:URIUniversalCommunication>\n");
        write_ident(s, 5, "URIID", ep);
        s.push_str("        </ram:URIUniversalCommunication>\n");
    }
    if let Some(vat) = party.vat_identifier.as_ref() {
        s.push_str("        <ram:SpecifiedTaxRegistration>\n");
        leaf_ram(s, 5, "ID", &vat.value, Some(("schemeID", "VA")));
        s.push_str("        </ram:SpecifiedTaxRegistration>\n");
    }
    if let Some(tin) = party.tax_registration.as_ref() {
        s.push_str("        <ram:SpecifiedTaxRegistration>\n");
        leaf_ram(
            s,
            5,
            "ID",
            &tin.value,
            Some(("schemeID", tin.scheme.as_deref().unwrap_or("FC"))),
        );
        s.push_str("        </ram:SpecifiedTaxRegistration>\n");
    }
    s.push_str(&format!("      </ram:{tag}>\n"));
}

fn write_tax_rep(s: &mut String, tr: Option<&TaxRepresentative>) {
    let Some(tr) = tr else {
        return;
    };
    s.push_str("      <ram:SellerTaxRepresentativeTradeParty>\n");
    if !tr.name.is_empty() {
        leaf_ram(s, 4, "Name", &tr.name, None);
    }
    if let Some(addr) = tr.address.as_ref() {
        write_postal(s, 4, addr);
    }
    if let Some(vat) = tr.vat_identifier.as_ref() {
        s.push_str("        <ram:SpecifiedTaxRegistration>\n");
        leaf_ram(s, 5, "ID", &vat.value, Some(("schemeID", "VA")));
        s.push_str("        </ram:SpecifiedTaxRegistration>\n");
    }
    s.push_str("      </ram:SellerTaxRepresentativeTradeParty>\n");
}

fn write_payee(s: &mut String, payee: Option<&Payee>) {
    let Some(p) = payee else {
        return;
    };
    s.push_str("      <ram:PayeeTradeParty>\n");
    if let Some(id) = p.identifier.as_ref() {
        if id.scheme.is_some() {
            write_ident(s, 4, "GlobalID", id);
        } else {
            leaf_ram(s, 4, "ID", &id.value, None);
        }
    }
    if !p.name.is_empty() {
        leaf_ram(s, 4, "Name", &p.name, None);
    }
    if let Some(legal) = p.legal_registration.as_ref() {
        s.push_str("        <ram:SpecifiedLegalOrganization>\n");
        write_ident(s, 5, "ID", legal);
        s.push_str("        </ram:SpecifiedLegalOrganization>\n");
    }
    s.push_str("      </ram:PayeeTradeParty>\n");
}

fn write_period(s: &mut String, invoice: &Invoice) {
    let Some(p) = invoice.period.as_ref() else {
        return;
    };
    if p.start.is_none() && p.end.is_none() {
        return;
    }
    s.push_str("      <ram:BillingSpecifiedPeriod>\n");
    if let Some(d) = p.start {
        s.push_str("        <ram:StartDateTime>\n");
        s.push_str(&format!(
            "          <udt:DateTimeString format=\"102\">{}</udt:DateTimeString>\n",
            to_102(d)
        ));
        s.push_str("        </ram:StartDateTime>\n");
    }
    if let Some(d) = p.end {
        s.push_str("        <ram:EndDateTime>\n");
        s.push_str(&format!(
            "          <udt:DateTimeString format=\"102\">{}</udt:DateTimeString>\n",
            to_102(d)
        ));
        s.push_str("        </ram:EndDateTime>\n");
    }
    s.push_str("      </ram:BillingSpecifiedPeriod>\n");
}

fn write_payment_terms(s: &mut String, invoice: &Invoice) {
    if invoice.due_date.is_none() && invoice.payment_terms.is_none() {
        return;
    }
    s.push_str("      <ram:SpecifiedTradePaymentTerms>\n");
    if let Some(note) = invoice.payment_terms.as_deref() {
        leaf_ram(s, 4, "Description", note, None);
    }
    if let Some(d) = invoice.due_date {
        s.push_str("        <ram:DueDateDateTime>\n");
        s.push_str(&format!(
            "          <udt:DateTimeString format=\"102\">{}</udt:DateTimeString>\n",
            to_102(d)
        ));
        s.push_str("        </ram:DueDateDateTime>\n");
    }
    s.push_str("      </ram:SpecifiedTradePaymentTerms>\n");
}

fn write_preceding(s: &mut String, invoice: &Invoice) {
    for p in &invoice.preceding {
        s.push_str("      <ram:InvoiceReferencedDocument>\n");
        leaf_ram(s, 4, "IssuerAssignedID", p.reference.as_str(), None);
        s.push_str("      </ram:InvoiceReferencedDocument>\n");
    }
}

fn write_delivery(s: &mut String, invoice: &Invoice) {
    let Some(d) = invoice.delivery.as_ref() else {
        s.push_str("    <ram:ApplicableHeaderTradeDelivery/>\n");
        return;
    };
    s.push_str("    <ram:ApplicableHeaderTradeDelivery>\n");
    // HeaderTradeDeliveryType: ShipToTradeParty before ActualDeliverySupplyChainEvent.
    if d.address.is_some() || d.name.is_some() || d.location_id.is_some() {
        s.push_str("      <ram:ShipToTradeParty>\n");
        if let Some(id) = d.location_id.as_ref() {
            if id.scheme.is_some() {
                write_ident(s, 4, "GlobalID", id);
            } else {
                leaf_ram(s, 4, "ID", &id.value, None);
            }
        }
        if let Some(n) = d.name.as_deref() {
            leaf_ram(s, 4, "Name", n, None);
        }
        if let Some(addr) = d.address.as_ref() {
            write_postal(s, 4, addr);
        }
        s.push_str("      </ram:ShipToTradeParty>\n");
    }
    if let Some(date) = d.date {
        s.push_str("      <ram:ActualDeliverySupplyChainEvent>\n");
        s.push_str("        <ram:OccurrenceDateTime>\n");
        s.push_str(&format!(
            "          <udt:DateTimeString format=\"102\">{}</udt:DateTimeString>\n",
            to_102(date)
        ));
        s.push_str("        </ram:OccurrenceDateTime>\n");
        s.push_str("      </ram:ActualDeliverySupplyChainEvent>\n");
    }
    s.push_str("    </ram:ApplicableHeaderTradeDelivery>\n");
}

fn write_payment(s: &mut String, invoice: &Invoice) {
    let Some(pay) = invoice.payment.as_ref() else {
        return;
    };
    if let Some(core_invoice::PaymentMeans::CreditTransfer(cts)) = pay.means.as_ref()
        && !cts.is_empty()
    {
        // One SpecifiedTradeSettlementPaymentMeans per account (IBANID maxOccurs 1).
        for ct in cts {
            s.push_str("      <ram:SpecifiedTradeSettlementPaymentMeans>\n");
            if let Some(code) = pay.means_code.as_ref() {
                leaf_ram(s, 4, "TypeCode", code.as_str(), None);
            }
            if let Some(note) = pay.means_text.as_deref() {
                leaf_ram(s, 4, "Information", note, None);
            }
            s.push_str("        <ram:PayeePartyCreditorFinancialAccount>\n");
            leaf_ram(s, 5, "IBANID", &ct.account_id.value, None);
            if let Some(n) = ct.account_name.as_deref() {
                leaf_ram(s, 5, "AccountName", n, None);
            }
            s.push_str("        </ram:PayeePartyCreditorFinancialAccount>\n");
            s.push_str("      </ram:SpecifiedTradeSettlementPaymentMeans>\n");
        }
        return;
    }
    s.push_str("      <ram:SpecifiedTradeSettlementPaymentMeans>\n");
    if let Some(code) = pay.means_code.as_ref() {
        leaf_ram(s, 4, "TypeCode", code.as_str(), None);
    }
    s.push_str("      </ram:SpecifiedTradeSettlementPaymentMeans>\n");
}

fn write_doc_ac(s: &mut String, invoice: &Invoice) {
    for a in &invoice.document_allowances {
        write_cii_ac(s, a, false, &invoice.currency);
    }
    for a in &invoice.document_charges {
        write_cii_ac(s, a, true, &invoice.currency);
    }
}

fn write_cii_ac(s: &mut String, a: &core_invoice::AllowanceCharge, charge: bool, cur: &str) {
    s.push_str("      <ram:SpecifiedTradeAllowanceCharge>\n");
    s.push_str("        <ram:ChargeIndicator>\n");
    s.push_str(&format!(
        "          <udt:Indicator>{}</udt:Indicator>\n",
        if charge { "true" } else { "false" }
    ));
    s.push_str("        </ram:ChargeIndicator>\n");
    amount_ram(s, 4, "ActualAmount", a.amount, cur);
    if let Some(r) = a.reason.as_deref() {
        leaf_ram(s, 4, "Reason", r, None);
    }
    s.push_str("      </ram:SpecifiedTradeAllowanceCharge>\n");
}

fn write_tax(s: &mut String, invoice: &Invoice) {
    for row in &invoice.tax_breakdown {
        s.push_str("      <ram:ApplicableTradeTax>\n");
        amount_ram(s, 4, "CalculatedAmount", row.tax, &invoice.currency);
        leaf_ram(
            s,
            4,
            "TypeCode",
            wire_scheme(invoice.profile, row.system, row.category.as_str()),
            None,
        );
        amount_ram(s, 4, "BasisAmount", row.taxable, &invoice.currency);
        leaf_ram(s, 4, "CategoryCode", row.category.as_str(), None);
        if let Some(r) = row.rate {
            leaf_ram(s, 4, "RateApplicablePercent", &r.to_string(), None);
        }
        s.push_str("      </ram:ApplicableTradeTax>\n");
    }
}

fn write_totals(s: &mut String, invoice: &Invoice) {
    let Some(t) = invoice.totals.as_ref() else {
        return;
    };
    s.push_str("      <ram:SpecifiedTradeSettlementHeaderMonetarySummation>\n");
    let cur = &invoice.currency;
    if let Some(v) = t.line_net {
        amount_ram(s, 4, "LineTotalAmount", v, cur);
    }
    if let Some(v) = t.without_tax {
        amount_ram(s, 4, "TaxBasisTotalAmount", v, cur);
    }
    if let Some(v) = t.tax_total {
        amount_ram(s, 4, "TaxTotalAmount", v, cur);
    }
    if let Some(v) = t.with_tax {
        amount_ram(s, 4, "GrandTotalAmount", v, cur);
    }
    if let Some(v) = t.payable {
        amount_ram(s, 4, "DuePayableAmount", v, cur);
    }
    s.push_str("      </ram:SpecifiedTradeSettlementHeaderMonetarySummation>\n");
}

fn read_postal(node: roxmltree::Node<'_, '_>) -> core_invoice::PostalAddress {
    core_invoice::PostalAddress {
        line1: child_text(node, "LineOne"),
        line2: child_text(node, "LineTwo"),
        line3: child_text(node, "LineThree"),
        city: child_text(node, "CityName"),
        post_code: child_text(node, "PostcodeCode"),
        subdivision: child_text(node, "CountrySubDivisionName"),
        country: child_text(node, "CountryID").map(Code::new),
    }
}

fn read_party(node: roxmltree::Node<'_, '_>, _profile: Profile) -> Party {
    let name = child_text(node, "Name").unwrap_or_default();
    let addr = child(node, "PostalTradeAddress");
    let country = addr
        .and_then(|n| child_text(n, "CountryID"))
        .unwrap_or_default();
    let mut party = Party::new(name, country);
    if let Some(a) = addr {
        party.address = Some(read_postal(a));
    }
    if let Some(ct) = child(node, "DefinedTradeContact") {
        party.contact = Some(core_invoice::Contact {
            point: child_text(ct, "PersonName"),
            phone: child(ct, "TelephoneUniversalCommunication")
                .and_then(|n| child_text(n, "CompleteNumber")),
            email: child(ct, "EmailURIUniversalCommunication").and_then(|n| child_text(n, "URIID")),
        });
    }
    if let Some(org) = child(node, "SpecifiedLegalOrganization") {
        if let Some(id) = child(org, "ID") {
            party.legal_registration = Some(Identifier {
                value: text(id).unwrap_or_default(),
                scheme: id.attribute("schemeID").map(str::to_owned),
                scheme_version: None,
            });
        }
        party.trading_name = child_text(org, "TradingBusinessName");
    }
    party.additional_legal = child_text(node, "Description");
    for idn in children(node, "GlobalID") {
        party.identifiers.push(Identifier {
            value: text(idn).unwrap_or_default(),
            scheme: idn.attribute("schemeID").map(str::to_owned),
            scheme_version: None,
        });
    }
    for idn in children(node, "ID") {
        party.identifiers.push(Identifier {
            value: text(idn).unwrap_or_default(),
            scheme: None,
            scheme_version: None,
        });
    }
    if let Some(ep) = child(node, "URIUniversalCommunication").and_then(|n| child(n, "URIID")) {
        party.electronic_address = Some(Identifier {
            value: text(ep).unwrap_or_default(),
            scheme: ep.attribute("schemeID").map(str::to_owned),
            scheme_version: None,
        });
    }
    for tax in children(node, "SpecifiedTaxRegistration") {
        let Some(id) = child(tax, "ID") else {
            continue;
        };
        let ident = Identifier {
            value: text(id).unwrap_or_default(),
            scheme: id.attribute("schemeID").map(str::to_owned),
            scheme_version: None,
        };
        if ident.scheme.as_deref() == Some("VA") {
            party.vat_identifier = Some(ident);
        } else if party.tax_registration.is_none() {
            party.tax_registration = Some(ident);
        }
    }
    party
}

fn read_line(
    node: roxmltree::Node<'_, '_>,
    profile: Profile,
    malformed: &mut Vec<String>,
) -> Option<Line> {
    let id = child(node, "AssociatedDocumentLineDocument")
        .and_then(|n| child_text(n, "LineID"))
        .unwrap_or_default();
    let product = child(node, "SpecifiedTradeProduct");
    let name = product
        .and_then(|n| child_text(n, "Name"))
        .unwrap_or_default();
    let net = child(node, "SpecifiedLineTradeSettlement")
        .and_then(|n| child(n, "SpecifiedTradeSettlementLineMonetarySummation"))
        .and_then(|n| child_amount(n, "LineTotalAmount", malformed, "CII-line"))
        .unwrap_or(Amount::ZERO);
    let tax = child(node, "SpecifiedLineTradeSettlement")
        .and_then(|n| child(n, "ApplicableTradeTax"))
        .map(|n| {
            let code = child_text(n, "CategoryCode").unwrap_or_default();
            let percent = child_text(n, "RateApplicablePercent")
                .and_then(|s| Decimal::from_str(&s).ok())
                .map(Percentage::new);
            let scheme = child_text(n, "TypeCode").unwrap_or_default();
            let system = if profile == Profile::PintMy && core_invoice::pint_my_category(&code) {
                TaxSystem::Sst
            } else {
                TaxSystem::parse(&scheme).unwrap_or(TaxSystem::Vat)
            };
            TaxCategory {
                system,
                code,
                percent,
            }
        })
        .unwrap_or_else(|| TaxCategory {
            // Missing line tax is empty + BR-CO-04, not invented SST/S.
            system: TaxSystem::Vat,
            code: String::new(),
            percent: None,
        });
    let mut line = Line::new(id, name, net, tax);
    if let Some(del) = child(node, "SpecifiedLineTradeDelivery")
        && let Some(q) = child(del, "BilledQuantity")
    {
        if let Some(t) = text(q) {
            line.quantity = core_invoice::Quantity::parse(&t).ok();
        }
        line.unit = q.attribute("unitCode").map(Code::new);
    }
    if let Some(agr) = child(node, "SpecifiedLineTradeAgreement")
        && let Some(price) = child(agr, "NetPriceProductTradePrice")
        && let Some(amt) = child_text(price, "ChargeAmount")
        && let Ok(net) = core_invoice::UnitPriceAmount::parse(&amt)
    {
        let gross = child(agr, "GrossPriceProductTradePrice")
            .and_then(|n| child_text(n, "ChargeAmount"))
            .and_then(|t| core_invoice::UnitPriceAmount::parse(&t).ok());
        line.price = Some(core_invoice::Price {
            net,
            discount: None,
            gross,
            base_qty: None,
            base_unit: None,
        });
    }
    line.description = product.and_then(|n| child_text(n, "Description"));
    line.note = child(node, "AssociatedDocumentLineDocument")
        .and_then(|n| child(n, "IncludedNote"))
        .and_then(|n| child_text(n, "Content"));
    Some(line)
}

fn read_tax(
    node: roxmltree::Node<'_, '_>,
    profile: Profile,
    malformed: &mut Vec<String>,
) -> Option<TaxBreakdown> {
    let code = child_text(node, "CategoryCode")?;
    let percent = child_text(node, "RateApplicablePercent")
        .and_then(|s| Decimal::from_str(&s).ok())
        .map(Percentage::new);
    let _scheme = child_text(node, "TypeCode").unwrap_or_else(|| "VAT".into());
    let system = if profile == Profile::PintMy && core_invoice::pint_my_category(&code) {
        TaxSystem::Sst
    } else {
        TaxSystem::Vat
    };
    Some(TaxBreakdown {
        system,
        scheme: wire_scheme(profile, system, &code).to_owned(),
        category: Code::new(code),
        rate: percent,
        taxable: child_amount(node, "BasisAmount", malformed, "CII-tax").unwrap_or(Amount::ZERO),
        tax: child_amount(node, "CalculatedAmount", malformed, "CII-tax").unwrap_or(Amount::ZERO),
        exemption_reason: None,
        exemption_code: None,
    })
}

fn read_totals(node: roxmltree::Node<'_, '_>, malformed: &mut Vec<String>) -> DocumentTotals {
    DocumentTotals {
        line_net: child_amount(node, "LineTotalAmount", malformed, "CII-totals"),
        allowance_total: None,
        charge_total: None,
        without_tax: child_amount(node, "TaxBasisTotalAmount", malformed, "CII-totals"),
        tax_total: child_amount(node, "TaxTotalAmount", malformed, "CII-totals"),
        tax_total_accounting: None,
        with_tax: child_amount(node, "GrandTotalAmount", malformed, "CII-totals"),
        paid: child_amount(node, "TotalPrepaidAmount", malformed, "CII-totals"),
        rounding: child_amount(node, "RoundingAmount", malformed, "CII-totals"),
        payable: child_amount(node, "DuePayableAmount", malformed, "CII-totals"),
    }
}

fn to_102(d: Date) -> String {
    format!("{:04}{:02}{:02}", d.year(), d.month(), d.day())
}

fn from_102(s: &str) -> Option<Date> {
    if s.len() != 8 {
        return None;
    }
    let y: i32 = s[..4].parse().ok()?;
    let m: u8 = s[4..6].parse().ok()?;
    let d: u8 = s[6..8].parse().ok()?;
    Date::new(y, m, d).ok()
}

fn from_102_node(node: roxmltree::Node<'_, '_>, malformed: &mut Vec<String>) -> Option<Date> {
    let s = text(node)?;
    let format = node.attribute("format").unwrap_or("102");
    if format != "102" {
        malformed.push(format!("DateTimeString format={format} (not 102): {s}"));
        return None;
    }
    from_102(&s).or_else(|| {
        malformed.push(format!("DateTimeString: {s}"));
        None
    })
}

fn local<'a>(node: roxmltree::Node<'a, 'a>) -> &'a str {
    node.tag_name().name()
}
fn child<'a>(node: roxmltree::Node<'a, 'a>, name: &str) -> Option<roxmltree::Node<'a, 'a>> {
    node.children()
        .find(|n| n.is_element() && n.tag_name().name() == name)
}
fn children<'a>(
    node: roxmltree::Node<'a, 'a>,
    name: &'a str,
) -> impl Iterator<Item = roxmltree::Node<'a, 'a>> {
    node.children()
        .filter(move |n| n.is_element() && n.tag_name().name() == name)
}
fn text(node: roxmltree::Node<'_, '_>) -> Option<String> {
    let t = node.text()?.trim();
    if t.is_empty() {
        None
    } else {
        Some(t.to_owned())
    }
}
fn child_text(node: roxmltree::Node<'_, '_>, name: &str) -> Option<String> {
    child(node, name).and_then(text)
}
fn child_amount(
    node: roxmltree::Node<'_, '_>,
    name: &str,
    malformed: &mut Vec<String>,
    path: &str,
) -> Option<InvoiceAmount> {
    let s = child_text(node, name)?;
    // Amount.Type: a third fraction digit is malformed for this codec, not a rounded InvoiceAmount.
    match InvoiceAmount::parse(&s) {
        Ok(a) => Some(a),
        Err(_) => {
            malformed.push(format!("{path}/{name}: {s}"));
            None
        }
    }
}

fn escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn leaf_ram(s: &mut String, indent: usize, tag: &str, value: &str, attr: Option<(&str, &str)>) {
    s.push_str(&"  ".repeat(indent));
    s.push_str("<ram:");
    s.push_str(tag);
    if let Some((k, v)) = attr {
        s.push(' ');
        s.push_str(k);
        s.push_str("=\"");
        s.push_str(&escape(v));
        s.push('"');
    }
    s.push('>');
    s.push_str(&escape(value));
    s.push_str("</ram:");
    s.push_str(tag);
    s.push_str(">\n");
}

fn amount_ram(s: &mut String, indent: usize, tag: &str, value: InvoiceAmount, cur: &str) {
    s.push_str(&"  ".repeat(indent));
    s.push_str("<ram:");
    s.push_str(tag);
    if !cur.is_empty() {
        s.push_str(" currencyID=\"");
        s.push_str(&escape(cur));
        s.push('"');
    }
    s.push('>');
    s.push_str(&value.to_string());
    s.push_str("</ram:");
    s.push_str(tag);
    s.push_str(">\n");
}

#[cfg(test)]
mod tests {
    use super::*;
    use core_invoice::reconcile;

    fn sample() -> Invoice {
        let mut inv = Invoice::blank(
            Profile::En16931,
            "INV-CII",
            "EUR",
            {
                let mut p = Party::new("Seller GmbH", "DE");
                p.vat_identifier = Some(Identifier::new("DE123456789"));
                p
            },
            Party::new("Buyer SARL", "FR"),
        );
        inv.issue_date = Date::parse("2026-01-15").ok();
        inv.type_code = Some(Code::new("380"));
        inv.lines = vec![{
            let mut line = Line::new(
                "1",
                "Service",
                Amount::parse("100.00").unwrap(),
                TaxCategory::vat("S", Decimal::from(19)),
            );
            line.quantity = Some(core_invoice::Quantity::parse("1").unwrap());
            line.unit = Some(Code::new("C62"));
            line.price = Some(core_invoice::Price {
                net: core_invoice::UnitPriceAmount::parse("100.00").unwrap(),
                discount: None,
                gross: None,
                base_qty: None,
                base_unit: None,
            });
            line
        }];
        inv.payment_terms = Some("Net 30".into());
        reconcile(&mut inv).unwrap();
        inv
    }

    #[test]
    fn lines_come_before_header_in_the_transaction() {
        let xml = write_unchecked(&sample()).unwrap();
        let line_at = xml.find("IncludedSupplyChainTradeLineItem").unwrap();
        let header_at = xml.find("ApplicableHeaderTradeAgreement").unwrap();
        assert!(line_at < header_at, "CII D16B puts lines before header");
        assert!(xml.contains("format=\"102\">20260115<"));
        assert!(!xml.contains("<Invoice "));
    }

    #[test]
    fn round_trip_cii() {
        let inv = sample();
        let xml = write_unchecked(&inv).unwrap();
        let back = read(&xml).unwrap().invoice;
        assert_eq!(back.number, "INV-CII");
        assert_eq!(back.lines[0].quantity, inv.lines[0].quantity);
        assert_eq!(
            back.lines[0].price.as_ref().map(|p| p.net),
            inv.lines[0].price.as_ref().map(|p| p.net)
        );
        assert_eq!(back.currency, "EUR");
        assert_eq!(back.lines[0].id, "1");
        assert_eq!(back.lines[0].tax.code, "S");
        assert_eq!(back.issue_date, inv.issue_date);
    }
}
