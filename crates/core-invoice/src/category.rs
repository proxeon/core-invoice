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

/// Which repeating group a family row applies to. Artefacts number
/// line (`-02`/`-05`), allowance (`-03`/`-06`), and charge (`-04`/`-07`) separately.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RateContext {
    Line,
    Allowance,
    Charge,
}

fn uses_category(inv: &Invoice, cat: VatCategory) -> bool {
    uses_in(inv, cat, RateContext::Line)
        || uses_in(inv, cat, RateContext::Allowance)
        || uses_in(inv, cat, RateContext::Charge)
}

fn uses_in(inv: &Invoice, cat: VatCategory, ctx: RateContext) -> bool {
    let code = cat.code();
    match ctx {
        RateContext::Line => inv
            .lines
            .iter()
            .any(|l| l.tax.system == TaxSystem::Vat && l.tax.code.eq_ignore_ascii_case(code)),
        RateContext::Allowance => inv.document_allowances.iter().any(|a| {
            a.tax
                .as_ref()
                .is_some_and(|t| t.system == TaxSystem::Vat && t.code.eq_ignore_ascii_case(code))
        }),
        RateContext::Charge => inv.document_charges.iter().any(|c| {
            c.tax
                .as_ref()
                .is_some_and(|t| t.system == TaxSystem::Vat && t.code.eq_ignore_ascii_case(code))
        }),
    }
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
            line.tax.percent
        };
        if !rate_ok(p.rate, rate) && p.category != VatCategory::OutOfScope {
            if !rate_ok(p.rate, line.tax.percent) {
                report.push(Finding::fatal(
                    id,
                    Path::at_term(Group::Line, i, BtId(152)),
                    format!(
                        "BT-152 rate {:?} is not valid for {}",
                        line.tax.percent, code
                    ),
                ));
            }
        } else if p.category == VatCategory::OutOfScope
            && line.tax.percent.is_some_and(Percentage::is_positive)
        {
            report.push(Finding::fatal(
                id,
                Path::at_term(Group::Line, i, BtId(152)),
                "category O shall not contain a positive rate",
            ));
        }
    }
}

