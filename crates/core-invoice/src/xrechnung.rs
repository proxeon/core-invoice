//! XRechnung extra rules. Optional Cargo feature, never default, never CORE.
//!
//! XRechnung is a German CIUS of EN 16931. BT-24 still looks up as
//! [`crate::profile::Profile::En16931`]. These rules overlay only when the
//! `xrechnung` feature is on **and** the document claims an XRechnung
//! specification identifier.

use crate::bt::{BtId, Group, Path};
use crate::invoice::Invoice;
use crate::payment::PaymentMeans;
use crate::report::{Finding, Report, Severity, Source};
use crate::rules::Rule;

/// KoSIT / xeinkauf XRechnung specification identifiers (any 1.x/2.x/3.x).
pub fn claimed(inv: &Invoice) -> bool {
    inv.specification_id
        .as_deref()
        .is_some_and(is_xrechnung_spec)
}

/// True when BT-24 names KoSIT / xeinkauf XRechnung (any 1.x/2.x/3.x).
pub fn is_xrechnung_spec(id: &str) -> bool {
    let id = id.to_ascii_lowercase();
    id.contains("xrechnung") || id.contains("xeinkauf.de")
}

fn br_de_15(inv: &Invoice, report: &mut Report) {
    if !claimed(inv) {
        return;
    }
    if inv
        .buyer_reference
        .as_ref()
        .map(|r| r.as_str().trim().is_empty())
        .unwrap_or(true)
    {
        report.push(Finding::fatal(
            "BR-DE-15",
            Path::term(BtId(10)),
            "XRechnung: Buyer reference (BT-10) shall be present",
        ));
    }
}

fn taxed_vat(inv: &Invoice) -> bool {
    const CODES: &[&str] = &["S", "Z", "E", "AE", "K", "G", "L", "M"];
    let hit = |code: &str| CODES.iter().any(|c| c.eq_ignore_ascii_case(code.trim()));
    inv.lines.iter().any(|l| hit(&l.tax.code))
        || inv
            .document_allowances
            .iter()
            .any(|a| a.tax.as_ref().is_some_and(|t| hit(&t.code)))
        || inv
            .document_charges
            .iter()
            .any(|a| a.tax.as_ref().is_some_and(|t| hit(&t.code)))
}

fn br_de_16(inv: &Invoice, report: &mut Report) {
    if !claimed(inv) || !taxed_vat(inv) {
        return;
    }
    let seller_id = inv.seller.vat_identifier.is_some() || inv.seller.tax_registration.is_some();
    let rep = inv.tax_representative.is_some();
    if !seller_id && !rep {
        report.push(Finding::fatal(
            "BR-DE-16",
            Path::group_term(Group::Seller, BtId(31)),
            "XRechnung: BT-31, BT-32 or tax representative shall be present for listed VAT categories",
        ));
    }
}

fn blank(s: Option<&str>) -> bool {
    s.map(|t| t.trim().is_empty()).unwrap_or(true)
}

fn br_de_1(inv: &Invoice, report: &mut Report) {
    if !claimed(inv) {
        return;
    }
    if inv.payment.is_none() {
        report.push(Finding::fatal(
            "BR-DE-1",
            Path::group(Group::Payment),
            "XRechnung: Payment instructions (BG-16) shall be present",
        ));
    }
}

fn br_de_2(inv: &Invoice, report: &mut Report) {
    if !claimed(inv) {
        return;
    }
    let empty = inv.seller.contact.as_ref().is_none_or(|c| {
        blank(c.point.as_deref()) && blank(c.phone.as_deref()) && blank(c.email.as_deref())
    });
    if empty {
        report.push(Finding::fatal(
            "BR-DE-2",
            Path::group_term(Group::Seller, BtId(41)),
            "XRechnung: Seller contact (BG-6) shall be present",
        ));
    }
}

fn br_de_3(inv: &Invoice, report: &mut Report) {
    if !claimed(inv) {
        return;
    }
    if blank(inv.seller.address.as_ref().and_then(|a| a.city.as_deref())) {
        report.push(Finding::fatal(
            "BR-DE-3",
            Path::group_term(Group::Seller, BtId(37)),
            "XRechnung: Seller city (BT-37) shall be present",
        ));
    }
}

fn br_de_4(inv: &Invoice, report: &mut Report) {
    if !claimed(inv) {
        return;
    }
    if blank(
        inv.seller
            .address
            .as_ref()
            .and_then(|a| a.post_code.as_deref()),
    ) {
        report.push(Finding::fatal(
            "BR-DE-4",
            Path::group_term(Group::Seller, BtId(38)),
            "XRechnung: Seller post code (BT-38) shall be present",
        ));
    }
}

fn br_de_5(inv: &Invoice, report: &mut Report) {
    if !claimed(inv) {
        return;
    }
    let Some(c) = inv.seller.contact.as_ref() else {
        return;
    };
    if blank(c.point.as_deref()) {
        report.push(Finding::fatal(
            "BR-DE-5",
            Path::group_term(Group::Seller, BtId(41)),
            "XRechnung: Seller contact point (BT-41) shall be present",
        ));
    }
}

fn br_de_6(inv: &Invoice, report: &mut Report) {
    if !claimed(inv) {
        return;
    }
    let Some(c) = inv.seller.contact.as_ref() else {
        return;
    };
    if blank(c.phone.as_deref()) {
        report.push(Finding::fatal(
            "BR-DE-6",
            Path::group_term(Group::Seller, BtId(42)),
            "XRechnung: Seller contact telephone (BT-42) shall be present",
        ));
    }
}

