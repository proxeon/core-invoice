//! UBL 2.1 Invoice and CreditNote. Tree walk, no first-tag-wins scrape.

use crate::xml::{self, parse_xs_boolean};
use crate::{FormatError, Read};
use core_invoice::kind::DocumentKind;
use core_invoice::numeric::{Percentage, Quantity};
use core_invoice::payment::{CreditTransfer, DirectDebit, PaymentCard, PaymentMeans};
use core_invoice::proof::{ProfileMarker, Validated};
use core_invoice::tax::{TaxCategory, TaxSystem, wire_scheme};
use core_invoice::{
    Amount, Attachment, Code, Contact, Date, Delivery, DocumentReference, DocumentTotals,
    Identifier, Invoice, InvoiceAmount, InvoiceNote, Line, LineAllowanceCharge, Party, PartyTax,
    Payee, PaymentInstructions, Period, Profile, ProfileLookup, SupportingDocument, TaxBreakdown,
    TaxRepresentative, UnitPriceAmount,
};
use rust_decimal::Decimal;
use std::str::FromStr;

const NS_INV: &str = "urn:oasis:names:specification:ubl:schema:xsd:Invoice-2";
const NS_CN: &str = "urn:oasis:names:specification:ubl:schema:xsd:CreditNote-2";
const NS_CAC: &str = "urn:oasis:names:specification:ubl:schema:xsd:CommonAggregateComponents-2";
const NS_CBC: &str = "urn:oasis:names:specification:ubl:schema:xsd:CommonBasicComponents-2";

pub fn sniff(xml: &str) -> Result<DocumentKind, FormatError> {
    xml::refuse_dtd(xml)?;
    let name = xml::document_element_local(xml)
        .ok_or_else(|| FormatError::Parse("document is not well-formed UBL (no element)".into()))?;
    match name {
        "Invoice" => Ok(DocumentKind::Invoice),
        "CreditNote" => Ok(DocumentKind::CreditNote),
        other => Err(FormatError::Parse(format!(
            "not UBL Invoice/CreditNote: {other}"
        ))),
    }
}

/// Production UBL write. Stamps BT-24 / BT-23 from `P` before serialising.
pub fn write_validated<P: ProfileMarker>(proof: &Validated<P>) -> String {
    let mut invoice = proof.invoice().clone();
    // BT-24 and BT-23 come from the proved profile, not leftover fields on Invoice.
    invoice.stamp_profile(P::profile());
    write_unchecked(&invoice)
}

