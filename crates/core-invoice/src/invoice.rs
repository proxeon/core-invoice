//! Semantic invoice: parties, lines, totals, and Table 2 groups.

use crate::amount::{Amount, InvoiceAmount, UnitPriceAmount};
use crate::attachment::Attachment;
use crate::code::Code;
use crate::date::Date;
use crate::identifier::{DocumentReference, Identifier};
use crate::kind::DocumentKind;
use crate::numeric::{Percentage, Quantity};
use crate::payment::PaymentMeans;
use crate::profile::Profile;
use crate::tax::{TaxCategory, TaxSystem};

/// Postal address (BG-5/8/12/15). Country is BT-40 / BT-55 / BT-69 / BT-80.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PostalAddress {
    /// BT-35 / BT-50 / BT-75 street.
    pub line1: Option<String>,
    /// BT-36 / BT-51 / BT-76 additional street.
    pub line2: Option<String>,
    /// BT-162 / BT-163 / BT-164 additional address line.
    pub line3: Option<String>,
    /// BT-37 / BT-52 / BT-77 city.
    pub city: Option<String>,
    /// BT-38 / BT-53 / BT-78 post code.
    pub post_code: Option<String>,
    /// BT-39 / BT-54 / BT-79 subdivision.
    pub subdivision: Option<String>,
    /// BT-40 / BT-55 / BT-69 / BT-80 country code.
    pub country: Option<Code>,
}

/// BG-6 / BG-9 contact.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Contact {
    /// BT-41 / BT-56 contact point.
    pub point: Option<String>,
    /// BT-42 / BT-57 telephone.
    pub phone: Option<String>,
    /// BT-43 / BT-58 email.
    pub email: Option<String>,
}

/// Extra PartyTaxScheme row (PINT-MY SST + TIN + TTx).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PartyTax {
    /// Tax identifier on this extra PartyTaxScheme row.
    pub id: Identifier,
    /// TaxScheme ID (`VAT`, `GST`, `AAL`). Never `SST`.
    pub scheme: String,
}

/// Party identifiers are four slots, not a leftover `tax_id`:
///
/// - `legal_registration` — BT-30 / BT-47 (PINT-MY BRN, unschemed)
/// - `vat_identifier` — BT-31 / BT-48
/// - `tax_registration` — BT-32 (PINT-MY TIN, scheme `GST`)
/// - `electronic_address` — BT-34 / BT-49 (PINT-MY endpoint scheme `0230`)
///
/// Country (BT-40 / BT-55) lives on [`PostalAddress::country`]. [`Party::country`]
/// reads that field.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Party {
    /// BT-27 / BT-44 name.
    pub name: String,
    /// BT-28 / BT-45 trading name.
    pub trading_name: Option<String>,
    /// BT-29 / BT-46 party identifiers.
    pub identifiers: Vec<Identifier>,
    /// BT-30 / BT-47 legal registration (PINT-MY BRN, unschemed).
    pub legal_registration: Option<Identifier>,
    /// BT-31 / BT-48 VAT identifier.
    pub vat_identifier: Option<Identifier>,
    /// BT-32 seller tax registration (PINT-MY TIN, scheme `GST`).
    pub tax_registration: Option<Identifier>,
    /// BT-34 / BT-49 electronic address (PINT-MY endpoint scheme `0230`).
    pub electronic_address: Option<Identifier>,
    /// Extra PartyTaxScheme rows (PINT-MY SST + TIN + TTX).
    pub party_taxes: Vec<PartyTax>,
    /// BT-33 additional legal information.
    pub additional_legal: Option<String>,
    /// BG-5 / BG-8 postal address. Country is BT-40 / BT-55.
    pub address: Option<PostalAddress>,
    /// BG-6 / BG-9 contact.
    pub contact: Option<Contact>,
}

