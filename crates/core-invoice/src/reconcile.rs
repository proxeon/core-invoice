//! Derive BG-23 and BG-22 from lines, document allowances and charges.
//!
//! This is EN/PINT **presentation** of arithmetic, not a billing engine. It
//! does not invent exemption reasons, due dates, or tax identifiers.
//!
//! Grouping is the same table the category `-08` rows check:
//! - EN / Peppol / PINT VAT: `(category, rate)` for families that may repeat
//!   (`S`, `L`, `M`, `B`); category alone for zero-tax families.
//! - PINT-MY: `(scheme, category, rate)` — SST is never grouped as UNCL 5305 `S`.
//!
//! Printed tax amounts use **commercial** rounding (half away from zero). The
//! validator uses [`crate::arith::xpath_round`]; ±1.00 slack on `BR-CO-17` is
//! what lets those two disagree by a unit.
//!
//! Empty document allowances/charges → BT-107/108 **absent**, not zero.

use rust_decimal::Decimal;

use crate::amount::InvoiceAmount;
use crate::bt::{BtId, Group, Path};
use crate::code::Code;
use crate::invoice::{DocumentTotals, Invoice, TaxBreakdown};
use crate::numeric::Percentage;
use crate::profile::Profile;
use crate::tax::{TaxSystem, wire_scheme};

/// Why an invoice could not be reconciled. Not a validation finding.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum ReconcileError {
    #[error("{term} overflowed while reconciling; the amounts involved are not representable")]
    Overflow { term: &'static str },
    #[error(
        "{at} is a taxed category with no rate; defaulting it to zero would silently under-declare tax"
    )]
    MissingRate { at: Path, category: String },
}

/// What reconciliation produced, before it is written back.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reconciled {
    pub tax_breakdown: Vec<TaxBreakdown>,
    pub totals: DocumentTotals,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Exemption {
    scheme: String,
    category: String,
    text: Option<String>,
    code: Option<Code>,
}

/// Computes BG-23 and BG-22 from an invoice's lines, allowances and charges.
#[derive(Debug, Clone, Default)]
pub struct Reconciler {
    exemptions: Vec<Exemption>,
    paid: Option<InvoiceAmount>,
    rounding: Option<InvoiceAmount>,
    tax_total_accounting: Option<InvoiceAmount>,
}

impl Reconciler {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// BT-120 / BT-121 for one category. Categories that forbid a reason drop it.
    #[must_use]
    pub fn exemption(
        mut self,
        category: impl Into<String>,
        text: Option<&str>,
        code: Option<&str>,
    ) -> Self {
        self.exemptions.push(Exemption {
            scheme: String::new(),
            category: category.into(),
            text: text.map(str::to_owned),
            code: code.map(Code::new),
        });
        self
    }

    /// BT-113. Absent is not zero.
    #[must_use]
    pub fn paid(mut self, amount: InvoiceAmount) -> Self {
        self.paid = Some(amount);
        self
    }

    /// BT-114. Malaysian 5-sen cash rounding belongs here, not as slack.
    #[must_use]
    pub fn rounding(mut self, amount: InvoiceAmount) -> Self {
        self.rounding = Some(amount);
        self
    }

    /// BT-111. Not derived.
    #[must_use]
    pub fn tax_total_accounting(mut self, amount: InvoiceAmount) -> Self {
        self.tax_total_accounting = Some(amount);
        self
    }

    pub fn compute(&self, inv: &Invoice) -> Result<Reconciled, ReconcileError> {
        let tax_breakdown = self.breakdown(inv)?;
        let totals = self.totals(inv, &tax_breakdown)?;
        Ok(Reconciled {
            tax_breakdown,
            totals,
        })
    }

