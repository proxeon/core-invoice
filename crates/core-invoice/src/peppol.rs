//! Peppol BIS Billing 3.0 extra_rules. Not CORE, not inherited by PINT.

use crate::bt::{BtId, Group, Path};
use crate::invoice::Invoice;
use crate::profile::Profile;
use crate::report::{Finding, Report, Severity, Source};
use crate::rules::Rule;
use rust_decimal::Decimal;

fn peppol_only(inv: &Invoice) -> bool {
    inv.profile == Profile::PeppolBis3
}

fn r001(inv: &Invoice, report: &mut Report) {
    if !peppol_only(inv) {
        return;
    }
    if inv
        .business_process
        .as_deref()
        .unwrap_or("")
        .trim()
        .is_empty()
    {
        report.push(Finding::fatal(
            "PEPPOL-EN16931-R001",
            Path::term(BtId(23)),
            "Business process type (BT-23 / ProfileID) shall be present",
        ));
    }
}

fn r007(inv: &Invoice, report: &mut Report) {
    if !peppol_only(inv) {
        return;
    }
    let Some(id) = inv.business_process.as_deref() else {
        return;
    };
    // Profile 01. Profile 02 is Later.
    if !id.starts_with("urn:fdc:peppol.eu:2017:poacc:billing:") || !id.ends_with(":1.0") {
        report.push(Finding::fatal(
            "PEPPOL-EN16931-R007",
            Path::term(BtId(23)),
            format!("BT-23 {id} is not urn:fdc:peppol.eu:2017:poacc:billing:NN:1.0"),
        ));
    }
}

fn r004(inv: &Invoice, report: &mut Report) {
    if !peppol_only(inv) {
        return;
    }
    let Some(id) = inv.specification_id.as_deref() else {
        return;
    };
    if !id.starts_with(Profile::PEPPOL_BIS3_PREFIX) {
        report.push(Finding::fatal(
            "PEPPOL-EN16931-R004",
            Path::term(BtId(24)),
            "BT-24 shall start with the official Peppol BIS Billing 3.0 identifier",
        ));
    }
}

fn r010(inv: &Invoice, report: &mut Report) {
    if !peppol_only(inv) {
        return;
    }
    // PEPPOL-EN16931-R010: Buyer EndpointID. Extra_rule, not CORE. (R020 is seller.)
    if inv
        .buyer
        .electronic_address
        .as_ref()
        .map(|i| i.value.trim())
        .unwrap_or("")
        .is_empty()
    {
        report.push(Finding::fatal(
            "PEPPOL-EN16931-R010",
            Path::term(BtId(49)),
            "Buyer electronic address MUST be provided",
        ));
    }
}

fn r020(inv: &Invoice, report: &mut Report) {
    if !peppol_only(inv) {
        return;
    }
    // PEPPOL-EN16931-R020: Seller EndpointID. Extra_rule, not CORE.
    if inv
        .seller
        .electronic_address
        .as_ref()
        .map(|i| i.value.trim())
        .unwrap_or("")
        .is_empty()
    {
        report.push(Finding::fatal(
            "PEPPOL-EN16931-R020",
            Path::term(BtId(34)),
            "Seller electronic address MUST be provided",
        ));
    }
}

fn r005(inv: &Invoice, report: &mut Report) {
    if !peppol_only(inv) {
        return;
    }
    let Some(tc) = inv.tax_currency.as_ref() else {
        return;
    };
    if tc.as_str().eq_ignore_ascii_case(&inv.currency) {
        report.push(Finding::fatal(
            "PEPPOL-EN16931-R005",
            Path::term(BtId(6)),
            "VAT accounting currency code MUST be different from invoice currency code when provided",
        ));
    }
}

fn r055(inv: &Invoice, report: &mut Report) {
    if !peppol_only(inv) {
        return;
    }
    let Some(totals) = inv.totals.as_ref() else {
        return;
    };
    let Some(acct) = totals.tax_total_accounting else {
        return;
    };
    let doc = totals
        .tax_total
        .unwrap_or(crate::amount::InvoiceAmount::ZERO);
    let doc_neg = doc.raw().is_sign_negative();
    let acct_neg = acct.raw().is_sign_negative();
    if doc.is_zero() || acct.is_zero() {
        return;
    }
    if doc_neg != acct_neg {
        report.push(Finding::fatal(
            "PEPPOL-EN16931-R055",
            Path::term(BtId(111)),
            "Invoice total VAT amount and Invoice total VAT amount in accounting currency MUST have the same operational sign",
        ));
    }
}

fn r061(inv: &Invoice, report: &mut Report) {
    if !peppol_only(inv) {
        return;
    }
    let Some(pay) = inv.payment.as_ref() else {
        return;
    };
    // PEPPOL-EN16931-R061: codes 49 and 59 (SEPA DD). Extra_rule, not CORE.
    let code = pay.means_code.as_ref().map(|c| c.as_str()).unwrap_or("");
    let is_dd = code == "49"
        || code == "59"
        || matches!(
            pay.means,
            Some(crate::payment::PaymentMeans::DirectDebit(_))
        );
    if !is_dd {
        return;
    }
    let mandate_ok = match &pay.means {
        Some(crate::payment::PaymentMeans::DirectDebit(d)) => {
            d.mandate.as_deref().is_some_and(|m| !m.trim().is_empty())
        }
        _ => false,
    };
    if !mandate_ok {
        report.push(Finding::fatal(
            "PEPPOL-EN16931-R061",
            Path::term(BtId(89)),
            "Mandate reference MUST be provided for direct debit",
        ));
    }
}