impl Party {
    /// Name plus country on [`PostalAddress::country`]. Empty country leaves `address` absent.
    pub fn new(name: impl Into<String>, country: impl Into<String>) -> Self {
        let country = country.into();
        let address = if country.trim().is_empty() {
            None
        } else {
            Some(PostalAddress {
                country: Some(Code::new(country)),
                ..PostalAddress::default()
            })
        };
        Self {
            name: name.into(),
            trading_name: None,
            identifiers: Vec::new(),
            legal_registration: None,
            vat_identifier: None,
            tax_registration: None,
            electronic_address: None,
            party_taxes: Vec::new(),
            additional_legal: None,
            address,
            contact: None,
        }
    }

    /// BT-40 / BT-55 from [`PostalAddress::country`]. Empty when address or code is absent.
    pub fn country(&self) -> &str {
        self.address
            .as_ref()
            .and_then(|a| a.country.as_ref())
            .map(Code::as_str)
            .unwrap_or("")
    }
}

/// BG-29 item price. BT-146 net; BT-147 discount; BT-148 gross (unit price, not Amount.Type).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Price {
    /// BT-146 item net price.
    pub net: UnitPriceAmount,
    /// BT-147 item price discount (`Price/AllowanceCharge/Amount`).
    pub discount: Option<UnitPriceAmount>,
    /// BT-148 item gross price (`Price/AllowanceCharge/BaseAmount`).
    pub gross: Option<UnitPriceAmount>,
    /// BT-149 item price base quantity.
    pub base_qty: Option<Quantity>,
    /// BT-150 item price base quantity unit.
    pub base_unit: Option<Code>,
}

/// Line allowance/charge (BG-27/28). No tax child: Peppol/PINT inherit the line category.
/// Line A/C already sits in BT-131. Do not add them again in taxable_for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LineAllowanceCharge {
    /// BT-136 / BT-141 amount.
    pub amount: InvoiceAmount,
    /// BT-137 / BT-142 base amount.
    pub base: Option<InvoiceAmount>,
    /// BT-138 / BT-143 percentage.
    pub percent: Option<Percentage>,
    /// BT-139 / BT-144 reason.
    pub reason: Option<String>,
    /// BT-140 / BT-145 reason code.
    pub reason_code: Option<Code>,
}

/// BG-25 invoice line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Line {
    /// BT-126 line identifier, not a GTIN (BT-157).
    pub id: String,
    /// BT-153 item name.
    pub name: String,
    /// BT-131 line net amount.
    pub net: Amount,
    /// BT-151 / BT-152 classified tax category.
    pub tax: TaxCategory,
    /// BT-129 invoiced quantity.
    pub quantity: Option<Quantity>,
    /// BT-130 unit of measure.
    pub unit: Option<Code>,
    /// BG-29 item price.
    pub price: Option<Price>,
    /// BT-127 line note.
    pub note: Option<String>,
    /// BT-154 item description.
    pub description: Option<String>,
    /// BG-26 invoicing period (BT-134/BT-135). Not [`Invoice::period`] (BG-14).
    pub period: Option<Period>,
    /// BG-27 line allowances.
    pub allowances: Vec<LineAllowanceCharge>,
    /// BG-28 line charges.
    pub charges: Vec<LineAllowanceCharge>,
    /// BT-157 Item standard identifier (often GTIN). UBL `StandardItemIdentification`; scheme ICD (BR-64 / BR-CL-21). Not BT-155.
    pub standard_id: Option<Identifier>,
    /// BT-155 Seller's item identifier. UBL `SellersItemIdentification`. Not BT-156, not BT-157.
    pub item_id: Option<Identifier>,
    /// BT-156 Buyer's item identifier. UBL `BuyersItemIdentification`. Not BT-155, not BT-157.
    pub buyer_id: Option<Identifier>,
    /// BT-132 referenced purchase order line (`OrderLineReference/LineID`). Not BT-13, not BT-126.
    pub order_line: Option<String>,
    /// BT-133 invoice line buyer accounting reference (`cbc:AccountingCost`). Not header BT-19.
    pub accounting_reference: Option<String>,
    /// BG-32 item attributes (BT-160 name, BT-161 value). Not BT-158 classifications.
    pub attributes: Vec<ItemAttribute>,
    /// BT-159 item origin country (BR-CL-15), not BT-80.
    pub origin_country: Option<Code>,
    /// BG-32 classification identifiers (PINT-MY CLASS is listID `CG`, not LHDN).
    pub classifications: Vec<Identifier>,
    /// Line invoiced object (BT-128). Peppol R101: DocumentTypeCode 130 only.
    pub invoiced_object: Option<Identifier>,
    /// UBL DocumentTypeCode on the line DocumentReference. Absent means 130.
    pub invoiced_object_code: Option<Code>,
    /// Extra ClassifiedTaxCategory after BT-151 (PINT-MY TTX beside HVG/SA). Not BT-151.
    pub extra_tax: Vec<TaxCategory>,
    /// Line `TaxTotal/cbc:TaxAmount`. ALIGNED-IBRP-TTX-09-MY sums this on TTX lines. Not BT-117.
    pub tax_total: Option<InvoiceAmount>,
}