    /// Write BG-23 and BG-22. Invoice is unchanged on error.
    /// Existing exemption reasons on matching groups are kept unless this
    /// reconciler supplied a replacement.
    pub fn apply(&self, inv: &mut Invoice) -> Result<(), ReconcileError> {
        let r = self.compute(inv)?;
        inv.tax_breakdown = r.tax_breakdown;
        // BT-110 / BT-115 live on DocumentTotals. Ghosts on Invoice are not a second identity.
        inv.totals = Some(r.totals);
        Ok(())
    }

    fn breakdown(&self, inv: &Invoice) -> Result<Vec<TaxBreakdown>, ReconcileError> {
        let mut keys: Vec<GroupKey> = Vec::new();
        for item in content(inv) {
            let key = group_key(inv, &item)?;
            if !keys.contains(&key) {
                keys.push(key);
            }
        }
        keys.sort();

        let mut rows = Vec::with_capacity(keys.len());
        for key in keys {
            let taxable = taxable_for(inv, &key)?;
            let rate = key.rate;
            let tax = tax_amount(inv, &key, taxable, rate)?;
            let (exemption_reason, exemption_code) = self.exemption_for(inv, &key);
            rows.push(TaxBreakdown {
                system: key.system,
                scheme: key.scheme.clone(),
                category: Code::new(key.category.clone()),
                rate,
                taxable,
                tax,
                exemption_reason,
                exemption_code,
            });
        }
        Ok(rows)
    }

    fn exemption_for(&self, inv: &Invoice, key: &GroupKey) -> (Option<String>, Option<Code>) {
        if forbids_exemption(&key.category) {
            return (None, None);
        }
        if let Some(ex) = self
            .exemptions
            .iter()
            .find(|e| e.category.eq_ignore_ascii_case(&key.category))
        {
            return (ex.text.clone(), ex.code.clone());
        }
        inv.tax_breakdown
            .iter()
            .find(|e| {
                e.category.as_str() == key.category
                    && e.scheme == key.scheme
                    && (e.exemption_reason.is_some() || e.exemption_code.is_some())
            })
            .map_or((None, None), |e| {
                (e.exemption_reason.clone(), e.exemption_code.clone())
            })
    }

    fn totals(
        &self,
        inv: &Invoice,
        breakdown: &[TaxBreakdown],
    ) -> Result<DocumentTotals, ReconcileError> {
        let sum = |it: Vec<InvoiceAmount>, term| {
            InvoiceAmount::checked_sum(it).ok_or(ReconcileError::Overflow { term })
        };

        let line_net = sum(inv.lines.iter().map(|l| l.net).collect(), "BT-106")?;

        let allowance_total = if inv.document_allowances.is_empty() {
            None
        } else {
            Some(sum(
                inv.document_allowances.iter().map(|a| a.amount).collect(),
                "BT-107",
            )?)
        };
        let charge_total = if inv.document_charges.is_empty() {
            None
        } else {
            Some(sum(
                inv.document_charges.iter().map(|c| c.amount).collect(),
                "BT-108",
            )?)
        };

        let without_tax = line_net
            .checked_sub(allowance_total.unwrap_or(InvoiceAmount::ZERO))
            .and_then(|v| v.checked_add(charge_total.unwrap_or(InvoiceAmount::ZERO)))
            .ok_or(ReconcileError::Overflow { term: "BT-109" })?;

        let vat_rows: Vec<InvoiceAmount> = breakdown
            .iter()
            .filter(|e| counts_toward_tax_total(inv.profile, e))
            .map(|e| e.tax)
            .collect();
        let tax_total = if breakdown.is_empty() {
            None
        } else {
            Some(sum(vat_rows, "BT-110")?)
        };

        let with_tax = without_tax
            .checked_add(tax_total.unwrap_or(InvoiceAmount::ZERO))
            .ok_or(ReconcileError::Overflow { term: "BT-112" })?;

        let payable = with_tax
            .checked_sub(self.paid.unwrap_or(InvoiceAmount::ZERO))
            .and_then(|v| v.checked_add(self.rounding.unwrap_or(InvoiceAmount::ZERO)))
            .ok_or(ReconcileError::Overflow { term: "BT-115" })?;

        Ok(DocumentTotals {
            line_net: Some(line_net),
            allowance_total,
            charge_total,
            without_tax: Some(without_tax),
            tax_total,
            tax_total_accounting: self.tax_total_accounting,
            with_tax: Some(with_tax),
            paid: self.paid,
            rounding: self.rounding,
            payable: Some(payable),
        })
    }
}

