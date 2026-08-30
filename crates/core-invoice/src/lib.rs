//! Semantic model for the core electronic invoice: EN 16931 and Peppol PINT.
//!
//! Tax is VAT, GST, SST, or consumption — not VAT only. No I/O, no tax-authority
//! APIs, no accounting UI.

pub mod amount;
pub mod attachment;
pub mod bt;
pub mod code;
pub mod date;
pub mod error;
pub mod identifier;
pub mod invoice;
pub mod kind;
pub mod numeric;
pub mod payment;
pub mod profile;
pub mod report;
pub mod rules;
pub mod tax;
pub mod validate;

pub use amount::{Amount, InvoiceAmount, UnitPriceAmount};
pub use attachment::Attachment;
pub use bt::{BtId, Group, Path};
pub use code::Code;
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
        inv.tax_total = Amount::parse("10.00").unwrap();
        inv.payable = Amount::parse("110.00").unwrap();
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
    fn payable_mismatch_does_not_claim_br_co_16() {
        let mut inv = sst_invoice(Profile::Pint);
        inv.payable = Amount::parse("999.00").unwrap();
        let report = validate(&inv);
        assert!(
            report.findings.iter().all(|f| f.id != "BR-CO-16"),
            "collapsed net+tax identity must not use the CEN id BR-CO-16: {report}"
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