impl Line {
    /// Line id, name, net (BT-131), and tax category. Does not invent quantity or price.
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        net: Amount,
        tax: TaxCategory,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            net,
            tax,
            quantity: None,
            unit: None,
            price: None,
            note: None,
            description: None,
            period: None,
            allowances: vec![],
            charges: vec![],
            standard_id: None,
            item_id: None,
            buyer_id: None,
            order_line: None,
            accounting_reference: None,
            attributes: vec![],
            origin_country: None,
            classifications: vec![],
            invoiced_object: None,
            invoiced_object_code: None,
            extra_tax: vec![],
            tax_total: None,
        }
    }
}

/// BG-32 item attribute: BT-160 name + BT-161 value (`AdditionalItemProperty`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemAttribute {
    /// BT-160 item attribute name.
    pub name: String,
    /// BT-161 item attribute value.
    pub value: String,
}

/// BT-21 / BT-22 invoice note.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvoiceNote {
    /// BT-21 note subject code (UNCL 4451).
    pub subject: Option<Code>,
    /// BT-22 note text.
    pub text: String,
}

/// BG-3 preceding invoice reference.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrecedingInvoice {
    /// BT-25 preceding invoice reference.
    pub reference: DocumentReference,
    /// BT-26 preceding invoice issue date.
    pub issue_date: Option<Date>,
}

/// Header invoicing period BG-14 (BT-73/74) or line period BG-26 (BT-134/135).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Period {
    /// BT-73 / BT-134 start.
    pub start: Option<Date>,
    /// BT-74 / BT-135 end.
    pub end: Option<Date>,
}

/// BG-10 payee (if different from seller).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Payee {
    /// BT-59 payee name.
    pub name: String,
    /// BT-60 payee identifier.
    pub identifier: Option<Identifier>,
    /// BT-61 payee legal registration identifier.
    pub legal_registration: Option<Identifier>,
}

/// BG-11 seller tax representative.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaxRepresentative {
    /// BT-62 name.
    pub name: String,
    /// BT-63 VAT identifier.
    pub vat_identifier: Option<Identifier>,
    /// BG-12 postal address (BT-69 country).
    pub address: Option<PostalAddress>,
}

/// BG-13 delivery information.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Delivery {
    /// BT-70 deliver-to party name.
    pub name: Option<String>,
    /// BT-71 deliver-to location identifier.
    pub location_id: Option<Identifier>,
    /// BT-72 actual delivery date.
    pub date: Option<Date>,
    /// BG-15 deliver-to address.
    pub address: Option<PostalAddress>,
}

/// Payment instructions. BT-81 is `means_code`. Account/IBAN/BIC, card PAN, and
/// mandate live only on [`PaymentMeans`] (exclusive BG-17/18/19).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaymentInstructions {
    /// BT-81 Payment means type code.
    pub means_code: Option<Code>,
    /// BT-82 Payment means text (`@name` on UBL PaymentMeansCode, not InstructionNote).
    pub means_text: Option<String>,
    /// BT-83 Remittance information (UBL PaymentID).
    pub remittance: Option<String>,
    /// Exclusive BG-17 / BG-18 / BG-19. Several IBANs are several credit-transfer accounts.
    pub means: Option<PaymentMeans>,
}