fn br_de_7(inv: &Invoice, report: &mut Report) {
    if !claimed(inv) {
        return;
    }
    let Some(c) = inv.seller.contact.as_ref() else {
        return;
    };
    if blank(c.email.as_deref()) {
        report.push(Finding::fatal(
            "BR-DE-7",
            Path::group_term(Group::Seller, BtId(43)),
            "XRechnung: Seller contact email (BT-43) shall be present",
        ));
    }
}

fn br_de_8(inv: &Invoice, report: &mut Report) {
    if !claimed(inv) {
        return;
    }
    if blank(inv.buyer.address.as_ref().and_then(|a| a.city.as_deref())) {
        report.push(Finding::fatal(
            "BR-DE-8",
            Path::group_term(Group::Buyer, BtId(52)),
            "XRechnung: Buyer city (BT-52) shall be present",
        ));
    }
}

fn br_de_9(inv: &Invoice, report: &mut Report) {
    if !claimed(inv) {
        return;
    }
    if blank(
        inv.buyer
            .address
            .as_ref()
            .and_then(|a| a.post_code.as_deref()),
    ) {
        report.push(Finding::fatal(
            "BR-DE-9",
            Path::group_term(Group::Buyer, BtId(53)),
            "XRechnung: Buyer post code (BT-53) shall be present",
        ));
    }
}

fn br_de_10(inv: &Invoice, report: &mut Report) {
    if !claimed(inv) {
        return;
    }
    let Some(addr) = inv.delivery.as_ref().and_then(|d| d.address.as_ref()) else {
        return;
    };
    if blank(addr.city.as_deref()) {
        report.push(Finding::fatal(
            "BR-DE-10",
            Path::group_term(Group::Delivery, BtId(77)),
            "XRechnung: Deliver-to city (BT-77) shall be present when BG-15 is present",
        ));
    }
}

fn br_de_11(inv: &Invoice, report: &mut Report) {
    if !claimed(inv) {
        return;
    }
    let Some(addr) = inv.delivery.as_ref().and_then(|d| d.address.as_ref()) else {
        return;
    };
    if blank(addr.post_code.as_deref()) {
        report.push(Finding::fatal(
            "BR-DE-11",
            Path::group_term(Group::Delivery, BtId(78)),
            "XRechnung: Deliver-to post code (BT-78) shall be present when BG-15 is present",
        ));
    }
}

fn br_de_14(inv: &Invoice, report: &mut Report) {
    if !claimed(inv) {
        return;
    }
    for (i, row) in inv.tax_breakdown.iter().enumerate() {
        if row.rate.is_none() {
            report.push(Finding::fatal(
                "BR-DE-14",
                Path::at_term(Group::TaxBreakdown, i, BtId(119)),
                "XRechnung: VAT category rate (BT-119) shall be present",
            ));
        }
    }
}

fn br_de_17(inv: &Invoice, report: &mut Report) {
    if !claimed(inv) {
        return;
    }
    let Some(code) = inv.type_code.as_ref() else {
        return;
    };
    const ALLOWED: &[&str] = &["326", "380", "384", "389", "381", "875", "876", "877"];
    if !ALLOWED.contains(&code.as_str()) {
        report.push(Finding::warning(
            "BR-DE-17",
            Path::term(BtId(3)),
            format!(
                "XRechnung: invoice type code {} is not in the German supported set",
                code.as_str()
            ),
        ));
    }
}

fn means_code(inv: &Invoice) -> Option<&str> {
    inv.payment
        .as_ref()
        .and_then(|p| p.means_code.as_ref())
        .map(crate::code::Code::as_str)
}

fn br_de_23_a(inv: &Invoice, report: &mut Report) {
    if !claimed(inv) {
        return;
    }
    if means_code(inv).is_some_and(|c| c == "30" || c == "58")
        && !matches!(
            inv.payment.as_ref().and_then(|p| p.means.as_ref()),
            Some(PaymentMeans::CreditTransfer(_))
        )
    {
        report.push(Finding::fatal(
            "BR-DE-23-a",
            Path::group_term(Group::Payment, BtId(81)),
            "XRechnung: BT-81 30/58 requires credit transfer (BG-17)",
        ));
    }
}

fn br_de_24_a(inv: &Invoice, report: &mut Report) {
    if !claimed(inv) {
        return;
    }
    if means_code(inv).is_some_and(|c| matches!(c, "48" | "54" | "55"))
        && !matches!(
            inv.payment.as_ref().and_then(|p| p.means.as_ref()),
            Some(PaymentMeans::Card(_))
        )
    {
        report.push(Finding::fatal(
            "BR-DE-24-a",
            Path::group_term(Group::Payment, BtId(81)),
            "XRechnung: BT-81 48/54/55 requires payment card (BG-18)",
        ));
    }
}

fn br_de_25_a(inv: &Invoice, report: &mut Report) {
    if !claimed(inv) {
        return;
    }
    if means_code(inv).is_some_and(|c| c == "59")
        && !matches!(
            inv.payment.as_ref().and_then(|p| p.means.as_ref()),
            Some(PaymentMeans::DirectDebit(_))
        )
    {
        report.push(Finding::fatal(
            "BR-DE-25-a",
            Path::group_term(Group::Payment, BtId(81)),
            "XRechnung: BT-81 59 requires direct debit (BG-19)",
        ));
    }
}

fn br_de_23_b(_inv: &Invoice, _report: &mut Report) {
    // Unrepresentable: PaymentMeans is an enum.
}