/// Unchecked UBL serialisation. Does not prove. Production write is [`write_validated`].
pub fn write_unchecked(invoice: &Invoice) -> String {
    let credit = invoice.kind == DocumentKind::CreditNote;
    let (root, xmlns, type_tag, line_tag, qty_tag) = if credit {
        (
            "CreditNote",
            NS_CN,
            "CreditNoteTypeCode",
            "CreditNoteLine",
            "CreditedQuantity",
        )
    } else {
        (
            "Invoice",
            NS_INV,
            "InvoiceTypeCode",
            "InvoiceLine",
            "InvoicedQuantity",
        )
    };
    let spec = invoice
        .specification_id
        .as_deref()
        .unwrap_or_else(|| invoice.profile.specification_id());
    let cur = &invoice.currency;
    let mut s = String::new();
    s.push_str(r#"<?xml version="1.0" encoding="UTF-8"?>"#);
    s.push('\n');
    s.push_str(&format!(
        r#"<{root} xmlns="{xmlns}" xmlns:cac="{NS_CAC}" xmlns:cbc="{NS_CBC}">"#
    ));
    s.push('\n');
    // UBL Invoice and CreditNote child sequences differ (DueDate, type code, line/quantity names).
    leaf(&mut s, 1, "CustomizationID", spec, None);
    // BT-23 / ProfileID: Peppol `…billing:01:1.0`; PINT `urn:peppol:bis:billing`; EN omits.
    if let Some(id) = invoice.profile.process_id() {
        leaf(&mut s, 1, "ProfileID", id, None);
    }
    leaf(&mut s, 1, "ID", &invoice.number, None);
    if let Some(d) = invoice.issue_date {
        leaf(&mut s, 1, "IssueDate", &d.to_string(), None);
    }
    if !credit && let Some(d) = invoice.due_date {
        leaf(&mut s, 1, "DueDate", &d.to_string(), None);
    }
    if credit && let Some(d) = invoice.tax_point_date {
        leaf(&mut s, 1, "TaxPointDate", &d.to_string(), None);
    }
    if let Some(code) = invoice.type_code.as_ref() {
        leaf(&mut s, 1, type_tag, code.as_str(), None);
    }
    for note in &invoice.notes {
        let text = match note.subject.as_ref() {
            Some(c) => format!("#{}#{}", c.as_str(), note.text),
            None => note.text.clone(),
        };
        leaf(&mut s, 1, "Note", &text, None);
    }
    if !credit && let Some(d) = invoice.tax_point_date {
        leaf(&mut s, 1, "TaxPointDate", &d.to_string(), None);
    }
    if !invoice.currency.is_empty() {
        leaf(&mut s, 1, "DocumentCurrencyCode", cur, None);
    }
    if let Some(tc) = invoice.tax_currency.as_ref() {
        leaf(&mut s, 1, "TaxCurrencyCode", tc.as_str(), None);
    }
    if let Some(acc) = invoice.buyer_accounting.as_deref() {
        leaf(&mut s, 1, "AccountingCost", acc, None);
    }
    if let Some(br) = invoice.buyer_reference.as_ref() {
        leaf(&mut s, 1, "BuyerReference", br.as_str(), None);
    }
    write_invoice_period(&mut s, invoice);
    if invoice.purchase_order.is_some() || invoice.sales_order.is_some() {
        open(&mut s, 1, "OrderReference");
        if let Some(po) = invoice.purchase_order.as_ref() {
            leaf(&mut s, 2, "ID", po.as_str(), None);
        }
        if let Some(so) = invoice.sales_order.as_ref() {
            leaf(&mut s, 2, "SalesOrderID", so.as_str(), None);
        }
        close(&mut s, 1, "OrderReference");
    }
    for p in &invoice.preceding {
        open(&mut s, 1, "BillingReference");
        open(&mut s, 2, "InvoiceDocumentReference");
        leaf(&mut s, 3, "ID", p.reference.as_str(), None);
        if let Some(d) = p.issue_date {
            leaf(&mut s, 3, "IssueDate", &d.to_string(), None);
        }
        close(&mut s, 2, "InvoiceDocumentReference");
        close(&mut s, 1, "BillingReference");
    }
    write_id_ref(
        &mut s,
        "DespatchDocumentReference",
        invoice.despatch.as_ref(),
    );
    write_id_ref(
        &mut s,
        "ReceiptDocumentReference",
        invoice.receiving_advice.as_ref(),
    );
    write_id_ref(
        &mut s,
        "OriginatorDocumentReference",
        invoice.tender.as_ref(),
    );
    write_id_ref(
        &mut s,
        "ContractDocumentReference",
        invoice.contract.as_ref(),
    );
    if let Some(obj) = invoice.invoiced_object.as_ref() {
        open(&mut s, 1, "AdditionalDocumentReference");
        leaf(
            &mut s,
            2,
            "ID",
            &obj.value,
            obj.scheme.as_deref().map(|sc| ("schemeID", sc)),
        );
        leaf(&mut s, 2, "DocumentTypeCode", "130", None);
        close(&mut s, 1, "AdditionalDocumentReference");
    }
    for doc in &invoice.supporting_documents {
        write_supporting(&mut s, doc);
    }
    if let Some(pr) = invoice.project.as_ref() {
        open(&mut s, 1, "ProjectReference");
        leaf(&mut s, 2, "ID", pr.as_str(), None);
        close(&mut s, 1, "ProjectReference");
    }
    write_party(
        &mut s,
        "AccountingSupplierParty",
        &invoice.seller,
        invoice.profile,
    );
    write_party(
        &mut s,
        "AccountingCustomerParty",
        &invoice.buyer,
        invoice.profile,
    );
    if let Some(payee) = invoice.payee.as_ref() {
        write_payee(&mut s, payee);
    }
    if let Some(tr) = invoice.tax_representative.as_ref() {
        write_tax_rep(&mut s, tr);
    }
    if let Some(d) = invoice.delivery.as_ref() {
        write_delivery(&mut s, d);
    }
    if let Some(pay) = invoice.payment.as_ref() {
        write_payment(&mut s, pay);
    }
    if let Some(terms) = invoice.payment_terms.as_deref() {
        open(&mut s, 1, "PaymentTerms");
        leaf(&mut s, 2, "Note", terms, None);
        close(&mut s, 1, "PaymentTerms");
    }
    for a in &invoice.document_allowances {
        write_allowance(&mut s, a, false, cur, invoice.profile);
    }
    for c in &invoice.document_charges {
        write_allowance(&mut s, c, true, cur, invoice.profile);
    }
    write_tax_total(&mut s, invoice, cur);
    write_totals(&mut s, invoice, cur);
    for line in &invoice.lines {
        write_line(&mut s, line, invoice, line_tag, qty_tag, cur);
    }
    s.push_str(&format!("</{root}>\n"));
    s
}

pub fn read(xml: &str) -> Result<Read, FormatError> {
    xml::refuse_dtd(xml)?;
    xml::refuse_depth(xml)?;
    let doc = roxmltree::Document::parse(xml)
        .map_err(|e| FormatError::Parse(format!("not well-formed: {e}")))?;
    let root = doc.root_element();
    let kind = match local(root) {
        "Invoice" => DocumentKind::Invoice,
        "CreditNote" => DocumentKind::CreditNote,
        other => {
            return Err(FormatError::Parse(format!(
                "document element must be Invoice or CreditNote, not {other}"
            )));
        }
    };
    let root_name = if kind == DocumentKind::CreditNote {
        "CreditNote"
    } else {
        "Invoice"
    };
    let customization = child_text(root, "CustomizationID");
    // Unknown BT-24 stays unknown; CORE-SPEC-01 is Fatal. Do not silently select En16931.
    let profile = match customization.as_deref() {
        Some(id) => match Profile::for_specification_id(id) {
            ProfileLookup::Profile(p) => p,
            ProfileLookup::WrongProcess | ProfileLookup::Unknown => Profile::Unknown,
        },
        None => Profile::Unknown,
    };
    let mut malformed = Vec::new();
    let number = child_text(root, "ID").unwrap_or_default();
    let currency = child_text(root, "DocumentCurrencyCode").unwrap_or_default();
    let seller = child(root, "AccountingSupplierParty")
        .and_then(|n| child(n, "Party"))
        .map(|n| read_party(n, profile))
        .unwrap_or_else(|| Party::new("", ""));
    let buyer = child(root, "AccountingCustomerParty")
        .and_then(|n| child(n, "Party"))
        .map(|n| read_party(n, profile))
        .unwrap_or_else(|| Party::new("", ""));
    let mut invoice = Invoice::blank(profile, number, currency, seller, buyer);
    invoice.kind = kind;
    invoice.specification_id = customization;
    invoice.business_process = child_text(root, "ProfileID");
    invoice.issue_date = child_date(root, "IssueDate", &mut malformed, root_name);
    let type_tag = if kind == DocumentKind::CreditNote {
        "CreditNoteTypeCode"
    } else {
        "InvoiceTypeCode"
    };
    invoice.type_code = child_text(root, type_tag).map(Code::new);
    invoice.tax_currency = child_text(root, "TaxCurrencyCode").map(Code::new);
    if let Some(d) = child_date(root, "DueDate", &mut malformed, root_name) {
        invoice.due_date = Some(d);
        if kind == DocumentKind::CreditNote {
            // UBL CreditNote has no DueDate. LHDN samples still put BT-9 here. Writer omits.
        }
    }
    invoice.tax_point_date = child_date(root, "TaxPointDate", &mut malformed, root_name);
    invoice.notes = children(root, "Note")
        .filter_map(text)
        .map(read_note)
        .collect();
    invoice.buyer_accounting = child_text(root, "AccountingCost");
    invoice.buyer_reference =
        child_text(root, "BuyerReference").map(core_invoice::DocumentReference::new);
    if let Some(p) = child(root, "InvoicePeriod") {
        invoice.period = Some(Period {
            start: child_date(p, "StartDate", &mut malformed, "InvoicePeriod"),
            end: child_date(p, "EndDate", &mut malformed, "InvoicePeriod"),
        });
        invoice.tax_point_code = child_text(p, "DescriptionCode").map(Code::new);
    }
    if let Some(or) = child(root, "OrderReference") {
        invoice.purchase_order = child_text(or, "ID").map(DocumentReference::new);
        invoice.sales_order = child_text(or, "SalesOrderID").map(DocumentReference::new);
    }
    invoice.preceding = children(root, "BillingReference")
        .filter_map(|n| {
            let r = child(n, "InvoiceDocumentReference")?;
            Some(core_invoice::PrecedingInvoice {
                reference: DocumentReference::new(child_text(r, "ID")?),
                issue_date: child_date(r, "IssueDate", &mut malformed, "BillingReference"),
            })
        })
        .collect();
    invoice.despatch = child(root, "DespatchDocumentReference")
        .and_then(|n| child_text(n, "ID"))
        .map(DocumentReference::new);
    invoice.receiving_advice = child(root, "ReceiptDocumentReference")
        .and_then(|n| child_text(n, "ID"))
        .map(DocumentReference::new);
    invoice.tender = child(root, "OriginatorDocumentReference")
        .and_then(|n| child_text(n, "ID"))
        .map(DocumentReference::new);
    invoice.contract = child(root, "ContractDocumentReference")
        .and_then(|n| child_text(n, "ID"))
        .map(DocumentReference::new);
    invoice.project = child(root, "ProjectReference")
        .and_then(|n| child_text(n, "ID"))
        .map(DocumentReference::new);
    for adr in children(root, "AdditionalDocumentReference") {
        let dtype = child_text(adr, "DocumentTypeCode");
        if dtype.as_deref() == Some("130") {
            if let Some(id) = child(adr, "ID") {
                invoice.invoiced_object = Some(ident(id));
            }
            continue;
        }
        if let Some(doc) = read_supporting(adr) {
            invoice.supporting_documents.push(doc);
        }
    }
    invoice.payee = child(root, "PayeeParty").map(read_payee);
    invoice.tax_representative = child(root, "TaxRepresentativeParty").map(read_tax_rep);
    invoice.delivery = child(root, "Delivery").map(|n| read_delivery(n, &mut malformed));
    invoice.payment = child(root, "PaymentMeans").map(read_payment);
    invoice.payment_terms = child(root, "PaymentTerms").and_then(|n| child_text(n, "Note"));
    invoice.document_allowances = children(root, "AllowanceCharge")
        .filter(|n| charge_indicator(*n) == Some(false))
        .filter_map(|n| read_allowance(n, profile, &mut malformed))
        .collect();
    invoice.document_charges = children(root, "AllowanceCharge")
        .filter(|n| charge_indicator(*n) == Some(true))
        .filter_map(|n| read_allowance(n, profile, &mut malformed))
        .collect();
    let mut doc_tax = None;
    let mut acct_tax = None;
    for tt in children(root, "TaxTotal") {
        let cid = child(tt, "TaxAmount")
            .and_then(|n| n.attribute("currencyID"))
            .unwrap_or("");
        let amt = child_amount(tt, "TaxAmount", &mut malformed, "TaxTotal");
        let has_sub = child(tt, "TaxSubtotal").is_some();
        if has_sub || cid.is_empty() || cid == invoice.currency {
            if has_sub {
                invoice.tax_breakdown.extend(
                    children(tt, "TaxSubtotal")
                        .filter_map(|n| read_subtotal(n, profile, &mut malformed)),
                );
            }
            doc_tax = amt.or(doc_tax);
        } else if invoice
            .tax_currency
            .as_ref()
            .is_some_and(|c| c.as_str() == cid)
        {
            acct_tax = amt;
        }
    }
    let saw_tax_total = children(root, "TaxTotal").next().is_some();
    if let Some(lmt) = child(root, "LegalMonetaryTotal") {
        // Absent BT-107 is None, not 0.00. Missing PayableAmount is None (BR-15), not 0.
        let mut totals = read_totals(lmt, &mut malformed);
        if totals.tax_total.is_none() {
            totals.tax_total = doc_tax;
        }
        totals.tax_total_accounting = acct_tax.or(totals.tax_total_accounting);
        invoice.totals = Some(totals);
    } else if saw_tax_total {
        // TaxTotal without LegalMonetaryTotal still carries BT-110 / BT-111 (BR-53, BR-CO-14).
        invoice.totals = Some(DocumentTotals {
            tax_total: doc_tax,
            tax_total_accounting: acct_tax,
            ..DocumentTotals::default()
        });
    }
    // CEN CreditNote fragments sometimes copy InvoiceLine. Read both.
    invoice.lines = children(root, "InvoiceLine")
        .chain(children(root, "CreditNoteLine"))
        .filter_map(|n| read_line(n, profile, kind, &mut malformed))
        .collect();
    let mut unmapped = unmapped_children(root, root_name, MAPPED_INVOICE_CHILDREN);
    for line_tag in ["InvoiceLine", "CreditNoteLine"] {
        for line in children(root, line_tag) {
            unmapped.extend(unmapped_children(line, line_tag, MAPPED_LINE_CHILDREN));
            if let Some(item) = child(line, "Item") {
                unmapped.extend(unmapped_children(item, "Item", MAPPED_ITEM_CHILDREN));
            }
        }
    }
    for (parent, party) in [
        (
            "AccountingSupplierParty",
            child(root, "AccountingSupplierParty").and_then(|n| child(n, "Party")),
        ),
        (
            "AccountingCustomerParty",
            child(root, "AccountingCustomerParty").and_then(|n| child(n, "Party")),
        ),
    ] {
        let _ = parent;
        if let Some(p) = party {
            unmapped.extend(unmapped_children(p, "Party", MAPPED_PARTY_CHILDREN));
            if let Some(addr) = child(p, "PostalAddress") {
                unmapped.extend(unmapped_children(
                    addr,
                    "PostalAddress",
                    MAPPED_ADDRESS_CHILDREN,
                ));
            }
        }
    }
    report_duplicate_singletons(root, root_name, SINGLETON_INVOICE, &mut unmapped);
    Ok(Read {
        invoice,
        unmapped,
        malformed,
    })
}

const MAPPED_INVOICE_CHILDREN: &[&str] = &[
    "CustomizationID",
    "ProfileID",
    "ID",
    "IssueDate",
    "DueDate",
    "TaxPointDate",
    "InvoiceTypeCode",
    "CreditNoteTypeCode",
    "Note",
    "DocumentCurrencyCode",
    "TaxCurrencyCode",
    "AccountingCost",
    "BuyerReference",
    "InvoicePeriod",
    "OrderReference",
    "BillingReference",
    "DespatchDocumentReference",
    "ReceiptDocumentReference",
    "OriginatorDocumentReference",
    "ContractDocumentReference",
    "AdditionalDocumentReference",
    "ProjectReference",
    "AccountingSupplierParty",
    "AccountingCustomerParty",
    "PayeeParty",
    "TaxRepresentativeParty",
    "Delivery",
    "PaymentMeans",
    "PaymentTerms",
    "AllowanceCharge",
    "TaxTotal",
    "LegalMonetaryTotal",
    "InvoiceLine",
    "CreditNoteLine",
];

const MAPPED_LINE_CHILDREN: &[&str] = &[
    "ID",
    "Note",
    "InvoicedQuantity",
    "CreditedQuantity",
    "LineExtensionAmount",
    "AccountingCost",
    "InvoicePeriod",
    "OrderLineReference",
    "DocumentReference",
    "AllowanceCharge",
    "TaxTotal",
    "Item",
    "Price",
];

const MAPPED_ITEM_CHILDREN: &[&str] = &[
    "Description",
    "Name",
    "BuyersItemIdentification",
    "SellersItemIdentification",
    "StandardItemIdentification",
    "OriginCountry",
    "CommodityClassification",
    "ClassifiedTaxCategory",
    "AdditionalItemProperty",
];

const MAPPED_PARTY_CHILDREN: &[&str] = &[
    "EndpointID",
    "PartyIdentification",
    "PartyName",
    "PostalAddress",
    "PartyTaxScheme",
    "PartyLegalEntity",
    "Contact",
];

const MAPPED_ADDRESS_CHILDREN: &[&str] = &[
    "StreetName",
    "AdditionalStreetName",
    "CityName",
    "PostalZone",
    "CountrySubentity",
    "Country",
];

const SINGLETON_INVOICE: &[&str] = &[
    "CustomizationID",
    "ProfileID",
    "ID",
    "IssueDate",
    "DueDate",
    "InvoiceTypeCode",
    "CreditNoteTypeCode",
    "DocumentCurrencyCode",
    "TaxCurrencyCode",
    "AccountingCost",
    "BuyerReference",
    "InvoicePeriod",
    "OrderReference",
    "LegalMonetaryTotal",
    "PayeeParty",
    "TaxRepresentativeParty",
    "Delivery",
    "PaymentMeans",
    "PaymentTerms",
];

fn unmapped_children(node: roxmltree::Node<'_, '_>, parent: &str, mapped: &[&str]) -> Vec<String> {
    node.children()
        .filter(|n| n.is_element())
        .map(local)
        .filter(|name| !mapped.contains(name))
        .map(|name| format!("{parent}/{name}"))
        .collect()
}

fn report_duplicate_singletons(
    node: roxmltree::Node<'_, '_>,
    parent: &str,
    names: &[&str],
    unmapped: &mut Vec<String>,
) {
    for name in names {
        if children(node, name).count() > 1 {
            unmapped.push(format!("{parent}/{name} (duplicate)"));
        }
    }
}

/// UBL CreditNote has no DueDate. LHDN samples still put BT-9 here. Writer omits.
pub fn write_drops(invoice: &Invoice) -> Vec<String> {
    let mut drops = Vec::new();
    if invoice.kind == DocumentKind::CreditNote && invoice.due_date.is_some() {
        drops.push("CreditNote/DueDate".into());
    }
    drops
}

fn charge_indicator(node: roxmltree::Node<'_, '_>) -> Option<bool> {
    child_text(node, "ChargeIndicator").and_then(|s| parse_xs_boolean(&s))
}

fn write_invoice_period(s: &mut String, invoice: &Invoice) {
    let has_period = invoice
        .period
        .as_ref()
        .is_some_and(|p| p.start.is_some() || p.end.is_some());
    let has_code = invoice.tax_point_code.is_some();
    if !has_period && !has_code {
        return;
    }
    open(s, 1, "InvoicePeriod");
    if let Some(p) = invoice.period.as_ref() {
        if let Some(d) = p.start {
            leaf(s, 2, "StartDate", &d.to_string(), None);
        }
        if let Some(d) = p.end {
            leaf(s, 2, "EndDate", &d.to_string(), None);
        }
    }
    if let Some(c) = invoice.tax_point_code.as_ref() {
        leaf(s, 2, "DescriptionCode", c.as_str(), None);
    }
    close(s, 1, "InvoicePeriod");
}

fn write_id_ref(s: &mut String, tag: &str, id: Option<&DocumentReference>) {
    let Some(id) = id else {
        return;
    };
    open(s, 1, tag);
    leaf(s, 2, "ID", id.as_str(), None);
    close(s, 1, tag);
}

fn write_supporting(s: &mut String, doc: &SupportingDocument) {
    open(s, 1, "AdditionalDocumentReference");
    leaf(s, 2, "ID", doc.id.as_str(), None);
    if let Some(d) = doc.description.as_deref() {
        leaf(s, 2, "DocumentDescription", d, None);
    }
    if doc.uri.is_some() || doc.attachment.is_some() {
        open(s, 2, "Attachment");
        if let Some(att) = doc.attachment.as_ref() {
            leaf_attrs(
                s,
                3,
                "EmbeddedDocumentBinaryObject",
                &b64_encode(&att.bytes),
                &[
                    ("mimeCode", att.mime.as_str()),
                    ("filename", att.filename.as_str()),
                ],
            );
        }
        if let Some(uri) = doc.uri.as_deref() {
            open(s, 3, "ExternalReference");
            leaf(s, 4, "URI", uri, None);
            close(s, 3, "ExternalReference");
        }
        close(s, 2, "Attachment");
    }
    close(s, 1, "AdditionalDocumentReference");
}

fn write_payee(s: &mut String, payee: &Payee) {
    open(s, 1, "PayeeParty");
    if let Some(id) = payee.identifier.as_ref() {
        open(s, 2, "PartyIdentification");
        leaf(
            s,
            3,
            "ID",
            &id.value,
            id.scheme.as_deref().map(|sc| ("schemeID", sc)),
        );
        close(s, 2, "PartyIdentification");
    }
    if !payee.name.is_empty() {
        open(s, 2, "PartyName");
        leaf(s, 3, "Name", &payee.name, None);
        close(s, 2, "PartyName");
    }
    if let Some(legal) = payee.legal_registration.as_ref() {
        open(s, 2, "PartyLegalEntity");
        leaf(
            s,
            3,
            "CompanyID",
            &legal.value,
            legal.scheme.as_deref().map(|sc| ("schemeID", sc)),
        );
        close(s, 2, "PartyLegalEntity");
    }
    close(s, 1, "PayeeParty");
}

fn write_tax_rep(s: &mut String, tr: &TaxRepresentative) {
    // TaxRepresentativeParty is PartyType: PartyName / PostalAddress / PartyTaxScheme
    // sit directly under it. Do not wrap an extra cac:Party (XSD-invalid).
    open(s, 1, "TaxRepresentativeParty");
    if !tr.name.is_empty() {
        open(s, 2, "PartyName");
        leaf(s, 3, "Name", &tr.name, None);
        close(s, 2, "PartyName");
    }
    if let Some(addr) = tr.address.as_ref()
        && let Some(cc) = addr.country.as_ref().map(Code::as_str)
        && !cc.is_empty()
    {
        open(s, 2, "PostalAddress");
        open(s, 3, "Country");
        leaf(s, 4, "IdentificationCode", cc, None);
        close(s, 3, "Country");
        close(s, 2, "PostalAddress");
    }
    if let Some(vat) = tr.vat_identifier.as_ref() {
        open(s, 2, "PartyTaxScheme");
        leaf(s, 3, "CompanyID", &vat.value, None);
        open(s, 3, "TaxScheme");
        leaf(s, 4, "ID", "VAT", None);
        close(s, 3, "TaxScheme");
        close(s, 2, "PartyTaxScheme");
    }
    close(s, 1, "TaxRepresentativeParty");
}

fn write_delivery(s: &mut String, d: &Delivery) {
    open(s, 1, "Delivery");
    if let Some(date) = d.date {
        leaf(s, 2, "ActualDeliveryDate", &date.to_string(), None);
    }
    let country = d
        .address
        .as_ref()
        .and_then(|a| a.country.as_ref().map(Code::as_str))
        .filter(|c| !c.is_empty());
    if d.name.is_some() || country.is_some() {
        open(s, 2, "DeliveryLocation");
        open(s, 3, "Address");
        if let Some(n) = d.name.as_deref() {
            open(s, 4, "AddressLine");
            leaf(s, 5, "Line", n, None);
            close(s, 4, "AddressLine");
        }
        if let Some(cc) = country {
            open(s, 4, "Country");
            leaf(s, 5, "IdentificationCode", cc, None);
            close(s, 4, "Country");
        }
        close(s, 3, "Address");
        close(s, 2, "DeliveryLocation");
    }
    close(s, 1, "Delivery");
}

fn write_party(s: &mut String, tag: &str, party: &Party, profile: Profile) {
    open(s, 1, tag);
    open(s, 2, "Party");
    if let Some(ep) = party.electronic_address.as_ref() {
        let scheme = ep.scheme.as_deref();
        leaf(
            s,
            3,
            "EndpointID",
            &ep.value,
            scheme.map(|sc| ("schemeID", sc)),
        );
    }
    for id in &party.identifiers {
        open(s, 3, "PartyIdentification");
        leaf(
            s,
            4,
            "ID",
            &id.value,
            id.scheme.as_deref().map(|sc| ("schemeID", sc)),
        );
        close(s, 3, "PartyIdentification");
    }
    if !party.name.is_empty() {
        open(s, 3, "PartyName");
        leaf(s, 4, "Name", &party.name, None);
        close(s, 3, "PartyName");
    }
    open(s, 3, "PostalAddress");
    if let Some(addr) = party.address.as_ref() {
        if let Some(v) = addr.line1.as_deref() {
            leaf(s, 4, "StreetName", v, None);
        }
        if let Some(v) = addr.city.as_deref() {
            leaf(s, 4, "CityName", v, None);
        }
        if let Some(v) = addr.post_code.as_deref() {
            leaf(s, 4, "PostalZone", v, None);
        }
    }
    if !party.country().is_empty() {
        open(s, 4, "Country");
        leaf(s, 5, "IdentificationCode", party.country(), None);
        close(s, 4, "Country");
    }
    close(s, 3, "PostalAddress");
    if let Some(vat) = party.vat_identifier.as_ref() {
        open(s, 3, "PartyTaxScheme");
        leaf(s, 4, "CompanyID", &vat.value, None);
        open(s, 4, "TaxScheme");
        leaf(s, 5, "ID", "VAT", None);
        close(s, 4, "TaxScheme");
        close(s, 3, "PartyTaxScheme");
    }
    // PINT-MY: TIN = PartyTaxScheme CompanyID schemeID GST + TaxScheme VAT.
    // SST uses TaxScheme VAT + category SA/…. BRN = PartyLegalEntity CompanyID, no scheme.
    // Endpoint 0230.
    if let Some(tin) = party.tax_registration.as_ref() {
        open(s, 3, "PartyTaxScheme");
        let scheme = if profile == Profile::PintMy {
            Some(("schemeID", "GST"))
        } else {
            tin.scheme.as_deref().map(|sc| ("schemeID", sc))
        };
        leaf(s, 4, "CompanyID", &tin.value, scheme);
        open(s, 4, "TaxScheme");
        leaf(s, 5, "ID", "VAT", None);
        close(s, 4, "TaxScheme");
        close(s, 3, "PartyTaxScheme");
    }
    if let Some(legal) = party.legal_registration.as_ref() {
        open(s, 3, "PartyLegalEntity");
        leaf(s, 4, "RegistrationName", &party.name, None);
        leaf(
            s,
            4,
            "CompanyID",
            &legal.value,
            legal.scheme.as_deref().map(|sc| ("schemeID", sc)),
        );
        close(s, 3, "PartyLegalEntity");
    }
    if let Some(c) = party.contact.as_ref() {
        open(s, 3, "Contact");
        if let Some(v) = c.point.as_deref() {
            leaf(s, 4, "Name", v, None);
        }
        if let Some(v) = c.phone.as_deref() {
            leaf(s, 4, "Telephone", v, None);
        }
        if let Some(v) = c.email.as_deref() {
            leaf(s, 4, "ElectronicMail", v, None);
        }
        close(s, 3, "Contact");
    }
    close(s, 2, "Party");
    close(s, 1, tag);
}

fn write_payment(s: &mut String, pay: &PaymentInstructions) {
    open(s, 1, "PaymentMeans");
    if let Some(c) = pay.means_code.as_ref() {
        // BT-82 is @name on PaymentMeansCode, not InstructionNote (UBL-CR-681).
        let name = pay.means_text.as_deref().map(|t| ("name", t));
        leaf(s, 2, "PaymentMeansCode", c.as_str(), name);
    }
    if let Some(r) = pay.remittance.as_deref() {
        // BT-83 remittance → PaymentID.
        leaf(s, 2, "PaymentID", r, None);
    }
    match pay.means.as_ref() {
        Some(PaymentMeans::CreditTransfer(accounts)) => {
            for a in accounts {
                write_financial_account(s, 2, "PayeeFinancialAccount", a);
            }
        }
        Some(PaymentMeans::Card(card)) => {
            open(s, 2, "CardAccount");
            leaf(s, 3, "PrimaryAccountNumberID", &card.pan, None);
            if let Some(h) = card.holder.as_deref() {
                leaf(s, 3, "HolderName", h, None);
            }
            close(s, 2, "CardAccount");
        }
        Some(PaymentMeans::DirectDebit(dd)) => {
            if let Some(cred) = dd.creditor_id.as_ref() {
                open(s, 2, "PayeeFinancialAccount");
                leaf(s, 3, "ID", &cred.value, None);
                close(s, 2, "PayeeFinancialAccount");
            }
            open(s, 2, "PaymentMandate");
            if let Some(m) = dd.mandate.as_deref() {
                leaf(s, 3, "ID", m, None);
            }
            if let Some(acc) = dd.debited_account.as_ref() {
                open(s, 3, "PayerFinancialAccount");
                leaf(s, 4, "ID", &acc.value, None);
                close(s, 3, "PayerFinancialAccount");
            }
            close(s, 2, "PaymentMandate");
        }
        None => {}
    }
    close(s, 1, "PaymentMeans");
}

fn write_financial_account(s: &mut String, indent: usize, tag: &str, a: &CreditTransfer) {
    open(s, indent, tag);
    leaf(s, indent + 1, "ID", &a.account_id.value, None);
    if let Some(n) = a.account_name.as_deref() {
        leaf(s, indent + 1, "Name", n, None);
    }
    if let Some(bic) = a.provider.as_deref() {
        open(s, indent + 1, "FinancialInstitutionBranch");
        leaf(s, indent + 2, "ID", bic, None);
        close(s, indent + 1, "FinancialInstitutionBranch");
    }
    close(s, indent, tag);
}

fn write_allowance(
    s: &mut String,
    a: &core_invoice::AllowanceCharge,
    charge: bool,
    cur: &str,
    profile: Profile,
) {
    open(s, 1, "AllowanceCharge");
    leaf(
        s,
        2,
        "ChargeIndicator",
        if charge { "true" } else { "false" },
        None,
    );
    if let Some(p) = a.percent {
        leaf(s, 2, "MultiplierFactorNumeric", &p.to_string(), None);
    }
    amount(s, 2, "Amount", a.amount, cur);
    if let Some(b) = a.base {
        amount(s, 2, "BaseAmount", b, cur);
    }
    if let Some(tax) = a.tax.as_ref() {
        open(s, 2, "TaxCategory");
        leaf(s, 3, "ID", &tax.code, None);
        if let Some(p) = tax.percent {
            leaf(s, 3, "Percent", &p.to_string(), None);
        }
        open(s, 3, "TaxScheme");
        leaf(
            s,
            4,
            "ID",
            wire_scheme(profile, tax.system, &tax.code),
            None,
        );
        close(s, 3, "TaxScheme");
        close(s, 2, "TaxCategory");
    }
    close(s, 1, "AllowanceCharge");
}

fn write_tax_total(s: &mut String, invoice: &Invoice, cur: &str) {
    // First TaxTotal @currencyID = BT-5 is BT-110 + BG-23. Second TaxTotal @currencyID = BT-6 is BT-111, no TaxSubtotal.
    if invoice.tax_total().is_none() && invoice.tax_breakdown.is_empty() {
        return;
    }
    open(s, 1, "TaxTotal");
    amount(
        s,
        2,
        "TaxAmount",
        invoice.tax_total().unwrap_or(Amount::ZERO),
        cur,
    );
    for row in &invoice.tax_breakdown {
        open(s, 2, "TaxSubtotal");
        amount(s, 3, "TaxableAmount", row.taxable, cur);
        amount(s, 3, "TaxAmount", row.tax, cur);
        open(s, 3, "TaxCategory");
        leaf(s, 4, "ID", row.category.as_str(), None);
        if let Some(r) = row.rate {
            leaf(s, 4, "Percent", &r.to_string(), None);
        }
        if let Some(reason) = row.exemption_reason.as_deref() {
            leaf(s, 4, "TaxExemptionReason", reason, None);
        }
        open(s, 4, "TaxScheme");
        leaf(
            s,
            5,
            "ID",
            wire_scheme(invoice.profile, row.system, row.category.as_str()),
            None,
        );
        close(s, 4, "TaxScheme");
        close(s, 3, "TaxCategory");
        close(s, 2, "TaxSubtotal");
    }
    close(s, 1, "TaxTotal");
    if let (Some(tc), Some(bt111)) = (
        invoice.tax_currency.as_ref(),
        invoice.totals.as_ref().and_then(|t| t.tax_total_accounting),
    ) {
        open(s, 1, "TaxTotal");
        amount(s, 2, "TaxAmount", bt111, tc.as_str());
        close(s, 1, "TaxTotal");
    }
}

fn write_totals(s: &mut String, invoice: &Invoice, cur: &str) {
    // UBL MonetaryTotalType sequence: LineExtension, TaxExclusive, TaxInclusive,
    // AllowanceTotal, ChargeTotal, Prepaid, PayableRounding, Payable.
    let Some(t) = invoice.totals.as_ref() else {
        // Absent BG-22 is not 0.00.
        return;
    };
    open(s, 1, "LegalMonetaryTotal");
    if let Some(v) = t.line_net {
        amount(s, 2, "LineExtensionAmount", v, cur);
    }
    if let Some(v) = t.without_tax {
        amount(s, 2, "TaxExclusiveAmount", v, cur);
    }
    if let Some(v) = t.with_tax {
        amount(s, 2, "TaxInclusiveAmount", v, cur);
    }
    if let Some(v) = t.allowance_total {
        amount(s, 2, "AllowanceTotalAmount", v, cur);
    }
    if let Some(v) = t.charge_total {
        amount(s, 2, "ChargeTotalAmount", v, cur);
    }
    if let Some(v) = t.paid {
        amount(s, 2, "PrepaidAmount", v, cur);
    }
    if let Some(v) = t.rounding {
        amount(s, 2, "PayableRoundingAmount", v, cur);
    }
    if let Some(v) = t.payable {
        amount(s, 2, "PayableAmount", v, cur);
    }
    close(s, 1, "LegalMonetaryTotal");
}

fn write_line(
    s: &mut String,
    line: &Line,
    invoice: &Invoice,
    line_tag: &str,
    qty_tag: &str,
    cur: &str,
) {
    open(s, 1, line_tag);
    leaf(s, 2, "ID", &line.id, None);
    if let Some(n) = line.note.as_deref() {
        leaf(s, 2, "Note", n, None);
    }
    if let Some(q) = line.quantity {
        let unit = line.unit.as_ref().map(|c| ("unitCode", c.as_str()));
        leaf(s, 2, qty_tag, &q.to_string(), unit);
    }
    amount(s, 2, "LineExtensionAmount", line.net, cur);
    if let Some(acc) = line.accounting_reference.as_deref() {
        // BT-133 InvoiceLine/cbc:AccountingCost. Not header BT-19.
        leaf(s, 2, "AccountingCost", acc, None);
    }
    if let Some(p) = line.period.as_ref() {
        open(s, 2, "InvoicePeriod");
        if let Some(d) = p.start {
            leaf(s, 3, "StartDate", &d.to_string(), None);
        }
        if let Some(d) = p.end {
            leaf(s, 3, "EndDate", &d.to_string(), None);
        }
        close(s, 2, "InvoicePeriod");
    }
    if let Some(ol) = line.order_line.as_deref() {
        // BT-132 OrderLineReference/LineID. Not BT-13, not BT-126.
        open(s, 2, "OrderLineReference");
        leaf(s, 3, "LineID", ol, None);
        close(s, 2, "OrderLineReference");
    }
    if let Some(obj) = line.invoiced_object.as_ref() {
        open(s, 2, "DocumentReference");
        leaf(s, 3, "ID", &obj.value, None);
        let code = line
            .invoiced_object_code
            .as_ref()
            .map(Code::as_str)
            .unwrap_or("130");
        leaf(s, 3, "DocumentTypeCode", code, None);
        close(s, 2, "DocumentReference");
    }
    for a in &line.allowances {
        write_line_ac(s, a, false, cur);
    }
    for c in &line.charges {
        write_line_ac(s, c, true, cur);
    }
    if let Some(tt) = line.tax_total {
        open(s, 2, "TaxTotal");
        amount(s, 3, "TaxAmount", tt, cur);
        close(s, 2, "TaxTotal");
    }
    open(s, 2, "Item");
    if let Some(d) = line.description.as_deref() {
        leaf(s, 3, "Description", d, None);
    }
    leaf(s, 3, "Name", &line.name, None);
    if let Some(id) = line.item_id.as_ref() {
        open(s, 3, "SellersItemIdentification");
        leaf(
            s,
            4,
            "ID",
            &id.value,
            id.scheme.as_deref().map(|sc| ("schemeID", sc)),
        );
        close(s, 3, "SellersItemIdentification");
    }
    if let Some(id) = line.buyer_id.as_ref() {
        // BT-156 BuyersItemIdentification. Not BT-155 (item_id), not BT-157 (standard_id).
        open(s, 3, "BuyersItemIdentification");
        leaf(
            s,
            4,
            "ID",
            &id.value,
            id.scheme.as_deref().map(|sc| ("schemeID", sc)),
        );
        close(s, 3, "BuyersItemIdentification");
    }
    if let Some(id) = line.standard_id.as_ref() {
        open(s, 3, "StandardItemIdentification");
        leaf(
            s,
            4,
            "ID",
            &id.value,
            id.scheme.as_deref().map(|sc| ("schemeID", sc)),
        );
        close(s, 3, "StandardItemIdentification");
    }
    if let Some(c) = line.origin_country.as_ref() {
        open(s, 3, "OriginCountry");
        leaf(s, 4, "IdentificationCode", c.as_str(), None);
        close(s, 3, "OriginCountry");
    }
    for cl in &line.classifications {
        open(s, 3, "CommodityClassification");
        leaf(
            s,
            4,
            "ItemClassificationCode",
            &cl.value,
            cl.scheme.as_deref().map(|sc| ("listID", sc)),
        );
        close(s, 3, "CommodityClassification");
    }
    for attr in &line.attributes {
        // BG-32 AdditionalItemProperty: BT-160 Name, BT-161 Value.
        open(s, 3, "AdditionalItemProperty");
        leaf(s, 4, "Name", &attr.name, None);
        leaf(s, 4, "Value", &attr.value, None);
        close(s, 3, "AdditionalItemProperty");
    }
    if !line.tax.code.trim().is_empty() {
        open(s, 3, "ClassifiedTaxCategory");
        leaf(s, 4, "ID", &line.tax.code, None);
        // TTX (scheme AAL): no IBT-119. Amount is the tax. Do not emit Percent.
        if let Some(p) = line.tax.percent {
            leaf(s, 4, "Percent", &p.to_string(), None);
        }
        open(s, 4, "TaxScheme");
        leaf(
            s,
            5,
            "ID",
            wire_scheme(invoice.profile, line.tax.system, &line.tax.code),
            None,
        );
        close(s, 4, "TaxScheme");
        close(s, 3, "ClassifiedTaxCategory");
    }
    for extra in &line.extra_tax {
        open(s, 3, "ClassifiedTaxCategory");
        leaf(s, 4, "ID", &extra.code, None);
        if let Some(p) = extra.percent {
            leaf(s, 4, "Percent", &p.to_string(), None);
        }
        open(s, 4, "TaxScheme");
        leaf(
            s,
            5,
            "ID",
            wire_scheme(invoice.profile, extra.system, &extra.code),
            None,
        );
        close(s, 4, "TaxScheme");
        close(s, 3, "ClassifiedTaxCategory");
    }
    close(s, 2, "Item");
    if let Some(price) = line.price.as_ref() {
        open(s, 2, "Price");
        amount_unit(s, 3, "PriceAmount", price.net, cur);
        if let Some(q) = price.base_qty {
            let unit = price.base_unit.as_ref().map(|c| ("unitCode", c.as_str()));
            leaf(s, 3, "BaseQuantity", &q.to_string(), unit);
        }
        if price.discount.is_some() || price.gross.is_some() {
            // R044: Price/AllowanceCharge is an allowance (ChargeIndicator false). BT-147 Amount, BT-148 BaseAmount.
            open(s, 3, "AllowanceCharge");
            leaf(s, 4, "ChargeIndicator", "false", None);
            if let Some(d) = price.discount {
                amount_unit(s, 4, "Amount", d, cur);
            }
            if let Some(g) = price.gross {
                amount_unit(s, 4, "BaseAmount", g, cur);
            }
            close(s, 3, "AllowanceCharge");
        }
        close(s, 2, "Price");
    }
    close(s, 1, line_tag);
}

fn write_line_ac(s: &mut String, a: &LineAllowanceCharge, charge: bool, cur: &str) {
    open(s, 2, "AllowanceCharge");
    leaf(
        s,
        3,
        "ChargeIndicator",
        if charge { "true" } else { "false" },
        None,
    );
    if let Some(c) = a.reason_code.as_ref() {
        leaf(s, 3, "AllowanceChargeReasonCode", c.as_str(), None);
    }
    if let Some(r) = a.reason.as_deref() {
        leaf(s, 3, "AllowanceChargeReason", r, None);
    }
    if let Some(p) = a.percent {
        leaf(s, 3, "MultiplierFactorNumeric", &p.to_string(), None);
    }
    amount(s, 3, "Amount", a.amount, cur);
    if let Some(b) = a.base {
        amount(s, 3, "BaseAmount", b, cur);
    }
    close(s, 2, "AllowanceCharge");
}

fn read_party(node: roxmltree::Node<'_, '_>, profile: Profile) -> Party {
    let name = child(node, "PartyName")
        .and_then(|n| child_text(n, "Name"))
        .or_else(|| child(node, "PartyLegalEntity").and_then(|n| child_text(n, "RegistrationName")))
        .unwrap_or_default();
    let country = child(node, "PostalAddress")
        .and_then(|n| child(n, "Country"))
        .and_then(|n| child_text(n, "IdentificationCode"))
        .unwrap_or_default();
    let mut party = Party::new(name, country);
    if let Some(ep) = child(node, "EndpointID") {
        let value = text(ep).unwrap_or_default();
        let scheme = ep.attribute("schemeID").map(str::to_owned);
        party.electronic_address = Some(Identifier {
            value,
            scheme,
            scheme_version: None,
        });
    }
    if let Some(legal) = child(node, "PartyLegalEntity")
        && let Some(id) = child(legal, "CompanyID")
    {
        party.legal_registration = Some(ident(id));
    }
    for pid in children(node, "PartyIdentification") {
        if let Some(id) = child(pid, "ID") {
            party.identifiers.push(ident(id));
        }
    }
    for tax in children(node, "PartyTaxScheme") {
        let Some(id) = child(tax, "CompanyID") else {
            continue;
        };
        let ident = ident(id);
        let scheme = child(tax, "TaxScheme")
            .and_then(|n| child_text(n, "ID"))
            .unwrap_or_default();
        if ident.scheme.as_deref() == Some("GST")
            || scheme.eq_ignore_ascii_case("GST")
            || scheme.eq_ignore_ascii_case("AAL")
            || scheme.eq_ignore_ascii_case("TAX")
        {
            if party.tax_registration.is_none() {
                party.tax_registration = Some(ident);
            }
        } else if scheme.eq_ignore_ascii_case("VAT") || scheme.is_empty() {
            if party.vat_identifier.is_none() {
                party.vat_identifier = Some(ident);
            } else if party.tax_registration.is_none() {
                party.tax_registration = Some(ident);
            }
        } else {
            party.party_taxes.push(PartyTax { id: ident, scheme });
        }
    }
    let _ = profile;
    if let Some(addr) = child(node, "PostalAddress") {
        party.address = Some(core_invoice::PostalAddress {
            line1: child_text(addr, "StreetName"),
            line2: child_text(addr, "AdditionalStreetName"),
            line3: None,
            city: child_text(addr, "CityName"),
            post_code: child_text(addr, "PostalZone"),
            subdivision: child_text(addr, "CountrySubentity"),
            country: child(addr, "Country")
                .and_then(|n| child_text(n, "IdentificationCode"))
                .map(Code::new),
        });
    }
    if let Some(c) = child(node, "Contact") {
        party.contact = Some(Contact {
            point: child_text(c, "Name"),
            phone: child_text(c, "Telephone"),
            email: child_text(c, "ElectronicMail"),
        });
    }
    party
}

fn read_payee(node: roxmltree::Node<'_, '_>) -> Payee {
    Payee {
        name: child(node, "PartyName")
            .and_then(|n| child_text(n, "Name"))
            .unwrap_or_default(),
        identifier: child(node, "PartyIdentification")
            .and_then(|n| child(n, "ID"))
            .map(ident),
        legal_registration: child(node, "PartyLegalEntity")
            .and_then(|n| child(n, "CompanyID"))
            .map(ident),
    }
}

fn read_tax_rep(node: roxmltree::Node<'_, '_>) -> TaxRepresentative {
    // Spec-valid: PartyType children directly. Nested cac:Party (older writer / some samples) also accepted.
    let party = child(node, "Party").unwrap_or(node);
    let country = child(party, "PostalAddress")
        .and_then(|n| child(n, "Country"))
        .and_then(|n| child_text(n, "IdentificationCode"))
        .map(Code::new);
    TaxRepresentative {
        name: child(party, "PartyName")
            .and_then(|n| child_text(n, "Name"))
            .unwrap_or_default(),
        vat_identifier: children(party, "PartyTaxScheme").find_map(|tax| {
            let scheme = child(tax, "TaxScheme")
                .and_then(|n| child_text(n, "ID"))
                .unwrap_or_default();
            if scheme.eq_ignore_ascii_case("VAT") || scheme.is_empty() {
                child(tax, "CompanyID").map(ident)
            } else {
                None
            }
        }),
        address: country.map(|c| core_invoice::PostalAddress {
            line1: None,
            line2: None,
            line3: None,
            city: None,
            post_code: None,
            subdivision: None,
            country: Some(c),
        }),
    }
}

fn read_delivery(node: roxmltree::Node<'_, '_>, malformed: &mut Vec<String>) -> Delivery {
    let loc = child(node, "DeliveryLocation").and_then(|n| child(n, "Address"));
    Delivery {
        name: loc
            .and_then(|a| child(a, "AddressLine"))
            .and_then(|n| child_text(n, "Line")),
        location_id: child(node, "DeliveryLocation")
            .and_then(|n| child(n, "ID"))
            .map(ident),
        date: child_date(node, "ActualDeliveryDate", malformed, "Delivery"),
        // Empty Address (no country) is still BG-15 so BR-57 can fire.
        address: loc.map(|a| core_invoice::PostalAddress {
            line1: child_text(a, "StreetName"),
            line2: child_text(a, "AdditionalStreetName"),
            line3: None,
            city: child_text(a, "CityName"),
            post_code: child_text(a, "PostalZone"),
            subdivision: child_text(a, "CountrySubentity"),
            country: child(a, "Country")
                .and_then(|n| child_text(n, "IdentificationCode"))
                .map(Code::new),
        }),
    }
}

fn ident(node: roxmltree::Node<'_, '_>) -> Identifier {
    Identifier {
        value: text(node).unwrap_or_default(),
        scheme: node.attribute("schemeID").map(str::to_owned),
        scheme_version: node.attribute("schemeVersionID").map(str::to_owned),
    }
}

fn read_payment(node: roxmltree::Node<'_, '_>) -> PaymentInstructions {
    let means_code_node = child(node, "PaymentMeansCode");
    let means_text = means_code_node
        .and_then(|n| n.attribute("name"))
        .map(str::to_owned)
        .or_else(|| child_text(node, "InstructionNote"));
    let mut accounts = Vec::new();
    for acc in children(node, "PayeeFinancialAccount") {
        accounts.push(CreditTransfer {
            account_id: Identifier::new(child_text(acc, "ID").unwrap_or_default()),
            account_name: child_text(acc, "Name"),
            provider: child(acc, "FinancialInstitutionBranch").and_then(|n| child_text(n, "ID")),
        });
    }
    let means = if let Some(card) = child(node, "CardAccount") {
        child_text(card, "PrimaryAccountNumberID").map(|pan| {
            PaymentMeans::Card(PaymentCard {
                pan,
                holder: child_text(card, "HolderName"),
            })
        })
    } else if let Some(man) = child(node, "PaymentMandate") {
        Some(PaymentMeans::DirectDebit(DirectDebit {
            mandate: child_text(man, "ID"),
            creditor_id: accounts.first().map(|a| a.account_id.clone()),
            debited_account: child(man, "PayerFinancialAccount")
                .and_then(|n| child_text(n, "ID"))
                .map(Identifier::new),
        }))
    } else if !accounts.is_empty() {
        Some(PaymentMeans::CreditTransfer(accounts))
    } else {
        None
    };
    PaymentInstructions {
        means_code: means_code_node.and_then(text).map(Code::new),
        means_text,
        remittance: child_text(node, "PaymentID"),
        means,
    }
}

fn read_note(text: String) -> InvoiceNote {
    if let Some(rest) = text.strip_prefix('#')
        && let Some((code, body)) = rest.split_once('#')
        && !code.is_empty()
    {
        return InvoiceNote {
            subject: Some(Code::new(code)),
            text: body.to_owned(),
        };
    }
    InvoiceNote {
        subject: None,
        text,
    }
}

fn read_supporting(node: roxmltree::Node<'_, '_>) -> Option<SupportingDocument> {
    let id = DocumentReference::new(child_text(node, "ID").unwrap_or_default());
    let att_n = child(node, "Attachment");
    let attachment = att_n
        .and_then(|n| child(n, "EmbeddedDocumentBinaryObject"))
        .map(|n| {
            let mime = n.attribute("mimeCode").unwrap_or("").to_owned();
            let filename = n.attribute("filename").unwrap_or("").to_owned();
            let bytes = text(n).and_then(|t| b64_decode(&t)).unwrap_or_default();
            Attachment {
                bytes,
                mime,
                filename,
            }
        });
    let uri = att_n
        .and_then(|n| child(n, "ExternalReference"))
        .and_then(|n| child_text(n, "URI"));
    Some(SupportingDocument {
        id,
        description: child_text(node, "DocumentDescription"),
        uri,
        attachment,
    })
}

fn read_allowance(
    node: roxmltree::Node<'_, '_>,
    profile: Profile,
    malformed: &mut Vec<String>,
) -> Option<core_invoice::AllowanceCharge> {
    // Amount may be absent on CEN unit-test fragments; keep the row so BT-95/102 still evaluate.
    let amount = child_amount(node, "Amount", malformed, "AllowanceCharge").unwrap_or(Amount::ZERO);
    let tax = child(node, "TaxCategory").map(|n| read_tax_cat(n, profile));
    Some(core_invoice::AllowanceCharge {
        amount,
        base: None,
        percent: None,
        reason: child_text(node, "AllowanceChargeReason"),
        reason_code: child_text(node, "AllowanceChargeReasonCode").map(Code::new),
        tax,
    })
}

fn read_subtotal(
    node: roxmltree::Node<'_, '_>,
    profile: Profile,
    malformed: &mut Vec<String>,
) -> Option<TaxBreakdown> {
    // Empty TaxSubtotal (no TaxCategory / no ID) is still BG-23 so BR-47 / BR-CO-18 fire.
    let cat = child(node, "TaxCategory");
    let tax = cat
        .map(|n| read_tax_cat(n, profile))
        .unwrap_or(TaxCategory {
            system: TaxSystem::Vat,
            code: String::new(),
            percent: None,
        });
    Some(TaxBreakdown {
        system: tax.system,
        scheme: wire_scheme(profile, tax.system, &tax.code).to_owned(),
        category: Code::new(tax.code),
        rate: tax.percent,
        taxable: child_amount(node, "TaxableAmount", malformed, "TaxSubtotal")
            .unwrap_or(Amount::ZERO),
        tax: child_amount(node, "TaxAmount", malformed, "TaxSubtotal").unwrap_or(Amount::ZERO),
        exemption_reason: cat.and_then(|c| child_text(c, "TaxExemptionReason")),
        exemption_code: cat
            .and_then(|c| child_text(c, "TaxExemptionReasonCode"))
            .map(Code::new),
    })
}

fn read_totals(node: roxmltree::Node<'_, '_>, malformed: &mut Vec<String>) -> DocumentTotals {
    DocumentTotals {
        line_net: child_amount(node, "LineExtensionAmount", malformed, "LegalMonetaryTotal"),
        allowance_total: child_amount(
            node,
            "AllowanceTotalAmount",
            malformed,
            "LegalMonetaryTotal",
        ),
        charge_total: child_amount(node, "ChargeTotalAmount", malformed, "LegalMonetaryTotal"),
        without_tax: child_amount(node, "TaxExclusiveAmount", malformed, "LegalMonetaryTotal"),
        tax_total: None,
        tax_total_accounting: None,
        with_tax: child_amount(node, "TaxInclusiveAmount", malformed, "LegalMonetaryTotal"),
        paid: child_amount(node, "PrepaidAmount", malformed, "LegalMonetaryTotal"),
        rounding: child_amount(
            node,
            "PayableRoundingAmount",
            malformed,
            "LegalMonetaryTotal",
        ),
        payable: child_amount(node, "PayableAmount", malformed, "LegalMonetaryTotal"),
    }
}

fn read_line(
    node: roxmltree::Node<'_, '_>,
    profile: Profile,
    kind: DocumentKind,
    malformed: &mut Vec<String>,
) -> Option<Line> {
    let id = child_text(node, "ID").unwrap_or_default();
    let item = child(node, "Item");
    let name = item.and_then(|n| child_text(n, "Name")).unwrap_or_default();
    let net =
        child_amount(node, "LineExtensionAmount", malformed, "InvoiceLine").unwrap_or(Amount::ZERO);
    // Missing BT-151 is a finding (BR-CO-04 / line tax presence), not category S.
    let tax = item
        .and_then(|n| child(n, "ClassifiedTaxCategory"))
        .map(|n| read_tax_cat(n, profile))
        .unwrap_or_else(|| TaxCategory {
            system: TaxSystem::Vat,
            code: String::new(),
            percent: None,
        });
    let qty_tag = if kind == DocumentKind::CreditNote {
        "CreditedQuantity"
    } else {
        "InvoicedQuantity"
    };
    let qty_node = child(node, qty_tag);
    let quantity = qty_node
        .and_then(text)
        .and_then(|t| Quantity::parse(&t).ok());
    let unit = qty_node
        .and_then(|n| n.attribute("unitCode"))
        .map(Code::new);
    let price = child(node, "Price").map(|p| {
        let net = child_text(p, "PriceAmount")
            .and_then(|t| UnitPriceAmount::parse(&t).ok())
            .unwrap_or(UnitPriceAmount::ZERO);
        // Price/AllowanceCharge is an allowance; CEN fragments omit ChargeIndicator.
        let ac = children(p, "AllowanceCharge").find(|n| charge_indicator(*n) != Some(true));
        core_invoice::Price {
            net,
            // BT-147 is UnitPriceAmount (uncapped decimals), not InvoiceAmount.
            discount: ac
                .and_then(|n| child_text(n, "Amount"))
                .and_then(|t| UnitPriceAmount::parse(&t).ok()),
            // BT-148 BaseAmount is the gross unit price.
            gross: ac
                .and_then(|n| child_text(n, "BaseAmount"))
                .and_then(|t| UnitPriceAmount::parse(&t).ok()),
            base_qty: child_text(p, "BaseQuantity").and_then(|t| Quantity::parse(&t).ok()),
            base_unit: child(p, "BaseQuantity")
                .and_then(|n| n.attribute("unitCode"))
                .map(Code::new),
        }
    });
    let period = child(node, "InvoicePeriod").map(|p| Period {
        start: child_date(p, "StartDate", malformed, "InvoiceLine/InvoicePeriod"),
        end: child_date(p, "EndDate", malformed, "InvoiceLine/InvoicePeriod"),
    });
    let allowances = children(node, "AllowanceCharge")
        .filter(|n| charge_indicator(*n) == Some(false))
        .filter_map(|n| read_line_ac(n, malformed))
        .collect();
    let charges = children(node, "AllowanceCharge")
        .filter(|n| charge_indicator(*n) == Some(true))
        .filter_map(|n| read_line_ac(n, malformed))
        .collect();
    Some(Line {
        id,
        name,
        net,
        tax,
        quantity,
        unit,
        price,
        note: child_text(node, "Note"),
        description: item.and_then(|n| child_text(n, "Description")),
        period,
        allowances,
        charges,
        // BT-133; not Invoice.buyer_accounting (BT-19).
        accounting_reference: child_text(node, "AccountingCost"),
        // BT-132 OrderLineReference/LineID.
        order_line: child(node, "OrderLineReference").and_then(|n| child_text(n, "LineID")),
        standard_id: item
            .and_then(|n| child(n, "StandardItemIdentification"))
            .and_then(|n| child(n, "ID"))
            .map(ident),
        item_id: item
            .and_then(|n| child(n, "SellersItemIdentification"))
            .and_then(|n| child(n, "ID"))
            .map(ident),
        buyer_id: item
            .and_then(|n| child(n, "BuyersItemIdentification"))
            .and_then(|n| child(n, "ID"))
            .map(ident),
        attributes: item
            .map(|n| {
                children(n, "AdditionalItemProperty")
                    .map(|p| core_invoice::ItemAttribute {
                        name: child_text(p, "Name").unwrap_or_default(),
                        value: child_text(p, "Value").unwrap_or_default(),
                    })
                    .collect()
            })
            .unwrap_or_default(),
        origin_country: item
            .and_then(|n| child(n, "OriginCountry"))
            .and_then(|n| child_text(n, "IdentificationCode"))
            .map(Code::new),
        classifications: item
            .map(|n| {
                children(n, "CommodityClassification")
                    .filter_map(|c| {
                        let code = child(c, "ItemClassificationCode")?;
                        Some(Identifier {
                            value: text(code).unwrap_or_default(),
                            scheme: code.attribute("listID").map(str::to_owned),
                            scheme_version: None,
                        })
                    })
                    .collect()
            })
            .unwrap_or_default(),
        invoiced_object: child(node, "DocumentReference").and_then(|n| {
            // BT-128 is DocumentTypeCode 130. Other type codes are not invoiced objects (BR-CL-07).
            if child_text(n, "DocumentTypeCode").as_deref() == Some("130") {
                child(n, "ID").map(ident)
            } else {
                None
            }
        }),
        invoiced_object_code: child(node, "DocumentReference")
            .and_then(|n| child_text(n, "DocumentTypeCode"))
            .map(Code::new),
        extra_tax: item
            .map(|n| {
                children(n, "ClassifiedTaxCategory")
                    .skip(1)
                    .map(|c| read_tax_cat(c, profile))
                    .collect()
            })
            .unwrap_or_default(),
        tax_total: child(node, "TaxTotal")
            .and_then(|n| child_amount(n, "TaxAmount", malformed, "InvoiceLine/TaxTotal")),
    })
}

fn read_line_ac(
    node: roxmltree::Node<'_, '_>,
    malformed: &mut Vec<String>,
) -> Option<LineAllowanceCharge> {
    Some(LineAllowanceCharge {
        amount: child_amount(node, "Amount", malformed, "LineAllowanceCharge")
            .unwrap_or(Amount::ZERO),
        base: None,
        percent: None,
        reason: child_text(node, "AllowanceChargeReason"),
        reason_code: child_text(node, "AllowanceChargeReasonCode").map(Code::new),
    })
}

fn read_tax_cat(node: roxmltree::Node<'_, '_>, profile: Profile) -> TaxCategory {
    let code = child_text(node, "ID").unwrap_or_default();
    let percent = child_text(node, "Percent")
        .and_then(|s| Decimal::from_str(&s).ok())
        .map(Percentage::new);
    let scheme = child(node, "TaxScheme")
        .and_then(|n| child_text(n, "ID"))
        .unwrap_or_default();
    let system = system_from_wire(profile, &scheme, &code);
    TaxCategory {
        system,
        code,
        percent,
    }
}

fn system_from_wire(profile: Profile, scheme: &str, category: &str) -> TaxSystem {
    if scheme.eq_ignore_ascii_case("GST") {
        return TaxSystem::Gst;
    }
    if scheme.eq_ignore_ascii_case("AAL") || category.eq_ignore_ascii_case("TTX") {
        return TaxSystem::Sst;
    }
    if profile == Profile::PintMy && core_invoice_pint_my(category) {
        return TaxSystem::Sst;
    }
    TaxSystem::parse(scheme).unwrap_or(TaxSystem::Vat)
}

fn core_invoice_pint_my(category: &str) -> bool {
    core_invoice::pint_my_category(category)
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

fn child_date(
    node: roxmltree::Node<'_, '_>,
    name: &str,
    malformed: &mut Vec<String>,
    path: &str,
) -> Option<Date> {
    let s = child_text(node, name)?;
    match Date::parse(&s) {
        Ok(d) => Some(d),
        Err(_) => {
            malformed.push(format!("{path}/{name}: {s}"));
            None
        }
    }
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

fn open(s: &mut String, indent: usize, tag: &str) {
    s.push_str(&"  ".repeat(indent));
    s.push_str("<cac:");
    s.push_str(tag);
    s.push_str(">\n");
}

fn close(s: &mut String, indent: usize, tag: &str) {
    s.push_str(&"  ".repeat(indent));
    s.push_str("</cac:");
    s.push_str(tag);
    s.push_str(">\n");
}

fn leaf_attrs(s: &mut String, indent: usize, tag: &str, value: &str, attrs: &[(&str, &str)]) {
    s.push_str(&"  ".repeat(indent));
    s.push_str("<cbc:");
    s.push_str(tag);
    for (k, v) in attrs {
        s.push(' ');
        s.push_str(k);
        s.push_str("=\"");
        s.push_str(&escape(v));
        s.push('"');
    }
    s.push('>');
    s.push_str(&escape(value));
    s.push_str("</cbc:");
    s.push_str(tag);
    s.push_str(">\n");
}

fn b64_encode(bytes: &[u8]) -> String {
    const T: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    let mut i = 0;
    while i < bytes.len() {
        let b0 = bytes[i];
        let b1 = if i + 1 < bytes.len() { bytes[i + 1] } else { 0 };
        let b2 = if i + 2 < bytes.len() { bytes[i + 2] } else { 0 };
        out.push(T[(b0 >> 2) as usize] as char);
        out.push(T[(((b0 & 3) << 4) | (b1 >> 4)) as usize] as char);
        if i + 1 < bytes.len() {
            out.push(T[(((b1 & 15) << 2) | (b2 >> 6)) as usize] as char);
        } else {
            out.push('=');
        }
        if i + 2 < bytes.len() {
            out.push(T[(b2 & 63) as usize] as char);
        } else {
            out.push('=');
        }
        i += 3;
    }
    out
}

fn b64_decode(s: &str) -> Option<Vec<u8>> {
    fn val(c: u8) -> Option<u8> {
        match c {
            b'A'..=b'Z' => Some(c - b'A'),
            b'a'..=b'z' => Some(c - b'a' + 26),
            b'0'..=b'9' => Some(c - b'0' + 52),
            b'+' => Some(62),
            b'/' => Some(63),
            _ => None,
        }
    }
    let bytes: Vec<u8> = s.bytes().filter(|c| !c.is_ascii_whitespace()).collect();
    if !bytes.len().is_multiple_of(4) {
        return None;
    }
    let mut out = Vec::new();
    for chunk in bytes.chunks(4) {
        let a = val(chunk[0])?;
        let b = val(chunk[1])?;
        out.push((a << 2) | (b >> 4));
        if chunk[2] != b'=' {
            let c = val(chunk[2])?;
            out.push((b << 4) | (c >> 2));
            if chunk[3] != b'=' {
                let d = val(chunk[3])?;
                out.push((c << 6) | d);
            }
        }
    }
    Some(out)
}

fn leaf(s: &mut String, indent: usize, tag: &str, value: &str, attr: Option<(&str, &str)>) {
    s.push_str(&"  ".repeat(indent));
    s.push_str("<cbc:");
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
    s.push_str("</cbc:");
    s.push_str(tag);
    s.push_str(">\n");
}

fn amount(s: &mut String, indent: usize, tag: &str, value: InvoiceAmount, cur: &str) {
    s.push_str(&"  ".repeat(indent));
    s.push_str("<cbc:");
    s.push_str(tag);
    if !cur.is_empty() {
        s.push_str(" currencyID=\"");
        s.push_str(&escape(cur));
        s.push('"');
    }
    s.push('>');
    s.push_str(&value.to_string());
    s.push_str("</cbc:");
    s.push_str(tag);
    s.push_str(">\n");
}

fn amount_unit(s: &mut String, indent: usize, tag: &str, value: UnitPriceAmount, cur: &str) {
    s.push_str(&"  ".repeat(indent));
    s.push_str("<cbc:");
    s.push_str(tag);
    if !cur.is_empty() {
        s.push_str(" currencyID=\"");
        s.push_str(&escape(cur));
        s.push('"');
    }
    s.push('>');
    s.push_str(&value.to_string());
    s.push_str("</cbc:");
    s.push_str(tag);
    s.push_str(">\n");
}

#[cfg(test)]
mod tests {
    use super::*;
    use core_invoice::{
        Delivery, Identifier, ItemAttribute, Line, Party, Payee, PostalAddress, TaxCategory,
        TaxRepresentative, reconcile,
    };
    use rust_decimal::Decimal;

    fn sample() -> Invoice {
        let mut inv = Invoice::blank(
            Profile::PeppolBis3,
            "EU-1",
            "EUR",
            {
                let mut p = Party::new("Seller GmbH", "DE");
                p.vat_identifier = Some(Identifier::new("DE123456789"));
                p.electronic_address = Some(Identifier::schemed("1234567890128", "0088"));
                p
            },
            {
                let mut b = Party::new("Buyer SARL", "FR");
                b.vat_identifier = Some(Identifier::new("FR12345678901"));
                b
            },
        );
        inv.issue_date = Date::parse("2026-01-15").ok();
        inv.type_code = Some(Code::new("380"));
        inv.lines = vec![Line::new(
            "1",
            "Service",
            Amount::parse("100.00").unwrap(),
            TaxCategory::vat("S", Decimal::from(19)),
        )];
        reconcile(&mut inv).unwrap();
        inv
    }

    #[test]
    fn round_trip_keeps_model_fields() {
        let inv = sample();
        let xml = write_unchecked(&inv);
        assert!(xml.contains("IssueDate"));
        assert!(xml.contains("InvoiceTypeCode"));
        assert!(xml.contains("EndpointID"));
        assert!(xml.contains("TaxSubtotal"));
        assert!(xml.contains("LineExtensionAmount"));
        assert!(!xml.contains(">SST<"));
        let back = read(&xml).unwrap().invoice;
        assert_eq!(back.number, inv.number);
        assert_eq!(back.currency, "EUR");
        assert_eq!(back.kind, DocumentKind::Invoice);
        assert_eq!(back.lines[0].id, "1");
        assert_eq!(back.lines[0].name, "Service");
        assert_eq!(back.lines[0].tax.code, "S");
        assert_eq!(back.seller.name, "Seller GmbH");
        assert_eq!(back.issue_date, inv.issue_date);
        assert_eq!(back.type_code.as_ref().map(Code::as_str), Some("380"));
        assert!(back.totals.is_some());
    }

    #[test]
    fn round_trip_keeps_line_bt132_133_156_and_bg32() {
        let mut inv = sample();
        inv.lines[0].order_line = Some("PO-LINE-9".into());
        inv.lines[0].accounting_reference = Some("ACC-441".into());
        inv.lines[0].buyer_id = Some(Identifier::new("BUY-SKU"));
        inv.lines[0].attributes.push(ItemAttribute {
            name: "Colour".into(),
            value: "blue".into(),
        });
        let xml = write_unchecked(&inv);
        assert!(xml.contains("OrderLineReference"), "{xml}");
        assert!(xml.contains("PO-LINE-9"), "{xml}");
        assert!(xml.contains("AccountingCost"), "{xml}");
        assert!(xml.contains("BuyersItemIdentification"), "{xml}");
        assert!(xml.contains("AdditionalItemProperty"), "{xml}");
        let back = read(&xml).unwrap().invoice;
        assert_eq!(back.lines[0].order_line.as_deref(), Some("PO-LINE-9"));
        assert_eq!(
            back.lines[0].accounting_reference.as_deref(),
            Some("ACC-441")
        );
        assert_eq!(
            back.lines[0].buyer_id.as_ref().map(|i| i.value.as_str()),
            Some("BUY-SKU")
        );
        assert_eq!(back.lines[0].attributes[0].name, "Colour");
        assert_eq!(back.lines[0].attributes[0].value, "blue");
    }

    #[test]
    fn round_trip_payee_tax_rep_delivery() {
        let mut inv = sample();
        inv.payee = Some(Payee {
            name: "Payee AG".into(),
            identifier: None,
            legal_registration: None,
        });
        inv.tax_representative = Some(TaxRepresentative {
            name: "TaxRep GmbH".into(),
            vat_identifier: Some(Identifier::new("DE123")),
            address: Some(PostalAddress {
                line1: None,
                line2: None,
                line3: None,
                city: None,
                post_code: None,
                subdivision: None,
                country: Some(Code::new("DE")),
            }),
        });
        inv.delivery = Some(Delivery {
            name: None,
            location_id: None,
            date: Date::parse("2026-01-20").ok(),
            address: Some(PostalAddress {
                line1: None,
                line2: None,
                line3: None,
                city: None,
                post_code: None,
                subdivision: None,
                country: Some(Code::new("DE")),
            }),
        });
        let xml = write_unchecked(&inv);
        assert!(
            !xml.contains("TaxRepresentativeParty><cac:Party>"),
            "PartyType children sit directly under TaxRepresentativeParty: {xml}"
        );
        let back = read(&xml).unwrap().invoice;
        assert_eq!(
            back.payee.as_ref().map(|p| p.name.as_str()),
            Some("Payee AG")
        );
        assert_eq!(
            back.tax_representative.as_ref().map(|t| t.name.as_str()),
            Some("TaxRep GmbH")
        );
        assert_eq!(
            back.delivery.as_ref().and_then(|d| d.date),
            inv.delivery.as_ref().and_then(|d| d.date)
        );
    }

    #[test]
    fn invoice_due_date_comes_before_type_code() {
        let mut inv = sample();
        inv.due_date = Date::parse("2026-02-01").ok();
        inv.notes.push(InvoiceNote {
            subject: Some(Code::new("AAA")),
            text: "hello".into(),
        });
        let xml = write_unchecked(&inv);
        let due = xml.find("DueDate").unwrap();
        let ty = xml.find("InvoiceTypeCode").unwrap();
        let note = xml.find("<cbc:Note>").unwrap();
        assert!(due < ty && ty < note, "{xml}");
        let back = read(&xml).unwrap().invoice;
        assert_eq!(back.due_date, inv.due_date);
        assert_eq!(
            back.notes[0].subject.as_ref().map(Code::as_str),
            Some("AAA")
        );
        assert_eq!(back.notes[0].text, "hello");
    }

    #[test]
    fn credit_note_duedate_is_stored_not_written() {
        let mut inv = sample();
        inv.kind = core_invoice::DocumentKind::CreditNote;
        inv.type_code = Some(Code::new("381"));
        inv.due_date = Date::parse("2026-02-01").ok();
        let xml = write_unchecked(&inv);
        assert!(
            !xml.contains("DueDate"),
            "UBL CreditNote writer omits DueDate"
        );
        let xml = xml.replacen(
            "<cbc:CreditNoteTypeCode>",
            "<cbc:DueDate>2026-02-01</cbc:DueDate><cbc:CreditNoteTypeCode>",
            1,
        );
        let traced = read(&xml).unwrap();
        assert!(
            traced.malformed.iter().all(|m| !m.contains("DueDate")),
            "{:?}",
            traced.malformed
        );
        assert_eq!(traced.invoice.due_date, Date::parse("2026-02-01").ok());
    }

    #[test]
    fn payment_credit_transfer_round_trips_iban() {
        let mut inv = sample();
        inv.payment = Some(PaymentInstructions {
            means_code: Some(Code::new("30")),
            means_text: Some("credit transfer".into()),
            remittance: Some("RF12".into()),
            means: Some(PaymentMeans::CreditTransfer(vec![CreditTransfer {
                account_id: Identifier::new("DE89370400440532013000"),
                account_name: Some("Seller GmbH".into()),
                provider: Some("COBADEFFXXX".into()),
            }])),
        });
        let xml = write_unchecked(&inv);
        assert!(xml.contains("name=\"credit transfer\""));
        assert!(!xml.contains("InstructionNote"));
        let back = read(&xml).unwrap().invoice;
        match back.payment.unwrap().means.unwrap() {
            PaymentMeans::CreditTransfer(a) => {
                assert_eq!(a[0].account_id.value, "DE89370400440532013000");
                assert_eq!(a[0].provider.as_deref(), Some("COBADEFFXXX"));
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn dual_taxtotal_does_not_swap_bt110_bt111() {
        let mut inv = sample();
        inv.tax_currency = Some(Code::new("USD"));
        inv.totals.as_mut().unwrap().tax_total_accounting = Some(Amount::parse("20.00").unwrap());
        let xml = write_unchecked(&inv);
        assert!(xml.matches("<cac:TaxTotal>").count() >= 2);
        let back = read(&xml).unwrap().invoice;
        assert_eq!(back.tax_currency.as_ref().map(Code::as_str), Some("USD"));
        assert_eq!(
            back.totals.unwrap().tax_total_accounting,
            Some(Amount::parse("20.00").unwrap())
        );
    }

    #[test]
    fn attachment_round_trips() {
        let mut inv = sample();
        inv.supporting_documents.push(SupportingDocument {
            id: DocumentReference::new("ATT-1"),
            description: Some("terms".into()),
            uri: None,
            attachment: Some(
                Attachment::new(b"%PDF-demo".to_vec(), "application/pdf", "terms.pdf").unwrap(),
            ),
        });
        let xml = write_unchecked(&inv);
        assert!(xml.contains("EmbeddedDocumentBinaryObject"));
        let back = read(&xml).unwrap().invoice;
        let att = back.supporting_documents[0].attachment.as_ref().unwrap();
        assert_eq!(att.filename, "terms.pdf");
        assert_eq!(att.bytes, b"%PDF-demo");
    }

    #[test]
    fn missing_currency_is_not_invented_eur() {
        let xml = r#"<Invoice xmlns="urn:oasis:names:specification:ubl:schema:xsd:Invoice-2" xmlns:cbc="urn:oasis:names:specification:ubl:schema:xsd:CommonBasicComponents-2"><cbc:ID>1</cbc:ID></Invoice>"#;
        let inv = read(xml).unwrap().invoice;
        assert!(inv.currency.is_empty());
        let xml = write_unchecked(&inv);
        assert!(!xml.contains("currencyID=\"EUR\""));
        assert!(!xml.contains("DocumentCurrencyCode"));
    }

    #[test]
    fn missing_country_is_not_invented_xx() {
        let xml = r#"<Invoice xmlns="urn:oasis:names:specification:ubl:schema:xsd:Invoice-2" xmlns:cbc="urn:oasis:names:specification:ubl:schema:xsd:CommonBasicComponents-2" xmlns:cac="urn:oasis:names:specification:ubl:schema:xsd:CommonAggregateComponents-2"><cbc:ID>1</cbc:ID><cac:AccountingSupplierParty><cac:Party><cac:PartyName><cbc:Name>A</cbc:Name></cac:PartyName></cac:Party></cac:AccountingSupplierParty></Invoice>"#;
        let inv = read(xml).unwrap().invoice;
        assert!(inv.seller.country().is_empty());
        assert_ne!(inv.seller.country(), "XX");
        let report = core_invoice::validate(&inv);
        assert!(report.findings.iter().any(|f| f.id == "BR-09"), "{report}");
    }

    #[test]
    fn dtd_is_refused() {
        let xml = r#"<!DOCTYPE Invoice [<!ENTITY x "a">]><Invoice/>"#;
        assert!(read(xml).is_err());
    }

    #[test]
    fn credit_note_root() {
        let mut inv = sample();
        inv.kind = DocumentKind::CreditNote;
        inv.type_code = Some(Code::new("381"));
        let xml = write_unchecked(&inv);
        assert!(xml.contains("<CreditNote "));
        assert!(xml.contains("CreditNoteTypeCode"));
        assert!(xml.contains("CreditNoteLine"));
        let back = read(&xml).unwrap().invoice;
        assert_eq!(back.kind, DocumentKind::CreditNote);
    }

    #[test]
    fn pint_my_never_emits_taxscheme_sst() {
        let mut inv = Invoice::blank(
            Profile::PintMy,
            "MY-1",
            "MYR",
            {
                let mut p = Party::new("Kedai", "MY");
                p.tax_registration = Some(Identifier::new("C12345678901"));
                p.legal_registration = Some(Identifier::new("2023010000001"));
                p.electronic_address = Some(Identifier::schemed("C12345678901", "0230"));
                p
            },
            {
                let mut b = Party::new("Pembeli", "MY");
                b.legal_registration = Some(Identifier::new("1999010000001"));
                b
            },
        );
        inv.issue_date = Date::parse("2026-01-15").ok();
        inv.type_code = Some(Code::new("380"));
        inv.lines = vec![Line::new(
            "1",
            "W",
            Amount::parse("100.00").unwrap(),
            TaxCategory::sst("SA", Decimal::from(10)),
        )];
        reconcile(&mut inv).unwrap();
        let xml = write_unchecked(&inv);
        assert!(!xml.contains(">SST<"), "{xml}");
        assert!(xml.contains("schemeID=\"GST\""));
        assert!(
            xml.contains("schemeID=\"GST\"") && xml.contains(">VAT<"),
            "{xml}"
        );
        assert!(xml.contains("schemeID=\"0230\""));
        let back = read(&xml).unwrap().invoice;
        assert_eq!(
            back.seller
                .tax_registration
                .as_ref()
                .map(|i| i.value.as_str()),
            Some("C12345678901")
        );
        assert_eq!(back.lines[0].tax.code, "SA");
        assert_eq!(back.lines[0].tax.system, TaxSystem::Sst);
    }

    #[test]
    fn sniff_skips_comment() {
        let xml = "<!-- Invoice in a comment --><CreditNote xmlns='urn:oasis:names:specification:ubl:schema:xsd:CreditNote-2'/>";
        assert_eq!(sniff(xml).unwrap(), DocumentKind::CreditNote);
    }

    #[test]
    fn legal_monetary_total_follows_xsd_sequence() {
        let mut inv = sample();
        let t = inv.totals.as_mut().unwrap();
        t.allowance_total = Some(Amount::parse("1.00").unwrap());
        t.charge_total = Some(Amount::parse("2.00").unwrap());
        t.paid = Some(Amount::parse("3.00").unwrap());
        t.rounding = Some(Amount::parse("0.01").unwrap());
        let xml = write_unchecked(&inv);
        let line = xml.find("LineExtensionAmount").unwrap();
        let excl = xml.find("TaxExclusiveAmount").unwrap();
        let incl = xml.find("TaxInclusiveAmount").unwrap();
        let allw = xml.find("AllowanceTotalAmount").unwrap();
        let chg = xml.find("ChargeTotalAmount").unwrap();
        let paid = xml.find("PrepaidAmount").unwrap();
        let round = xml.find("PayableRoundingAmount").unwrap();
        let pay = xml.find("PayableAmount").unwrap();
        assert!(
            line < excl
                && excl < incl
                && incl < allw
                && allw < chg
                && chg < paid
                && paid < round
                && round < pay,
            "{xml}"
        );
        assert!(!xml.contains("AllowanceTotalAmount currencyID=\"EUR\">0<"));
    }

    #[test]
    fn price_discount_gross_round_trip() {
        let mut inv = sample();
        inv.lines[0].price = Some(core_invoice::Price {
            net: UnitPriceAmount::parse("9.00").unwrap(),
            discount: Some(UnitPriceAmount::parse("1.005").unwrap()),
            gross: Some(UnitPriceAmount::parse("10.005").unwrap()),
            base_qty: None,
            base_unit: None,
        });
        let xml = write_unchecked(&inv);
        assert!(xml.contains("<cbc:ChargeIndicator>false</cbc:ChargeIndicator>"));
        let back = read(&xml).unwrap().invoice;
        let p = back.lines[0].price.as_ref().unwrap();
        assert_eq!(p.discount.as_ref().unwrap().to_string(), "1.005");
        assert_eq!(p.gross.as_ref().unwrap().to_string(), "10.005");
    }

    #[test]
    fn malformed_issue_date_is_not_silent() {
        let xml = r#"<?xml version="1.0"?><Invoice xmlns="urn:oasis:names:specification:ubl:schema:xsd:Invoice-2" xmlns:cbc="urn:oasis:names:specification:ubl:schema:xsd:CommonBasicComponents-2"><cbc:ID>1</cbc:ID><cbc:IssueDate>2026-01-15Z</cbc:IssueDate></Invoice>"#;
        let traced = read(xml).unwrap();
        assert!(
            traced.malformed.iter().any(|m| m.contains("IssueDate")),
            "{:?}",
            traced.malformed
        );
        assert!(traced.invoice.issue_date.is_none());
        let xml = r#"<?xml version="1.0"?><Invoice xmlns="urn:oasis:names:specification:ubl:schema:xsd:Invoice-2" xmlns:cbc="urn:oasis:names:specification:ubl:schema:xsd:CommonBasicComponents-2"><cbc:ID>1</cbc:ID><cbc:DueDate>2026-02-30</cbc:DueDate></Invoice>"#;
        let traced = read(xml).unwrap();
        assert!(
            traced.malformed.iter().any(|m| m.contains("DueDate")),
            "{:?}",
            traced.malformed
        );
        assert!(traced.invoice.due_date.is_none());
    }

    #[test]
    fn duplicate_duedate_is_unmapped() {
        let xml = r#"<?xml version="1.0"?><Invoice xmlns="urn:oasis:names:specification:ubl:schema:xsd:Invoice-2" xmlns:cbc="urn:oasis:names:specification:ubl:schema:xsd:CommonBasicComponents-2"><cbc:ID>1</cbc:ID><cbc:DueDate>2026-02-01</cbc:DueDate><cbc:DueDate>2026-02-02</cbc:DueDate></Invoice>"#;
        let traced = read(xml).unwrap();
        assert!(
            traced
                .unmapped
                .iter()
                .any(|u| u.contains("DueDate") && u.contains("duplicate")),
            "{:?}",
            traced.unmapped
        );
        assert_eq!(traced.invoice.due_date, Date::parse("2026-02-01").ok());
    }

    #[test]
    fn nested_item_unknown_is_unmapped() {
        let xml = r#"<?xml version="1.0"?><Invoice xmlns="urn:oasis:names:specification:ubl:schema:xsd:Invoice-2" xmlns:cbc="urn:oasis:names:specification:ubl:schema:xsd:CommonBasicComponents-2" xmlns:cac="urn:oasis:names:specification:ubl:schema:xsd:CommonAggregateComponents-2"><cbc:ID>1</cbc:ID><cac:InvoiceLine><cbc:ID>1</cbc:ID><cac:Item><cbc:Name>A</cbc:Name><cbc:Foo>x</cbc:Foo></cac:Item></cac:InvoiceLine></Invoice>"#;
        let traced = read(xml).unwrap();
        assert!(
            traced.unmapped.iter().any(|u| u.contains("Item/Foo")),
            "{:?}",
            traced.unmapped
        );
        assert!(
            traced.unmapped.iter().all(|u| !u.contains("Item/Name")),
            "{:?}",
            traced.unmapped
        );
    }

    #[test]
    fn credit_note_duedate_write_drop_is_named() {
        let mut inv = sample();
        inv.kind = DocumentKind::CreditNote;
        inv.type_code = Some(Code::new("381"));
        inv.due_date = Date::parse("2026-02-01").ok();
        let drops = write_drops(&inv);
        assert!(drops.iter().any(|d| d == "CreditNote/DueDate"), "{drops:?}");
        assert!(!write_unchecked(&inv).contains("DueDate"));
    }

    #[test]
    fn line_taxtotal_and_documentreference_are_mapped() {
        let xml = r#"<?xml version="1.0"?><Invoice xmlns="urn:oasis:names:specification:ubl:schema:xsd:Invoice-2" xmlns:cbc="urn:oasis:names:specification:ubl:schema:xsd:CommonBasicComponents-2" xmlns:cac="urn:oasis:names:specification:ubl:schema:xsd:CommonAggregateComponents-2"><cbc:ID>1</cbc:ID><cac:InvoiceLine><cbc:ID>1</cbc:ID><cac:DocumentReference><cbc:ID>OBJ</cbc:ID><cbc:DocumentTypeCode>130</cbc:DocumentTypeCode></cac:DocumentReference><cac:TaxTotal><cbc:TaxAmount currencyID="EUR">1.00</cbc:TaxAmount></cac:TaxTotal><cac:Item><cbc:Name>A</cbc:Name></cac:Item></cac:InvoiceLine></Invoice>"#;
        let traced = read(xml).unwrap();
        assert!(
            traced
                .unmapped
                .iter()
                .all(|u| !u.contains("InvoiceLine/TaxTotal")
                    && !u.contains("InvoiceLine/DocumentReference")),
            "{:?}",
            traced.unmapped
        );
        assert!(traced.invoice.lines[0].tax_total.is_some());
        assert!(traced.invoice.lines[0].invoiced_object.is_some());
    }
}
