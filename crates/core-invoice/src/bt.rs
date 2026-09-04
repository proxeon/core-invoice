//! Finding locations: [`BtId`], [`Group`], [`Path`]. Repeating-group index is 0-based.

use std::fmt;

/// Business term id (`BT-151`). Finding location, not a typed table-2 field.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BtId(
    /// Numeric suffix (`151` in `BT-151`).
    pub u16,
);

impl fmt::Display for BtId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "BT-{}", self.0)
    }
}

/// Repeating or distinguishable groups. Index in [`Path`] is **0-based**;
/// `BG-25[2]/BT-151` is the third line.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[non_exhaustive]
pub enum Group {
    /// Invoice root; no BG number.
    Document,
    /// Seller (BG-4).
    Seller,
    /// Buyer (BG-7).
    Buyer,
    /// Payee (BG-10).
    Payee,
    /// Seller tax representative (BG-11).
    TaxRepresentative,
    /// Delivery (BG-13).
    Delivery,
    /// Payment instructions (BG-16).
    Payment,
    /// Document level allowance (BG-20).
    DocumentAllowance,
    /// Document level charge (BG-21).
    DocumentCharge,
    /// Document totals (BG-22).
    Totals,
    /// Tax breakdown (BG-23 / IBG-23), not VAT-only.
    TaxBreakdown,
    /// Additional supporting document (BG-24).
    Attachment,
    /// Invoice line (BG-25).
    Line,
}

impl Group {
    /// EN 16931 BG number, or `None` for [`Group::Document`].
    pub fn bg_id(self) -> Option<u16> {
        Some(match self {
            Self::Document => return None,
            Self::Seller => 4,
            Self::Buyer => 7,
            Self::Payee => 10,
            Self::TaxRepresentative => 11,
            Self::Delivery => 13,
            Self::Payment => 16,
            Self::DocumentAllowance => 20,
            Self::DocumentCharge => 21,
            Self::Totals => 22,
            Self::TaxBreakdown => 23,
            Self::Attachment => 24,
            Self::Line => 25,
        })
    }
}

/// Finding location: group + optional 0-based index + optional [`BtId`].
///
/// Display: `BT-1`, `Invoice`, `BG-22`, `BG-22/BT-109`, `BG-25[2]/BT-151`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Path {
    /// Repeating or distinguishable group.
    pub group: Group,
    /// 0-based occurrence. `None` if the group is unindexed.
    pub index: Option<usize>,
    /// Business term, if the finding is on a specific BT.
    pub term: Option<BtId>,
}

impl Path {
    /// Document-level term (`BT-1`).
    pub fn term(term: BtId) -> Self {
        Self {
            group: Group::Document,
            index: None,
            term: Some(term),
        }
    }

    /// Repeating group at `index` (0-based) plus term (`BG-25[2]/BT-151`).
    pub fn at_term(group: Group, index: usize, term: BtId) -> Self {
        Self {
            group,
            index: Some(index),
            term: Some(term),
        }
    }

    /// Whole group, no term (`BG-22`).
    pub fn group(group: Group) -> Self {
        Self {
            group,
            index: None,
            term: None,
        }
    }

    /// Group plus term, no index (`BG-22/BT-109`).
    pub fn group_term(group: Group, term: BtId) -> Self {
        Self {
            group,
            index: None,
            term: Some(term),
        }
    }
}

impl fmt::Display for Path {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match (self.group.bg_id(), self.index, self.term) {
            (None, _, Some(t)) => write!(f, "{t}"),
            (None, _, None) => write!(f, "Invoice"),
            (Some(bg), None, None) => write!(f, "BG-{bg}"),
            (Some(bg), Some(i), None) => write!(f, "BG-{bg}[{i}]"),
            (Some(bg), None, Some(t)) => write!(f, "BG-{bg}/{t}"),
            (Some(bg), Some(i), Some(t)) => write!(f, "BG-{bg}[{i}]/{t}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_shapes() {
        assert_eq!(Path::term(BtId(1)).to_string(), "BT-1");
        assert_eq!(Path::group(Group::Totals).to_string(), "BG-22");
        assert_eq!(
            Path {
                group: Group::Totals,
                index: None,
                term: Some(BtId(109))
            }
            .to_string(),
            "BG-22/BT-109"
        );
        assert_eq!(Path::group(Group::Line).to_string(), "BG-25");
        assert_eq!(
            Path::at_term(Group::Line, 2, BtId(151)).to_string(),
            "BG-25[2]/BT-151"
        );
    }
}
