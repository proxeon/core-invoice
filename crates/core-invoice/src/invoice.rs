use crate::amount::Amount;
use crate::kind::DocumentKind;
use crate::profile::Profile;
use crate::tax::TaxCategory;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Party {
    pub name: String,
    pub country: String,
    pub tax_id: Option<String>,
    /// Scheme id for the tax identifier (e.g. TIN, BRN, VAT).
    pub id_scheme: Option<String>,
}

impl Party {
    pub fn new(name: impl Into<String>, country: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            country: country.into(),
            tax_id: None,
            id_scheme: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Line {
    pub id: String,
    pub name: String,
    pub net: Amount,
    pub tax: TaxCategory,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Invoice {
    /// Derived cache. Source of truth for BT-24 is [`Self::specification_id`].
    pub profile: Profile,
    /// BT-24 raw string, if the document stated one.
    pub specification_id: Option<String>,
    pub kind: DocumentKind,
    pub number: String,
    pub currency: String,
    pub seller: Party,
    pub buyer: Party,
    pub lines: Vec<Line>,
    pub tax_total: Amount,
    pub payable: Amount,
}

impl Invoice {
    pub fn line_net_sum(&self) -> Option<Amount> {
        self.lines
            .iter()
            .try_fold(Amount::ZERO, |acc, line| acc.checked_add(line.net))
    }
}
