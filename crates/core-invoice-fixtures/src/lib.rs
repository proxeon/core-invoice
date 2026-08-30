//! On-disk and in-crate sample invoices.

use core_invoice::{Amount, Identifier, Invoice, Line, Party, Profile, TaxCategory, reconcile};
use rust_decimal::Decimal;

pub fn pint_my_sst() -> Invoice {
    let mut inv = Invoice::blank(
        Profile::PintMy,
        "MY-2026-0001",
        "MYR",
        {
            let mut p = Party::new("Kedai Contoh Sdn Bhd", "MY");
            p.tax_registration = Some(Identifier::new("C12345678901"));
            p.legal_registration = Some(Identifier::new("2023010000001"));
            p.electronic_address = Some(Identifier::schemed("C12345678901", "0230"));
            p
        },
        {
            let mut b = Party::new("Pembeli Sdn Bhd", "MY");
            b.legal_registration = Some(Identifier::new("1999010000001"));
            b
        },
    );
    inv.lines = vec![Line::new(
        "1",
        "Widget",
        Amount::parse("100.00").unwrap(),
        TaxCategory::sst("SA", Decimal::from(10)),
    )];
    inv.issue_date = core_invoice::Date::parse("2026-01-15").ok();
    inv.type_code = Some(core_invoice::Code::new("380"));
    let _ = reconcile(&mut inv);
    inv
}

pub fn pint_gst_sr() -> Invoice {
    let mut inv = Invoice::blank(
        Profile::Pint,
        "SG-2026-0001",
        "SGD",
        {
            let mut p = Party::new("Seller Pte Ltd", "SG");
            p.tax_registration = Some(Identifier::new("GST123456789"));
            p
        },
        Party::new("Buyer Pte Ltd", "SG"),
    );
    inv.lines = vec![Line::new(
        "1",
        "Service",
        Amount::parse("100.00").unwrap(),
        TaxCategory::gst("SR", Decimal::from(9)),
    )];
    inv.issue_date = core_invoice::Date::parse("2026-01-15").ok();
    inv.type_code = Some(core_invoice::Code::new("380"));
    let _ = reconcile(&mut inv);
    inv
}

pub fn peppol_vat() -> Invoice {
    let mut inv = Invoice::blank(
        Profile::PeppolBis3,
        "EU-2026-0001",
        "EUR",
        {
            let mut p = Party::new("Seller GmbH", "DE");
            p.vat_identifier = Some(Identifier::new("DE123456789"));
            p
        },
        {
            let mut b = Party::new("Buyer SARL", "FR");
            b.vat_identifier = Some(Identifier::new("FR12345678901"));
            b
        },
    );
    inv.lines = vec![Line::new(
        "1",
        "Service",
        Amount::parse("100.00").unwrap(),
        TaxCategory::vat("S", Decimal::from(19)),
    )];
    inv.issue_date = core_invoice::Date::parse("2026-01-15").ok();
    inv.type_code = Some(core_invoice::Code::new("380"));
    inv.business_process = Some("urn:fdc:peppol.eu:2017:poacc:billing:01:1.0".into());
    inv.buyer_reference = Some(core_invoice::DocumentReference::new("PO-1"));
    let _ = reconcile(&mut inv);
    inv
}

#[cfg(test)]
mod tests {
    use super::*;
    use core_invoice::validate;
    use core_invoice::{PeppolBis3Marker, Validated};
    use core_invoice_formats::{Syntax, read, write_unchecked, write_validated};

    #[test]
    fn pint_my_round_trip_ubl_keeps_profile() {
        let xml = write_unchecked(&pint_my_sst(), Syntax::Ubl).unwrap();
        let back = read(&xml).unwrap();
        assert_eq!(back.profile, Profile::PintMy);
        assert_eq!(back.seller.tax_id, None);
        assert_eq!(back.seller.id_scheme, None);
        assert_eq!(
            back.seller
                .tax_registration
                .as_ref()
                .map(|i| i.value.as_str()),
            Some("C12345678901")
        );
        assert_eq!(back.lines[0].tax.code, "SA");
    }

    #[test]
    fn cii_write_is_three_part_d16b_for_en_peppol() {
        let xml = write_unchecked(&peppol_vat(), Syntax::Cii).unwrap();
        assert!(xml.contains("CrossIndustryInvoice"));
        let line = xml.find("IncludedSupplyChainTradeLineItem").unwrap();
        let header = xml.find("ApplicableHeaderTradeAgreement").unwrap();
        assert!(line < header);
        assert!(!xml.contains(">SST<"));
    }

    #[test]
    fn pint_my_cii_is_refused() {
        let err = write_unchecked(&pint_my_sst(), Syntax::Cii).unwrap_err();
        assert!(
            matches!(err, core_invoice_formats::FormatError::CiiNotForProfile),
            "{err}"
        );
    }

    #[test]
    fn peppol_vat_is_valid() {
        assert!(validate(&peppol_vat()).ok());
    }

    #[test]
    fn write_validated_stamps_peppol_bt24_over_leftover_pint() {
        let mut inv = peppol_vat();
        inv.specification_id = Some("urn:peppol:pint:billing-1".into());
        let proof = Validated::<PeppolBis3Marker>::new(inv).unwrap();
        let xml = write_validated(&proof, Syntax::Ubl).unwrap();
        assert!(xml.contains(core_invoice::Profile::PEPPOL_BIS3_PREFIX));
        assert!(!xml.contains(">urn:peppol:pint:billing-1<"));
        assert!(xml.contains("urn:fdc:peppol.eu:2017:poacc:billing:01:1.0"));
    }

    #[test]
    fn sst_on_peppol_is_invalid() {
        let mut inv = pint_my_sst();
        inv.profile = Profile::PeppolBis3;
        assert!(!validate(&inv).ok());
    }

    #[test]
    fn pint_gst_sr_is_valid_on_pint_not_pint_my() {
        assert!(
            validate(&pint_gst_sr()).ok(),
            "{}",
            validate(&pint_gst_sr())
        );
        let mut my = pint_gst_sr();
        my.profile = Profile::PintMy;
        my.seller.legal_registration = Some(Identifier::new("2023010000001"));
        my.seller.tax_registration = Some(Identifier::new("C12345678901"));
        my.buyer.legal_registration = Some(Identifier::new("1999010000001"));
        let report = validate(&my);
        assert!(
            report
                .findings
                .iter()
                .any(|f| f.id == "ALIGNED-IBRP-CL-01-MY"),
            "{report}"
        );
    }
}
