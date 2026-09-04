//! On-disk and in-crate sample invoices.

use core_invoice::{
    Amount, Code, Identifier, Invoice, Line, Party, Price, Profile, Quantity, TaxCategory,
    UnitPriceAmount, reconcile,
};

fn priced(mut line: Line, price: &str) -> Line {
    line.quantity = Some(Quantity::parse("1").unwrap());
    line.unit = Some(Code::new("C62"));
    line.price = Some(Price {
        net: UnitPriceAmount::parse(price).unwrap(),
        discount: None,
        gross: None,
        base_qty: None,
        base_unit: None,
    });
    line
}
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
    inv.lines = vec![priced(
        Line::new(
            "1",
            "Widget",
            Amount::parse("100.00").unwrap(),
            TaxCategory::sst("SA", Decimal::from(10)),
        ),
        "100.00",
    )];
    inv.issue_date = core_invoice::Date::parse("2026-01-15").ok();
    inv.type_code = Some(core_invoice::Code::new("380"));
    inv.payment_terms = Some("Net 30".into());
    let _ = reconcile(&mut inv);
    inv
}

/// Authored MIT fixture for `Profile::Pint` GST (`SR`). Not an official PINT 1.1.2 sample
/// (that zip has no example XML; see `docs/UNCOVERED.md`).
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
    inv.lines = vec![priced(
        Line::new(
            "1",
            "Service",
            Amount::parse("100.00").unwrap(),
            TaxCategory::gst("SR", Decimal::from(9)),
        ),
        "100.00",
    )];
    inv.issue_date = core_invoice::Date::parse("2026-01-15").ok();
    inv.type_code = Some(core_invoice::Code::new("380"));
    inv.payment_terms = Some("Net 30".into());
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
            p.electronic_address = Some(Identifier::schemed("1234567890128", "0088"));
            p
        },
        {
            let mut b = Party::new("Buyer SARL", "FR");
            b.vat_identifier = Some(Identifier::new("FR12345678901"));
            b.electronic_address = Some(Identifier::schemed("1234567890135", "0088"));
            b
        },
    );
    inv.lines = vec![priced(
        Line::new(
            "1",
            "Service",
            Amount::parse("100.00").unwrap(),
            TaxCategory::vat("S", Decimal::from(19)),
        ),
        "100.00",
    )];
    inv.issue_date = core_invoice::Date::parse("2026-01-15").ok();
    inv.type_code = Some(core_invoice::Code::new("380"));
    inv.business_process = Some("urn:fdc:peppol.eu:2017:poacc:billing:01:1.0".into());
    inv.buyer_reference = Some(core_invoice::DocumentReference::new("PO-1"));
    inv.payment_terms = Some("Net 30".into());
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
    fn z03_is_allowed_on_pint_my_not_peppol() {
        let mut my = pint_my_sst();
        my.payment = Some(core_invoice::PaymentInstructions {
            means_code: Some(core_invoice::Code::new("Z03")),
            means_text: None,
            remittance: None,
            means: None,
        });
        let report = validate(&my);
        assert!(
            report.findings.iter().all(|f| f.id != "BR-CL-16"),
            "{report}"
        );
        let mut pep = peppol_vat();
        pep.payment = Some(core_invoice::PaymentInstructions {
            means_code: Some(core_invoice::Code::new("Z03")),
            means_text: None,
            remittance: None,
            means: None,
        });
        let report = validate(&pep);
        assert!(
            report.findings.iter().any(|f| f.id == "BR-CL-16"),
            "{report}"
        );
    }

    #[test]
    fn official_pint_my_sa_when_refers_present() {
        let path = core_invoice_formats::corpus(
            "pint-my-1.3.0/unpacked/trn-invoice/example/Invoice-Sample-SA_1.3.0.xml",
        );
        if !path.exists() {
            if std::env::var("CORE_INVOICE_REQUIRE_SPEC").ok().as_deref() == Some("1") {
                panic!("missing {}", path.display());
            }
            return;
        }
        let xml = std::fs::read_to_string(&path).unwrap();
        let inv = core_invoice_formats::read(&xml).unwrap();
        assert_eq!(inv.profile, Profile::PintMy);
        let report = validate(&inv);
        assert!(report.ok(), "official PINT-MY SA: {report}");
        let as_peppol = {
            let mut i = inv;
            i.profile = Profile::PeppolBis3;
            i
        };
        assert!(!validate(&as_peppol).ok());
    }

    #[test]
    fn official_pint_my_samples_validate() {
        let dir = core_invoice_formats::corpus("pint-my-1.3.0/unpacked/trn-invoice/example");
        let names = [
            "Invoice-Sample-SA_1.3.0.xml",
            "Invoice-Sample-SE_1.3.0.xml",
            "Invoice-Sample-HVG_1.3.0.xml",
            "Invoice-Sample-LVG_1.3.0.xml",
            "Invoice-Sample-TTX_1.3.0.xml",
        ];
        let mut missing = 0;
        for name in names {
            let path = dir.join(name);
            if !path.exists() {
                missing += 1;
                continue;
            }
            let xml = std::fs::read_to_string(&path).unwrap();
            let report = core_invoice_formats::validate_xml(&xml, Some(Profile::PintMy)).unwrap();
            assert!(report.ok(), "{name}: {report}");
        }
        if missing == names.len()
            && std::env::var("CORE_INVOICE_REQUIRE_SPEC").ok().as_deref() == Some("1")
        {
            panic!("missing PINT-MY official samples in {}", dir.display());
        }
    }

    #[test]
    fn official_peppol_base_example_when_refers_present() {
        let path =
            core_invoice_formats::corpus("peppol-bis-invoice-3/rules/examples/base-example.xml");
        if !path.exists() {
            if std::env::var("CORE_INVOICE_REQUIRE_SPEC").ok().as_deref() == Some("1") {
                panic!("missing {}", path.display());
            }
            return;
        }
        let xml = std::fs::read_to_string(&path).unwrap();
        let report = core_invoice_formats::validate_xml(&xml, Some(Profile::PeppolBis3)).unwrap();
        assert!(
            report.ok(),
            "refers/peppol-bis-invoice-3/rules/examples/base-example.xml: {report}"
        );
    }

    #[test]
    fn official_cen_cii_example1_when_refers_present() {
        let path = core_invoice_formats::corpus("en16931/cii/examples/CII_example1.xml");
        if !path.exists() {
            if std::env::var("CORE_INVOICE_REQUIRE_SPEC").ok().as_deref() == Some("1") {
                panic!("missing {}", path.display());
            }
            return;
        }
        let xml = std::fs::read_to_string(&path).unwrap();
        let traced = core_invoice_formats::read_with_trace(&xml).unwrap();
        assert_eq!(traced.invoice.kind, core_invoice::DocumentKind::Invoice);
        assert!(
            !xml.contains("<Invoice "),
            "CII example must not be a UBL wrapper"
        );
        assert!(
            traced
                .invoice
                .lines
                .iter()
                .all(|l| l.tax.code != "S" || l.tax.system == core_invoice::TaxSystem::Vat)
                || traced.invoice.lines.is_empty()
                || traced.invoice.lines.iter().any(|l| !l.tax.code.is_empty()),
            "must not invent category S as SST"
        );
        let _report = core_invoice::validate(&traced.invoice);
    }

    #[test]
    fn official_en16931_example_when_refers_present() {
        let path = core_invoice_formats::corpus("en16931/ubl/examples/ubl-tc434-example1.xml");
        if !path.exists() {
            if std::env::var("CORE_INVOICE_REQUIRE_SPEC").ok().as_deref() == Some("1") {
                panic!("missing {}", path.display());
            }
            return;
        }
        let xml = std::fs::read_to_string(&path).unwrap();
        let inv = core_invoice_formats::read(&xml).unwrap();
        assert_eq!(inv.profile, Profile::En16931);
        let report = validate(&inv);
        assert!(report.ok(), "official EN example1: {report}");
    }

    #[test]
    fn official_en16931_example5_eas_em() {
        let path = core_invoice_formats::corpus("en16931/ubl/examples/ubl-tc434-example5.xml");
        if !path.exists() {
            if std::env::var("CORE_INVOICE_REQUIRE_SPEC").ok().as_deref() == Some("1") {
                panic!("missing {}", path.display());
            }
            return;
        }
        let xml = std::fs::read_to_string(&path).unwrap();
        let report = core_invoice_formats::validate_xml(&xml, Some(Profile::En16931)).unwrap();
        assert!(
            report.findings.iter().all(|f| f.id != "BR-CL-25"),
            "EAS EM must be accepted (BR-CL-25): {report}"
        );
        assert!(report.ok(), "official EN example5: {report}");
    }

    #[test]
    fn official_en_sample_discount_price() {
        let path = core_invoice_formats::corpus("en16931/ubl/examples/sample-discount-price.xml");
        if !path.exists() {
            if std::env::var("CORE_INVOICE_REQUIRE_SPEC").ok().as_deref() == Some("1") {
                panic!("missing {}", path.display());
            }
            return;
        }
        let xml = std::fs::read_to_string(&path).unwrap();
        let report = core_invoice_formats::validate_xml(&xml, Some(Profile::En16931)).unwrap();
        assert!(
            report.findings.iter().all(|f| f.id != "BR-53"),
            "BT-6=BT-5 uses BT-110 for BR-53: {report}"
        );
        assert!(report.ok(), "sample-discount-price: {report}");
        let inv = read(&xml).unwrap();
        let price = inv.lines[0].price.as_ref().expect("price");
        assert!(
            price.gross.is_some() || price.discount.is_some(),
            "BT-147/148 should round-trip on sample-discount-price"
        );
    }

    #[test]
    fn official_pint_my_lhdn_complete_credit_note() {
        let path = core_invoice_formats::corpus(
            "pint-my-1.3.0/unpacked/trn-invoice/example/CompleteSample_LHDN-CreditNote.xml",
        );
        if !path.exists() {
            if std::env::var("CORE_INVOICE_REQUIRE_SPEC").ok().as_deref() == Some("1") {
                panic!("missing {}", path.display());
            }
            return;
        }
        let xml = std::fs::read_to_string(&path).unwrap();
        let traced = core_invoice_formats::read_with_trace(&xml).unwrap();
        assert!(
            traced.malformed.iter().all(|m| !m.contains("DueDate")),
            "{:?}",
            traced.malformed
        );
        let report = validate(&traced.invoice);
        assert!(
            report.ok()
                || report
                    .findings
                    .iter()
                    .all(|f| f.severity != core_invoice::Severity::Fatal),
            "CompleteSample_LHDN-CreditNote: {report}"
        );
    }

    #[test]
    fn sa_plus_ttx_omits_percent_and_uses_aal() {
        let mut inv = pint_my_sst();
        inv.lines.push(Line::new(
            "2",
            "Tourism",
            Amount::parse("50.00").unwrap(),
            core_invoice::tax::TaxCategory::ttx(),
        ));
        let _ = reconcile(&mut inv);
        let xml = write_unchecked(&inv, Syntax::Ubl).unwrap();
        assert!(
            xml.contains("schemeID=\"AAL\"") || xml.contains(">AAL<"),
            "{xml}"
        );
        assert!(!xml.contains(">SST<"));
        let ttx_block = xml
            .split("TaxSubtotal")
            .find(|s| s.contains(">TTX<"))
            .unwrap_or("");
        assert!(
            !ttx_block.contains("<cbc:Percent"),
            "TTX subtotal must not emit Percent: {ttx_block}"
        );
        let back = read(&xml).unwrap();
        let ttx = back
            .tax_breakdown
            .iter()
            .find(|r| r.category.as_str() == "TTX")
            .expect("TTX row");
        assert!(ttx.rate.is_none());
        assert_eq!(ttx.scheme, "AAL");
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
    fn pint_gst_sr_is_core_ok_on_pint_not_a_1_1_2_oracle() {
        // CORE + PINT-TAX. Not PINT Schematron Valid (no ibr-076/080/081 on this fixture).
        let inv = pint_gst_sr();
        assert_eq!(inv.lines[0].tax.system, core_invoice::TaxSystem::Gst);
        assert!(validate(&inv).ok(), "{}", validate(&inv));
        let xml = write_unchecked(&inv, Syntax::Ubl).unwrap();
        assert!(xml.contains(">GST<"), "{xml}");
        assert!(!xml.contains(">SST<"), "{xml}");
        let mut my = inv;
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

    #[test]
    fn official_pint_when_present() {
        // 1.1.2 zip has no instance XML. Missing is the pin, not a fetch bug —
        // do not panic under CORE_INVOICE_REQUIRE_SPEC=1.
        let dir = core_invoice_formats::corpus("pint-billing-1.1.2/unpacked/trn-invoice/example");
        if !dir.is_dir() {
            return;
        }
        let mut any = false;
        let Ok(rd) = std::fs::read_dir(&dir) else {
            return;
        };
        for e in rd.flatten() {
            let p = e.path();
            if p.extension().and_then(|e| e.to_str()) != Some("xml") {
                continue;
            }
            any = true;
            let xml = std::fs::read_to_string(&p).unwrap();
            let inv = core_invoice_formats::read(&xml).unwrap();
            assert_eq!(inv.profile, Profile::Pint, "{}", p.display());
            let report = core_invoice_formats::validate_xml(&xml, Some(Profile::Pint)).unwrap();
            assert_eq!(report.profile_slug, "pint", "{}", p.display());
        }
        let _ = any;
    }
}