/// BG-20 / BG-21 document level allowance or charge.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AllowanceCharge {
    /// BT-92 / BT-99 amount.
    pub amount: InvoiceAmount,
    /// BT-93 / BT-100 base amount.
    pub base: Option<InvoiceAmount>,
    /// BT-94 / BT-101 percentage.
    pub percent: Option<Percentage>,
    /// BT-97 / BT-104 reason.
    pub reason: Option<String>,
    /// BT-98 / BT-105 reason code.
    pub reason_code: Option<Code>,
    /// BT-95/96 or BT-102/103 tax category and rate.
    pub tax: Option<TaxCategory>,
}

/// BG-23 VAT/GST/SST breakdown row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaxBreakdown {
    /// In-memory tax system. SST is never TaxScheme `SST` on the wire.
    pub system: TaxSystem,
    /// TaxScheme/cbc:ID (`VAT`, `GST`, `AAL`). Never `SST`.
    pub scheme: String,
    /// BT-118 category code.
    pub category: Code,
    /// BT-119 rate. None for EN `O` and PINT-MY TTX.
    pub rate: Option<Percentage>,
    /// BT-116 taxable amount.
    pub taxable: InvoiceAmount,
    /// BT-117 tax amount.
    pub tax: InvoiceAmount,
    /// BT-120 exemption reason.
    pub exemption_reason: Option<String>,
    /// BT-121 exemption reason code.
    pub exemption_code: Option<Code>,
}

/// BG-22 document totals. Absent optional amounts are `None`, not 0.00.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DocumentTotals {
    /// BT-106 sum of invoice line net amounts.
    pub line_net: Option<InvoiceAmount>,
    /// BT-107 sum of allowances on document level.
    pub allowance_total: Option<InvoiceAmount>,
    /// BT-108 sum of charges on document level.
    pub charge_total: Option<InvoiceAmount>,
    /// BT-109 invoice total amount without VAT.
    pub without_tax: Option<InvoiceAmount>,
    /// BT-110 invoice total VAT amount.
    pub tax_total: Option<InvoiceAmount>,
    /// BT-111 invoice total VAT amount in accounting currency.
    pub tax_total_accounting: Option<InvoiceAmount>,
    /// BT-112 invoice total amount with VAT.
    pub with_tax: Option<InvoiceAmount>,
    /// BT-113 paid amount.
    pub paid: Option<InvoiceAmount>,
    /// BT-114 rounding amount.
    pub rounding: Option<InvoiceAmount>,
    /// BT-115 amount due for payment. Missing PayableAmount is `None`, not 0 (BR-15).
    pub payable: Option<InvoiceAmount>,
}

/// BG-24 additional supporting document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SupportingDocument {
    /// BT-122 supporting document reference.
    pub id: DocumentReference,
    /// BT-123 supporting document description.
    pub description: Option<String>,
    /// BT-124 external document location.
    pub uri: Option<String>,
    /// BT-125 attached document.
    pub attachment: Option<Attachment>,
}