fn br_de_30(inv: &Invoice, report: &mut Report) {
    if !claimed(inv) {
        return;
    }
    if let Some(PaymentMeans::DirectDebit(d)) = inv.payment.as_ref().and_then(|p| p.means.as_ref())
        && d.creditor_id
            .as_ref()
            .map(|c| c.value.trim().is_empty())
            .unwrap_or(true)
    {
        report.push(Finding::fatal(
            "BR-DE-30",
            Path::group_term(Group::Payment, BtId(90)),
            "XRechnung: BG-19 requires creditor identifier (BT-90)",
        ));
    }
}

/// XRechnung `$XR-SKONTO-REGEX`, hand-parsed. No regex crate.
///
/// `#SKONTO#TAGE=<digits>#PROZENT=<digits>.<exactly 2 dp>[#BASISBETRAG=[-]<digits>.<2 dp>]#`
fn is_skonto_line(line: &str) -> bool {
    let Some(rest) = line.strip_prefix("#SKONTO#TAGE=") else {
        return false;
    };
    let Some((days, rest)) = rest.split_once("#PROZENT=") else {
        return false;
    };
    if days.is_empty() || !days.bytes().all(|b| b.is_ascii_digit()) {
        return false;
    }
    let Some(body) = rest.strip_suffix('#') else {
        return false;
    };
    match body.split_once("#BASISBETRAG=") {
        Some((pct, base)) => is_two_dp(pct, false) && is_two_dp(base, true),
        None => is_two_dp(body, false),
    }
}

fn is_two_dp(s: &str, allow_sign: bool) -> bool {
    let s = if allow_sign {
        s.strip_prefix('-').unwrap_or(s)
    } else {
        s
    };
    let Some((int, frac)) = s.split_once('.') else {
        return false;
    };
    !int.is_empty()
        && int.bytes().all(|b| b.is_ascii_digit())
        && frac.len() == 2
        && frac.bytes().all(|b| b.is_ascii_digit())
}

fn br_de_18(inv: &Invoice, report: &mut Report) {
    if !claimed(inv) {
        return;
    }
    let Some(terms) = inv.payment_terms.as_deref() else {
        return;
    };
    let mut bad = terms
        .lines()
        .map(str::trim)
        .any(|line| line.starts_with('#') && !is_skonto_line(line));
    if let Some(last_hash) = terms.rfind('#') {
        let tail = &terms[last_hash + 1..];
        if terms.contains("#SKONTO#")
            && !tail.trim_start_matches([' ', '\t', '\r']).starts_with('\n')
        {
            bad = true;
        }
    }
    if bad {
        report.push(Finding::fatal(
            "BR-DE-18",
            Path::term(BtId(20)),
            "XRechnung: Skonto lines in BT-20 must match #SKONTO#TAGE=n#PROZENT=n.nn[#BASISBETRAG=n.nn]# and end with a newline",
        ));
    }
}

/// ISO 7064 mod-97-10 checksum. Not a country-registry check; no `sepa` crate.
fn is_valid_iban_checksum(s: &str) -> bool {
    let compact: String = s.chars().filter(|c| !c.is_whitespace()).collect();
    if compact.len() < 5 || compact.len() > 34 {
        return false;
    }
    if !compact.bytes().all(|b| b.is_ascii_alphanumeric()) {
        return false;
    }
    let bytes = compact.as_bytes();
    if !bytes[0].is_ascii_alphabetic()
        || !bytes[1].is_ascii_alphabetic()
        || !bytes[2].is_ascii_digit()
        || !bytes[3].is_ascii_digit()
    {
        return false;
    }
    let rearranged = compact[4..].chars().chain(compact[..4].chars());
    let mut remainder: u32 = 0;
    for c in rearranged {
        let value = if c.is_ascii_digit() {
            u32::from(c as u8 - b'0')
        } else {
            u32::from(c.to_ascii_uppercase() as u8 - b'A') + 10
        };
        remainder = if value >= 10 {
            (remainder * 100 + value) % 97
        } else {
            (remainder * 10 + value) % 97
        };
    }
    remainder == 1
}

fn br_de_19(inv: &Invoice, report: &mut Report) {
    if !claimed(inv) {
        return;
    }
    if means_code(inv) != Some("58") {
        return;
    }
    let Some(PaymentMeans::CreditTransfer(accounts)) =
        inv.payment.as_ref().and_then(|p| p.means.as_ref())
    else {
        return;
    };
    for (i, a) in accounts.iter().enumerate() {
        let v = a.account_id.value.trim();
        if v.is_empty() {
            continue;
        }
        if !is_valid_iban_checksum(v) {
            report.push(Finding::warning(
                "BR-DE-19",
                Path::at_term(Group::Payment, i, BtId(84)),
                "XRechnung: BT-84 should be a valid IBAN when BT-81 is 58",
            ));
        }
    }
}

fn br_de_20(inv: &Invoice, report: &mut Report) {
    if !claimed(inv) {
        return;
    }
    if means_code(inv) != Some("59") {
        return;
    }
    let Some(PaymentMeans::DirectDebit(d)) = inv.payment.as_ref().and_then(|p| p.means.as_ref())
    else {
        return;
    };
    let Some(acc) = d.debited_account.as_ref() else {
        return;
    };
    let v = acc.value.trim();
    if v.is_empty() {
        return;
    }
    if !is_valid_iban_checksum(v) {
        report.push(Finding::warning(
            "BR-DE-20",
            Path::group_term(Group::Payment, BtId(91)),
            "XRechnung: BT-91 should be a valid IBAN when BT-81 is 59",
        ));
    }
}

