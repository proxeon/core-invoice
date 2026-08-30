//! Nine VAT category families plus PINT-MY aligned tables.
//!
//! Finding ids are real (`BR-S-08`, `ALIGNED-IBRP-SA-09`). SST never emits
//! `BR-S-*`. Reconcile groups with [`grouped_by_rate`] from this table.

use rust_decimal::Decimal;

use crate::amount::InvoiceAmount;
use crate::arith::{derived_vat, within_vat_tolerance};
use crate::bt::{BtId, Group, Path};
use crate::invoice::Invoice;
use crate::numeric::Percentage;
use crate::profile::Profile;
use crate::report::{Finding, Report, Severity, Source};
use crate::rules::Rule;
use crate::tax::TaxSystem;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VatCategory {
    Standard,
    ZeroRated,
    Exempt,
    ReverseCharge,
    IntraCommunity,
    Export,
    OutOfScope,
    CanaryIslands,
    CeutaMelilla,
    SplitPayment,
}

impl VatCategory {
    pub fn code(self) -> &'static str {
        match self {
            Self::Standard => "S",
            Self::ZeroRated => "Z",
            Self::Exempt => "E",
            Self::ReverseCharge => "AE",
            Self::IntraCommunity => "K",
            Self::Export => "G",
            Self::OutOfScope => "O",
            Self::CanaryIslands => "L",
            Self::CeutaMelilla => "M",
            Self::SplitPayment => "B",
        }
    }

    pub fn parse(code: &str) -> Option<Self> {
        Some(match code {
            "S" | "s" => Self::Standard,
            "Z" | "z" => Self::ZeroRated,
            "E" | "e" => Self::Exempt,
            "AE" | "ae" => Self::ReverseCharge,
            "K" | "k" => Self::IntraCommunity,
            "G" | "g" => Self::Export,
            "O" | "o" => Self::OutOfScope,
            "L" | "l" => Self::CanaryIslands,
            "M" | "m" => Self::CeutaMelilla,
            "B" | "b" => Self::SplitPayment,
            _ => return None,
        })
    }

    pub fn requires_exemption_reason(self) -> bool {
        matches!(
            self,
            Self::Exempt
                | Self::ReverseCharge
                | Self::IntraCommunity
                | Self::Export
                | Self::OutOfScope
        )
    }

    pub fn forbids_exemption_reason(self) -> bool {
        matches!(
            self,
            Self::Standard | Self::ZeroRated | Self::CanaryIslands | Self::CeutaMelilla
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Groups {
    AtLeastOne,
    ExactlyOne,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RateRule {
    Positive,
    Zero,
    ZeroOrPositive,
    Absent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaxRule {
    Zero,
    Derived,
}

#[derive(Debug, Clone, Copy)]
pub struct CategoryProfile {
    pub category: VatCategory,
    pub groups: Groups,
    pub rate: RateRule,
    pub tax: TaxRule,
}

impl CategoryProfile {
    pub const fn grouped_by_rate(self) -> bool {
        matches!(self.groups, Groups::AtLeastOne)
    }
}

pub const fn profile(category: VatCategory) -> CategoryProfile {
    use Groups::{AtLeastOne, ExactlyOne};
    use RateRule::{Absent, Positive, Zero as RZero, ZeroOrPositive};
    use TaxRule::{Derived, Zero as TZero};
    use VatCategory::*;
    let (groups, rate, tax) = match category {
        Standard => (AtLeastOne, Positive, Derived),
        CanaryIslands | CeutaMelilla => (AtLeastOne, ZeroOrPositive, Derived),
        ZeroRated | Exempt | ReverseCharge | IntraCommunity | Export => (ExactlyOne, RZero, TZero),
        OutOfScope => (ExactlyOne, Absent, TZero),
        SplitPayment => (AtLeastOne, ZeroOrPositive, Derived),
    };
    CategoryProfile {
        category,
        groups,
        rate,
        tax,
    }
}

/// Shared with [`crate::reconcile`]: which families key BG-23 on rate.
pub fn grouped_by_rate(profile_id: Profile, category: &str) -> bool {
    if profile_id == Profile::PintMy {
        return matches!(
            category,
            "SA" | "SE" | "HVG" | "LVG" | "sa" | "se" | "hvg" | "lvg"
        );
    }
    if let Some(c) = VatCategory::parse(category) {
        return profile(c).grouped_by_rate();
    }
    !matches!(category, "O" | "Z" | "E" | "ZR" | "o" | "z" | "e" | "zr")
}

fn families_ready(inv: &Invoice) -> bool {
    inv.totals.is_some() || !inv.tax_breakdown.is_empty()
}

fn vat_families_apply(inv: &Invoice) -> bool {
    families_ready(inv) && !matches!(inv.profile, Profile::PintMy | Profile::Unknown)
}

fn my_families_apply(inv: &Invoice) -> bool {
    families_ready(inv) && inv.profile == Profile::PintMy
}

fn uses_category(inv: &Invoice, cat: VatCategory) -> bool {
    let code = cat.code();
    inv.lines
        .iter()
        .any(|l| l.tax.system == TaxSystem::Vat && l.tax.code.eq_ignore_ascii_case(code))
        || inv.document_allowances.iter().any(|a| {
            a.tax
                .as_ref()
                .is_some_and(|t| t.system == TaxSystem::Vat && t.code.eq_ignore_ascii_case(code))
        })
        || inv.document_charges.iter().any(|c| {
            c.tax
                .as_ref()
                .is_some_and(|t| t.system == TaxSystem::Vat && t.code.eq_ignore_ascii_case(code))
        })
}

fn breakdown_of(
    inv: &Invoice,
    cat: VatCategory,
) -> impl Iterator<Item = (usize, &crate::invoice::TaxBreakdown)> {
    let code = cat.code();
    inv.tax_breakdown
        .iter()
        .enumerate()
        .filter(move |(_, e)| e.category.as_str().eq_ignore_ascii_case(code))
}

fn check_groups(inv: &Invoice, report: &mut Report, p: CategoryProfile, id: &'static str) {
    if !vat_families_apply(inv) || !uses_category(inv, p.category) {
        return;
    }
    let n = breakdown_of(inv, p.category).count();
    let ok = match p.groups {
        Groups::AtLeastOne => n >= 1,
        Groups::ExactlyOne => n == 1,
    };
    if !ok {
        report.push(Finding::fatal(
            id,
            Path::group(Group::TaxBreakdown),
            format!(
                "category {} requires {:?} BG-23 group(s), found {n}",
                p.category.code(),
                p.groups
            ),
        ));
    }
}

fn rate_ok(rule: RateRule, rate: Option<Percentage>) -> bool {
    match rule {
        RateRule::Positive => rate.is_some_and(Percentage::is_positive),
        RateRule::Zero => rate.is_some_and(Percentage::is_zero),
        RateRule::ZeroOrPositive => rate.is_some_and(|r| !r.is_negative()),
        RateRule::Absent => rate.is_none() || rate.is_some_and(Percentage::is_zero),
    }
}

fn check_rate_line(inv: &Invoice, report: &mut Report, p: CategoryProfile, id: &'static str) {
    if !vat_families_apply(inv) {
        return;
    }
    let code = p.category.code();
    for (i, line) in inv.lines.iter().enumerate() {
        if line.tax.system != TaxSystem::Vat || !line.tax.code.eq_ignore_ascii_case(code) {
            continue;
        }
        let rate = if p.category == VatCategory::OutOfScope {
            None
        } else {
            Some(line.tax.percent)
        };
        if !rate_ok(p.rate, rate) && p.category != VatCategory::OutOfScope {
            if !rate_ok(p.rate, Some(line.tax.percent)) {
                report.push(Finding::fatal(
                    id,
                    Path::at_term(Group::Line, i, BtId(152)),
                    format!("BT-152 rate {} is not valid for {}", line.tax.percent, code),
                ));
            }
        } else if p.category == VatCategory::OutOfScope && line.tax.percent.is_positive() {
            report.push(Finding::fatal(
                id,
                Path::at_term(Group::Line, i, BtId(152)),
                "category O shall not contain a positive rate",
            ));
        }
    }
}

fn seller_vat(inv: &Invoice) -> bool {
    inv.seller.vat_identifier.is_some()
}
fn seller_tax(inv: &Invoice) -> bool {
    inv.seller.tax_registration.is_some()
}
fn rep_vat(inv: &Invoice) -> bool {
    inv.tax_representative
        .as_ref()
        .is_some_and(|r| r.vat_identifier.is_some())
}
fn buyer_vat(inv: &Invoice) -> bool {
    inv.buyer.vat_identifier.is_some()
}

fn check_identifiers(inv: &Invoice, report: &mut Report, p: CategoryProfile, id: &'static str) {
    if !vat_families_apply(inv) || !uses_category(inv, p.category) {
        return;
    }
    let ok = match p.category {
        VatCategory::Export => seller_vat(inv) || rep_vat(inv),
        VatCategory::ReverseCharge => {
            (seller_vat(inv) || seller_tax(inv) || rep_vat(inv))
                && (buyer_vat(inv) || inv.buyer.legal_registration.is_some())
        }
        VatCategory::IntraCommunity => (seller_vat(inv) || rep_vat(inv)) && buyer_vat(inv),
        VatCategory::OutOfScope => !seller_vat(inv) && !rep_vat(inv) && !buyer_vat(inv),
        _ => seller_vat(inv) || seller_tax(inv) || rep_vat(inv),
    };
    if !ok {
        report.push(Finding::fatal(
            id,
            Path::group_term(Group::Seller, BtId(31)),
            format!(
                "tax identifier requirement for category {} is not met",
                p.category.code()
            ),
        ));
    }
}

fn line_matches(
    inv: &Invoice,
    e: &crate::invoice::TaxBreakdown,
    p: CategoryProfile,
) -> impl Fn(&crate::tax::TaxCategory) -> bool {
    let cat = p.category;
    let grouped = p.grouped_by_rate();
    let entry_rate = e.rate;
    let _ = inv;
    move |t: &crate::tax::TaxCategory| {
        t.system == TaxSystem::Vat
            && t.code.eq_ignore_ascii_case(cat.code())
            && (!grouped || Some(t.percent) == entry_rate)
    }
}

fn check_taxable(inv: &Invoice, report: &mut Report, p: CategoryProfile, id: &'static str) {
    if !vat_families_apply(inv) {
        return;
    }
    for (i, e) in breakdown_of(inv, p.category) {
        let matches = line_matches(inv, e, p);
        let lines = inv.lines.iter().filter(|l| matches(&l.tax)).map(|l| l.net);
        let charges = inv
            .document_charges
            .iter()
            .filter(|c| c.tax.as_ref().is_some_and(&matches))
            .map(|c| c.amount);
        let allowances = inv
            .document_allowances
            .iter()
            .filter(|a| a.tax.as_ref().is_some_and(&matches))
            .map(|a| a.amount);
        let Some(pos) = InvoiceAmount::checked_sum(lines.chain(charges)) else {
            continue;
        };
        let Some(neg) = InvoiceAmount::checked_sum(allowances) else {
            continue;
        };
        let Some(expected) = pos.checked_sub(neg) else {
            continue;
        };
        if !within_vat_tolerance(e.taxable.raw(), expected.raw()) {
            report.push(Finding::fatal(
                id,
                Path::at_term(Group::TaxBreakdown, i, BtId(116)),
                format!(
                    "BT-116 {} is not within ±1.00 of group sum {expected}",
                    e.taxable
                ),
            ));
        }
    }
}

fn check_tax(inv: &Invoice, report: &mut Report, p: CategoryProfile, id: &'static str) {
    if !vat_families_apply(inv) {
        return;
    }
    for (i, e) in breakdown_of(inv, p.category) {
        let path = Path::at_term(Group::TaxBreakdown, i, BtId(117));
        match p.tax {
            TaxRule::Zero => {
                if !e.tax.is_zero() {
                    report.push(Finding::fatal(
                        id,
                        path,
                        format!("BT-117 shall be 0 for category {}", p.category.code()),
                    ));
                }
            }
            TaxRule::Derived => {
                let rate = e.rate.map_or(Decimal::ZERO, Percentage::as_percent);
                let Some(expected) = derived_vat(e.taxable.raw(), rate) else {
                    continue;
                };
                if !within_vat_tolerance(e.tax.raw().abs(), expected) {
                    report.push(Finding::fatal(
                        id,
                        path,
                        format!("BT-117 {} is not derived from BT-116 × rate", e.tax),
                    ));
                }
            }
        }
    }
}

fn check_exemption(inv: &Invoice, report: &mut Report, p: CategoryProfile, id: &'static str) {
    if !vat_families_apply(inv) {
        return;
    }
    for (i, e) in breakdown_of(inv, p.category) {
        let has = e
            .exemption_reason
            .as_ref()
            .is_some_and(|s| !s.trim().is_empty())
            || e.exemption_code.as_ref().is_some_and(|c| !c.is_empty());
        let bad = (p.category.requires_exemption_reason() && !has)
            || (p.category.forbids_exemption_reason() && has);
        if bad {
            report.push(Finding::fatal(
                id,
                Path::at_term(Group::TaxBreakdown, i, BtId(120)),
                format!("exemption reason rule {} failed", id),
            ));
        }
    }
}

fn check_o_exclusive(inv: &Invoice, report: &mut Report) {
    if !vat_families_apply(inv) || !uses_category(inv, VatCategory::OutOfScope) {
        return;
    }
    let other = inv
        .lines
        .iter()
        .any(|l| l.tax.system == TaxSystem::Vat && !l.tax.code.eq_ignore_ascii_case("O"));
    if other {
        report.push(Finding::fatal(
            "BR-O-11",
            Path::group(Group::TaxBreakdown),
            "category O shall not be mixed with other VAT categories",
        ));
    }
}

fn check_b_not_with_s(inv: &Invoice, report: &mut Report) {
    if !vat_families_apply(inv) {
        return;
    }
    if uses_category(inv, VatCategory::SplitPayment) && uses_category(inv, VatCategory::Standard) {
        report.push(Finding::fatal(
            "BR-B-02",
            Path::group(Group::TaxBreakdown),
            "category B cannot coexist with S",
        ));
    }
}

fn br_co_18(inv: &Invoice, report: &mut Report) {
    if inv.tax_breakdown.is_empty() && inv.totals.is_some() {
        report.push(Finding::fatal(
            "BR-CO-18",
            Path::group(Group::TaxBreakdown),
            "An Invoice shall at least have one tax breakdown group (BG-23)",
        ));
    }
}

fn my_uses(inv: &Invoice, code: &str) -> bool {
    inv.lines
        .iter()
        .any(|l| l.tax.code.eq_ignore_ascii_case(code))
}

fn check_my_groups(inv: &Invoice, report: &mut Report, code: &str, id: &'static str) {
    if !my_families_apply(inv) || !my_uses(inv, code) {
        return;
    }
    let n = inv
        .tax_breakdown
        .iter()
        .filter(|e| e.category.as_str().eq_ignore_ascii_case(code))
        .count();
    if n == 0 {
        report.push(Finding::fatal(
            id,
            Path::group(Group::TaxBreakdown),
            format!("PINT-MY category {code} needs at least one IBG-23 group"),
        ));
    }
}

fn check_my_taxable(inv: &Invoice, report: &mut Report, code: &str, id: &'static str) {
    if !my_families_apply(inv) {
        return;
    }
    for (i, e) in inv
        .tax_breakdown
        .iter()
        .enumerate()
        .filter(|(_, e)| e.category.as_str().eq_ignore_ascii_case(code))
    {
        let expected = InvoiceAmount::checked_sum(
            inv.lines
                .iter()
                .filter(|l| {
                    l.tax.code.eq_ignore_ascii_case(code)
                        && (e.rate.is_none() || Some(l.tax.percent) == e.rate)
                })
                .map(|l| l.net),
        );
        let Some(expected) = expected else {
            continue;
        };
        if e.taxable != expected {
            report.push(Finding::fatal(
                id,
                Path::at_term(Group::TaxBreakdown, i, BtId(116)),
                format!("IBT-116 {} ≠ Σ lines {expected}", e.taxable),
            ));
        }
    }
}

fn check_my_tax(inv: &Invoice, report: &mut Report, code: &str, id: &'static str, derived: bool) {
    if !my_families_apply(inv) {
        return;
    }
    for (i, e) in inv
        .tax_breakdown
        .iter()
        .enumerate()
        .filter(|(_, e)| e.category.as_str().eq_ignore_ascii_case(code))
    {
        let path = Path::at_term(Group::TaxBreakdown, i, BtId(117));
        if !derived {
            if !e.tax.is_zero() && !code.eq_ignore_ascii_case("TTX") {
                report.push(Finding::fatal(
                    id,
                    path,
                    format!("IBT-117 shall be 0 for {code}"),
                ));
            }
            if code.eq_ignore_ascii_case("TTX") && e.tax != e.taxable {
                report.push(Finding::fatal(
                    id,
                    path,
                    "TTX tax amount shall equal Σ TTX lines + charges − allowances",
                ));
            }
            continue;
        }
        let rate = e.rate.map_or(Decimal::ZERO, Percentage::as_percent);
        let Some(expected) = derived_vat(e.taxable.raw(), rate) else {
            continue;
        };
        if !within_vat_tolerance(e.tax.raw().abs(), expected) {
            report.push(Finding::fatal(
                id,
                path,
                format!("IBT-117 {} ≠ IBT-116 × IBT-119 / 100", e.tax),
            ));
        }
    }
}

fn check_my_no_exemption(inv: &Invoice, report: &mut Report, code: &str, id: &'static str) {
    if !my_families_apply(inv) {
        return;
    }
    for (i, e) in inv
        .tax_breakdown
        .iter()
        .enumerate()
        .filter(|(_, e)| e.category.as_str().eq_ignore_ascii_case(code))
    {
        if e.exemption_reason.is_some() || e.exemption_code.is_some() {
            report.push(Finding::fatal(
                id,
                Path::at_term(Group::TaxBreakdown, i, BtId(120)),
                format!("{code} shall not carry an exemption reason"),
            ));
        }
    }
}

fn check_my_o_exclusive(inv: &Invoice, report: &mut Report) {
    if !my_families_apply(inv) || !my_uses(inv, "O") {
        return;
    }
    if inv
        .lines
        .iter()
        .any(|l| !l.tax.code.eq_ignore_ascii_case("O"))
    {
        report.push(Finding::fatal(
            "ALIGNED-IBRP-O-11-MY",
            Path::group(Group::TaxBreakdown),
            "PINT-MY category O is exclusive",
        ));
    }
}

macro_rules! vat_row {
    ($fn:ident, $id:literal, $cat:ident, $checker:ident) => {
        fn $fn(inv: &Invoice, report: &mut Report) {
            $checker(inv, report, profile(VatCategory::$cat), $id);
        }
    };
}

vat_row!(br_s_01, "BR-S-01", Standard, check_groups);
vat_row!(br_s_02, "BR-S-02", Standard, check_identifiers);
vat_row!(br_s_05, "BR-S-05", Standard, check_rate_line);
vat_row!(br_s_08, "BR-S-08", Standard, check_taxable);
vat_row!(br_s_09, "BR-S-09", Standard, check_tax);
vat_row!(br_s_10, "BR-S-10", Standard, check_exemption);

vat_row!(br_z_01, "BR-Z-01", ZeroRated, check_groups);
vat_row!(br_z_02, "BR-Z-02", ZeroRated, check_identifiers);
vat_row!(br_z_05, "BR-Z-05", ZeroRated, check_rate_line);
vat_row!(br_z_08, "BR-Z-08", ZeroRated, check_taxable);
vat_row!(br_z_09, "BR-Z-09", ZeroRated, check_tax);
vat_row!(br_z_10, "BR-Z-10", ZeroRated, check_exemption);

vat_row!(br_e_01, "BR-E-01", Exempt, check_groups);
vat_row!(br_e_02, "BR-E-02", Exempt, check_identifiers);
vat_row!(br_e_05, "BR-E-05", Exempt, check_rate_line);
vat_row!(br_e_08, "BR-E-08", Exempt, check_taxable);
vat_row!(br_e_09, "BR-E-09", Exempt, check_tax);
vat_row!(br_e_10, "BR-E-10", Exempt, check_exemption);

vat_row!(br_ae_01, "BR-AE-01", ReverseCharge, check_groups);
vat_row!(br_ae_02, "BR-AE-02", ReverseCharge, check_identifiers);
vat_row!(br_ae_05, "BR-AE-05", ReverseCharge, check_rate_line);
vat_row!(br_ae_08, "BR-AE-08", ReverseCharge, check_taxable);
vat_row!(br_ae_09, "BR-AE-09", ReverseCharge, check_tax);
vat_row!(br_ae_10, "BR-AE-10", ReverseCharge, check_exemption);

vat_row!(br_ic_01, "BR-IC-01", IntraCommunity, check_groups);
vat_row!(br_ic_02, "BR-IC-02", IntraCommunity, check_identifiers);
vat_row!(br_ic_05, "BR-IC-05", IntraCommunity, check_rate_line);
vat_row!(br_ic_08, "BR-IC-08", IntraCommunity, check_taxable);
vat_row!(br_ic_09, "BR-IC-09", IntraCommunity, check_tax);
vat_row!(br_ic_10, "BR-IC-10", IntraCommunity, check_exemption);

vat_row!(br_g_01, "BR-G-01", Export, check_groups);
vat_row!(br_g_02, "BR-G-02", Export, check_identifiers);
vat_row!(br_g_05, "BR-G-05", Export, check_rate_line);
vat_row!(br_g_08, "BR-G-08", Export, check_taxable);
vat_row!(br_g_09, "BR-G-09", Export, check_tax);
vat_row!(br_g_10, "BR-G-10", Export, check_exemption);

vat_row!(br_o_01, "BR-O-01", OutOfScope, check_groups);
vat_row!(br_o_02, "BR-O-02", OutOfScope, check_identifiers);
vat_row!(br_o_05, "BR-O-05", OutOfScope, check_rate_line);
vat_row!(br_o_08, "BR-O-08", OutOfScope, check_taxable);
vat_row!(br_o_09, "BR-O-09", OutOfScope, check_tax);
vat_row!(br_o_10, "BR-O-10", OutOfScope, check_exemption);

vat_row!(br_af_01, "BR-AF-01", CanaryIslands, check_groups);
vat_row!(br_af_02, "BR-AF-02", CanaryIslands, check_identifiers);
vat_row!(br_af_05, "BR-AF-05", CanaryIslands, check_rate_line);
vat_row!(br_af_08, "BR-AF-08", CanaryIslands, check_taxable);
vat_row!(br_af_09, "BR-AF-09", CanaryIslands, check_tax);
vat_row!(br_af_10, "BR-AF-10", CanaryIslands, check_exemption);

vat_row!(br_ag_01, "BR-AG-01", CeutaMelilla, check_groups);
vat_row!(br_ag_02, "BR-AG-02", CeutaMelilla, check_identifiers);
vat_row!(br_ag_05, "BR-AG-05", CeutaMelilla, check_rate_line);
vat_row!(br_ag_08, "BR-AG-08", CeutaMelilla, check_taxable);
vat_row!(br_ag_09, "BR-AG-09", CeutaMelilla, check_tax);
vat_row!(br_ag_10, "BR-AG-10", CeutaMelilla, check_exemption);

fn br_b_01(inv: &Invoice, report: &mut Report) {
    check_groups(inv, report, profile(VatCategory::SplitPayment), "BR-B-01");
}

fn br_s_03(inv: &Invoice, report: &mut Report) {
    check_identifiers(inv, report, profile(VatCategory::Standard), "BR-S-03");
}
fn br_s_04(inv: &Invoice, report: &mut Report) {
    check_identifiers(inv, report, profile(VatCategory::Standard), "BR-S-04");
}
fn br_s_06(inv: &Invoice, report: &mut Report) {
    check_rate_line(inv, report, profile(VatCategory::Standard), "BR-S-06");
}
fn br_s_07(inv: &Invoice, report: &mut Report) {
    check_rate_line(inv, report, profile(VatCategory::Standard), "BR-S-07");
}

fn my_sa_01(i: &Invoice, r: &mut Report) {
    check_my_groups(i, r, "SA", "ALIGNED-IBRP-SA-01-MY");
}
fn my_sa_08(i: &Invoice, r: &mut Report) {
    check_my_taxable(i, r, "SA", "ALIGNED-IBRP-SA-08-MY");
}
fn my_sa_09(i: &Invoice, r: &mut Report) {
    check_my_tax(i, r, "SA", "ALIGNED-IBRP-SA-09-MY", true);
}
fn my_sa_10(i: &Invoice, r: &mut Report) {
    check_my_no_exemption(i, r, "SA", "ALIGNED-IBRP-SA-10-MY");
}
fn my_se_08(i: &Invoice, r: &mut Report) {
    check_my_taxable(i, r, "SE", "ALIGNED-IBRP-SE-08-MY");
}
fn my_se_09(i: &Invoice, r: &mut Report) {
    check_my_tax(i, r, "SE", "ALIGNED-IBRP-SE-09-MY", true);
}
fn my_hvg_08(i: &Invoice, r: &mut Report) {
    check_my_taxable(i, r, "HVG", "ALIGNED-IBRP-HVG-08-MY");
}
fn my_hvg_09(i: &Invoice, r: &mut Report) {
    check_my_tax(i, r, "HVG", "ALIGNED-IBRP-HVG-09-MY", true);
}
fn my_lvg_08(i: &Invoice, r: &mut Report) {
    check_my_taxable(i, r, "LVG", "ALIGNED-IBRP-LVG-08-MY");
}
fn my_lvg_09(i: &Invoice, r: &mut Report) {
    check_my_tax(i, r, "LVG", "ALIGNED-IBRP-LVG-09-MY", true);
}
fn my_e_09(i: &Invoice, r: &mut Report) {
    check_my_tax(i, r, "E", "ALIGNED-IBRP-E-09-MY", false);
}
fn my_ttx_09(i: &Invoice, r: &mut Report) {
    check_my_tax(i, r, "TTX", "ALIGNED-IBRP-TTX-09-MY", false);
}

const fn r(id: &'static str, text: &'static str, eval: fn(&Invoice, &mut Report)) -> Rule {
    Rule {
        id,
        severity: Severity::Fatal,
        text,
        source: Source::Both,
        eval,
    }
}

const fn my(id: &'static str, text: &'static str, eval: fn(&Invoice, &mut Report)) -> Rule {
    Rule {
        id,
        severity: Severity::Fatal,
        text,
        source: Source::Crate,
        eval,
    }
}

pub static RULES: &[Rule] = &[
    r(
        "BR-CO-18",
        "An Invoice shall at least have one tax breakdown group (BG-23).",
        br_co_18,
    ),
    r(
        "BR-S-01",
        "Standard VAT: at least one BG-23 group per used rate.",
        br_s_01,
    ),
    r(
        "BR-S-02",
        "Standard VAT: seller tax identifier (BT-31, BT-32 or BT-63).",
        br_s_02,
    ),
    r(
        "BR-S-03",
        "Standard VAT: identifier requirement on document allowance.",
        br_s_03,
    ),
    r(
        "BR-S-04",
        "Standard VAT: identifier requirement on document charge.",
        br_s_04,
    ),
    r(
        "BR-S-05",
        "Standard VAT: line rate (BT-152) greater than zero.",
        br_s_05,
    ),
    r(
        "BR-S-06",
        "Standard VAT: allowance rate greater than zero.",
        br_s_06,
    ),
    r(
        "BR-S-07",
        "Standard VAT: charge rate greater than zero.",
        br_s_07,
    ),
    r(
        "BR-S-08",
        "Standard VAT: BT-116 = Σ line net + charges − allowances in the group (±1.00 signed).",
        br_s_08,
    ),
    r(
        "BR-S-09",
        "Standard VAT: BT-117 derived from BT-116 × rate (±1.00 abs).",
        br_s_09,
    ),
    r(
        "BR-S-10",
        "Standard VAT: exemption reason forbidden.",
        br_s_10,
    ),
    r(
        "BR-Z-01",
        "Zero-rated VAT: exactly one BG-23 group.",
        br_z_01,
    ),
    r("BR-Z-02", "Zero-rated VAT: seller tax identifier.", br_z_02),
    r("BR-Z-05", "Zero-rated VAT: rate = 0.", br_z_05),
    r("BR-Z-08", "Zero-rated VAT: BT-116 group sum.", br_z_08),
    r("BR-Z-09", "Zero-rated VAT: BT-117 = 0.", br_z_09),
    r(
        "BR-Z-10",
        "Zero-rated VAT: exemption reason forbidden.",
        br_z_10,
    ),
    r("BR-E-01", "Exempt VAT: exactly one BG-23 group.", br_e_01),
    r("BR-E-02", "Exempt VAT: seller tax identifier.", br_e_02),
    r("BR-E-05", "Exempt VAT: rate = 0.", br_e_05),
    r("BR-E-08", "Exempt VAT: BT-116 group sum.", br_e_08),
    r("BR-E-09", "Exempt VAT: BT-117 = 0.", br_e_09),
    r("BR-E-10", "Exempt VAT: exemption reason required.", br_e_10),
    r(
        "BR-AE-01",
        "Reverse charge: exactly one BG-23 group.",
        br_ae_01,
    ),
    r(
        "BR-AE-02",
        "Reverse charge: seller and buyer identifiers.",
        br_ae_02,
    ),
    r("BR-AE-05", "Reverse charge: rate = 0.", br_ae_05),
    r("BR-AE-08", "Reverse charge: BT-116 group sum.", br_ae_08),
    r("BR-AE-09", "Reverse charge: BT-117 = 0.", br_ae_09),
    r(
        "BR-AE-10",
        "Reverse charge: exemption reason required.",
        br_ae_10,
    ),
    r(
        "BR-IC-01",
        "Intra-community: exactly one BG-23 group.",
        br_ic_01,
    ),
    r(
        "BR-IC-02",
        "Intra-community: seller VAT and buyer VAT.",
        br_ic_02,
    ),
    r("BR-IC-05", "Intra-community: rate = 0.", br_ic_05),
    r("BR-IC-08", "Intra-community: BT-116 group sum.", br_ic_08),
    r("BR-IC-09", "Intra-community: BT-117 = 0.", br_ic_09),
    r(
        "BR-IC-10",
        "Intra-community: exemption reason required.",
        br_ic_10,
    ),
    r("BR-G-01", "Export: exactly one BG-23 group.", br_g_01),
    r(
        "BR-G-02",
        "Export: seller VAT identifier (BT-31 or BT-63).",
        br_g_02,
    ),
    r("BR-G-05", "Export: rate = 0.", br_g_05),
    r("BR-G-08", "Export: BT-116 group sum.", br_g_08),
    r("BR-G-09", "Export: BT-117 = 0.", br_g_09),
    r("BR-G-10", "Export: exemption reason required.", br_g_10),
    r("BR-O-01", "Out of scope: exactly one BG-23 group.", br_o_01),
    r(
        "BR-O-02",
        "Out of scope: VAT identifiers shall not be present.",
        br_o_02,
    ),
    r("BR-O-05", "Out of scope: rate absent.", br_o_05),
    r("BR-O-08", "Out of scope: BT-116 group sum.", br_o_08),
    r("BR-O-09", "Out of scope: BT-117 = 0.", br_o_09),
    r(
        "BR-O-10",
        "Out of scope: exemption reason required.",
        br_o_10,
    ),
    r(
        "BR-O-11",
        "Out of scope is exclusive of other VAT categories.",
        check_o_exclusive,
    ),
    r("BR-AF-01", "IGIC: at least one BG-23 group.", br_af_01),
    r("BR-AF-02", "IGIC: seller tax identifier.", br_af_02),
    r("BR-AF-05", "IGIC: rate ≥ 0.", br_af_05),
    r("BR-AF-08", "IGIC: BT-116 group sum.", br_af_08),
    r("BR-AF-09", "IGIC: derived tax.", br_af_09),
    r("BR-AF-10", "IGIC: exemption reason forbidden.", br_af_10),
    r("BR-AG-01", "IPSI: at least one BG-23 group.", br_ag_01),
    r("BR-AG-02", "IPSI: seller tax identifier.", br_ag_02),
    r("BR-AG-05", "IPSI: rate ≥ 0.", br_ag_05),
    r("BR-AG-08", "IPSI: BT-116 group sum.", br_ag_08),
    r("BR-AG-09", "IPSI: derived tax.", br_ag_09),
    r("BR-AG-10", "IPSI: exemption reason forbidden.", br_ag_10),
    r(
        "BR-B-01",
        "Split payment: at least one BG-23 group.",
        br_b_01,
    ),
    r(
        "BR-B-02",
        "Split payment cannot coexist with standard rated S.",
        check_b_not_with_s,
    ),
    my(
        "ALIGNED-IBRP-SA-01-MY",
        "PINT-MY SA: at least one IBG-23 group.",
        my_sa_01,
    ),
    my(
        "ALIGNED-IBRP-SA-08-MY",
        "PINT-MY SA: IBT-116 = Σ SA lines.",
        my_sa_08,
    ),
    my(
        "ALIGNED-IBRP-SA-09-MY",
        "PINT-MY SA: IBT-117 = IBT-116 × IBT-119 / 100.",
        my_sa_09,
    ),
    my(
        "ALIGNED-IBRP-SA-10-MY",
        "PINT-MY SA: exemption reason forbidden.",
        my_sa_10,
    ),
    my(
        "ALIGNED-IBRP-SE-08-MY",
        "PINT-MY SE: IBT-116 = Σ SE lines.",
        my_se_08,
    ),
    my(
        "ALIGNED-IBRP-SE-09-MY",
        "PINT-MY SE: tax from rate.",
        my_se_09,
    ),
    my(
        "ALIGNED-IBRP-HVG-08-MY",
        "PINT-MY HVG: IBT-116 group sum.",
        my_hvg_08,
    ),
    my(
        "ALIGNED-IBRP-HVG-09-MY",
        "PINT-MY HVG: tax from rate.",
        my_hvg_09,
    ),
    my(
        "ALIGNED-IBRP-LVG-08-MY",
        "PINT-MY LVG: IBT-116 group sum.",
        my_lvg_08,
    ),
    my(
        "ALIGNED-IBRP-LVG-09-MY",
        "PINT-MY LVG: tax from rate.",
        my_lvg_09,
    ),
    my("ALIGNED-IBRP-E-09-MY", "PINT-MY E: tax = 0.", my_e_09),
    my(
        "ALIGNED-IBRP-TTX-09-MY",
        "PINT-MY TTX: amount = Σ TTX lines.",
        my_ttx_09,
    ),
    my(
        "ALIGNED-IBRP-O-11-MY",
        "PINT-MY O is exclusive.",
        check_my_o_exclusive,
    ),
];

/// PINT GST subset (not invented): S, Z, AA, O, plus SG SR on Pint only.
pub fn pint_gst_category(code: &str) -> bool {
    matches!(code, "S" | "Z" | "AA" | "O" | "SR" | "ZR")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::amount::InvoiceAmount;
    use crate::code::Code;
    use crate::date::Date;
    use crate::identifier::Identifier;
    use crate::invoice::{Invoice, Line, Party, TaxBreakdown};
    use crate::reconcile::reconcile;
    use crate::tax::TaxCategory;
    use crate::validate;

    fn amt(s: &str) -> InvoiceAmount {
        InvoiceAmount::parse(s).unwrap()
    }

    fn en_s() -> Invoice {
        let mut inv = Invoice::blank(
            Profile::En16931,
            "INV-1",
            "EUR",
            {
                let mut p = Party::new("S", "DE");
                p.vat_identifier = Some(Identifier::new("DE123456789"));
                p
            },
            Party::new("B", "FR"),
        );
        inv.issue_date = Date::parse("2026-01-15").ok();
        inv.type_code = Some(Code::new("380"));
        inv.lines = vec![Line::new(
            "1",
            "A",
            amt("100.00"),
            TaxCategory::vat("S", Decimal::from(19)),
        )];
        reconcile(&mut inv).unwrap();
        inv
    }

    #[test]
    fn wrong_bt116_fails_br_s_08() {
        let mut inv = en_s();
        inv.tax_breakdown[0].taxable = amt("1.00");
        let report = validate(&inv);
        assert!(
            report.findings.iter().any(|f| f.id == "BR-S-08"),
            "{report}"
        );
    }

    #[test]
    fn exempt_without_reason_fails_br_e_10() {
        let mut inv = en_s();
        inv.lines[0].tax = TaxCategory::vat("E", Decimal::from(0));
        reconcile(&mut inv).unwrap();
        let report = validate(&inv);
        assert!(
            report.findings.iter().any(|f| f.id == "BR-E-10"),
            "{report}"
        );
    }

    #[test]
    fn zero_rated_with_exemption_fails_br_z_10() {
        let mut inv = en_s();
        inv.lines[0].tax = TaxCategory::vat("Z", Decimal::from(0));
        reconcile(&mut inv).unwrap();
        inv.tax_breakdown[0].exemption_reason = Some("no".into());
        let report = validate(&inv);
        assert!(
            report.findings.iter().any(|f| f.id == "BR-Z-10"),
            "{report}"
        );
    }

    #[test]
    fn o_mixed_with_s_fails_exclusivity() {
        let mut inv = en_s();
        inv.lines.push(Line::new(
            "2",
            "Out",
            amt("10.00"),
            TaxCategory::vat("O", Decimal::from(0)),
        ));
        reconcile(&mut inv).unwrap();
        let report = validate(&inv);
        assert!(
            report.findings.iter().any(|f| f.id == "BR-O-11"),
            "{report}"
        );
    }

    #[test]
    fn sst_does_not_emit_br_s_08() {
        let mut inv = Invoice::blank(
            Profile::PintMy,
            "MY-1",
            "MYR",
            {
                let mut p = Party::new("Kedai", "MY");
                p.tax_registration = Some(Identifier::new("C12345678901"));
                p.legal_registration = Some(Identifier::new("2023010000001"));
                p
            },
            {
                let mut b = Party::new("Pembeli", "MY");
                b.legal_registration = Some(Identifier::new("1999010000001"));
                b
            },
        );
        inv.issue_date = Date::parse("2026-01-15").ok();
        inv.type_code = Some(Code::new("380"));
        inv.lines = vec![Line::new(
            "1",
            "W",
            amt("100.00"),
            TaxCategory::sst("SA", Decimal::from(10)),
        )];
        inv.tax_breakdown = vec![TaxBreakdown {
            system: TaxSystem::Sst,
            scheme: "VAT".into(),
            category: Code::new("SA"),
            rate: Some(Percentage::new(Decimal::from(10))),
            taxable: amt("1.00"),
            tax: amt("10.00"),
            exemption_reason: None,
            exemption_code: None,
        }];
        inv.totals = Some(crate::invoice::DocumentTotals {
            line_net: Some(amt("100.00")),
            allowance_total: None,
            charge_total: None,
            without_tax: Some(amt("100.00")),
            tax_total: Some(amt("10.00")),
            tax_total_accounting: None,
            with_tax: Some(amt("110.00")),
            paid: None,
            rounding: None,
            payable: amt("110.00"),
        });
        let report = validate(&inv);
        assert!(
            report.findings.iter().all(|f| f.id != "BR-S-08"),
            "{report}"
        );
        assert!(
            report
                .findings
                .iter()
                .any(|f| f.id == "ALIGNED-IBRP-SA-08-MY"),
            "{report}"
        );
    }
}
