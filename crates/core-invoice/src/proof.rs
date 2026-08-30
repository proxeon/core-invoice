//! Proof that an invoice passed a named profile. Formats stamp BT-24 from this.

use std::marker::PhantomData;

use crate::invoice::Invoice;
use crate::profile::Profile;
use crate::report::Report;
use crate::validate;

/// A profile at the type level.
pub trait ProfileMarker {
    fn profile() -> Profile;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct En16931;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PeppolBis3;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pint;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PintMy;

impl ProfileMarker for En16931 {
    fn profile() -> Profile {
        Profile::En16931
    }
}
impl ProfileMarker for PeppolBis3 {
    fn profile() -> Profile {
        Profile::PeppolBis3
    }
}
impl ProfileMarker for Pint {
    fn profile() -> Profile {
        Profile::Pint
    }
}
impl ProfileMarker for PintMy {
    fn profile() -> Profile {
        Profile::PintMy
    }
}

/// `Q: Underlies<P>` means every document valid under `P` is valid under `Q`.
///
/// Implemented **only** where CEN §4.4.4 holds: Peppol BIS 3.0 is a CIUS of
/// EN 16931, so a Peppol proof may be widened to core. PINT is Extension-shaped
/// on the tax axiom, **not** a CIUS — there is no `Underlies<En16931> for Pint`
/// and no `Underlies<PeppolBis3> for PintMy`.
pub trait Underlies<P: ProfileMarker>: ProfileMarker {}

impl Underlies<PeppolBis3> for En16931 {}

/// An invoice validated against `P`. Cannot be constructed from a suppressed run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Validated<P: ProfileMarker> {
    invoice: Invoice,
    _profile: PhantomData<P>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProveError {
    Rejected(Report),
    /// `Check::without` was used; prove refuses to mint a proof.
    Suppressed(String),
}

impl std::fmt::Display for ProveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Rejected(r) => write!(f, "invoice failed {r}"),
            Self::Suppressed(id) => {
                write!(f, "cannot prove a profile after suppressing {id}")
            }
        }
    }
}
impl std::error::Error for ProveError {}

impl<P: ProfileMarker> Validated<P> {
    pub fn new(mut invoice: Invoice) -> Result<Self, Box<(Invoice, Report)>> {
        invoice.profile = P::profile();
        if invoice.specification_id.is_none() {
            invoice.specification_id = Some(invoice.profile.specification_id().into());
        }
        let report = validate(&invoice);
        if report.ok() {
            Ok(Self {
                invoice,
                _profile: PhantomData,
            })
        } else {
            Err(Box::new((invoice, report)))
        }
    }

    pub fn invoice(&self) -> &Invoice {
        &self.invoice
    }

    pub fn into_inner(self) -> Invoice {
        self.invoice
    }

    /// Re-badge as a *less* restrictive profile. Only Peppol → EN 16931.
    pub fn widen<Q: Underlies<P>>(self) -> Validated<Q> {
        Validated {
            invoice: self.invoice,
            _profile: PhantomData,
        }
    }
}

/// Builder that records suppressions. [`prove`](Check::prove) fails if any id was
/// suppressed — a proof must see every fatal.
#[derive(Debug, Clone, Default)]
pub struct Check {
    suppressed: Vec<String>,
}

impl Check {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn without(mut self, id: impl Into<String>) -> Self {
        self.suppressed.push(id.into());
        self
    }

    pub fn prove<P: ProfileMarker>(self, invoice: Invoice) -> Result<Validated<P>, ProveError> {
        if let Some(id) = self.suppressed.first() {
            return Err(ProveError::Suppressed(id.clone()));
        }
        Validated::new(invoice).map_err(|rejected| ProveError::Rejected(rejected.1))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::amount::InvoiceAmount;
    use crate::code::Code;
    use crate::date::Date;
    use crate::identifier::Identifier;
    use crate::invoice::{Invoice, Line, Party};
    use crate::reconcile::reconcile;
    use crate::tax::TaxCategory;
    use rust_decimal::Decimal;

    fn peppol_ok() -> Invoice {
        let mut inv = Invoice::blank(
            Profile::PeppolBis3,
            "EU-1",
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
        inv.issue_date = Date::parse("2026-01-15").ok();
        inv.type_code = Some(Code::new("380"));
        inv.business_process = Some("urn:fdc:peppol.eu:2017:poacc:billing:01:1.0".into());
        inv.buyer_reference = Some(crate::identifier::DocumentReference::new("PO-1"));
        inv.lines = vec![Line::new(
            "1",
            "A",
            InvoiceAmount::parse("100.00").unwrap(),
            TaxCategory::vat("S", Decimal::from(19)),
        )];
        reconcile(&mut inv).unwrap();
        inv
    }

    #[test]
    fn invalid_invoice_cannot_produce_validated() {
        let mut inv = peppol_ok();
        inv.number.clear();
        assert!(Validated::<PeppolBis3>::new(inv).is_err());
    }

    #[test]
    fn suppression_cannot_prove() {
        let inv = peppol_ok();
        let err = Check::new().without("BR-02").prove::<PeppolBis3>(inv);
        assert!(matches!(err, Err(ProveError::Suppressed(_))));
    }

    #[test]
    fn peppol_proof_widens_to_en16931() {
        let inv = peppol_ok();
        let v = Validated::<PeppolBis3>::new(inv).unwrap();
        let _core: Validated<En16931> = v.widen();
    }

    #[test]
    fn underlies_pairs_are_cius_only() {
        fn _peppol_to_core(v: Validated<PeppolBis3>) -> Validated<En16931> {
            v.widen()
        }
        // Pint / PintMy do not implement Underlies<En16931> or Underlies<PeppolBis3>.
        // If they did, the following would compile:
        // fn _pint_to_core(v: Validated<Pint>) -> Validated<En16931> { v.widen() }
    }
}