/// Reconcile with every default.
pub fn reconcile(inv: &mut Invoice) -> Result<(), ReconcileError> {
    Reconciler::new().apply(inv)
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct GroupKey {
    scheme: String,
    category: String,
    rate: Option<Percentage>,
    system: TaxSystem,
}

struct ContentRow<'a> {
    path: Path,
    system: TaxSystem,
    category: &'a str,
    percent: Option<Percentage>,
    net: InvoiceAmount,
    is_allowance: bool,
}

fn content(inv: &Invoice) -> Vec<ContentRow<'_>> {
    let mut rows = Vec::new();
    for (i, line) in inv.lines.iter().enumerate() {
        rows.push(ContentRow {
            path: Path::at_term(Group::Line, i, BtId(131)),
            system: line.tax.system,
            category: &line.tax.code,
            percent: line.tax.percent,
            net: line.net,
            is_allowance: false,
        });
    }
    for (i, a) in inv.document_allowances.iter().enumerate() {
        let tax = a.tax.as_ref();
        rows.push(ContentRow {
            path: Path::at_term(Group::DocumentAllowance, i, BtId(92)),
            system: tax.map(|t| t.system).unwrap_or(TaxSystem::Vat),
            category: tax.map(|t| t.code.as_str()).unwrap_or(""),
            percent: tax.and_then(|t| t.percent),
            net: a.amount,
            is_allowance: true,
        });
    }
    for (i, c) in inv.document_charges.iter().enumerate() {
        let tax = c.tax.as_ref();
        rows.push(ContentRow {
            path: Path::at_term(Group::DocumentCharge, i, BtId(99)),
            system: tax.map(|t| t.system).unwrap_or(TaxSystem::Vat),
            category: tax.map(|t| t.code.as_str()).unwrap_or(""),
            percent: tax.and_then(|t| t.percent),
            net: c.amount,
            is_allowance: false,
        });
    }
    rows
}

fn group_key(inv: &Invoice, row: &ContentRow<'_>) -> Result<GroupKey, ReconcileError> {
    let scheme = wire_scheme(inv.profile, row.system, row.category).to_owned();
    let rate = if crate::category::grouped_by_rate(inv.profile, row.category) {
        if needs_rate(row.category)
            && row.percent.is_none_or(Percentage::is_zero)
            && !zero_tax_family(row.category)
        {
            return Err(ReconcileError::MissingRate {
                at: row.path,
                category: row.category.to_owned(),
            });
        }
        row.percent
    } else if row.category.eq_ignore_ascii_case("O") || row.category.eq_ignore_ascii_case("TTX") {
        None
    } else {
        Some(Percentage::ZERO)
    };
    Ok(GroupKey {
        scheme,
        category: row.category.to_owned(),
        rate,
        system: row.system,
    })
}

fn needs_rate(category: &str) -> bool {
    matches!(
        category,
        "S" | "L"
            | "M"
            | "B"
            | "SA"
            | "SE"
            | "HVG"
            | "LVG"
            | "s"
            | "l"
            | "m"
            | "b"
            | "sa"
            | "se"
            | "hvg"
            | "lvg"
    )
}

fn zero_tax_family(category: &str) -> bool {
    // PINT-MY SE is service tax (rated). It is not EN category E / zero-rated Z.
    matches!(
        category,
        "Z" | "E" | "AE" | "K" | "G" | "O" | "z" | "e" | "ae" | "k" | "g" | "o"
    )
}