/// Semantic invoice. Fields are `pub` on 2.x so embedders can [`Invoice::blank`]
/// then set terms. Do not match on the struct layout — `#[non_exhaustive]` would
/// be a 3.0 break. A proved document is [`crate::Validated`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Invoice {
    /// In-memory rule set used by [`crate::validate()`]. Not BT-24; BT-24 is [`Self::specification_id`].
    pub profile: Profile,
    /// BT-24 specification identifier.
    pub specification_id: Option<String>,
    /// Syntax root analogue. Not derived from BT-3.
    pub kind: DocumentKind,
    /// BT-1 invoice number.
    pub number: String,
    /// BT-5 invoice currency.
    pub currency: String,
    /// BT-2 issue date.
    pub issue_date: Option<Date>,
    /// BT-3 invoice type code.
    pub type_code: Option<Code>,
    /// BT-6 VAT accounting currency.
    pub tax_currency: Option<Code>,
    /// BT-9 payment due date.
    pub due_date: Option<Date>,
    /// BT-7 / BT-8. BR-CO-03 when both the date and the code rules apply.
    pub tax_point_date: Option<Date>,
    /// BT-8 VAT point date code.
    pub tax_point_code: Option<Code>,
    /// BT-23 business process type.
    pub business_process: Option<String>,
    /// BT-10 buyer reference. Do not overload with BT-13.
    pub buyer_reference: Option<DocumentReference>,
    /// BT-11 project reference.
    pub project: Option<DocumentReference>,
    /// BT-12 contract reference.
    pub contract: Option<DocumentReference>,
    /// BT-13 purchase order reference (Peppol R003 with BT-10).
    pub purchase_order: Option<DocumentReference>,
    /// BT-14 sales order reference.
    pub sales_order: Option<DocumentReference>,
    /// BT-15 receiving advice reference.
    pub receiving_advice: Option<DocumentReference>,
    /// BT-16 despatch advice reference.
    pub despatch: Option<DocumentReference>,
    /// BT-17 tender or lot reference.
    pub tender: Option<DocumentReference>,
    /// BT-18 invoiced object identifier (not a BG-24 supporting document).
    pub invoiced_object: Option<Identifier>,
    /// BT-19 buyer accounting reference.
    pub buyer_accounting: Option<String>,
    /// BT-20 payment terms.
    pub payment_terms: Option<String>,
    /// BG-1 notes (BT-21/22).
    pub notes: Vec<InvoiceNote>,
    /// BG-3 preceding invoice reference.
    pub preceding: Vec<PrecedingInvoice>,
    /// BG-4 seller.
    pub seller: Party,
    /// BG-7 buyer.
    pub buyer: Party,
    /// BG-10 payee.
    pub payee: Option<Payee>,
    /// BG-11 seller tax representative.
    pub tax_representative: Option<TaxRepresentative>,
    /// BG-13 delivery.
    pub delivery: Option<Delivery>,
    /// BG-14 invoicing period.
    pub period: Option<Period>,
    /// BG-16 payment instructions.
    pub payment: Option<PaymentInstructions>,
    /// BG-20 document level allowances.
    pub document_allowances: Vec<AllowanceCharge>,
    /// BG-21 document level charges.
    pub document_charges: Vec<AllowanceCharge>,
    /// BG-23 VAT/GST/SST breakdown.
    pub tax_breakdown: Vec<TaxBreakdown>,
    /// BG-22 document totals.
    pub totals: Option<DocumentTotals>,
    /// BG-24 additional supporting documents.
    pub supporting_documents: Vec<SupportingDocument>,
    /// BG-25 invoice lines.
    pub lines: Vec<Line>,
}

impl Invoice {
    /// 2.x constructor. Stamps BT-24 from [`Profile::specification_id`]. Then set `pub` fields.
    pub fn blank(
        profile: Profile,
        number: impl Into<String>,
        currency: impl Into<String>,
        seller: Party,
        buyer: Party,
    ) -> Self {
        Self {
            profile,
            specification_id: Some(profile.specification_id().into()),
            kind: DocumentKind::Invoice,
            number: number.into(),
            currency: currency.into(),
            issue_date: None,
            type_code: None,
            tax_currency: None,
            due_date: None,
            tax_point_date: None,
            tax_point_code: None,
            business_process: None,
            buyer_reference: None,
            project: None,
            contract: None,
            purchase_order: None,
            sales_order: None,
            receiving_advice: None,
            despatch: None,
            tender: None,
            invoiced_object: None,
            buyer_accounting: None,
            payment_terms: None,
            notes: vec![],
            preceding: vec![],
            seller,
            buyer,
            payee: None,
            tax_representative: None,
            delivery: None,
            period: None,
            payment: None,
            document_allowances: vec![],
            document_charges: vec![],
            tax_breakdown: vec![],
            totals: None,
            supporting_documents: vec![],
            lines: vec![],
        }
    }

    /// BT-115 from [`DocumentTotals`]. Absent BG-22 or absent PayableAmount is not 0.00 (BR-15).
    pub fn payable(&self) -> Option<Amount> {
        self.totals.as_ref().and_then(|t| t.payable)
    }

    /// BT-110 from [`DocumentTotals`]. Absent totals is not 0.00.
    pub fn tax_total(&self) -> Option<Amount> {
        self.totals.as_ref().and_then(|t| t.tax_total)
    }