fn p0100(inv: &Invoice, report: &mut Report) {
    if !peppol_only(inv) || inv.kind != crate::kind::DocumentKind::Invoice {
        return;
    }
    let Some(code) = inv.type_code.as_ref() else {
        return;
    };
    const ALLOWED: &[&str] = &[
        "71", "80", "82", "84", "102", "218", "219", "326", "331", "380", "382", "383", "384",
        "386", "388", "393", "395", "553", "575", "623", "780", "817", "870", "875", "876", "877",
    ];
    if !ALLOWED.contains(&code.as_str()) {
        report.push(Finding::fatal(
            "PEPPOL-EN16931-P0100",
            Path::term(BtId(3)),
            format!(
                "Invoice type code {} is not allowed for Peppol billing profile 01",
                code.as_str()
            ),
        ));
    }
}

fn vatex_pair(inv: &Invoice, report: &mut Report, vatex: &str, cat: &str, id: &'static str) {
    if !peppol_only(inv) {
        return;
    }
    for (i, row) in inv.tax_breakdown.iter().enumerate() {
        let Some(code) = row.exemption_code.as_ref() else {
            continue;
        };
        if code.as_str().eq_ignore_ascii_case(vatex)
            && !row.category.as_str().eq_ignore_ascii_case(cat)
        {
            report.push(Finding::fatal(
                id,
                Path::at_term(Group::TaxBreakdown, i, BtId(121)),
                format!("Tax Category {cat} MUST be used when exemption reason code is {vatex}"),
            ));
        }
    }
}

fn p0104(i: &Invoice, r: &mut Report) {
    vatex_pair(i, r, "VATEX-EU-G", "G", "PEPPOL-EN16931-P0104");
}
fn p0105(i: &Invoice, r: &mut Report) {
    vatex_pair(i, r, "VATEX-EU-O", "O", "PEPPOL-EN16931-P0105");
}
fn p0106(i: &Invoice, r: &mut Report) {
    vatex_pair(i, r, "VATEX-EU-IC", "K", "PEPPOL-EN16931-P0106");
}
fn p0107(i: &Invoice, r: &mut Report) {
    vatex_pair(i, r, "VATEX-EU-AE", "AE", "PEPPOL-EN16931-P0107");
}
fn p0108(i: &Invoice, r: &mut Report) {
    vatex_pair(i, r, "VATEX-EU-D", "E", "PEPPOL-EN16931-P0108");
}
fn p0109(i: &Invoice, r: &mut Report) {
    vatex_pair(i, r, "VATEX-EU-F", "E", "PEPPOL-EN16931-P0109");
}
fn p0110(i: &Invoice, r: &mut Report) {
    vatex_pair(i, r, "VATEX-EU-I", "E", "PEPPOL-EN16931-P0110");
}
fn p0111(i: &Invoice, r: &mut Report) {
    vatex_pair(i, r, "VATEX-EU-J", "E", "PEPPOL-EN16931-P0111");
}

fn r003(inv: &Invoice, report: &mut Report) {
    if !peppol_only(inv) {
        return;
    }
    // PEPPOL-EN16931-R003: BT-10 or BT-13, not BG-3.
    let has_buyer_ref = inv
        .buyer_reference
        .as_ref()
        .is_some_and(|r| !r.as_str().trim().is_empty());
    let has_order = inv
        .purchase_order
        .as_ref()
        .is_some_and(|r| !r.as_str().trim().is_empty());
    if !has_buyer_ref && !has_order {
        report.push(Finding::fatal(
            "PEPPOL-EN16931-R003",
            Path::term(BtId(10)),
            "Buyer reference (BT-10) or order reference (BT-13) shall be present",
        ));
    }
}

/// ±0.02 **inclusive**. A rule instance, not a crate constant. R046 is exact.
fn r120(inv: &Invoice, report: &mut Report) {
    if !peppol_only(inv) {
        return;
    }
    let two_cents = Decimal::new(2, 2);
    for (i, line) in inv.lines.iter().enumerate() {
        let (Some(qty), Some(price)) = (line.quantity, line.price.as_ref()) else {
            continue;
        };
        let base = price
            .base_qty
            .map(|q| q.raw())
            .filter(|d| !d.is_zero())
            .unwrap_or(Decimal::ONE);
        let Some(mut expected) = qty
            .raw()
            .checked_mul(price.net.raw())
            .and_then(|v| v.checked_div(base))
        else {
            continue;
        };
        for c in &line.charges {
            let Some(v) = expected.checked_add(c.amount.raw()) else {
                continue;
            };
            expected = v;
        }
        for a in &line.allowances {
            let Some(v) = expected.checked_sub(a.amount.raw()) else {
                continue;
            };
            expected = v;
        }
        let delta = (expected - line.net.raw()).abs();
        if delta > two_cents {
            report.push(Finding::fatal(
                "PEPPOL-EN16931-R120",
                Path::at_term(Group::Line, i, BtId(131)),
                format!(
                    "BT-131 {} differs from qty×price/base by {delta} (slack ±0.02 inclusive)",
                    line.net
                ),
            ));
        }
    }
}