const XR_CIUS_ID: &str = "urn:cen.eu:en16931:2017#compliant#urn:xeinkauf.de:kosit:xrechnung_3.0";
const XR_EXTENSION_ID: &str = "urn:cen.eu:en16931:2017#compliant#urn:xeinkauf.de:kosit:xrechnung_3.0#conformant#urn:xeinkauf.de:kosit:extension:xrechnung_3.0";
const XR_CVD_ID: &str = "urn:cen.eu:en16931:2017#compliant#urn:xeinkauf.de:kosit:xrechnung_3.0#compliant#urn:xeinkauf.de:kosit:xrechnung:cvd_0.9";

fn br_de_21(inv: &Invoice, report: &mut Report) {
    if !claimed(inv) {
        return;
    }
    let Some(id) = inv.specification_id.as_deref().map(str::trim) else {
        return;
    };
    if id != XR_CIUS_ID && id != XR_EXTENSION_ID && id != XR_CVD_ID {
        report.push(Finding::warning(
            "BR-DE-21",
            Path::term(BtId(24)),
            "XRechnung: BT-24 should be a 3.0 CIUS, Extension, or CVD identifier",
        ));
    }
}

fn br_de_22(inv: &Invoice, report: &mut Report) {
    if !claimed(inv) {
        return;
    }
    for (i, doc) in inv.supporting_documents.iter().enumerate() {
        let Some(name) = doc.attachment.as_ref().map(|a| a.filename.as_str()) else {
            continue;
        };
        let dup = inv.supporting_documents[..i]
            .iter()
            .filter_map(|d| d.attachment.as_ref())
            .any(|a| a.filename == name);
        if dup {
            report.push(Finding::fatal(
                "BR-DE-22",
                Path::at_term(Group::Attachment, i, BtId(125)),
                "XRechnung: BT-125 filenames must be unique",
            ));
        }
    }
}

fn br_de_26(inv: &Invoice, report: &mut Report) {
    if !claimed(inv) {
        return;
    }
    if inv.type_code.as_ref().is_some_and(|c| c.as_str() == "384") && inv.preceding.is_empty() {
        report.push(Finding::warning(
            "BR-DE-26",
            Path::term(BtId(25)),
            "XRechnung: type 384 should cite a preceding invoice (BG-3)",
        ));
    }
}

fn br_de_27(inv: &Invoice, report: &mut Report) {
    if !claimed(inv) {
        return;
    }
    let Some(phone) = inv.seller.contact.as_ref().and_then(|c| c.phone.as_deref()) else {
        return;
    };
    if phone.trim().is_empty() {
        return;
    }
    if phone.chars().filter(char::is_ascii_digit).count() < 3 {
        report.push(Finding::warning(
            "BR-DE-27",
            Path::group_term(Group::Seller, BtId(42)),
            "XRechnung: BT-42 should contain at least three digits",
        ));
    }
}

/// BR-DE-28 shape test: weaker than RFC 5322.
fn plausible_email(s: &str) -> bool {
    let at: Vec<usize> = s.match_indices('@').map(|(i, _)| i).collect();
    if at.len() != 1 {
        return false;
    }
    let (local, domain) = s.split_at(at[0]);
    let domain = &domain[1..];
    if local.chars().count() < 2 || domain.chars().count() < 2 {
        return false;
    }
    let bad = |c: Option<char>| matches!(c, Some(' ' | '.'));
    if bad(local.chars().next_back()) || bad(domain.chars().next()) {
        return false;
    }
    !s.starts_with('.') && !s.ends_with('.')
}

fn br_de_28(inv: &Invoice, report: &mut Report) {
    if !claimed(inv) {
        return;
    }
    let Some(email) = inv.seller.contact.as_ref().and_then(|c| c.email.as_deref()) else {
        return;
    };
    if email.trim().is_empty() {
        return;
    }
    if !plausible_email(email) {
        report.push(Finding::warning(
            "BR-DE-28",
            Path::group_term(Group::Seller, BtId(43)),
            "XRechnung: BT-43 should contain exactly one @ flanked by at least two characters",
        ));
    }
}

fn br_de_31(inv: &Invoice, report: &mut Report) {
    if !claimed(inv) {
        return;
    }
    if let Some(PaymentMeans::DirectDebit(d)) = inv.payment.as_ref().and_then(|p| p.means.as_ref())
        && d.debited_account
            .as_ref()
            .map(|c| c.value.trim().is_empty())
            .unwrap_or(true)
    {
        report.push(Finding::fatal(
            "BR-DE-31",
            Path::group_term(Group::Payment, BtId(91)),
            "XRechnung: BG-19 requires debited account identifier (BT-91)",
        ));
    }
}

fn br_de_tmp_32(inv: &Invoice, report: &mut Report) {
    if !claimed(inv) {
        return;
    }
    let has_delivery_date = inv.delivery.as_ref().is_some_and(|d| d.date.is_some());
    let has_period = inv.period.is_some();
    let every_line_has_one = !inv.lines.is_empty() && inv.lines.iter().all(|l| l.period.is_some());
    if !(has_delivery_date || has_period || every_line_has_one) {
        report.push(Finding::info(
            "BR-DE-TMP-32",
            Path::term(BtId(72)),
            "XRechnung: should state a delivery/performance date (BT-72, BG-14, or BG-26 on every line)",
        ));
    }
}

fn is_absolute_url(s: &str) -> bool {
    let Some((scheme, rest)) = s.split_once("://") else {
        return false;
    };
    !scheme.is_empty()
        && scheme.starts_with(|c: char| c.is_ascii_alphabetic())
        && scheme
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'+' | b'-' | b'.'))
        && !rest.is_empty()
}

