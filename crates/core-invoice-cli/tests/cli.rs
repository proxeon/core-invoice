use std::io::Write;
use std::process::Command;

fn bin() -> Command {
    let mut c = Command::new(env!("CARGO_BIN_EXE_core-invoice"));
    c.env("NO_COLOR", "1");
    c
}

fn peppol_xml() -> String {
    core_invoice_formats::write(
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
    let xml = core_invoice_formats::write(&inv, core_invoice_formats::Syntax::Ubl).unwrap();
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
        &core_invoice_formats::write(&other, core_invoice_formats::Syntax::Ubl).unwrap(),
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
fn inspect_has_no_verdict() {
    let path = write_tmp("insp", &peppol_xml());
    let out = bin()
        .args(["inspect", path.to_str().unwrap()])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("number="));
    assert!(!stdout.contains("valid ("));
}