fn r046(inv: &Invoice, report: &mut Report) {
    if !peppol_only(inv) {
        return;
    }
    for (i, line) in inv.lines.iter().enumerate() {
        let Some(price) = line.price.as_ref() else {
            continue;
        };
        let Some(gross) = price.gross else {
            continue;
        };
        let discount = price
            .discount
            .unwrap_or(crate::amount::UnitPriceAmount::ZERO);
        let Some(expected) = gross.raw().checked_sub(discount.raw()) else {
            continue;
        };
        if expected != price.net.raw() {
            report.push(Finding::fatal(
                "PEPPOL-EN16931-R046",
                Path::at_term(Group::Line, i, BtId(146)),
                "net price = gross − discount, exact (not R120 slack)",
            ));
        }
    }
}

fn r040(inv: &Invoice, report: &mut Report) {
    if !peppol_only(inv) {
        return;
    }
    let two_cents = Decimal::new(2, 2);
    for (i, a) in inv
        .document_allowances
        .iter()
        .chain(inv.document_charges.iter())
        .enumerate()
    {
        let (Some(base), Some(pct)) = (a.base, a.percent) else {
            continue;
        };
        let Some(expected) = base
            .raw()
            .checked_mul(pct.as_percent())
            .map(|v| v / Decimal::ONE_HUNDRED)
        else {
            continue;
        };
        if (expected - a.amount.raw()).abs() > two_cents {
            report.push(Finding::fatal(
                "PEPPOL-EN16931-R040",
                Path::at_term(Group::DocumentAllowance, i, BtId(92)),
                format!(
                    "allowance/charge amount {} differs from base×percent/100 by more than 0.02",
                    a.amount
                ),
            ));
        }
    }
}

fn both_de(inv: &Invoice) -> bool {
    inv.seller.country().eq_ignore_ascii_case("DE")
        && inv.buyer.country().eq_ignore_ascii_case("DE")
}

fn r002(inv: &Invoice, report: &mut Report) {
    if !peppol_only(inv) {
        return;
    }
    // PEPPOL-EN16931-R002: at most one Note, unless both parties are DE.
    if inv.notes.len() > 1 && !both_de(inv) {
        report.push(Finding::fatal(
            "PEPPOL-EN16931-R002",
            Path::term(BtId(22)),
            "No more than one note is allowed on document level, unless both buyer and seller are German",
        ));
    }
}

fn each_ac(inv: &Invoice, mut f: impl FnMut(&crate::invoice::AllowanceCharge, Path)) {
    for (i, a) in inv.document_allowances.iter().enumerate() {
        f(a, Path::at_term(Group::DocumentAllowance, i, BtId(92)));
    }
    for (i, a) in inv.document_charges.iter().enumerate() {
        f(a, Path::at_term(Group::DocumentCharge, i, BtId(99)));
    }
}

fn r041(inv: &Invoice, report: &mut Report) {
    if !peppol_only(inv) {
        return;
    }
    // PEPPOL-EN16931-R041: base amount MUST be provided when percentage is provided.
    each_ac(inv, |a, path| {
        if a.percent.is_some() && a.base.is_none() {
            report.push(Finding::fatal(
                "PEPPOL-EN16931-R041",
                path,
                "Allowance/charge base amount MUST be provided when allowance/charge percentage is provided",
            ));
        }
    });
    for (i, line) in inv.lines.iter().enumerate() {
        for a in line.allowances.iter().chain(line.charges.iter()) {
            if a.percent.is_some() && a.base.is_none() {
                report.push(Finding::fatal(
                    "PEPPOL-EN16931-R041",
                    Path::at_term(Group::Line, i, BtId(136)),
                    "Allowance/charge base amount MUST be provided when allowance/charge percentage is provided",
                ));
            }
        }
    }
}

fn r042(inv: &Invoice, report: &mut Report) {
    if !peppol_only(inv) {
        return;
    }
    // PEPPOL-EN16931-R042: percentage MUST be provided when base amount is provided.
    each_ac(inv, |a, path| {
        if a.base.is_some() && a.percent.is_none() {
            report.push(Finding::fatal(
                "PEPPOL-EN16931-R042",
                path,
                "Allowance/charge percentage MUST be provided when allowance/charge base amount is provided",
            ));
        }
    });
    for (i, line) in inv.lines.iter().enumerate() {
        for a in line.allowances.iter().chain(line.charges.iter()) {
            if a.base.is_some() && a.percent.is_none() {
                report.push(Finding::fatal(
                    "PEPPOL-EN16931-R042",
                    Path::at_term(Group::Line, i, BtId(137)),
                    "Allowance/charge percentage MUST be provided when allowance/charge base amount is provided",
                ));
            }
        }
    }
}

fn r054(inv: &Invoice, report: &mut Report) {
    if !peppol_only(inv) {
        return;
    }
    // PEPPOL-EN16931-R054: tax total without subtotals (BT-111) iff tax currency (BT-6).
    let has_tax_ccy = inv.tax_currency.is_some();
    let has_acct = inv
        .totals
        .as_ref()
        .and_then(|t| t.tax_total_accounting)
        .is_some();
    if has_tax_ccy != has_acct {
        report.push(Finding::fatal(
            "PEPPOL-EN16931-R054",
            Path::term(BtId(111)),
            "Only one tax total without tax subtotals MUST be provided when tax currency code is provided",
        ));
    }
}