fn check_rate_ac(
    inv: &Invoice,
    report: &mut Report,
    p: CategoryProfile,
    id: &'static str,
    ctx: RateContext,
) {
    if !vat_families_apply(inv) {
        return;
    }
    let code = p.category.code();
    let rows: Vec<(usize, Option<Percentage>)> = match ctx {
        RateContext::Allowance => inv
            .document_allowances
            .iter()
            .enumerate()
            .filter_map(|(i, a)| {
                let t = a.tax.as_ref()?;
                (t.system == TaxSystem::Vat && t.code.eq_ignore_ascii_case(code))
                    .then_some((i, t.percent))
            })
            .collect(),
        RateContext::Charge => inv
            .document_charges
            .iter()
            .enumerate()
            .filter_map(|(i, a)| {
                let t = a.tax.as_ref()?;
                (t.system == TaxSystem::Vat && t.code.eq_ignore_ascii_case(code))
                    .then_some((i, t.percent))
            })
            .collect(),
        RateContext::Line => return,
    };
    let group = match ctx {
        RateContext::Allowance => Group::DocumentAllowance,
        RateContext::Charge => Group::DocumentCharge,
        RateContext::Line => Group::Line,
    };
    for (i, rate) in rows {
        if !rate_ok(p.rate, rate) {
            report.push(Finding::fatal(
                id,
                Path::at_term(group, i, BtId(96)),
                format!("rate {rate:?} is not valid for {code} in this context"),
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
    check_identifiers_in(inv, report, p, id, RateContext::Line);
}

fn check_identifiers_in(
    inv: &Invoice,
    report: &mut Report,
    p: CategoryProfile,
    id: &'static str,
    ctx: RateContext,
) {
    if !vat_families_apply(inv) || !uses_in(inv, p.category, ctx) {
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
            && (!grouped || t.percent == entry_rate)
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
                format!("exemption reason rule {id} failed"),
            ));
        }
    }
}

fn o_group_present(inv: &Invoice) -> bool {
    inv.tax_breakdown
        .iter()
        .any(|e| e.category.as_str().eq_ignore_ascii_case("O"))
}

fn br_o_11(inv: &Invoice, report: &mut Report) {
    if !vat_families_apply(inv) || !o_group_present(inv) {
        return;
    }
    // BR-O-11: O group forbids other BG-23 groups.
    let other_groups = inv
        .tax_breakdown
        .iter()
        .any(|e| !e.category.as_str().eq_ignore_ascii_case("O"));
    if other_groups {
        report.push(Finding::fatal(
            "BR-O-11",
            Path::group(Group::TaxBreakdown),
            "An Invoice with VAT category O shall not contain other VAT breakdown groups",
        ));
    }
}

fn br_o_12(inv: &Invoice, report: &mut Report) {
    if !vat_families_apply(inv) || !o_group_present(inv) {
        return;
    }
    // BR-O-12: O group forbids non-O lines.
    if inv
        .lines
        .iter()
        .any(|l| l.tax.system == TaxSystem::Vat && !l.tax.code.eq_ignore_ascii_case("O"))
    {
        report.push(Finding::fatal(
            "BR-O-12",
            Path::group(Group::Line),
            "An Invoice with VAT category O shall not contain a line that is not O",
        ));
    }
}

fn br_o_13(inv: &Invoice, report: &mut Report) {
    if !vat_families_apply(inv) || !o_group_present(inv) {
        return;
    }
    if inv.document_allowances.iter().any(|a| {
        a.tax
            .as_ref()
            .is_some_and(|t| t.system == TaxSystem::Vat && !t.code.eq_ignore_ascii_case("O"))
    }) {
        report.push(Finding::fatal(
            "BR-O-13",
            Path::group(Group::DocumentAllowance),
            "An Invoice with VAT category O shall not contain a document allowance that is not O",
        ));
    }
}

fn br_o_14(inv: &Invoice, report: &mut Report) {
    if !vat_families_apply(inv) || !o_group_present(inv) {
        return;
    }
    if inv.document_charges.iter().any(|a| {
        a.tax
            .as_ref()
            .is_some_and(|t| t.system == TaxSystem::Vat && !t.code.eq_ignore_ascii_case("O"))
    }) {
        report.push(Finding::fatal(
            "BR-O-14",
            Path::group(Group::DocumentCharge),
            "An Invoice with VAT category O shall not contain a document charge that is not O",
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
    // BR-CO-18: at least one BG-23. Not gated on the caller having run reconcile.
    if inv.tax_breakdown.is_empty() && !inv.lines.is_empty() {
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

fn line_has_ttx(line: &crate::invoice::Line) -> bool {
    line.tax.code.eq_ignore_ascii_case("TTX")
        || line
            .extra_tax
            .iter()
            .any(|t| t.code.eq_ignore_ascii_case("TTX"))
}

fn ttx_line_tax_sum(inv: &Invoice) -> Decimal {
    inv.lines
        .iter()
        .filter(|l| line_has_ttx(l))
        .filter_map(|l| l.tax_total)
        .map(|a| a.raw())
        .fold(Decimal::ZERO, |acc, v| acc + v)
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
        // ALIGNED-IBRP-*-08-MY: exact IBT-116 vs lines + charges − allowances (same as reconcile).
        let Ok(expected) = crate::reconcile::taxable_for_breakdown(inv, e) else {
            continue;
        };
        if e.taxable != expected {
            report.push(Finding::fatal(
                id,
                Path::at_term(Group::TaxBreakdown, i, BtId(116)),
                format!(
                    "IBT-116 {} ≠ Σ lines + charges − allowances {expected}",
                    e.taxable
                ),
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
            if code.eq_ignore_ascii_case("TTX")
                && inv
                    .lines
                    .iter()
                    .any(|l| line_has_ttx(l) && l.tax_total.is_some())
            {
                // ALIGNED-IBRP-TTX-09-MY: IBT-117 = Σ line TaxTotal on lines with TTX (±0.02).
                let expected = ttx_line_tax_sum(inv);
                let two = Decimal::new(2, 2);
                if (e.tax.raw() - expected).abs() > two {
                    report.push(Finding::fatal(
                        id,
                        path,
                        format!(
                            "TTX IBT-117 {} ≠ Σ line TaxTotal on TTX lines {expected}",
                            e.tax
                        ),
                    ));
                }
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

fn br_s_03(inv: &Invoice, report: &mut Report) {
    check_identifiers_in(
        inv,
        report,
        profile(VatCategory::Standard),
        "BR-S-03",
        RateContext::Allowance,
    );
}
fn br_s_04(inv: &Invoice, report: &mut Report) {
    check_identifiers_in(
        inv,
        report,
        profile(VatCategory::Standard),
        "BR-S-04",
        RateContext::Charge,
    );
}
fn br_s_06(inv: &Invoice, report: &mut Report) {
    check_rate_ac(
        inv,
        report,
        profile(VatCategory::Standard),
        "BR-S-06",
        RateContext::Allowance,
    );
}
fn br_s_07(inv: &Invoice, report: &mut Report) {
    check_rate_ac(
        inv,
        report,
        profile(VatCategory::Standard),
        "BR-S-07",
        RateContext::Charge,
    );
}

macro_rules! family_ac {
    ($cat:expr, $f03:ident, $f04:ident, $f06:ident, $f07:ident, $i03:literal, $i04:literal, $i06:literal, $i07:literal) => {
        fn $f03(inv: &Invoice, report: &mut Report) {
            check_identifiers_in(inv, report, profile($cat), $i03, RateContext::Allowance);
        }
        fn $f04(inv: &Invoice, report: &mut Report) {
            check_identifiers_in(inv, report, profile($cat), $i04, RateContext::Charge);
        }
        fn $f06(inv: &Invoice, report: &mut Report) {
            check_rate_ac(inv, report, profile($cat), $i06, RateContext::Allowance);
        }
        fn $f07(inv: &Invoice, report: &mut Report) {
            check_rate_ac(inv, report, profile($cat), $i07, RateContext::Charge);
        }
    };
}

family_ac!(
    VatCategory::ZeroRated,
    br_z_03,
    br_z_04,
    br_z_06,
    br_z_07,
    "BR-Z-03",
    "BR-Z-04",
    "BR-Z-06",
    "BR-Z-07"
);
family_ac!(
    VatCategory::Exempt,
    br_e_03,
    br_e_04,
    br_e_06,
    br_e_07,
    "BR-E-03",
    "BR-E-04",
    "BR-E-06",
    "BR-E-07"
);
family_ac!(
    VatCategory::ReverseCharge,
    br_ae_03,
    br_ae_04,
    br_ae_06,
    br_ae_07,
    "BR-AE-03",
    "BR-AE-04",
    "BR-AE-06",
    "BR-AE-07"
);
family_ac!(
    VatCategory::IntraCommunity,
    br_ic_03,
    br_ic_04,
    br_ic_06,
    br_ic_07,
    "BR-IC-03",
    "BR-IC-04",
    "BR-IC-06",
    "BR-IC-07"
);
family_ac!(
    VatCategory::Export,
    br_g_03,
    br_g_04,
    br_g_06,
    br_g_07,
    "BR-G-03",
    "BR-G-04",
    "BR-G-06",
    "BR-G-07"
);
family_ac!(
    VatCategory::OutOfScope,
    br_o_03,
    br_o_04,
    br_o_06,
    br_o_07,
    "BR-O-03",
    "BR-O-04",
    "BR-O-06",
    "BR-O-07"
);
family_ac!(
    VatCategory::CanaryIslands,
    br_af_03,
    br_af_04,
    br_af_06,
    br_af_07,
    "BR-AF-03",
    "BR-AF-04",
    "BR-AF-06",
    "BR-AF-07"
);
family_ac!(
    VatCategory::CeutaMelilla,
    br_ag_03,
    br_ag_04,
    br_ag_06,
    br_ag_07,
    "BR-AG-03",
    "BR-AG-04",
    "BR-AG-06",
    "BR-AG-07"
);

fn br_ic_11(inv: &Invoice, report: &mut Report) {
    // BR-IC-11: intra-community invoices need BT-72 or BG-14 dates.
    if !vat_families_apply(inv) || !uses_category(inv, VatCategory::IntraCommunity) {
        return;
    }
    let has_delivery = inv.delivery.as_ref().and_then(|d| d.date).is_some();
    let has_period = inv
        .period
        .as_ref()
        .is_some_and(|p| p.start.is_some() || p.end.is_some());
    if !has_delivery && !has_period {
        report.push(Finding::fatal(
            "BR-IC-11",
            Path::term(BtId(72)),
            "Intra-community: actual delivery date (BT-72) or invoicing period (BG-14) shall not be blank",
        ));
    }
}

fn br_ic_12(inv: &Invoice, report: &mut Report) {
    // BR-IC-12: intra-community deliver-to country (BT-80) shall not be blank.
    if !vat_families_apply(inv) || !uses_category(inv, VatCategory::IntraCommunity) {
        return;
    }
    let country = inv
        .delivery
        .as_ref()
        .and_then(|d| d.address.as_ref())
        .and_then(|a| a.country.as_ref())
        .map(|c| c.as_str().trim())
        .unwrap_or("");
    if country.is_empty() {
        report.push(Finding::fatal(
            "BR-IC-12",
            Path::term(BtId(80)),
            "Intra-community: deliver-to country (BT-80) shall not be blank",
        ));
    }
}

fn br_b_01(inv: &Invoice, report: &mut Report) {
    // BR-B-01: split payment (B) shall be a domestic Italian invoice.
    if !vat_families_apply(inv) || !uses_category(inv, VatCategory::SplitPayment) {
        return;
    }
    let seller_it = inv.seller.country().eq_ignore_ascii_case("IT");
    let buyer_it = inv.buyer.country().eq_ignore_ascii_case("IT");
    if !(seller_it && buyer_it) {
        report.push(Finding::fatal(
            "BR-B-01",
            Path::term(BtId(118)),
            "Split payment (B) shall be a domestic Italian invoice",
        ));
    }
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
fn my_se_01(i: &Invoice, r: &mut Report) {
    check_my_groups(i, r, "SE", "ALIGNED-IBRP-SE-01-MY");
}
fn my_se_08(i: &Invoice, r: &mut Report) {
    check_my_taxable(i, r, "SE", "ALIGNED-IBRP-SE-08-MY");
}
fn my_se_09(i: &Invoice, r: &mut Report) {
    check_my_tax(i, r, "SE", "ALIGNED-IBRP-SE-09-MY", true);
}
fn my_se_10(i: &Invoice, r: &mut Report) {
    check_my_no_exemption(i, r, "SE", "ALIGNED-IBRP-SE-10-MY");
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
fn my_hvg_10(i: &Invoice, r: &mut Report) {
    check_my_no_exemption(i, r, "HVG", "ALIGNED-IBRP-HVG-10-MY");
}
fn my_lvg_10(i: &Invoice, r: &mut Report) {
    check_my_no_exemption(i, r, "LVG", "ALIGNED-IBRP-LVG-10-MY");
}
fn my_e_05(inv: &Invoice, report: &mut Report) {
    if !my_families_apply(inv) {
        return;
    }
    for (i, line) in inv.lines.iter().enumerate() {
        if line.tax.code.eq_ignore_ascii_case("E")
            && line
                .tax
                .percent
                .is_some_and(|p| p.as_percent() != Decimal::ZERO)
        {
            report.push(Finding::fatal(
                "ALIGNED-IBRP-E-05-MY",
                Path::at_term(Group::Line, i, BtId(152)),
                "PINT-MY E line rate MUST be 0",
            ));
        }
    }
}
fn my_e_08(i: &Invoice, r: &mut Report) {
    check_my_taxable(i, r, "E", "ALIGNED-IBRP-E-08-MY");
}
fn my_o_09(i: &Invoice, r: &mut Report) {
    check_my_tax(i, r, "O", "ALIGNED-IBRP-O-09-MY", false);
}
fn my_ttx_08(inv: &Invoice, report: &mut Report) {
    if !my_families_apply(inv) {
        return;
    }
    for (i, e) in inv.tax_breakdown.iter().enumerate() {
        let aal =
            e.scheme.eq_ignore_ascii_case("AAL") || e.category.as_str().eq_ignore_ascii_case("TTX");
        if aal && e.rate.is_some() {
            report.push(Finding::fatal(
                "ALIGNED-IBRP-TTX-08-MY",
                Path::at_term(Group::TaxBreakdown, i, BtId(119)),
                "TTX/AAL MUST NOT include a tax percentage",
            ));
        }
    }
}
fn my_002(inv: &Invoice, report: &mut Report) {
    if !my_families_apply(inv) {
        return;
    }
    // Writer stamps process_id(); empty in-memory BT-23 is not this id. Wrong present value is.
    let Some(p) = inv
        .business_process
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    else {
        return;
    };
    if !p.starts_with("urn:peppol:bis:billing") {
        report.push(Finding::fatal(
            "ALIGNED-IBRP-002",
            Path::term(BtId(23)),
            "PINT-MY BT-23 must be urn:peppol:bis:billing",
        ));
    }
}
fn my_046(_inv: &Invoice, _report: &mut Report) {
    // ALIGNED-IBRP-046: IBT-117 is InvoiceAmount on TaxBreakdown; type-retired. explain works.
}
fn my_047(inv: &Invoice, report: &mut Report) {
    if !my_families_apply(inv) {
        return;
    }
    for (i, e) in inv.tax_breakdown.iter().enumerate() {
        if e.category.as_str().trim().is_empty() {
            report.push(Finding::fatal(
                "ALIGNED-IBRP-047",
                Path::at_term(Group::TaxBreakdown, i, BtId(118)),
                "Each IBG-23 must have a category code",
            ));
        }
        if e.scheme.eq_ignore_ascii_case("AAL") && !e.category.as_str().eq_ignore_ascii_case("TTX")
        {
            report.push(Finding::fatal(
                "ALIGNED-IBRP-047",
                Path::at_term(Group::TaxBreakdown, i, BtId(118)),
                "AAL subtotals must be category TTX",
            ));
        }
    }
}
fn my_048(inv: &Invoice, report: &mut Report) {
    if !my_families_apply(inv) {
        return;
    }
    for (i, e) in inv.tax_breakdown.iter().enumerate() {
        let ttx =
            e.scheme.eq_ignore_ascii_case("AAL") || e.category.as_str().eq_ignore_ascii_case("TTX");
        let o = e.category.as_str().eq_ignore_ascii_case("O");
        if ttx && e.rate.is_some() {
            report.push(Finding::fatal(
                "ALIGNED-IBRP-048",
                Path::at_term(Group::TaxBreakdown, i, BtId(119)),
                "AAL/TTX must not have a rate",
            ));
        }
        if !ttx && !o && e.rate.is_none() {
            report.push(Finding::fatal(
                "ALIGNED-IBRP-048",
                Path::at_term(Group::TaxBreakdown, i, BtId(119)),
                "VAT subtotals must have a rate except O",
            ));
        }
    }
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
    r(
        "BR-Z-03",
        "Zero-rated VAT: identifier on document allowance.",
        br_z_03,
    ),
    r(
        "BR-Z-04",
        "Zero-rated VAT: identifier on document charge.",
        br_z_04,
    ),
    r("BR-Z-05", "Zero-rated VAT: rate = 0.", br_z_05),
    r("BR-Z-06", "Zero-rated VAT: allowance rate.", br_z_06),
    r("BR-Z-07", "Zero-rated VAT: charge rate.", br_z_07),
    r("BR-Z-08", "Zero-rated VAT: BT-116 group sum.", br_z_08),
    r("BR-Z-09", "Zero-rated VAT: BT-117 = 0.", br_z_09),
    r(
        "BR-Z-10",
        "Zero-rated VAT: exemption reason forbidden.",
        br_z_10,
    ),
    r("BR-E-01", "Exempt VAT: exactly one BG-23 group.", br_e_01),
    r("BR-E-02", "Exempt VAT: seller tax identifier.", br_e_02),
    r(
        "BR-E-03",
        "Exempt VAT: identifier on document allowance.",
        br_e_03,
    ),
    r(
        "BR-E-04",
        "Exempt VAT: identifier on document charge.",
        br_e_04,
    ),
    r("BR-E-05", "Exempt VAT: rate = 0.", br_e_05),
    r("BR-E-06", "Exempt VAT: allowance rate.", br_e_06),
    r("BR-E-07", "Exempt VAT: charge rate.", br_e_07),
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
    r(
        "BR-AE-03",
        "Reverse charge: identifier on document allowance.",
        br_ae_03,
    ),
    r(
        "BR-AE-04",
        "Reverse charge: identifier on document charge.",
        br_ae_04,
    ),
    r("BR-AE-05", "Reverse charge: rate = 0.", br_ae_05),
    r("BR-AE-06", "Reverse charge: allowance rate.", br_ae_06),
    r("BR-AE-07", "Reverse charge: charge rate.", br_ae_07),
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
    r(
        "BR-IC-03",
        "Intra-community: identifier on document allowance.",
        br_ic_03,
    ),
    r(
        "BR-IC-04",
        "Intra-community: identifier on document charge.",
        br_ic_04,
    ),
    r("BR-IC-05", "Intra-community: rate = 0.", br_ic_05),
    r("BR-IC-06", "Intra-community: allowance rate.", br_ic_06),
    r("BR-IC-07", "Intra-community: charge rate.", br_ic_07),
    r(
        "BR-IC-11",
        "Intra-community: actual delivery date (BT-72) or invoicing period (BG-14).",
        br_ic_11,
    ),
    r(
        "BR-IC-12",
        "Intra-community: deliver-to country (BT-80).",
        br_ic_12,
    ),
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
    r(
        "BR-G-03",
        "Export: identifier on document allowance.",
        br_g_03,
    ),
    r("BR-G-04", "Export: identifier on document charge.", br_g_04),
    r("BR-G-05", "Export: rate = 0.", br_g_05),
    r("BR-G-06", "Export: allowance rate.", br_g_06),
    r("BR-G-07", "Export: charge rate.", br_g_07),
    r("BR-G-08", "Export: BT-116 group sum.", br_g_08),
    r("BR-G-09", "Export: BT-117 = 0.", br_g_09),
    r("BR-G-10", "Export: exemption reason required.", br_g_10),
    r("BR-O-01", "Out of scope: exactly one BG-23 group.", br_o_01),
    r(
        "BR-O-02",
        "Out of scope: VAT identifiers shall not be present.",
        br_o_02,
    ),
    r(
        "BR-O-03",
        "Out of scope: identifier on document allowance.",
        br_o_03,
    ),
    r(
        "BR-O-04",
        "Out of scope: identifier on document charge.",
        br_o_04,
    ),
    r("BR-O-05", "Out of scope: rate absent.", br_o_05),
    r("BR-O-06", "Out of scope: allowance rate.", br_o_06),
    r("BR-O-07", "Out of scope: charge rate.", br_o_07),
    r("BR-O-08", "Out of scope: BT-116 group sum.", br_o_08),
    r("BR-O-09", "Out of scope: BT-117 = 0.", br_o_09),
    r(
        "BR-O-10",
        "Out of scope: exemption reason required.",
        br_o_10,
    ),
    r(
        "BR-O-11",
        "Out of scope VAT breakdown forbids other BG-23 groups.",
        br_o_11,
    ),
    r(
        "BR-O-12",
        "Out of scope VAT breakdown forbids non-O invoice lines.",
        br_o_12,
    ),
    r(
        "BR-O-13",
        "Out of scope VAT breakdown forbids non-O document allowances.",
        br_o_13,
    ),
    r(
        "BR-O-14",
        "Out of scope VAT breakdown forbids non-O document charges.",
        br_o_14,
    ),
    r("BR-AF-01", "IGIC: at least one BG-23 group.", br_af_01),
    r("BR-AF-02", "IGIC: seller tax identifier.", br_af_02),
    r(
        "BR-AF-03",
        "IGIC: identifier on document allowance.",
        br_af_03,
    ),
    r("BR-AF-04", "IGIC: identifier on document charge.", br_af_04),
    r("BR-AF-05", "IGIC: rate ≥ 0.", br_af_05),
    r("BR-AF-06", "IGIC: allowance rate.", br_af_06),
    r("BR-AF-07", "IGIC: charge rate.", br_af_07),
    r("BR-AF-08", "IGIC: BT-116 group sum.", br_af_08),
    r("BR-AF-09", "IGIC: derived tax.", br_af_09),
    r("BR-AF-10", "IGIC: exemption reason forbidden.", br_af_10),
    r("BR-AG-01", "IPSI: at least one BG-23 group.", br_ag_01),
    r("BR-AG-02", "IPSI: seller tax identifier.", br_ag_02),
    r(
        "BR-AG-03",
        "IPSI: identifier on document allowance.",
        br_ag_03,
    ),
    r("BR-AG-04", "IPSI: identifier on document charge.", br_ag_04),
    r("BR-AG-05", "IPSI: rate ≥ 0.", br_ag_05),
    r("BR-AG-06", "IPSI: allowance rate.", br_ag_06),
    r("BR-AG-07", "IPSI: charge rate.", br_ag_07),
    r("BR-AG-08", "IPSI: BT-116 group sum.", br_ag_08),
    r("BR-AG-09", "IPSI: derived tax.", br_ag_09),
    r("BR-AG-10", "IPSI: exemption reason forbidden.", br_ag_10),
    r(
        "BR-B-01",
        "Split payment (B) shall be a domestic Italian invoice.",
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
        "ALIGNED-IBRP-SE-01-MY",
        "PINT-MY SE: at least one IBG-23 group.",
        my_se_01,
    ),
    my(
        "ALIGNED-IBRP-SE-08-MY",
        "PINT-MY SE: IBT-116 = Σ SE lines + charges − allowances.",
        my_se_08,
    ),
    my(
        "ALIGNED-IBRP-SE-09-MY",
        "PINT-MY SE: tax from rate.",
        my_se_09,
    ),
    my(
        "ALIGNED-IBRP-SE-10-MY",
        "PINT-MY SE: exemption reason forbidden.",
        my_se_10,
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
    my(
        "ALIGNED-IBRP-002",
        "PINT-MY BT-23 must be urn:peppol:bis:billing.",
        my_002,
    ),
    my("ALIGNED-IBRP-046", "Each IBG-23 must have IBT-117.", my_046),
    my(
        "ALIGNED-IBRP-047",
        "VAT subtotals need a category; AAL subtotals must be TTX.",
        my_047,
    ),
    my(
        "ALIGNED-IBRP-048",
        "VAT subtotals must have a rate except O; TTX/AAL must not.",
        my_048,
    ),
    my(
        "ALIGNED-IBRP-HVG-10-MY",
        "PINT-MY HVG: exemption reason forbidden.",
        my_hvg_10,
    ),
    my(
        "ALIGNED-IBRP-LVG-10-MY",
        "PINT-MY LVG: exemption reason forbidden.",
        my_lvg_10,
    ),
    my(
        "ALIGNED-IBRP-TTX-08-MY",
        "TTX/AAL MUST NOT include a tax percentage.",
        my_ttx_08,
    ),
    my(
        "ALIGNED-IBRP-E-05-MY",
        "PINT-MY E line rate MUST be 0.",
        my_e_05,
    ),
    my(
        "ALIGNED-IBRP-E-08-MY",
        "PINT-MY E: IBT-116 group sum.",
        my_e_08,
    ),
    my("ALIGNED-IBRP-O-09-MY", "PINT-MY O: tax = 0.", my_o_09),
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
        inv.payment_terms = Some("Net 30".into());
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

    #[test]
    fn s_line_missing_vat_is_only_br_s_02() {
        let mut inv = en_s();
        inv.seller.vat_identifier = None;
        let report = validate(&inv);
        let ids: Vec<_> = report.findings.iter().map(|f| f.id).collect();
        assert!(ids.contains(&"BR-S-02"), "{report}");
        assert!(!ids.contains(&"BR-S-03"), "{report}");
        assert!(!ids.contains(&"BR-S-04"), "{report}");
    }

    #[test]
    fn s_charge_missing_vat_is_only_br_s_04() {
        let mut inv = en_s();
        inv.seller.vat_identifier = None;
        inv.lines[0].tax = TaxCategory::vat("Z", Decimal::from(0));
        inv.document_charges.push(crate::invoice::AllowanceCharge {
            amount: amt("10.00"),
            base: None,
            percent: None,
            reason: None,
            reason_code: None,
            tax: Some(TaxCategory::vat("S", Decimal::from(19))),
        });
        let _ = reconcile(&mut inv);
        let report = validate(&inv);
        let ids: Vec<_> = report.findings.iter().map(|f| f.id).collect();
        assert!(ids.contains(&"BR-S-04"), "{report}");
        assert!(!ids.contains(&"BR-S-02"), "{report}");
        assert!(!ids.contains(&"BR-S-03"), "{report}");
    }

    #[test]
    fn o_group_plus_s_group_is_o_11() {
        let mut inv = en_s();
        inv.lines[0].tax = TaxCategory {
            system: TaxSystem::Vat,
            code: "O".into(),
            percent: None,
        };
        reconcile(&mut inv).unwrap();
        inv.tax_breakdown.push(crate::invoice::TaxBreakdown {
            system: TaxSystem::Vat,
            scheme: "VAT".into(),
            category: Code::new("S"),
            rate: Some(Percentage::new(Decimal::from(19))),
            taxable: amt("0.00"),
            tax: amt("0.00"),
            exemption_reason: None,
            exemption_code: None,
        });
        let report = validate(&inv);
        assert!(
            report.findings.iter().any(|f| f.id == "BR-O-11"),
            "{report}"
        );
    }

    #[test]
    fn o_group_plus_s_line_is_o_12() {
        let mut inv = en_s();
        inv.lines[0].tax = TaxCategory {
            system: TaxSystem::Vat,
            code: "O".into(),
            percent: None,
        };
        inv.lines.push(Line::new(
            "2",
            "Std",
            amt("10.00"),
            TaxCategory::vat("S", Decimal::from(19)),
        ));
        let _ = reconcile(&mut inv);
        // Keep only the O group so O-12 is the mix on lines, not extra groups.
        inv.tax_breakdown
            .retain(|e| e.category.as_str().eq_ignore_ascii_case("O"));
        let report = validate(&inv);
        assert!(
            report.findings.iter().any(|f| f.id == "BR-O-12"),
            "{report}"
        );
    }
}