fn br_tmp_2(inv: &Invoice, report: &mut Report) {
    if !claimed(inv) {
        return;
    }
    for (i, doc) in inv.supporting_documents.iter().enumerate() {
        if let Some(uri) = doc.uri.as_deref()
            && !is_absolute_url(uri)
        {
            report.push(Finding::fatal(
                "BR-TMP-2",
                Path::at_term(Group::Attachment, i, BtId(124)),
                "XRechnung: BT-124 must be an absolute URL",
            ));
        }
    }
}

const fn r(id: &'static str, text: &'static str, eval: fn(&Invoice, &mut Report)) -> Rule {
    Rule {
        id,
        severity: Severity::Fatal,
        text,
        source: Source::Crate,
        eval,
    }
}

const fn rw(id: &'static str, text: &'static str, eval: fn(&Invoice, &mut Report)) -> Rule {
    Rule {
        id,
        severity: Severity::Warning,
        text,
        source: Source::Crate,
        eval,
    }
}

const fn ri(id: &'static str, text: &'static str, eval: fn(&Invoice, &mut Report)) -> Rule {
    Rule {
        id,
        severity: Severity::Info,
        text,
        source: Source::Crate,
        eval,
    }
}

/// Extra rules. Not CORE. Remaining KoSIT Extension / CVD stay in UNCOVERED.
pub static RULES: &[Rule] = &[
    r(
        "BR-DE-1",
        "XRechnung: Payment instructions (BG-16) shall be present.",
        br_de_1,
    ),
    r(
        "BR-DE-2",
        "XRechnung: Seller contact (BG-6) shall be present.",
        br_de_2,
    ),
    r(
        "BR-DE-3",
        "XRechnung: Seller city (BT-37) shall be present.",
        br_de_3,
    ),
    r(
        "BR-DE-4",
        "XRechnung: Seller post code (BT-38) shall be present.",
        br_de_4,
    ),
    r(
        "BR-DE-5",
        "XRechnung: Seller contact point (BT-41) shall be present when BG-6 exists.",
        br_de_5,
    ),
    r(
        "BR-DE-6",
        "XRechnung: Seller contact telephone (BT-42) shall be present when BG-6 exists.",
        br_de_6,
    ),
    r(
        "BR-DE-7",
        "XRechnung: Seller contact email (BT-43) shall be present when BG-6 exists.",
        br_de_7,
    ),
    r(
        "BR-DE-8",
        "XRechnung: Buyer city (BT-52) shall be present.",
        br_de_8,
    ),
    r(
        "BR-DE-9",
        "XRechnung: Buyer post code (BT-53) shall be present.",
        br_de_9,
    ),
    r(
        "BR-DE-10",
        "XRechnung: Deliver-to city (BT-77) when BG-15 is present.",
        br_de_10,
    ),
    r(
        "BR-DE-11",
        "XRechnung: Deliver-to post code (BT-78) when BG-15 is present.",
        br_de_11,
    ),
    r(
        "BR-DE-14",
        "XRechnung: VAT category rate (BT-119) shall be present on every BG-23 row.",
        br_de_14,
    ),
    r(
        "BR-DE-15",
        "XRechnung: Buyer reference (BT-10) shall be present.",
        br_de_15,
    ),
    r(
        "BR-DE-16",
        "XRechnung: seller VAT/tax id or tax representative when listed VAT categories are used.",
        br_de_16,
    ),
    rw(
        "BR-DE-17",
        "XRechnung: invoice type code should be one of 326/380/384/389/381/875/876/877 (warning).",
        br_de_17,
    ),
    r(
        "BR-DE-18",
        "XRechnung: Skonto lines in BT-20 must match the German micro-syntax and end with a newline.",
        br_de_18,
    ),
    rw(
        "BR-DE-19",
        "XRechnung: BT-84 should be a valid IBAN when BT-81 is 58 (warning).",
        br_de_19,
    ),
    rw(
        "BR-DE-20",
        "XRechnung: BT-91 should be a valid IBAN when BT-81 is 59 (warning).",
        br_de_20,
    ),
    rw(
        "BR-DE-21",
        "XRechnung: BT-24 should be a 3.0 CIUS, Extension, or CVD identifier (warning).",
        br_de_21,
    ),
    r(
        "BR-DE-22",
        "XRechnung: BT-125 filenames must be unique.",
        br_de_22,
    ),
    r(
        "BR-DE-23-a",
        "XRechnung: BT-81 30/58 requires credit transfer (BG-17).",
        br_de_23_a,
    ),
    r(
        "BR-DE-24-a",
        "XRechnung: BT-81 48/54/55 requires payment card (BG-18).",
        br_de_24_a,
    ),
    r(
        "BR-DE-25-a",
        "XRechnung: BT-81 59 requires direct debit (BG-19).",
        br_de_25_a,
    ),
    r(
        "BR-DE-23-b",
        "XRechnung: BT-81 credit transfer forbids BG-18/BG-19 (type-retired: PaymentMeans enum).",
        br_de_23_b,
    ),
    r(
        "BR-DE-24-b",
        "XRechnung: BT-81 card forbids BG-17/BG-19 (type-retired: PaymentMeans enum).",
        br_de_23_b,
    ),
    r(
        "BR-DE-25-b",
        "XRechnung: BT-81 direct debit forbids BG-17/BG-18 (type-retired: PaymentMeans enum).",
        br_de_23_b,
    ),
    rw(
        "BR-DE-26",
        "XRechnung: type 384 should cite a preceding invoice (BG-3) (warning).",
        br_de_26,
    ),
    rw(
        "BR-DE-27",
        "XRechnung: BT-42 should contain at least three digits (warning).",
        br_de_27,
    ),
    rw(
        "BR-DE-28",
        "XRechnung: BT-43 should contain exactly one @ flanked by at least two characters (warning).",
        br_de_28,
    ),
    r(
        "BR-DE-30",
        "XRechnung: BG-19 requires creditor identifier (BT-90).",
        br_de_30,
    ),
    r(
        "BR-DE-31",
        "XRechnung: BG-19 requires debited account identifier (BT-91).",
        br_de_31,
    ),
    ri(
        "BR-DE-TMP-32",
        "XRechnung: should state a delivery/performance date (BT-72, BG-14, or BG-26 on every line).",
        br_de_tmp_32,
    ),
    r(
        "BR-TMP-2",
        "XRechnung: BT-124 must be an absolute URL.",
        br_tmp_2,
    ),
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::amount::InvoiceAmount;
    use crate::attachment::Attachment;
    use crate::code::Code;
    use crate::date::Date;
    use crate::identifier::{DocumentReference, Identifier};
    use crate::invoice::{
        Contact, Delivery, Invoice, Line, Party, PaymentInstructions, Period, PrecedingInvoice,
        SupportingDocument, TaxBreakdown,
    };
    use crate::payment::{CreditTransfer, DirectDebit, PaymentMeans};
    use crate::profile::Profile;
    use crate::tax::{TaxCategory, TaxSystem};
    use crate::validate::validate;
    use rust_decimal::Decimal;

    fn xr() -> Invoice {
        let mut inv = Invoice::blank(
            Profile::En16931,
            "1",
            "EUR",
            Party::new("S", "DE"),
            Party::new("B", "DE"),
        );
        inv.specification_id =
            Some("urn:cen.eu:en16931:2017#compliant#urn:xeinkauf.de:kosit:xrechnung_3.0".into());
        inv.lines = vec![Line::new(
            "1",
            "A",
            crate::amount::Amount::parse("10.00").unwrap(),
            TaxCategory::vat("S", Decimal::from(19)),
        )];
        inv
    }

    #[test]
    fn detect_kosit_urn() {
        assert!(is_xrechnung_spec(
            "urn:cen.eu:en16931:2017#compliant#urn:xeinkauf.de:kosit:xrechnung_3.0"
        ));
        assert!(is_xrechnung_spec(XR_EXTENSION_ID));
        assert!(is_xrechnung_spec(XR_CVD_ID));
        assert!(is_xrechnung_spec(
            "urn:cen.eu:en16931:2017#compliant#urn:xoev-de:kosit:standard:xrechnung_3.0"
        ));
        assert!(!is_xrechnung_spec("urn:cen.eu:en16931:2017"));
    }

    #[test]
    fn br_de_15_requires_buyer_reference() {
        let inv = xr();
        let report = validate(&inv);
        assert!(
            report.findings.iter().any(|f| f.id == "BR-DE-15"),
            "{report}"
        );
        let mut inv = inv;
        inv.buyer_reference = Some(DocumentReference::new("PO-1"));
        inv.seller.vat_identifier = Some(crate::identifier::Identifier::new("DE123"));
        let report = validate(&inv);
        assert!(
            report.findings.iter().all(|f| f.id != "BR-DE-15"),
            "{report}"
        );
    }

    #[test]
    fn br_de_16_requires_seller_tax_id() {
        let mut inv = xr();
        inv.buyer_reference = Some(DocumentReference::new("PO-1"));
        inv.type_code = Some(Code::new("380"));
        let report = validate(&inv);
        assert!(
            report.findings.iter().any(|f| f.id == "BR-DE-16"),
            "{report}"
        );
    }

    #[test]
    fn en_core_does_not_run_br_de() {
        let mut inv = xr();
        inv.specification_id = Some("urn:cen.eu:en16931:2017".into());
        let report = validate(&inv);
        assert!(
            report
                .findings
                .iter()
                .all(|f| !f.id.starts_with("BR-DE-") && f.id != "BR-TMP-2"),
            "{report}"
        );
    }

    #[test]
    fn br_de_1_is_payment_not_contact() {
        let inv = xr();
        let report = validate(&inv);
        assert!(
            report.findings.iter().any(|f| f.id == "BR-DE-1"),
            "{report}"
        );
        assert!(
            report.findings.iter().any(|f| f.id == "BR-DE-2"),
            "{report}"
        );
        let mut inv = inv;
        inv.payment = Some(crate::invoice::PaymentInstructions {
            means_code: Some(Code::new("30")),
            means_text: None,
            remittance: None,
            means: None,
        });
        let report = validate(&inv);
        assert!(
            report.findings.iter().all(|f| f.id != "BR-DE-1"),
            "{report}"
        );
    }

    #[test]
    fn br_de_3_requires_seller_city() {
        let inv = xr();
        let report = validate(&inv);
        assert!(
            report.findings.iter().any(|f| f.id == "BR-DE-3"),
            "{report}"
        );
    }

    #[test]
    fn br_de_17_is_warning_not_fatal() {
        let mut inv = xr();
        inv.type_code = Some(Code::new("393"));
        let report = validate(&inv);
        let f = report
            .findings
            .iter()
            .find(|f| f.id == "BR-DE-17")
            .unwrap_or_else(|| panic!("{report}"));
        assert_eq!(f.severity, Severity::Warning);
    }

    #[test]
    fn br_de_18_skonto_grammar_and_trailing_newline() {
        let mut inv = xr();
        inv.payment_terms = Some("Net 30".into());
        let report = validate(&inv);
        assert!(
            report.findings.iter().all(|f| f.id != "BR-DE-18"),
            "{report}"
        );
        inv.payment_terms = Some("#SKONTO#TAGE=10#PROZENT=3#\n".into());
        let report = validate(&inv);
        assert!(
            report.findings.iter().any(|f| f.id == "BR-DE-18"),
            "{report}"
        );
        inv.payment_terms = Some("#SKONTO#TAGE=10#PROZENT=3.00#".into());
        let report = validate(&inv);
        assert!(
            report.findings.iter().any(|f| f.id == "BR-DE-18"),
            "{report}"
        );
        inv.payment_terms = Some("#SKONTO#TAGE=10#PROZENT=3.00#\n".into());
        let report = validate(&inv);
        assert!(
            report.findings.iter().all(|f| f.id != "BR-DE-18"),
            "{report}"
        );
    }

    #[test]
    fn br_de_19_20_iban_warning() {
        let mut inv = xr();
        inv.payment = Some(PaymentInstructions {
            means_code: Some(Code::new("58")),
            means_text: None,
            remittance: None,
            means: Some(PaymentMeans::CreditTransfer(vec![CreditTransfer {
                account_id: Identifier::new("DE89370400440532013001"),
                account_name: None,
                provider: None,
            }])),
        });
        let report = validate(&inv);
        let f = report
            .findings
            .iter()
            .find(|f| f.id == "BR-DE-19")
            .unwrap_or_else(|| panic!("{report}"));
        assert_eq!(f.severity, Severity::Warning);

        inv.payment = Some(PaymentInstructions {
            means_code: Some(Code::new("58")),
            means_text: None,
            remittance: None,
            means: Some(PaymentMeans::CreditTransfer(vec![CreditTransfer {
                account_id: Identifier::new("DE89370400440532013000"),
                account_name: None,
                provider: None,
            }])),
        });
        let report = validate(&inv);
        assert!(
            report.findings.iter().all(|f| f.id != "BR-DE-19"),
            "{report}"
        );

        inv.payment = Some(PaymentInstructions {
            means_code: Some(Code::new("59")),
            means_text: None,
            remittance: None,
            means: Some(PaymentMeans::DirectDebit(DirectDebit {
                mandate: Some("M-1".into()),
                creditor_id: Some(Identifier::new("DE98ZZZ09999999999")),
                debited_account: Some(Identifier::new("DE89370400440532013001")),
            })),
        });
        let report = validate(&inv);
        let f = report
            .findings
            .iter()
            .find(|f| f.id == "BR-DE-20")
            .unwrap_or_else(|| panic!("{report}"));
        assert_eq!(f.severity, Severity::Warning);
    }

    #[test]
    fn br_de_21_warns_on_old_urn() {
        let mut inv = xr();
        let report = validate(&inv);
        assert!(
            report.findings.iter().all(|f| f.id != "BR-DE-21"),
            "{report}"
        );
        inv.specification_id = Some(XR_EXTENSION_ID.into());
        let report = validate(&inv);
        assert!(
            report.findings.iter().all(|f| f.id != "BR-DE-21"),
            "{report}"
        );
        inv.specification_id = Some(XR_CVD_ID.into());
        let report = validate(&inv);
        assert!(
            report.findings.iter().all(|f| f.id != "BR-DE-21"),
            "{report}"
        );
        inv.specification_id = Some(
            "urn:cen.eu:en16931:2017#compliant#urn:xoev-de:kosit:standard:xrechnung_3.0".into(),
        );
        let report = validate(&inv);
        let f = report
            .findings
            .iter()
            .find(|f| f.id == "BR-DE-21")
            .unwrap_or_else(|| panic!("{report}"));
        assert_eq!(f.severity, Severity::Warning);
        assert!(
            report
                .findings
                .iter()
                .all(|f| !f.id.starts_with("BR-DE-CVD") && !f.id.starts_with("BR-DEX-")),
            "{report}"
        );
    }

    #[test]
    fn br_de_22_duplicate_filename() {
        let mut inv = xr();
        let att = || Attachment::new(b"%PDF".to_vec(), "application/pdf", "terms.pdf").unwrap();
        inv.supporting_documents = vec![
            SupportingDocument {
                id: DocumentReference::new("A"),
                description: None,
                uri: None,
                attachment: Some(att()),
            },
            SupportingDocument {
                id: DocumentReference::new("B"),
                description: None,
                uri: None,
                attachment: Some(att()),
            },
        ];
        let report = validate(&inv);
        assert!(
            report.findings.iter().any(|f| f.id == "BR-DE-22"),
            "{report}"
        );
    }

    #[test]
    fn br_de_26_corrected_invoice_warning() {
        let mut inv = xr();
        inv.type_code = Some(Code::new("384"));
        let report = validate(&inv);
        let f = report
            .findings
            .iter()
            .find(|f| f.id == "BR-DE-26")
            .unwrap_or_else(|| panic!("{report}"));
        assert_eq!(f.severity, Severity::Warning);
        inv.preceding = vec![PrecedingInvoice {
            reference: DocumentReference::new("INV-0"),
            issue_date: None,
        }];
        let report = validate(&inv);
        assert!(
            report.findings.iter().all(|f| f.id != "BR-DE-26"),
            "{report}"
        );
    }

    #[test]
    fn br_de_27_28_contact_shape() {
        let mut inv = xr();
        inv.seller.contact = Some(Contact {
            point: Some("AP".into()),
            phone: Some("12".into()),
            email: Some("a@b.de".into()),
        });
        let report = validate(&inv);
        let phone = report
            .findings
            .iter()
            .find(|f| f.id == "BR-DE-27")
            .unwrap_or_else(|| panic!("{report}"));
        let email = report
            .findings
            .iter()
            .find(|f| f.id == "BR-DE-28")
            .unwrap_or_else(|| panic!("{report}"));
        assert_eq!(phone.severity, Severity::Warning);
        assert_eq!(email.severity, Severity::Warning);
        inv.seller.contact = Some(Contact {
            point: Some("AP".into()),
            phone: Some("+49 30 123456".into()),
            email: Some("rechnung@seller.de".into()),
        });
        let report = validate(&inv);
        assert!(
            report
                .findings
                .iter()
                .all(|f| f.id != "BR-DE-27" && f.id != "BR-DE-28"),
            "{report}"
        );
    }

    #[test]
    fn br_de_30_31_direct_debit() {
        let mut inv = xr();
        inv.payment = Some(PaymentInstructions {
            means_code: Some(Code::new("59")),
            means_text: None,
            remittance: None,
            means: Some(PaymentMeans::DirectDebit(DirectDebit {
                mandate: Some("M-1".into()),
                creditor_id: None,
                debited_account: None,
            })),
        });
        let report = validate(&inv);
        assert!(
            report.findings.iter().any(|f| f.id == "BR-DE-30"),
            "{report}"
        );
        assert!(
            report.findings.iter().any(|f| f.id == "BR-DE-31"),
            "{report}"
        );
        inv.payment = Some(PaymentInstructions {
            means_code: Some(Code::new("59")),
            means_text: None,
            remittance: None,
            means: Some(PaymentMeans::DirectDebit(DirectDebit {
                mandate: Some("M-1".into()),
                creditor_id: Some(Identifier::new("DE98ZZZ09999999999")),
                debited_account: Some(Identifier::new("DE89370400440532013000")),
            })),
        });
        let report = validate(&inv);
        assert!(
            report
                .findings
                .iter()
                .all(|f| f.id != "BR-DE-30" && f.id != "BR-DE-31"),
            "{report}"
        );
    }

    #[test]
    fn br_de_tmp_32_is_info() {
        let inv = xr();
        let report = validate(&inv);
        let f = report
            .findings
            .iter()
            .find(|f| f.id == "BR-DE-TMP-32")
            .unwrap_or_else(|| panic!("{report}"));
        assert_eq!(f.severity, Severity::Info);
        let mut inv = inv;
        inv.delivery = Some(Delivery {
            name: None,
            location_id: None,
            date: Date::parse("2026-01-20").ok(),
            address: None,
        });
        let report = validate(&inv);
        assert!(
            report.findings.iter().all(|f| f.id != "BR-DE-TMP-32"),
            "{report}"
        );
        inv.delivery = None;
        inv.period = Some(Period {
            start: Date::parse("2026-01-01").ok(),
            end: Date::parse("2026-01-31").ok(),
        });
        let report = validate(&inv);
        assert!(
            report.findings.iter().all(|f| f.id != "BR-DE-TMP-32"),
            "{report}"
        );
    }

    #[test]
    fn br_tmp_2_absolute_url() {
        let mut inv = xr();
        inv.supporting_documents = vec![SupportingDocument {
            id: DocumentReference::new("A"),
            description: None,
            uri: Some("nabs".into()),
            attachment: None,
        }];
        let report = validate(&inv);
        assert!(
            report.findings.iter().any(|f| f.id == "BR-TMP-2"),
            "{report}"
        );
        inv.supporting_documents[0].uri = Some("https://example.com/doc".into());
        let report = validate(&inv);
        assert!(
            report.findings.iter().all(|f| f.id != "BR-TMP-2"),
            "{report}"
        );
    }

    #[test]
    fn b_halves_do_not_emit() {
        let inv = xr();
        let report = validate(&inv);
        assert!(
            report
                .findings
                .iter()
                .all(|f| f.id != "BR-DE-23-b" && f.id != "BR-DE-24-b" && f.id != "BR-DE-25-b"),
            "{report}"
        );
    }

    #[test]
    fn iban_mod_97_accepts_real_ibans_and_rejects_typos() {
        for ok in [
            "DE89370400440532013000",
            "GB82 WEST 1234 5698 7654 32",
            "FR1420041010050500013M02606",
            "NL91ABNA0417164300",
        ] {
            assert!(is_valid_iban_checksum(ok), "{ok} should be valid");
        }
        for bad in [
            "DE89370400440532013001",
            "DE8937040044053201300",
            "XX00",
            "0089370400440532013000",
            "DE89-3704-0044",
            "",
        ] {
            assert!(!is_valid_iban_checksum(bad), "{bad:?} should be invalid");
        }
    }

    #[test]
    fn br_de_28_is_a_shape_test_not_an_rfc_parser() {
        for ok in ["rechnung@seller.de", "a.b@example.co.uk"] {
            assert!(plausible_email(ok), "{ok}");
        }
        for bad in [
            "no-at-sign",
            "two@@ats.de",
            "a@b.de",
            "ab@c",
            "ab .@seller.de",
            "ab.@seller.de",
            ".ab@seller.de",
            "ab@seller.de.",
        ] {
            assert!(!plausible_email(bad), "{bad:?} should be rejected");
        }
    }

    #[test]
    fn br_de_14_rate_required_including_o() {
        let mut inv = xr();
        inv.tax_breakdown = vec![TaxBreakdown {
            system: TaxSystem::Vat,
            scheme: "VAT".into(),
            category: Code::new("O"),
            rate: None,
            taxable: InvoiceAmount::parse("10.00").unwrap(),
            tax: InvoiceAmount::parse("0.00").unwrap(),
            exemption_reason: Some("out of scope".into()),
            exemption_code: None,
        }];
        let report = validate(&inv);
        assert!(
            report.findings.iter().any(|f| f.id == "BR-DE-14"),
            "{report}"
        );
    }
}