fn r101(inv: &Invoice, report: &mut Report) {
    if !peppol_only(inv) {
        return;
    }
    for (i, line) in inv.lines.iter().enumerate() {
        if line.invoiced_object.is_none() {
            continue;
        }
        let code = line
            .invoiced_object_code
            .as_ref()
            .map(crate::code::Code::as_str)
            .unwrap_or("130");
        if code != "130" {
            report.push(Finding::fatal(
                "PEPPOL-EN16931-R101",
                Path::at_term(Group::Line, i, BtId(128)),
                "Element Document reference can only be used for Invoice line object (code 130)",
            ));
        }
    }
}

fn r110(inv: &Invoice, report: &mut Report) {
    if !peppol_only(inv) {
        return;
    }
    let Some(start) = inv.period.as_ref().and_then(|p| p.start) else {
        return;
    };
    for (i, line) in inv.lines.iter().enumerate() {
        if let Some(ls) = line.period.as_ref().and_then(|p| p.start)
            && ls < start
        {
            report.push(Finding::fatal(
                "PEPPOL-EN16931-R110",
                Path::at_term(Group::Line, i, BtId(134)),
                "Start date of line period MUST be within invoice period",
            ));
        }
    }
}

fn r111(inv: &Invoice, report: &mut Report) {
    if !peppol_only(inv) {
        return;
    }
    let Some(end) = inv.period.as_ref().and_then(|p| p.end) else {
        return;
    };
    for (i, line) in inv.lines.iter().enumerate() {
        if let Some(le) = line.period.as_ref().and_then(|p| p.end)
            && le > end
        {
            report.push(Finding::fatal(
                "PEPPOL-EN16931-R111",
                Path::at_term(Group::Line, i, BtId(135)),
                "End date of line period MUST be within invoice period",
            ));
        }
    }
}

fn r130(inv: &Invoice, report: &mut Report) {
    if !peppol_only(inv) {
        return;
    }
    for (i, line) in inv.lines.iter().enumerate() {
        let Some(price) = line.price.as_ref() else {
            continue;
        };
        let (Some(bu), Some(u)) = (price.base_unit.as_ref(), line.unit.as_ref()) else {
            continue;
        };
        if bu.as_str() != u.as_str() {
            report.push(Finding::fatal(
                "PEPPOL-EN16931-R130",
                Path::at_term(Group::Line, i, BtId(150)),
                "Unit code of price base quantity MUST be same as invoiced quantity",
            ));
        }
    }
}

fn cl001(inv: &Invoice, report: &mut Report) {
    if !peppol_only(inv) {
        return;
    }
    for (i, doc) in inv.supporting_documents.iter().enumerate() {
        let Some(att) = doc.attachment.as_ref() else {
            continue;
        };
        if !crate::codes::mime(&att.mime) {
            report.push(Finding::fatal(
                "PEPPOL-EN16931-CL001",
                Path::at_term(Group::Attachment, i, BtId(125)),
                "Mime code must be according to subset of IANA code list",
            ));
        }
    }
}

fn cl002(inv: &Invoice, report: &mut Report) {
    if !peppol_only(inv) {
        return;
    }
    for (i, a) in inv.document_allowances.iter().enumerate() {
        let Some(code) = a.reason_code.as_ref() else {
            continue;
        };
        if !crate::generated_codes::UNCL_5189.contains(&code.as_str()) {
            report.push(Finding::fatal(
                "PEPPOL-EN16931-CL002",
                Path::at_term(Group::DocumentAllowance, i, BtId(98)),
                "Reason code MUST be according to subset of UNCL 5189 D.16B",
            ));
        }
    }
}

fn cl003(inv: &Invoice, report: &mut Report) {
    if !peppol_only(inv) {
        return;
    }
    for (i, a) in inv.document_charges.iter().enumerate() {
        let Some(code) = a.reason_code.as_ref() else {
            continue;
        };
        if !crate::generated_codes::UNCL_7161.contains(&code.as_str()) {
            report.push(Finding::fatal(
                "PEPPOL-EN16931-CL003",
                Path::at_term(Group::DocumentCharge, i, BtId(105)),
                "Reason code MUST be according to UNCL 7161 D.16B",
            ));
        }
    }
}

fn cl006(inv: &Invoice, report: &mut Report) {
    if !peppol_only(inv) {
        return;
    }
    let Some(code) = inv.tax_point_code.as_ref() else {
        return;
    };
    if !crate::generated_codes::UNCL_2005.contains(&code.as_str()) {
        report.push(Finding::fatal(
            "PEPPOL-EN16931-CL006",
            Path::term(BtId(8)),
            "Invoice period description code must be according to UNCL 2005 D.16B",
        ));
    }
}

fn cl008(inv: &Invoice, report: &mut Report) {
    if !peppol_only(inv) {
        return;
    }
    for (party, group, bt) in [
        (&inv.seller, Group::Seller, 34u16),
        (&inv.buyer, Group::Buyer, 49u16),
    ] {
        let Some(ep) = party.electronic_address.as_ref() else {
            continue;
        };
        let Some(scheme) = ep.scheme.as_deref() else {
            continue;
        };
        if !crate::codes::eas(scheme) {
            report.push(Finding::fatal(
                "PEPPOL-EN16931-CL008",
                Path::group_term(group, BtId(bt)),
                "Electronic address identifier scheme must be from the Electronic Address Identifier Scheme list",
            ));
        }
    }
}

fn f001(_inv: &Invoice, _report: &mut Report) {
    // PEPPOL-EN16931-F001: dates are YYYY-MM-DD. Date::parse already refuses other shapes.
}

