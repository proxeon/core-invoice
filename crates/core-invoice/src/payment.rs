//! Exclusive payment means: BG-17 credit transfer, BG-18 card, or BG-19 direct debit.

use crate::identifier::Identifier;

/// BG-17 credit transfer. BT-84 account id (IBAN), BT-85 name, BT-86 provider (BIC).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreditTransfer {
    /// BT-84 payment account identifier (IBAN).
    pub account_id: Identifier,
    /// BT-85 payment account name.
    pub account_name: Option<String>,
    /// BT-86 payment service provider identifier (BIC).
    pub provider: Option<String>,
}

/// BG-18 card. BT-87 PAN, BT-88 holder. Tests use obviously fake PANs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaymentCard {
    /// BT-87 payment card primary account number.
    pub pan: String,
    /// BT-88 payment card holder name.
    pub holder: Option<String>,
}

/// BG-19 direct debit. BT-89 mandate, BT-90 creditor id, BT-91 debited account.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirectDebit {
    /// BT-89 mandate reference identifier.
    pub mandate: Option<String>,
    /// BT-90 bank assigned creditor identifier.
    pub creditor_id: Option<Identifier>,
    /// BT-91 debited account identifier.
    pub debited_account: Option<Identifier>,
}

/// BG-17 xor BG-18 xor BG-19. Several IBANs are several credit-transfer accounts.
/// Do not store IBAN as a string beside this enum.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PaymentMeans {
    /// BG-17 credit transfer. Several IBANs are several accounts.
    CreditTransfer(Vec<CreditTransfer>),
    /// BG-18 payment card.
    Card(PaymentCard),
    /// BG-19 direct debit.
    DirectDebit(DirectDebit),
}
