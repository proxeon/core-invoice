//! On-disk and in-crate sample invoices.

use core_invoice::{Amount, Invoice, Line, Party, Profile, TaxCategory};
use rust_decimal::Decimal;

pub fn pint_my_sst() -> Invoice {
    Invoice {
        profile: Profile::PintMy,
        number: "MY-2026-0001".into(),
        currency: "MYR".into(),
        seller: {
            let mut p = Party::new("Kedai Contoh Sdn Bhd", "MY");
            p.tax_id = Some("C12345678901".into());
            p.id_scheme = Some("TIN".into());
            p
        },
        buyer: Party::new("Pembeli Sdn Bhd", "MY"),
        lines: vec![Line {
            id: "1".into(),
            name: "Widget".into(),
            net: Amount::parse("100.00").unwrap(),
            tax: TaxCategory::sst("SR", Decimal::new(10, 2)),
        }],
        tax_total: Amount::parse("10.00").unwrap(),
        payable: Amount::parse("110.00").unwrap(),
    }
}

pub fn peppol_vat() -> Invoice {
    Invoice {
        profile: Profile::PeppolBis3,
        number: "EU-2026-0001".into(),
        currency: "EUR".into(),
        seller: Party::new("Seller GmbH", "DE"),
        buyer: Party::new("Buyer SARL", "FR"),
        lines: vec![Line {
            id: "1".into(),
            name: "Service".into(),
            net: Amount::parse("100.00").unwrap(),
            tax: TaxCategory::vat("S", Decimal::new(19, 2)),
        }],
        tax_total: Amount::parse("19.00").unwrap(),
        payable: Amount::parse("119.00").unwrap(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core_invoice::validate;
    use core_invoice_formats::{read, write, Syntax};

    #[test]
    fn pint_my_round_trip_ubl() {
        let xml = write(&pint_my_sst(), Syntax::Ubl).unwrap();
        let back = read(&xml).unwrap();
        assert_eq!(back.profile, Profile::PintMy);
        assert!(validate(&back).ok(), "{}", validate(&back));
    }

    #[test]
    fn peppol_vat_is_valid() {
        assert!(validate(&peppol_vat()).ok());
    }

    #[test]
    fn sst_on_peppol_is_invalid() {
        let mut inv = pint_my_sst();
        inv.profile = Profile::PeppolBis3;
        assert!(!validate(&inv).ok());
    }
}