fn syntax_or_option_pass(_inv: &Invoice, _report: &mut Report) {
    // Syntax-only or Option-at-most-one: explainable so SVRL unmatched is intentional, not missing from catalogue.
}

fn p0101(inv: &Invoice, report: &mut Report) {
    if !peppol_only(inv) || inv.kind != crate::kind::DocumentKind::CreditNote {
        return;
    }
    let Some(code) = inv.type_code.as_ref() else {
        return;
    };
    const ALLOWED: &[&str] = &["381", "396", "81", "83", "532"];
    if !ALLOWED.contains(&code.as_str()) {
        report.push(Finding::fatal(
            "PEPPOL-EN16931-P0101",
            Path::term(BtId(3)),
            format!(
                "Credit note type code {} is not allowed for Peppol billing profile 01",
                code.as_str()
            ),
        ));
    }
}

fn p0112(inv: &Invoice, report: &mut Report) {
    if !peppol_only(inv) {
        return;
    }
    let Some(code) = inv.type_code.as_ref() else {
        return;
    };
    if matches!(code.as_str(), "326" | "384") && !both_de(inv) {
        report.push(Finding::fatal(
            "PEPPOL-EN16931-P0112",
            Path::term(BtId(3)),
            "Invoice type code 326 or 384 are only allowed when both buyer and seller are German organizations",
        ));
    }
}

/// GS1 check digit (PEPPOL-COMMON-R040 / u:gln). Any length ≥ 2; weight 3 from the right of the data digits.
fn gln_ok(s: &str) -> bool {
    let s = s.trim();
    if s.len() < 2 || !s.bytes().all(|b| b.is_ascii_digit()) {
        return false;
    }
    let digits: Vec<u32> = s.bytes().map(|b| u32::from(b - b'0')).collect();
    let n = digits.len();
    let mut sum = 0u32;
    for (i, d) in digits[..n - 1].iter().rev().enumerate() {
        sum += d * if i % 2 == 0 { 3 } else { 1 };
    }
    let check = (10 - (sum % 10)) % 10;
    digits[n - 1] == check
}

fn common_r040(inv: &Invoice, report: &mut Report) {
    if !peppol_only(inv) {
        return;
    }
    for (party, group, bt) in [
        (&inv.seller, Group::Seller, 34u16),
        (&inv.buyer, Group::Buyer, 49u16),
    ] {
        let Some(ep) = party.electronic_address.as_ref() else {
            continue;
        };
        if ep.scheme.as_deref() != Some("0088") {
            continue;
        }
        if !gln_ok(&ep.value) {
            report.push(Finding::fatal(
                "PEPPOL-COMMON-R040",
                Path::group_term(group, BtId(bt)),
                "GLN must have a valid format according to GS1 rules",
            ));
        }
    }
}