fn forbids_exemption(category: &str) -> bool {
    matches!(
        category,
        "S" | "Z"
            | "L"
            | "M"
            | "SA"
            | "SE"
            | "HVG"
            | "LVG"
            | "s"
            | "z"
            | "l"
            | "m"
            | "sa"
            | "se"
            | "hvg"
            | "lvg"
    )
}

fn same_group(inv: &Invoice, row: &ContentRow<'_>, key: &GroupKey) -> bool {
    let Ok(k) = group_key(inv, row) else {
        return false;
    };
    k == *key
}

/// ALIGNED-IBRP-*-08-MY uses this same content (lines + charges − allowances). Exact, no slack.
pub(crate) fn taxable_for_breakdown(
    inv: &Invoice,
    row: &TaxBreakdown,
) -> Result<InvoiceAmount, ReconcileError> {
    taxable_for(
        inv,
        &GroupKey {
            scheme: row.scheme.clone(),
            category: row.category.as_str().to_owned(),
            rate: row.rate,
            system: row.system,
        },
    )
}

fn taxable_for(inv: &Invoice, key: &GroupKey) -> Result<InvoiceAmount, ReconcileError> {
    // Line A/C already sits in BT-131. Do not add them again in taxable_for.
    let mut pos = InvoiceAmount::ZERO;
    let mut neg = InvoiceAmount::ZERO;
    for row in content(inv) {
        if !same_group(inv, &row, key) {
            continue;
        }
        if row.is_allowance {
            neg = neg
                .checked_add(row.net)
                .ok_or(ReconcileError::Overflow { term: "BT-116" })?;
        } else {
            pos = pos
                .checked_add(row.net)
                .ok_or(ReconcileError::Overflow { term: "BT-116" })?;
        }
    }
    pos.checked_sub(neg)
        .ok_or(ReconcileError::Overflow { term: "BT-116" })
}

fn tax_amount(
    inv: &Invoice,
    key: &GroupKey,
    taxable: InvoiceAmount,
    rate: Option<Percentage>,
) -> Result<InvoiceAmount, ReconcileError> {
    if key.category.eq_ignore_ascii_case("TTX") {
        return Ok(taxable);
    }
    if zero_tax_family(&key.category) {
        return Ok(InvoiceAmount::ZERO);
    }
    let _ = inv;
    let rate = rate.map_or(Decimal::ZERO, Percentage::as_percent);
    let exact = taxable
        .raw()
        .checked_mul(rate)
        .map(|v| v / Decimal::ONE_HUNDRED)
        .ok_or(ReconcileError::Overflow { term: "BT-117" })?;
    InvoiceAmount::from_decimal_rounded(exact)
        .map_err(|_| ReconcileError::Overflow { term: "BT-117" })
}

