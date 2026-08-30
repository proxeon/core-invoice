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

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PostalAddress {
    pub line1: Option<String>,
    pub line2: Option<String>,
    pub line3: Option<String>,
    pub city: Option<String>,
    pub post_code: Option<String>,
    pub subdivision: Option<String>,
    pub country: Option<Code>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Contact {
    pub point: Option<String>,
    pub phone: Option<String>,
    pub email: Option<String>,
}

/// Extra PartyTaxScheme row (PINT-MY SST + TIN + TTx).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PartyTax {
    pub id: Identifier,
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
    pub name: String,
    pub trading_name: Option<String>,
    pub identifiers: Vec<Identifier>,
    pub legal_registration: Option<Identifier>,
    pub vat_identifier: Option<Identifier>,
    pub tax_registration: Option<Identifier>,
    pub electronic_address: Option<Identifier>,
    pub party_taxes: Vec<PartyTax>,
    pub additional_legal: Option<String>,
    pub address: Option<PostalAddress>,
    pub contact: Option<Contact>,
}

impl Party {
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Price {
    pub net: UnitPriceAmount,
    pub discount: Option<UnitPriceAmount>,
    pub gross: Option<UnitPriceAmount>,
    pub base_qty: Option<Quantity>,
    pub base_unit: Option<Code>,
}

/// Line allowance/charge (BG-27/28). No tax child: Peppol/PINT inherit the line category.
/// Line A/C already sits in BT-131. Do not add them again in taxable_for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LineAllowanceCharge {
    pub amount: InvoiceAmount,
    pub base: Option<InvoiceAmount>,
    pub percent: Option<Percentage>,
    pub reason: Option<String>,
    pub reason_code: Option<Code>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Line {
    /// BT-126 line identifier, not a GTIN (BT-157).
    pub id: String,
    pub name: String,
    pub net: Amount,
    pub tax: TaxCategory,
    pub quantity: Option<Quantity>,
    pub unit: Option<Code>,
    pub price: Option<Price>,
    pub note: Option<String>,
    pub description: Option<String>,
    /// BG-26 invoicing period (BT-134/BT-135). Not [`Invoice::period`] (BG-14).
    pub period: Option<Period>,
    pub allowances: Vec<LineAllowanceCharge>,
    pub charges: Vec<LineAllowanceCharge>,
    /// BT-155 standard item identification (often GTIN).
    pub standard_id: Option<Identifier>,
    /// BT-157 item identifier + BT-156/BT-158 scheme.
    pub item_id: Option<Identifier>,
    /// BT-159 item origin country (BR-CL-15), not BT-80.
    pub origin_country: Option<Code>,
    /// BG-32 classification identifiers (PINT-MY CLASS may use this).
    pub classifications: Vec<Identifier>,
}

impl Line {
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
            origin_country: None,
            classifications: vec![],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvoiceNote {
    pub subject: Option<Code>,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PrecedingInvoice {
    pub reference: DocumentReference,
    pub issue_date: Option<Date>,
}

/// Header invoicing period BG-14 (BT-73/74) or line period BG-26 (BT-134/135).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Period {
    pub start: Option<Date>,
    pub end: Option<Date>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Payee {
    pub name: String,
    pub identifier: Option<Identifier>,
    pub legal_registration: Option<Identifier>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaxRepresentative {
    pub name: String,
    pub vat_identifier: Option<Identifier>,
    pub address: Option<PostalAddress>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Delivery {
    pub name: Option<String>,
    pub location_id: Option<Identifier>,
    pub date: Option<Date>,
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
    pub means: Option<PaymentMeans>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AllowanceCharge {
    pub amount: InvoiceAmount,
    pub base: Option<InvoiceAmount>,
    pub percent: Option<Percentage>,
    pub reason: Option<String>,
    pub reason_code: Option<Code>,
    pub tax: Option<TaxCategory>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaxBreakdown {
    pub system: TaxSystem,
    pub scheme: String,
    pub category: Code,
    pub rate: Option<Percentage>,
    pub taxable: InvoiceAmount,
    pub tax: InvoiceAmount,
    pub exemption_reason: Option<String>,
    pub exemption_code: Option<Code>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentTotals {
    pub line_net: Option<InvoiceAmount>,
    pub allowance_total: Option<InvoiceAmount>,
    pub charge_total: Option<InvoiceAmount>,
    pub without_tax: Option<InvoiceAmount>,
    pub tax_total: Option<InvoiceAmount>,
    pub tax_total_accounting: Option<InvoiceAmount>,
    pub with_tax: Option<InvoiceAmount>,
    pub paid: Option<InvoiceAmount>,
    pub rounding: Option<InvoiceAmount>,
    pub payable: InvoiceAmount,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SupportingDocument {
    pub id: DocumentReference,
    pub description: Option<String>,
    pub uri: Option<String>,
    pub attachment: Option<Attachment>,
}

/// Semantic invoice. Fields stay public through 0.1–0.2; Table 2 terms will
/// still be added. Construct with [`Invoice::blank`], then set fields. Do not
/// match on the struct layout.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Invoice {
    pub profile: Profile,
    pub specification_id: Option<String>,
    pub kind: DocumentKind,
    pub number: String,
    pub currency: String,
    pub issue_date: Option<Date>,
    pub type_code: Option<Code>,
    pub tax_currency: Option<Code>,
    pub due_date: Option<Date>,
    /// BT-7 / BT-8. BR-CO-03 when both the date and the code rules apply.
    pub tax_point_date: Option<Date>,
    pub tax_point_code: Option<Code>,
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
    pub payment_terms: Option<String>,
    pub notes: Vec<InvoiceNote>,
    pub preceding: Vec<PrecedingInvoice>,
    pub seller: Party,
    pub buyer: Party,
    pub payee: Option<Payee>,
    pub tax_representative: Option<TaxRepresentative>,
    pub delivery: Option<Delivery>,
    pub period: Option<Period>,
    pub payment: Option<PaymentInstructions>,
    pub document_allowances: Vec<AllowanceCharge>,
    pub document_charges: Vec<AllowanceCharge>,
    pub tax_breakdown: Vec<TaxBreakdown>,
    pub totals: Option<DocumentTotals>,
    pub supporting_documents: Vec<SupportingDocument>,
    pub lines: Vec<Line>,
}

impl Invoice {
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

    /// BT-115 from DocumentTotals. Ghosts on Invoice are not a second identity.
    pub fn payable(&self) -> Amount {
        self.totals
            .as_ref()
            .map(|t| t.payable)
            .unwrap_or(Amount::ZERO)
    }

    /// BT-110 from DocumentTotals. Absent totals is not 0.00 for BR-CO-16.
    pub fn tax_total(&self) -> Amount {
        self.totals
            .as_ref()
            .and_then(|t| t.tax_total)
            .unwrap_or(Amount::ZERO)
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