    /// BT-24 and BT-23 come from the proved profile, not leftover fields on Invoice.
    ///
    /// EN 16931 has no required ProfileID: BT-23 is left as-is (the UBL writer omits it).
    pub fn stamp_profile(&mut self, profile: Profile) {
        self.profile = profile;
        if profile == Profile::Unknown {
            return;
        }
        self.specification_id = Some(profile.specification_id().into());
        if let Some(bt23) = profile.process_id() {
            self.business_process = Some(bt23.into());
        }
    }

    /// Sum of line BT-131. Overflow is `None`. Not BT-106 unless totals exist.
    pub fn line_net_sum(&self) -> Option<Amount> {
        self.lines
            .iter()
            .try_fold(Amount::ZERO, |acc, line| acc.checked_add(line.net))
    }

    /// Credit note: new number/date, BG-3 to the original, amounts **not** negated.
    pub fn to_credit_note(&self, new_number: impl Into<String>, new_issue_date: Date) -> Self {
        let mut next = self.clone();
        next.kind = DocumentKind::CreditNote;
        next.type_code = Some(Code::new("381"));
        next.preceding = vec![PrecedingInvoice {
            reference: DocumentReference::new(self.number.clone()),
            issue_date: self.issue_date,
        }];
        next.number = new_number.into();
        next.issue_date = Some(new_issue_date);
        next
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identifier::Identifier;
    use crate::tax::TaxCategory;
    use rust_decimal::Decimal;

    #[test]
    fn credit_note_does_not_negate() {
        let mut inv = Invoice::blank(
            Profile::En16931,
            "INV-1",
            "EUR",
            Party::new("S", "DE"),
            Party::new("B", "FR"),
        );
        inv.issue_date = Date::parse("2026-01-15").ok();
        inv.type_code = Some(Code::new("380"));
        inv.lines = vec![Line::new(
            "1",
            "A",
            Amount::parse("100.00").unwrap(),
            TaxCategory::vat("S", Decimal::from(19)),
        )];
        let _ = crate::reconcile::reconcile(&mut inv);
        let cn = inv.to_credit_note("CN-1", Date::parse("2026-01-16").unwrap());
        assert_eq!(cn.kind, DocumentKind::CreditNote);
        assert_eq!(cn.payable(), inv.payable());
        assert_eq!(cn.preceding[0].reference.as_str(), "INV-1");
        assert_eq!(cn.type_code.as_ref().map(Code::as_str), Some("381"));
    }

    #[test]
    fn table2_refs_and_line_groups_exist() {
        let mut inv = Invoice::blank(
            Profile::En16931,
            "INV-1",
            "EUR",
            Party::new("S", "DE"),
            Party::new("B", "FR"),
        );
        inv.tax_point_date = Date::parse("2026-01-10").ok();
        inv.tax_point_code = Some(Code::new("3"));
        inv.purchase_order = Some(DocumentReference::new("PO-9"));
        inv.invoiced_object = Some(Identifier::new("OBJ-1"));
        inv.lines.push({
            let mut line = Line::new(
                "1",
                "A",
                Amount::parse("90.00").unwrap(),
                TaxCategory::vat("S", Decimal::from(19)),
            );
            line.period = Some(Period {
                start: Date::parse("2026-01-01").ok(),
                end: Date::parse("2026-01-31").ok(),
            });
            line.allowances.push(LineAllowanceCharge {
                amount: Amount::parse("10.00").unwrap(),
                base: None,
                percent: None,
                reason: Some("discount".into()),
                reason_code: None,
            });
            line.standard_id = Some(Identifier::schemed("01234567890128", "0160"));
            line.origin_country = Some(Code::new("DE"));
            line
        });
        assert_eq!(inv.purchase_order.as_ref().unwrap().as_str(), "PO-9");
        assert_eq!(inv.lines[0].allowances.len(), 1);
        assert_eq!(inv.lines[0].origin_country.as_ref().unwrap().as_str(), "DE");
        assert!(inv.invoiced_object.is_some());
    }
}