pub(crate) fn counts_toward_tax_total(_profile: Profile, _row: &TaxBreakdown) -> bool {
    // IBR-CO-14 / BR-CO-14: BT-110 = Σ every BG-23 / IBG-23 tax amount, including TTX (AAL).
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::code::Code;
    use crate::date::Date;
    use crate::invoice::{Invoice, Line, Party};
    use crate::kind::DocumentKind;
    use crate::numeric::Quantity;
    use crate::tax::TaxCategory;
    use crate::validate;

    fn amt(s: &str) -> InvoiceAmount {
        InvoiceAmount::parse(s).unwrap()
    }

    fn with_price(mut line: Line, price: &str) -> Line {
        line.quantity = Some(Quantity::parse("1").unwrap());
        line.unit = Some(Code::new("C62"));
        line.price = Some(crate::invoice::Price {
            net: crate::amount::UnitPriceAmount::parse(price).unwrap(),
            discount: None,
            gross: None,
            base_qty: None,
            base_unit: None,
        });
        line
    }

    fn en_blank() -> Invoice {
        let mut inv = Invoice::blank(
            Profile::En16931,
            "INV-1",
            "EUR",
            {
                let mut p = Party::new("Seller GmbH", "DE");
                p.vat_identifier = Some(crate::identifier::Identifier::new("DE123456789"));
                p
            },
            Party::new("Buyer SARL", "FR"),
        );
        inv.issue_date = Date::parse("2026-01-15").ok();
        inv.type_code = Some(Code::new("380"));
        inv
    }

    #[test]
    fn two_standard_rates_are_two_breakdown_rows() {
        let mut inv = en_blank();
        inv.lines = vec![
            with_price(
                Line::new(
                    "1",
                    "A",
                    amt("100.00"),
                    TaxCategory::vat("S", Decimal::from(19)),
                ),
                "100.00",
            ),
            with_price(
                Line::new(
                    "2",
                    "B",
                    amt("50.00"),
                    TaxCategory::vat("S", Decimal::from(7)),
                ),
                "50.00",
            ),
        ];
        reconcile(&mut inv).unwrap();
        assert_eq!(inv.tax_breakdown.len(), 2);
        let totals = inv.totals.as_ref().unwrap();
        assert_eq!(totals.line_net.unwrap(), amt("150.00"));
        assert_eq!(totals.allowance_total, None);
        assert_eq!(totals.charge_total, None);
        assert_eq!(totals.tax_total.unwrap(), amt("22.50"));
        assert_eq!(totals.with_tax.unwrap(), amt("172.50"));
        assert_eq!(totals.payable, Some(amt("172.50")));
        assert!(validate(&inv).ok(), "{}", validate(&inv));
    }

    #[test]
    fn empty_document_allowances_leave_bt_107_absent() {
        let mut inv = en_blank();
        inv.lines = vec![Line::new(
            "1",
            "A",
            amt("100.00"),
            TaxCategory::vat("S", Decimal::from(19)),
        )];
        reconcile(&mut inv).unwrap();
        let t = inv.totals.as_ref().unwrap();
        assert_eq!(t.allowance_total, None);
        assert_eq!(t.charge_total, None);
    }

    #[test]
    fn prepaid_may_make_payable_negative() {
        let mut inv = en_blank();
        inv.lines = vec![with_price(
            Line::new(
                "1",
                "A",
                amt("125.00"),
                TaxCategory::vat("S", Decimal::from(10)),
            ),
            "125.00",
        )];
        Reconciler::new()
            .paid(amt("250.00"))
            .apply(&mut inv)
            .unwrap();
        let t = inv.totals.as_ref().unwrap();
        assert_eq!(t.with_tax.unwrap(), amt("137.50"));
        assert_eq!(t.paid, Some(amt("250.00")));
        assert_eq!(t.payable, Some(amt("-112.50")));
        assert!(validate(&inv).ok(), "{}", validate(&inv));
    }

    #[test]
    fn stuffed_payable_fails_real_br_co_16() {
        let mut inv = en_blank();
        inv.lines = vec![with_price(
            Line::new(
                "1",
                "A",
                amt("125.00"),
                TaxCategory::vat("S", Decimal::from(10)),
            ),
            "125.00",
        )];
        Reconciler::new()
            .paid(amt("250.00"))
            .apply(&mut inv)
            .unwrap();
        inv.totals.as_mut().unwrap().payable = Some(amt("137.50"));
        let report = validate(&inv);
        assert!(
            report.findings.iter().any(|f| f.id == "BR-CO-16"),
            "{report}"
        );
    }

    #[test]
    fn credit_note_keeps_positive_amounts() {
        let mut inv = en_blank();
        inv.lines = vec![Line::new(
            "1",
            "A",
            amt("100.00"),
            TaxCategory::vat("S", Decimal::from(19)),
        )];
        reconcile(&mut inv).unwrap();
        let cn = inv.to_credit_note("CN-1", Date::parse("2026-01-16").unwrap());
        assert_eq!(cn.kind, DocumentKind::CreditNote);
        assert_eq!(cn.payable(), inv.payable());
    }

    #[test]
    fn pint_my_sa_and_se_are_two_rows() {
        let mut inv = Invoice::blank(
            Profile::PintMy,
            "MY-1",
            "MYR",
            {
                let mut p = Party::new("Kedai", "MY");
                p.tax_registration = Some(crate::identifier::Identifier::new("C12345678901"));
                p.legal_registration = Some(crate::identifier::Identifier::new("2023010000001"));
                p
            },
            {
                let mut b = Party::new("Pembeli", "MY");
                b.legal_registration = Some(crate::identifier::Identifier::new("1999010000001"));
                b
            },
        );
        inv.issue_date = Date::parse("2026-01-15").ok();
        inv.type_code = Some(Code::new("380"));
        inv.lines = vec![
            {
                let mut l = Line::new(
                    "1",
                    "Taxed",
                    amt("100.00"),
                    TaxCategory::sst("SA", Decimal::from(10)),
                );
                l.quantity = Some(Quantity::parse("1").unwrap());
                l.unit = Some(Code::new("C62"));
                l.price = Some(crate::invoice::Price {
                    net: crate::amount::UnitPriceAmount::parse("100.00").unwrap(),
                    discount: None,
                    gross: None,
                    base_qty: None,
                    base_unit: None,
                });
                l
            },
            {
                let mut l = Line::new(
                    "2",
                    "Exempt",
                    amt("40.00"),
                    TaxCategory::sst("SE", Decimal::from(8)),
                );
                l.quantity = Some(Quantity::parse("1").unwrap());
                l.unit = Some(Code::new("C62"));
                l.price = Some(crate::invoice::Price {
                    net: crate::amount::UnitPriceAmount::parse("40.00").unwrap(),
                    discount: None,
                    gross: None,
                    base_qty: None,
                    base_unit: None,
                });
                l
            },
        ];
        reconcile(&mut inv).unwrap();
        assert_eq!(inv.tax_breakdown.len(), 2);
        assert!(
            inv.tax_breakdown
                .iter()
                .any(|r| r.category.as_str() == "SA" && r.tax == amt("10.00"))
        );
        assert!(
            inv.tax_breakdown
                .iter()
                .any(|r| r.category.as_str() == "SE" && r.tax == amt("3.20"))
        );
        assert!(validate(&inv).ok(), "{}", validate(&inv));
    }

    #[test]
    fn o_is_exclusive_one_group() {
        let mut inv = en_blank();
        inv.lines = vec![with_price(
            Line::new("1", "Out", amt("10.00"), TaxCategory::out_of_scope()),
            "10.00",
        )];
        reconcile(&mut inv).unwrap();
        assert_eq!(inv.tax_breakdown.len(), 1);
        assert_eq!(inv.tax_breakdown[0].category.as_str(), "O");
        assert_eq!(inv.tax_breakdown[0].rate, None);
        assert_eq!(inv.tax_breakdown[0].tax, amt("0.00"));
    }

    #[test]
    fn does_not_overwrite_existing_exemption_reason() {
        let mut inv = en_blank();
        inv.lines = vec![Line::new(
            "1",
            "Exempt",
            amt("10.00"),
            TaxCategory::vat("E", Decimal::from(0)),
        )];
        inv.tax_breakdown = vec![TaxBreakdown {
            system: TaxSystem::Vat,
            scheme: "VAT".into(),
            category: Code::new("E"),
            rate: Some(Percentage::ZERO),
            taxable: amt("10.00"),
            tax: amt("0.00"),
            exemption_reason: Some("exempt goods".into()),
            exemption_code: None,
        }];
        reconcile(&mut inv).unwrap();
        assert_eq!(
            inv.tax_breakdown[0].exemption_reason.as_deref(),
            Some("exempt goods")
        );
    }
}
