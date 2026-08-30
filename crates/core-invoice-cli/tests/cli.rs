use std::io::Write;
use std::process::Command;

fn bin() -> Command {
    let mut c = Command::new(env!("CARGO_BIN_EXE_core-invoice"));
    c.env("NO_COLOR", "1");
    c
}

fn peppol_xml() -> String {
    core_invoice_formats::write_unchecked(
        &core_invoice_fixtures::peppol_vat(),
        core_invoice_formats::Syntax::Ubl,
    )
    .unwrap()
}

fn write_tmp(name: &str, xml: &str) -> std::path::PathBuf {
    let path = std::env::temp_dir().join(format!(
        "core-invoice-cli-{name}-{}.xml",
        std::process::id()
    ));
    let mut f = std::fs::File::create(&path).unwrap();
    f.write_all(xml.as_bytes()).unwrap();
    path
}

#[test]
fn neither_invoice_nor_cii_root_is_2() {
    let path = write_tmp("root", "<NotAnInvoice/>");
    let status = bin()
        .args(["validate", path.to_str().unwrap()])
        .status()
        .unwrap();
    assert_eq!(status.code(), Some(2));
}

#[test]
fn missing_file_is_2() {
    let status = bin()
        .args(["validate", "/no/such/invoice.xml"])
        .status()
        .unwrap();
    assert_eq!(status.code(), Some(2));
}

#[test]
fn unreadable_xml_is_2() {
    let path = write_tmp("bad", "not xml <<<");
    let status = bin()
        .args(["validate", path.to_str().unwrap()])
        .status()
        .unwrap();
    assert_eq!(status.code(), Some(2));
}

#[test]
fn validate_json_ok_is_0() {
    let path = write_tmp("ok-json", &peppol_xml());
    let out = bin()
        .args([
            "validate",
            "--format",
            "json",
            "--profile",
            "peppol",
            path.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("\"ok\":true"), "{stdout}");
    assert!(stdout.contains("\"profile\":\"peppol\""));
}

#[test]
fn valid_peppol_is_0() {
    let path = write_tmp("ok", &peppol_xml());
    let out = bin()
        .args(["validate", "--profile", "peppol", path.to_str().unwrap()])
        .output()
        .unwrap();
    assert_eq!(
        out.status.code(),
        Some(0),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
}

#[test]
fn empty_number_is_1_on_stdout() {
    let mut inv = core_invoice_fixtures::peppol_vat();
    inv.number.clear();
    let xml =
        core_invoice_formats::write_unchecked(&inv, core_invoice_formats::Syntax::Ubl).unwrap();
    let path = write_tmp("empty", &xml);
    let out = bin()
        .args(["validate", path.to_str().unwrap()])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(1));
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("BR-02"), "{stdout}");
}

#[test]
fn explain_unknown_is_2() {
    let status = bin().args(["explain", "NO-SUCH"]).status().unwrap();
    assert_eq!(status.code(), Some(2));
}

#[test]
fn explain_known_is_0() {
    let out = bin().args(["explain", "BR-CO-16"]).output().unwrap();
    assert_eq!(out.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("BT-115"));
}

#[test]
fn diff_identical_is_0_different_is_1() {
    let path = write_tmp("same", &peppol_xml());
    let status = bin()
        .args(["diff", path.to_str().unwrap(), path.to_str().unwrap()])
        .status()
        .unwrap();
    assert_eq!(status.code(), Some(0));
    let mut other = core_invoice_fixtures::peppol_vat();
    other.number = "OTHER".into();
    let p2 = write_tmp(
        "other",
        &core_invoice_formats::write_unchecked(&other, core_invoice_formats::Syntax::Ubl).unwrap(),
    );
    let status = bin()
        .args(["diff", path.to_str().unwrap(), p2.to_str().unwrap()])
        .status()
        .unwrap();
    assert_eq!(status.code(), Some(1));
}

#[test]
fn rules_json_lists_br_co_16() {
    let out = bin().args(["rules", "--format", "json"]).output().unwrap();
    assert_eq!(out.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("BR-CO-16"));
}

