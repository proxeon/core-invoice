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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Party {
    pub name: String,
    pub country: String,
    pub tax_id: Option<String>,
    pub id_scheme: Option<String>,
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
        Self {
            name: name.into(),
            country: country.into(),
            tax_id: None,
            id_scheme: None,
            trading_name: None,
            identifiers: Vec::new(),
            legal_registration: None,
            vat_identifier: None,
            tax_registration: None,
            electronic_address: None,
            party_taxes: Vec::new(),
            additional_legal: None,
            address: None,
            contact: None,
        }
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Line {
    pub id: String,
    pub name: String,
    pub net: Amount,
    pub tax: TaxCategory,
    pub quantity: Option<Quantity>,
    pub unit: Option<Code>,
    pub price: Option<Price>,
    pub note: Option<String>,
    pub description: Option<String>,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaymentInstructions {
    pub means_code: Option<Code>,
    pub means_text: Option<String>,
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
    pub business_process: Option<String>,
    pub buyer_reference: Option<DocumentReference>,
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
    pub tax_total: Amount,
    pub payable: Amount,
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
            business_process: None,
            buyer_reference: None,
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
            tax_total: Amount::ZERO,
            payable: Amount::ZERO,
        }
    }

    /// BT-24 and BT-23 come from the proved profile, not leftover fields on Invoice.
    ///
    /// EN 16931 has no required ProfileID: BT-23 is left as-is (the UBL writer omits it).
    pub fn stamp_profile(&mut self, profile: Profile) {
        self.profile = profile;
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
        inv.tax_total = Amount::parse("19.00").unwrap();
        inv.payable = Amount::parse("119.00").unwrap();
        let cn = inv.to_credit_note("CN-1", Date::parse("2026-01-16").unwrap());
        assert_eq!(cn.kind, DocumentKind::CreditNote);
        assert_eq!(cn.payable, inv.payable);
        assert_eq!(cn.preceding[0].reference.as_str(), "INV-1");
        assert_eq!(cn.type_code.as_ref().map(Code::as_str), Some("381"));
    }
}
