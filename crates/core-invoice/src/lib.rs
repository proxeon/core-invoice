//! Semantic model for the core electronic invoice: EN 16931 and Peppol PINT.
//!
//! Tax is VAT, GST, SST, or consumption — not VAT only. No I/O, no tax-authority
//! APIs, no accounting UI.

pub mod amount;
pub mod arith;
pub mod attachment;
pub mod bt;
pub mod category;
pub mod code;
pub mod codes;
pub mod date;
pub mod error;
pub mod identifier;
pub mod invoice;
pub mod kind;
pub mod numeric;
pub mod payment;
pub mod peppol;
pub mod profile;
pub mod proof;
pub mod reconcile;
pub mod report;
pub mod rules;
pub mod tax;
pub mod validate;

pub use amount::{Amount, InvoiceAmount, UnitPriceAmount};
pub use attachment::Attachment;
pub use bt::{BtId, Group, Path};
pub use category::{CategoryProfile, VatCategory, pint_gst_category};
pub use code::Code;
pub use codes::{ARTEFACT_VERSION, currency as is_currency};
pub use date::Date;
pub use error::{AmountError, AttachmentError, DateError};
pub use identifier::{DocumentReference, Identifier};
pub use invoice::{
    AllowanceCharge, Contact, Delivery, DocumentTotals, Invoice, InvoiceNote, Line, Party,
    PartyTax, Payee, PaymentInstructions, Period, PostalAddress, PrecedingInvoice, Price,
    SupportingDocument, TaxBreakdown, TaxRepresentative,
};
pub use kind::DocumentKind;
pub use numeric::{Percentage, Quantity};
pub use payment::{CreditTransfer, DirectDebit, PaymentCard, PaymentMeans};
pub use profile::{Edition, Profile, ProfileLookup};
pub use proof::{
    Check, En16931 as En16931Marker, PeppolBis3 as PeppolBis3Marker, Pint as PintMarker,
    PintMy as PintMyMarker, ProfileMarker, ProveError, Underlies, Validated,
};
pub use reconcile::{ReconcileError, Reconciled, Reconciler, reconcile};
pub use report::{Finding, Report, Severity, Source};
pub use rules::{catalogue, explain};
pub use tax::{TaxCategory, TaxSystem, pint_my_category, wire_scheme};
pub use validate::validate;

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::Decimal;

    fn sst_invoice(profile: Profile) -> Invoice {
        let mut inv = Invoice::blank(
            profile,
            "INV-1",
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
        inv.lines = vec![Line::new(
            "1",
            "Goods",
            Amount::parse("100.00").unwrap(),
            TaxCategory::sst("SA", Decimal::from(10)),
        )];
        inv.issue_date = Date::parse("2026-01-15").ok();
        inv.type_code = Some(Code::new("380"));
        let _ = reconcile(&mut inv);
        inv
    }

    #[test]
    fn pint_my_accepts_sst() {
        let report = validate(&sst_invoice(Profile::PintMy));
        assert!(report.ok(), "{report}");
    }

    #[test]
    fn peppol_bis_rejects_sst() {
        let report = validate(&sst_invoice(Profile::PeppolBis3));
        assert!(!report.ok());
        assert!(
            report.findings.iter().any(|f| f.id == "PINT-TAX"),
            "{report}"
        );
        assert!(
            report
                .findings
                .iter()
                .any(|f| f.path.to_string().starts_with("BG-25")),
            "{report}"
        );
    }

    #[test]
    fn br_co_18_without_reconcile() {
        let mut inv = Invoice::blank(
            Profile::En16931,
            "INV-1",
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
        inv.lines = vec![Line::new(
            "1",
            "A",
            Amount::parse("100.00").unwrap(),
            TaxCategory::vat("S", Decimal::from(19)),
        )];
        let report = validate(&inv);
        assert!(
            report.findings.iter().any(|f| f.id == "BR-CO-18"),
            "{report}"
        );
    }

    #[test]
    fn br_53_bt6_without_bt111() {
        let mut inv = sst_invoice(Profile::PintMy);
        inv.tax_currency = Some(Code::new("USD"));
        let report = validate(&inv);
        assert!(report.findings.iter().any(|f| f.id == "BR-53"), "{report}");
        inv.totals.as_mut().unwrap().tax_total_accounting = Some(Amount::parse("10.00").unwrap());
        let report = validate(&inv);
        assert!(report.findings.iter().all(|f| f.id != "BR-53"), "{report}");
    }

    #[test]
    fn stuffed_payable_emits_br_co_16_when_totals_exist() {
        let mut inv = sst_invoice(Profile::Pint);
        inv.totals.as_mut().unwrap().payable = Amount::parse("999.00").unwrap();
        let report = validate(&inv);
        assert!(
            report.findings.iter().any(|f| f.id == "BR-CO-16"),
            "{report}"
        );
    }

    #[test]
    fn gst_on_pint_my_is_pint_tax() {
        let mut inv = sst_invoice(Profile::PintMy);
        inv.lines[0].tax = TaxCategory::gst("SA", Decimal::from(10));
        let report = validate(&inv);
        assert!(
            report.findings.iter().any(|f| f.id == "PINT-TAX"),
            "{report}"
        );
    }

    #[test]
    fn br_05_is_presence_not_length() {
        let mut inv = sst_invoice(Profile::PintMy);
        inv.currency.clear();
        let report = validate(&inv);
        assert!(report.findings.iter().any(|f| f.id == "BR-05"));
        inv.currency = "MYR".into();
        assert!(validate(&inv).ok(), "{}", validate(&inv));
    }
}