#[test]
fn convert_empty_number_is_1_without_xml() {
    let mut inv = core_invoice_fixtures::peppol_vat();
    inv.number.clear();
    let xml =
        core_invoice_formats::write_unchecked(&inv, core_invoice_formats::Syntax::Ubl).unwrap();
    let path = write_tmp("empty-cvt", &xml);
    let out = bin()
        .args(["convert", path.to_str().unwrap(), "--to", "ubl"])
        .output()
        .unwrap();
    assert_eq!(
        out.status.code(),
        Some(1),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(!stdout.contains("<Invoice"), "{stdout}");
    assert!(!stdout.contains("CrossIndustryInvoice"), "{stdout}");
    assert!(stdout.contains("BR-02"), "{stdout}");
}

#[test]
fn convert_valid_peppol_to_ubl_is_0() {
    let path = write_tmp("ok-cvt", &peppol_xml());
    let out = bin()
        .args(["convert", path.to_str().unwrap(), "--to", "ubl"])
        .output()
        .unwrap();
    assert_eq!(
        out.status.code(),
        Some(0),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("<Invoice"));
    assert!(stdout.contains(core_invoice::Profile::PEPPOL_BIS3_PREFIX));
}

#[test]
fn convert_pint_my_self_billing_is_1_without_xml() {
    convert_self_billing_is_1("urn:peppol:pint:selfbilling-1@my-1", "sb-my");
}

#[test]
fn convert_peppol_self_billing_is_1_without_xml() {
    convert_self_billing_is_1(
        "urn:cen.eu:en16931:2017#compliant#urn:fdc:peppol.eu:2017:poacc:selfbilling:3.0",
        "sb-peppol",
    );
}

fn convert_self_billing_is_1(customization: &str, name: &str) {
    let xml = peppol_xml().replace(
        core_invoice::Profile::PeppolBis3.specification_id(),
        customization,
    );
    let path = write_tmp(name, &xml);
    let out = bin()
        .args(["convert", path.to_str().unwrap(), "--to", "ubl"])
        .output()
        .unwrap();
    assert_eq!(
        out.status.code(),
        Some(1),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(!stdout.contains("<Invoice"), "{stdout}");
    assert!(!stdout.contains("CrossIndustryInvoice"), "{stdout}");
    assert!(stdout.contains("CORE-PROCESS-01"), "{stdout}");
}

#[test]
fn convert_pint_my_to_cii_is_2() {
    let xml = core_invoice_formats::write_unchecked(
        &core_invoice_fixtures::pint_my_sst(),
        core_invoice_formats::Syntax::Ubl,
    )
    .unwrap();
    let path = write_tmp("pint-my", &xml);
    let out = bin()
        .args(["convert", path.to_str().unwrap(), "--to", "cii"])
        .output()
        .unwrap();
    assert_eq!(
        out.status.code(),
        Some(2),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("UBL-only"), "{stderr}");
    assert!(out.stdout.is_empty());
}

#[test]
fn inspect_has_no_verdict() {
    let path = write_tmp("insp", &peppol_xml());
    let out = bin()
        .args(["inspect", path.to_str().unwrap()])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("number="));
    assert!(stdout.contains("bt-24="));
    assert!(!stdout.contains("valid ("));
}

#[test]
fn inspect_uses_document_element_not_substring() {
    let xml = peppol_xml().replacen("<Invoice", "<!-- CrossIndustryInvoice --><Invoice", 1);
    let path = write_tmp("insp-comment", &xml);
    let out = bin()
        .args(["inspect", path.to_str().unwrap()])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("syntax=ubl"), "{stdout}");
    assert!(!stdout.contains("syntax=cii"), "{stdout}");
}

#[test]
fn validate_batch_worst_exit_is_1() {
    let ok = write_tmp("batch-ok", &peppol_xml());
    let mut inv = core_invoice_fixtures::peppol_vat();
    inv.number.clear();
    let bad = write_tmp(
        "batch-bad",
        &core_invoice_formats::write_unchecked(&inv, core_invoice_formats::Syntax::Ubl).unwrap(),
    );
    let out = bin()
        .args([
            "validate",
            "--profile",
            "peppol",
            ok.to_str().unwrap(),
            bad.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(1));
}

#[test]
fn validate_quiet_ok_is_silent() {
    let path = write_tmp("quiet", &peppol_xml());
    let out = bin()
        .args([
            "validate",
            "--quiet",
            "--profile",
            "peppol",
            path.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(0));
    assert!(out.stdout.is_empty(), "{:?}", out.stdout);
}

#[test]
fn rules_peppol_lists_r010() {
    let out = bin()
        .args(["rules", "--profile", "peppol", "--format", "json"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("PEPPOL-EN16931-R010"));
    assert!(stdout.contains("source"));
}

#[test]
fn rules_en16931_does_not_list_peppol_r010() {
    let out = bin()
        .args(["rules", "--profile", "en16931", "--format", "json"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("BR-CO-16"));
    assert!(!stdout.contains("PEPPOL-EN16931-R010"), "{stdout}");
}

#[test]
fn profiles_prints_pins() {
    let out = bin().args(["profiles"]).output().unwrap();
    assert_eq!(out.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("validation-1.3.16"));
    assert!(stdout.contains("v3.0.20"));
}
