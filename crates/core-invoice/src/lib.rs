//! Semantic model for the core electronic invoice: EN 16931 and Peppol PINT.
//!
//! Tax is VAT, GST, SST, or consumption — not VAT only. No I/O, no tax-authority
//! APIs, no accounting UI.

pub mod amount;
pub mod invoice;
pub mod profile;
pub mod report;
pub mod tax;
pub mod validate;

pub use amount::Amount;
pub use invoice::{Invoice, Line, Party};
pub use profile::Profile;
pub use report::{Finding, Report, Severity};
pub use tax::{TaxCategory, TaxSystem};
pub use validate::validate;

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::Decimal;

    fn sst_invoice(profile: Profile) -> Invoice {
        Invoice {
            profile,
            number: "INV-1".into(),
            currency: "MYR".into(),
            seller: {
                let mut p = Party::new("Kedai", "MY");
                p.tax_id = Some("C12345678901".into());
                p.id_scheme = Some("TIN".into());
                p
            },
            buyer: Party::new("Pembeli", "MY"),
            lines: vec![Line {
                id: "1".into(),
                name: "Goods".into(),
                net: Amount::parse("100.00").unwrap(),
                tax: TaxCategory::sst("SR", Decimal::new(10, 2)),
            }],
            tax_total: Amount::parse("10.00").unwrap(),
            payable: Amount::parse("110.00").unwrap(),
        }
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
    }

    #[test]
    fn payable_must_match_net_plus_tax() {
        let mut inv = sst_invoice(Profile::Pint);
        inv.payable = Amount::parse("999.00").unwrap();
        let report = validate(&inv);
        assert!(report.findings.iter().any(|f| f.id == "BR-CO-16"));
    }
}