fn r121(inv: &Invoice, report: &mut Report) {
    if !peppol_only(inv) {
        return;
    }
    for (i, line) in inv.lines.iter().enumerate() {
        let Some(price) = line.price.as_ref() else {
            continue;
        };
        if let Some(q) = price.base_qty
            && (!q.raw().is_sign_positive() || q.raw().is_zero())
        {
            report.push(Finding::fatal(
                "PEPPOL-EN16931-R121",
                Path::at_term(Group::Line, i, BtId(149)),
                "base quantity shall be greater than zero",
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

pub static RULES: &[Rule] = &[
    r("PEPPOL-EN16931-R001", "BT-23 shall be present.", r001),
    r(
        "PEPPOL-EN16931-R007",
        "BT-23 shall match urn:fdc:peppol.eu:2017:poacc:billing:NN:1.0.",
        r007,
    ),
    r(
        "PEPPOL-EN16931-R004",
        "BT-24 shall start with the official Peppol BIS Billing 3.0 id.",
        r004,
    ),
    r(
        "PEPPOL-EN16931-R003",
        "BT-10 or BT-13 shall be present.",
        r003,
    ),
    r(
        "PEPPOL-EN16931-R010",
        "Buyer electronic address MUST be provided.",
        r010,
    ),
    r(
        "PEPPOL-EN16931-R020",
        "Seller electronic address MUST be provided.",
        r020,
    ),
    r(
        "PEPPOL-EN16931-R005",
        "VAT accounting currency MUST differ from invoice currency when provided.",
        r005,
    ),
    r(
        "PEPPOL-EN16931-R055",
        "BT-110 and BT-111 MUST have the same operational sign.",
        r055,
    ),
    r(
        "PEPPOL-EN16931-R061",
        "Mandate reference MUST be provided for direct debit.",
        r061,
    ),
    r(
        "PEPPOL-EN16931-P0100",
        "Invoice type code must be in the Peppol billing profile 01 list (not 389).",
        p0100,
    ),
    r(
        "PEPPOL-EN16931-P0104",
        "VATEX-EU-G requires tax category G.",
        p0104,
    ),
    r(
        "PEPPOL-EN16931-P0105",
        "VATEX-EU-O requires tax category O.",
        p0105,
    ),
    r(
        "PEPPOL-EN16931-P0106",
        "VATEX-EU-IC requires tax category K.",
        p0106,
    ),
    r(
        "PEPPOL-EN16931-P0107",
        "VATEX-EU-AE requires tax category AE.",
        p0107,
    ),
    r(
        "PEPPOL-EN16931-P0108",
        "VATEX-EU-D requires tax category E.",
        p0108,
    ),
    r(
        "PEPPOL-EN16931-P0109",
        "VATEX-EU-F requires tax category E.",
        p0109,
    ),
    r(
        "PEPPOL-EN16931-P0110",
        "VATEX-EU-I requires tax category E.",
        p0110,
    ),
    r(
        "PEPPOL-EN16931-P0111",
        "VATEX-EU-J requires tax category E.",
        p0111,
    ),
    r(
        "PEPPOL-EN16931-R120",
        "Line net ≈ qty × (price / base qty), slack ±0.02 inclusive.",
        r120,
    ),
    r(
        "PEPPOL-EN16931-R040",
        "Allowance/charge amount ≈ base × percent/100, slack ±0.02 inclusive.",
        r040,
    ),
    r(
        "PEPPOL-EN16931-R046",
        "Net price = gross − discount, exact.",
        r046,
    ),
    r(
        "PEPPOL-EN16931-R121",
        "Base quantity shall be greater than zero.",
        r121,
    ),
    r(
        "PEPPOL-EN16931-R002",
        "No more than one note on document level unless both parties are German.",
        r002,
    ),
    r(
        "PEPPOL-EN16931-R041",
        "Allowance/charge base amount MUST be provided when percentage is provided.",
        r041,
    ),
    r(
        "PEPPOL-EN16931-R042",
        "Allowance/charge percentage MUST be provided when base amount is provided.",
        r042,
    ),
    r(
        "PEPPOL-EN16931-R054",
        "Tax total without subtotals (BT-111) iff tax currency (BT-6).",
        r054,
    ),
    r(
        "PEPPOL-EN16931-R101",
        "Line document reference is only for invoiced object (code 130).",
        r101,
    ),
    r(
        "PEPPOL-EN16931-R110",
        "Line period start MUST be within invoice period.",
        r110,
    ),
    r(
        "PEPPOL-EN16931-R111",
        "Line period end MUST be within invoice period.",
        r111,
    ),
    r(
        "PEPPOL-EN16931-R130",
        "Price base quantity unit MUST equal invoiced quantity unit.",
        r130,
    ),
    r(
        "PEPPOL-EN16931-CL001",
        "Attachment mime code must be from the IANA subset.",
        cl001,
    ),
    r(
        "PEPPOL-EN16931-CL002",
        "Allowance reason code MUST be UNCL 5189.",
        cl002,
    ),
    r(
        "PEPPOL-EN16931-CL003",
        "Charge reason code MUST be UNCL 7161.",
        cl003,
    ),
    r(
        "PEPPOL-EN16931-CL006",
        "Invoice period description code MUST be UNCL 2005.",
        cl006,
    ),
    r(
        "PEPPOL-EN16931-CL008",
        "Endpoint scheme MUST be from the Electronic Address Identifier Scheme list.",
        cl008,
    ),
    r(
        "PEPPOL-EN16931-F001",
        "A date MUST be formatted YYYY-MM-DD (enforced by Date).",
        f001,
    ),
    r(
        "PEPPOL-EN16931-P0101",
        "Credit note type code must be in the Peppol billing profile 01 list.",
        p0101,
    ),
    r(
        "PEPPOL-EN16931-P0112",
        "Invoice type 326 or 384 only when both parties are German.",
        p0112,
    ),
    r(
        "PEPPOL-COMMON-R040",
        "GLN (EAS 0088) must have a valid GS1 check digit.",
        common_r040,
    ),
    r(
        "PEPPOL-EN16931-R006",
        "CII-only: at most one invoiced object. UBL is Invoice.invoiced_object: Option.",
        syntax_or_option_pass,
    ),
    r(
        "PEPPOL-EN16931-R008",
        "Empty XML elements are forbidden (syntax walk, not the semantic model).",
        syntax_or_option_pass,
    ),
    r(
        "PEPPOL-EN16931-R043",
        "ChargeIndicator must be true or false. Model uses two vecs; writer emits the boolean.",
        syntax_or_option_pass,
    ),
    r(
        "PEPPOL-EN16931-R044",
        "Price-level charge is forbidden. Price has discount only.",
        syntax_or_option_pass,
    ),
    r(
        "PEPPOL-EN16931-R051",
        "@currencyID on amounts must equal BT-5 except BT-111 (wire-only).",
        syntax_or_option_pass,
    ),
    r(
        "PEPPOL-EN16931-R053",
        "Exactly one TaxTotal with subtotals. Model has one tax_breakdown vec.",
        syntax_or_option_pass,
    ),
    r(
        "PEPPOL-EN16931-R080",
        "At most one project reference. Invoice.project is Option.",
        syntax_or_option_pass,
    ),
    r(
        "PEPPOL-EN16931-R100",
        "At most one line DocumentReference. Line.invoiced_object is Option.",
        syntax_or_option_pass,
    ),
    r(
        "PEPPOL-EN16931-CL007",
        "@currencyID must be ISO 4217 (wire). CORE BR-CL-04 covers BT-5.",
        syntax_or_option_pass,
    ),
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::amount::InvoiceAmount;
    use crate::code::Code;
    use crate::date::Date;
    use crate::identifier::Identifier;
    use crate::invoice::{Line, Party, Price};
    use crate::numeric::Quantity;
    use crate::reconcile::reconcile;
    use crate::tax::TaxCategory;
    use crate::validate;
    use rust_decimal::Decimal;

    fn peppol() -> Invoice {
        let mut inv = Invoice::blank(
            Profile::PeppolBis3,
            "EU-1",
            "EUR",
            {
                let mut p = Party::new("S", "DE");
                p.vat_identifier = Some(Identifier::new("DE123456789"));
                p
            },
            {
                let mut b = Party::new("B", "FR");
                b.vat_identifier = Some(Identifier::new("FR12345678901"));
                b
            },
        );
        inv.issue_date = Date::parse("2026-01-15").ok();
        inv.type_code = Some(Code::new("380"));
        inv.payment_terms = Some("Net 30".into());
        inv.business_process = Some("urn:fdc:peppol.eu:2017:poacc:billing:01:1.0".into());
        inv.buyer_reference = Some(crate::identifier::DocumentReference::new("PO-1"));
        inv.seller.electronic_address = Some(Identifier::schemed("1234567890128", "0088"));
        inv.buyer.electronic_address = Some(Identifier::schemed("1234567890135", "0088"));
        inv.lines = vec![{
            let mut line = Line::new(
                "1",
                "A",
                InvoiceAmount::parse("100.00").unwrap(),
                TaxCategory::vat("S", Decimal::from(19)),
            );
            line.quantity = Some(Quantity::parse("1").unwrap());
            line.unit = Some(Code::new("C62"));
            line.price = Some(Price {
                net: crate::amount::UnitPriceAmount::parse("100.00").unwrap(),
                discount: None,
                gross: None,
                base_qty: None,
                base_unit: None,
            });
            line
        }];
        reconcile(&mut inv).unwrap();
        inv
    }

    #[test]
    fn r120_fails_at_three_cents_not_on_en16931() {
        let mut inv = peppol();
        inv.lines[0].quantity = Some(Quantity::parse("1").unwrap());
        inv.lines[0].price = Some(Price {
            net: crate::amount::UnitPriceAmount::parse("100.03").unwrap(),
            discount: None,
            gross: None,
            base_qty: None,
            base_unit: None,
        });
        let report = validate(&inv);
        assert!(
            report
                .findings
                .iter()
                .any(|f| f.id == "PEPPOL-EN16931-R120"),
            "{report}"
        );
        inv.profile = Profile::En16931;
        inv.specification_id = Some(Profile::En16931.specification_id().into());
        let report = validate(&inv);
        assert!(
            report
                .findings
                .iter()
                .all(|f| f.id != "PEPPOL-EN16931-R120"),
            "{report}"
        );
    }

    #[test]
    fn r120_passes_at_two_cents() {
        let mut inv = peppol();
        inv.lines[0].quantity = Some(Quantity::parse("1").unwrap());
        inv.lines[0].price = Some(Price {
            net: crate::amount::UnitPriceAmount::parse("100.02").unwrap(),
            discount: None,
            gross: None,
            base_qty: None,
            base_unit: None,
        });
        let report = validate(&inv);
        assert!(
            report
                .findings
                .iter()
                .all(|f| f.id != "PEPPOL-EN16931-R120"),
            "{report}"
        );
    }

    #[test]
    fn r010_buyer_endpoint_not_on_en() {
        let mut inv = peppol();
        inv.buyer.electronic_address = None;
        let report = validate(&inv);
        assert!(
            report
                .findings
                .iter()
                .any(|f| f.id == "PEPPOL-EN16931-R010"),
            "{report}"
        );
        inv.profile = Profile::En16931;
        inv.specification_id = Some(Profile::En16931.specification_id().into());
        let report = validate(&inv);
        assert!(
            report
                .findings
                .iter()
                .all(|f| f.id != "PEPPOL-EN16931-R010"),
            "{report}"
        );
    }

    #[test]
    fn p0100_forbids_389_on_peppol_not_en() {
        let mut inv = peppol();
        inv.type_code = Some(Code::new("389"));
        let report = validate(&inv);
        assert!(
            report
                .findings
                .iter()
                .any(|f| f.id == "PEPPOL-EN16931-P0100"),
            "{report}"
        );
        inv.profile = Profile::En16931;
        inv.specification_id = Some(Profile::En16931.specification_id().into());
        let report = validate(&inv);
        assert!(
            report
                .findings
                .iter()
                .all(|f| f.id != "PEPPOL-EN16931-P0100"),
            "{report}"
        );
    }

    #[test]
    fn r003_bt13_not_preceding() {
        let mut inv = peppol();
        inv.buyer_reference = None;
        inv.preceding.push(crate::invoice::PrecedingInvoice {
            reference: crate::identifier::DocumentReference::new("INV-OLD"),
            issue_date: None,
        });
        let report = validate(&inv);
        assert!(
            report
                .findings
                .iter()
                .any(|f| f.id == "PEPPOL-EN16931-R003"),
            "BG-3 alone must not satisfy R003: {report}"
        );
        inv.purchase_order = Some(crate::identifier::DocumentReference::new("PO-13"));
        let report = validate(&inv);
        assert!(
            report
                .findings
                .iter()
                .all(|f| f.id != "PEPPOL-EN16931-R003"),
            "{report}"
        );
    }

    #[test]
    fn r120_includes_line_charges_minus_allowances() {
        let mut inv = peppol();
        inv.lines[0].quantity = Some(Quantity::parse("1").unwrap());
        inv.lines[0].price = Some(Price {
            net: crate::amount::UnitPriceAmount::parse("100.00").unwrap(),
            discount: None,
            gross: None,
            base_qty: None,
            base_unit: None,
        });
        inv.lines[0]
            .charges
            .push(crate::invoice::LineAllowanceCharge {
                amount: InvoiceAmount::parse("5.00").unwrap(),
                base: None,
                percent: None,
                reason: None,
                reason_code: None,
            });
        inv.lines[0].net = InvoiceAmount::parse("105.00").unwrap();
        let _ = reconcile(&mut inv);
        let report = validate(&inv);
        assert!(
            report
                .findings
                .iter()
                .all(|f| f.id != "PEPPOL-EN16931-R120"),
            "{report}"
        );
    }

    #[test]
    fn pint_my_does_not_run_peppol_r001() {
        let mut inv = peppol();
        inv.profile = Profile::PintMy;
        inv.specification_id = Some(Profile::PintMy.specification_id().into());
        inv.business_process = None;
        inv.seller.legal_registration = Some(Identifier::new("2023010000001"));
        inv.seller.tax_registration = Some(Identifier::new("C12345678901"));
        inv.buyer.legal_registration = Some(Identifier::new("1999010000001"));
        inv.lines[0].tax = TaxCategory::sst("SA", Decimal::from(10));
        let _ = reconcile(&mut inv);
        let report = validate(&inv);
        assert!(
            report
                .findings
                .iter()
                .all(|f| !f.id.starts_with("PEPPOL-EN16931")),
            "{report}"
        );
    }

    #[test]
    fn r002_two_notes_fail_unless_both_de() {
        let mut inv = peppol();
        inv.notes = vec![
            crate::invoice::InvoiceNote {
                subject: None,
                text: "a".into(),
            },
            crate::invoice::InvoiceNote {
                subject: None,
                text: "b".into(),
            },
        ];
        let report = validate(&inv);
        assert!(
            report
                .findings
                .iter()
                .any(|f| f.id == "PEPPOL-EN16931-R002"),
            "{report}"
        );
        inv.buyer = {
            let mut b = crate::invoice::Party::new("B", "DE");
            b.vat_identifier = Some(Identifier::new("DE000"));
            b.electronic_address = Some(Identifier::schemed("1234567890135", "0088"));
            b
        };
        let report = validate(&inv);
        assert!(
            report
                .findings
                .iter()
                .all(|f| f.id != "PEPPOL-EN16931-R002"),
            "{report}"
        );
    }

    #[test]
    fn r041_percent_without_base() {
        let mut inv = peppol();
        inv.document_charges.push(crate::invoice::AllowanceCharge {
            amount: InvoiceAmount::parse("1.00").unwrap(),
            base: None,
            percent: Some(crate::numeric::Percentage::new(Decimal::from(10))),
            reason: None,
            reason_code: None,
            tax: Some(TaxCategory::vat("S", Decimal::from(19))),
        });
        let report = validate(&inv);
        assert!(
            report
                .findings
                .iter()
                .any(|f| f.id == "PEPPOL-EN16931-R041"),
            "{report}"
        );
    }

    #[test]
    fn r101_rejects_non_130() {
        let mut inv = peppol();
        inv.lines[0].invoiced_object = Some(Identifier::new("OBJ"));
        inv.lines[0].invoiced_object_code = Some(Code::new("50"));
        let report = validate(&inv);
        assert!(
            report
                .findings
                .iter()
                .any(|f| f.id == "PEPPOL-EN16931-R101"),
            "{report}"
        );
    }

    #[test]
    fn p0101_forbids_380_on_credit_note() {
        let mut inv = peppol();
        inv.kind = crate::kind::DocumentKind::CreditNote;
        inv.type_code = Some(Code::new("380"));
        let report = validate(&inv);
        assert!(
            report
                .findings
                .iter()
                .any(|f| f.id == "PEPPOL-EN16931-P0101"),
            "{report}"
        );
    }

    #[test]
    fn p0112_326_needs_both_de() {
        let mut inv = peppol();
        inv.type_code = Some(Code::new("326"));
        let report = validate(&inv);
        assert!(
            report
                .findings
                .iter()
                .any(|f| f.id == "PEPPOL-EN16931-P0112"),
            "{report}"
        );
    }

    #[test]
    fn common_r040_bad_gln() {
        let mut inv = peppol();
        inv.seller.electronic_address = Some(Identifier::schemed("1234567890129", "0088"));
        let report = validate(&inv);
        assert!(
            report.findings.iter().any(|f| f.id == "PEPPOL-COMMON-R040"),
            "{report}"
        );
    }

    #[test]
    fn r046_one_cent_fails_exact() {
        let mut inv = peppol();
        inv.lines[0].price = Some(Price {
            net: crate::amount::UnitPriceAmount::parse("100.00").unwrap(),
            discount: Some(crate::amount::UnitPriceAmount::parse("1.00").unwrap()),
            gross: Some(crate::amount::UnitPriceAmount::parse("100.99").unwrap()),
            base_qty: None,
            base_unit: None,
        });
        let report = validate(&inv);
        assert!(
            report
                .findings
                .iter()
                .any(|f| f.id == "PEPPOL-EN16931-R046"),
            "{report}"
        );
    }

    #[test]
    fn r061_fires_for_means_code_59_without_mandate() {
        let mut inv = peppol();
        inv.payment = Some(crate::invoice::PaymentInstructions {
            means_code: Some(Code::new("59")),
            means_text: None,
            remittance: None,
            means: None,
        });
        let report = validate(&inv);
        assert!(
            report
                .findings
                .iter()
                .any(|f| f.id == "PEPPOL-EN16931-R061"),
            "{report}"
        );
    }
}
