//! Rounding and slack used by VAT/GST/SST derivation.
//!
//! # Four tolerance regimes — never mixed
//!
//! Slack is a **rule instance**, never a crate constant applied to every amount,
//! never [`InvoiceAmount`](crate::InvoiceAmount) policy.
//!
//! 1. **Totals `BR-CO-10`…`BR-CO-16`** — exact. CEN. Absent ≠ 0; overflow is a
//!    finding, not wrap. Do not put ±1.00 or ±0.02 here.
//! 2. **VAT derivation (`BR-CO-17`, family `-09`)** — ±1.00 **exclusive** on
//!    **absolute** values (`|Δ| < 1`; `Δ = 1.00` fails). Artefact-only slack;
//!    EN 16931-1 §6.4.2 writes a plain equation. Credit notes use `abs` so a
//!    negative base still derives.
//! 3. **Peppol `R120` / `R040`** — ±0.02 **inclusive**. Lives on those Peppol
//!    extra rules (P13), not here. **`R046` is exact** (the classic trap of
//!    copying R120's slack onto line VAT).
//! 4. **XRechnung HUF 0.5** — those two Peppol-shaped rules only, and **out of
//!    this crate** (P18). No HUF branch in core or Peppol.
//!
//! Malaysian 5-sen cash rounding is **BT-114**, not slack. There is no fatal
//! “payable multiple of 0.05” rule.
//!
//! # Why rounding needs its own module
//!
//! The CEN artefacts spell derived tax in XPath:
//!
//! ```xpath
//! round(abs(TaxableAmount) * (Percent div 100) * 10 * 10) div 100
//! ```
//!
//! and pick the zero-rate branch with `round(Percent) = 0`. Both `round`s are
//! XPath's `fn:round`: closest integer, ties toward **+∞** = `floor(x + 0.5)`.
//!
//! | | `round(0.5)` | `round(2.5)` | `round(-0.5)` |
//! |---|---|---|---|
//! | XPath `fn:round` | `1` | `3` | `0` |
//! | `Decimal::round` (banker's) | `0` | `2` | `0` |
//! | half away from zero | `1` | `3` | `-1` |
//!
//! A rate of **0.5 %** (Spanish recargo) rounds to `1` for the artefact and to
//! `0` for banker's rounding, which sent `BR-CO-17` down its zero-rate branch
//! and rejected a valid invoice.
//!
//! The producer ([`crate::reconcile`]) may use commercial rounding for the
//! printed numbers; the validator uses [`xpath_round`]. The ±1.00 slack is what
//! lets those two disagree by a unit.

use rust_decimal::Decimal;

/// The ±1.00 the artefacts allow on the VAT derivation family.
///
/// **Not in the standard.** Shared by `BR-CO-17` and by the `-08`/`-09` rows.
/// Not a policy on [`crate::InvoiceAmount`].
pub const VAT_TOLERANCE: Decimal = Decimal::ONE;

const HALF: Decimal = Decimal::from_parts(5, 0, 0, false, 1);

/// XPath `fn:round` — closest integer, ties toward **+∞**.
///
/// `floor(x + 0.5)`. Saturates: if `x + 0.5` overflows, returns `x`.
pub fn xpath_round(x: Decimal) -> Decimal {
    x.checked_add(HALF).map_or(x, |shifted| shifted.floor())
}

/// Artefact derived tax: `round(|base| × rate) / 100`.
///
/// `rate` is a per cent (`19`, not `0.19`). `None` only on overflow.
pub fn derived_vat(base: Decimal, rate: Decimal) -> Option<Decimal> {
    base.abs()
        .checked_mul(rate)
        .map(|product| xpath_round(product) / Decimal::ONE_HUNDRED)
}

/// Whether `stated` is within the artefacts' ±1.00 of `expected` (**exclusive**).
///
/// `stated - 1 < expected` and `stated + 1 > expected`: a difference of exactly
/// 1.00 is a finding.
pub fn within_vat_tolerance(stated: Decimal, expected: Decimal) -> bool {
    (stated - expected).abs() < VAT_TOLERANCE
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::Decimal;
    use std::str::FromStr;

    fn d(s: &str) -> Decimal {
        Decimal::from_str(s).unwrap()
    }

    #[test]
    fn ties_go_towards_positive_infinity() {
        assert_eq!(xpath_round(d("0.5")), d("1"));
        assert_eq!(xpath_round(d("2.5")), d("3"));
        assert_eq!(xpath_round(d("-0.5")), d("0"));
        assert_eq!(xpath_round(d("-1.5")), d("-1"));
        assert_eq!(xpath_round(d("0.4")), d("0"));
        assert_eq!(xpath_round(d("0.6")), d("1"));
        assert_eq!(xpath_round(d("-0.6")), d("-1"));
        assert_eq!(xpath_round(d("19")), d("19"));
    }

    #[test]
    fn a_rate_of_half_a_per_cent_is_not_a_zero_rate() {
        assert_ne!(xpath_round(d("0.5")), Decimal::ZERO);
        assert_eq!(derived_vat(d("1000.00"), d("0.5")), Some(d("5")));
    }

    #[test]
    fn the_derivation_is_taken_on_absolute_values() {
        assert_eq!(
            derived_vat(d("-1000.00"), d("19")),
            derived_vat(d("1000.00"), d("19"))
        );
    }

    #[test]
    fn a_full_currency_unit_of_slack_excludes_its_own_boundary() {
        assert!(within_vat_tolerance(d("190.99"), d("190.00")));
        assert!(!within_vat_tolerance(d("191.00"), d("190.00")));
        assert!(!within_vat_tolerance(d("189.00"), d("190.00")));
    }
}
