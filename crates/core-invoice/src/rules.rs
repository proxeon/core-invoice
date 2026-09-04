//! Rule registry. [`crate::validate()`] and [`explain`] share one table.

use crate::bt::{BtId, Group, Path};
use crate::invoice::Invoice;
use crate::numeric::Percentage;
use crate::report::{Finding, Report, Severity, Source};

/// One registered rule. `eval` is the only emitter of this `id`.
#[derive(Clone, Copy)]
pub struct Rule {
    /// Registered id (`BR-02`, `PEPPOL-EN16931-R010`).
    pub id: &'static str,
    /// Fatal fails [`Report::ok`]; Warning and Info do not.
    pub severity: Severity,
    /// Authority text returned by [`explain`].
    pub text: &'static str,
    /// Provenance of `id`.
    pub source: Source,
    /// Semantic check. Pushes findings onto the report.
    pub eval: fn(&Invoice, &mut Report),
}

/// Case-insensitive id match with numeric zero-padding (`br-2` = `BR-02`).
pub fn matches_id(registered: &str, query: &str) -> bool {
    let a = canonical(registered);
    let b = canonical(query);
    a.eq_ignore_ascii_case(&b)
}

fn canonical(id: &str) -> String {
    let id = id.trim();
    let Some((head, tail)) = id.rsplit_once('-') else {
        return id.to_ascii_uppercase();
    };
    if tail.chars().all(|c| c.is_ascii_digit()) {
        return format!("{}-{tail:0>2}", head.to_ascii_uppercase());
    }
    id.to_ascii_uppercase()
}

/// Authority text for `id`, or `None` if unregistered. Same table as [`crate::validate()`].
pub fn explain(id: &str) -> Option<&'static str> {
    if let Some(text) = catalogue()
        .iter()
        .find(|r| matches_id(r.id, id))
        .map(|r| r.text)
    {
        return Some(text);
    }
    #[cfg(feature = "xrechnung")]
    if let Some(r) = crate::xrechnung::RULES
        .iter()
        .find(|r| matches_id(r.id, id))
    {
        return Some(r.text);
    }
    None
}

/// CORE rules only. Peppol extras are `Profile::extra_rules`, not this list.
pub fn core_rules() -> &'static [Rule] {
    static CELL: std::sync::OnceLock<Vec<Rule>> = std::sync::OnceLock::new();
    CELL.get_or_init(|| {
        ALL.iter()
            .copied()
            .chain(crate::category::RULES.iter().copied())
            .chain(crate::codes::RULES.iter().copied())
            .chain(DEC.iter().copied())
            .collect()
    })
}

/// Markdown table of `catalogue()` × shipped profiles. Not a legal-validator claim.
pub fn conformance_matrix() -> String {
    use crate::profile::Profile;
    let profiles = [
        Profile::En16931,
        Profile::PeppolBis3,
        Profile::Pint,
        Profile::PintMy,
    ];
    let mut s = String::from(
        "# Rule matrix\n\nIds **we** emit. Fatal ids comparable to pinned ConnectingEurope / PINT-MY as evidenced by `task svrl`. Not OpenPEPPOL Valid (BIS pin is .sch). Not IRBM Valid.\n\nCORE runs on every profile. Extra rules are `Profile::extra_rules`.\n\n| id | en16931 | peppol | pint | pint-my |\n|---|---|---|---|---|\n",
    );
    for rule in catalogue() {
        s.push_str("| ");
        s.push_str(rule.id);
        for p in profiles {
            let core = crate::rules::core_rules().iter().any(|r| r.id == rule.id);
            let extra = p.extra_rules().iter().any(|r| r.id == rule.id);
            let cell = if core {
                "CORE"
            } else if extra {
                "extra"
            } else {
                "—"
            };
            s.push_str(" | ");
            s.push_str(cell);
        }
        s.push_str(" |\n");
    }
    s
}

/// Explain/rules dump: CORE plus every profile's extras so `explain PEPPOL-EN16931-R010` works.
pub fn catalogue() -> &'static [Rule] {
    static CELL: std::sync::OnceLock<Vec<Rule>> = std::sync::OnceLock::new();
    CELL.get_or_init(|| {
        #[allow(unused_mut)]
        core_rules()
            .iter()
            .copied()
            .chain(crate::peppol::RULES.iter().copied())
            .collect()
    })
}

fn spec_lookup(invoice: &Invoice, report: &mut Report) {
    let Some(id) = invoice.specification_id.as_deref() else {
        return;
    };
    if id.contains('*') {
        return;
    }
    match crate::profile::Profile::for_specification_id(id) {
        crate::profile::ProfileLookup::Unknown => {
            report.push(Finding::fatal(
                "CORE-SPEC-01",
                Path::term(BtId(24)),
                "Unrecognised specification identifier (BT-24)",
            ));
        }
        crate::profile::ProfileLookup::WrongProcess | crate::profile::ProfileLookup::Profile(_) => {
        }
    }
}

fn core_process_01(invoice: &Invoice, report: &mut Report) {
    let Some(id) = invoice.specification_id.as_deref() else {
        return;
    };
    if matches!(
        crate::profile::Profile::for_specification_id(id),
        crate::profile::ProfileLookup::WrongProcess
    ) {
        report.push(Finding::fatal(
            "CORE-PROCESS-01",
            Path::term(BtId(24)),
            "Specification identifier is a self-billing (or other) process; not billing",
        ));
    }
}

fn ibr_sr_63(invoice: &Invoice, report: &mut Report) {
    let Some(id) = invoice.specification_id.as_deref() else {
        return;
    };
    if id.contains('*') {
        report.push(Finding::fatal(
            "IBR-SR-63",
            Path::term(BtId(24)),
            "BT-24 shall not contain '*' (wildcard is an SMP capability, not an instance id)",
        ));
    }
}

fn br_01(invoice: &Invoice, report: &mut Report) {
    if invoice
        .specification_id
        .as_deref()
        .unwrap_or("")
        .trim()
        .is_empty()
    {
        report.push(Finding::fatal(
            "BR-01",
            Path::term(BtId(24)),
            "An Invoice shall have a Specification identifier (BT-24)",
        ));
    }
}

fn br_03(invoice: &Invoice, report: &mut Report) {
    if invoice.issue_date.is_none() {
        report.push(Finding::fatal(
            "BR-03",
            Path::term(BtId(2)),
            "An Invoice shall have an Invoice issue date (BT-2)",
        ));
    }
}

fn br_04(invoice: &Invoice, report: &mut Report) {
    if invoice
        .type_code
        .as_ref()
        .map(|c| c.is_empty())
        .unwrap_or(true)
    {
        report.push(Finding::fatal(
            "BR-04",
            Path::term(BtId(3)),
            "An Invoice shall have an Invoice type code (BT-3)",
        ));
    }
}

fn br_08(invoice: &Invoice, report: &mut Report) {
    if invoice.seller.address.is_none() {
        report.push(Finding::fatal(
            "BR-08",
            Path::group(Group::Seller),
            "The Seller shall have a Seller postal address (BG-5)",
        ));
    }
}

fn br_10(invoice: &Invoice, report: &mut Report) {
    if invoice.buyer.address.is_none() {
        report.push(Finding::fatal(
            "BR-10",
            Path::group(Group::Buyer),
            "The Buyer shall have a Buyer postal address (BG-8)",
        ));
    }
}

fn br_22(invoice: &Invoice, report: &mut Report) {
    for (i, line) in invoice.lines.iter().enumerate() {
        if line.quantity.is_none() {
            report.push(Finding::fatal(
                "BR-22",
                Path::at_term(Group::Line, i, BtId(129)),
                "Each Invoice line shall have an Invoiced quantity (BT-129)",
            ));
        }
    }
}

fn br_23(invoice: &Invoice, report: &mut Report) {
    // BR-23: exists(@unitCode) independent of quantity. Missing qty still fires BR-22 and BR-23.
    for (i, line) in invoice.lines.iter().enumerate() {
        if line.unit.is_none() {
            report.push(Finding::fatal(
                "BR-23",
                Path::at_term(Group::Line, i, BtId(130)),
                "An Invoice line shall have an Invoiced quantity unit of measure code (BT-130)",
            ));
        }
    }
}

fn br_24(_invoice: &Invoice, _report: &mut Report) {
    // BR-24: Line.net is not Option (BT-131); type-retired. explain still works.
}

fn br_17(invoice: &Invoice, report: &mut Report) {
    // BR-17: Payee name (BT-59) if BG-10 present.
    if let Some(p) = invoice.payee.as_ref()
        && p.name.trim().is_empty()
    {
        report.push(Finding::fatal(
            "BR-17",
            Path::term(BtId(59)),
            "Payee name (BT-59) shall be provided if Payee (BG-10) is used",
        ));
    }
}

fn br_18(invoice: &Invoice, report: &mut Report) {
    if let Some(tr) = invoice.tax_representative.as_ref()
        && tr.name.trim().is_empty()
    {
        report.push(Finding::fatal(
            "BR-18",
            Path::term(BtId(62)),
            "Seller tax representative name (BT-62) shall be provided if BG-11 is used",
        ));
    }
}

fn br_20(invoice: &Invoice, report: &mut Report) {
    if let Some(tr) = invoice.tax_representative.as_ref() {
        let cc = tr
            .address
            .as_ref()
            .and_then(|a| a.country.as_ref())
            .map(|c| c.as_str().trim())
            .unwrap_or("");
        if cc.is_empty() {
            report.push(Finding::fatal(
                "BR-20",
                Path::term(BtId(69)),
                "Tax representative country (BT-69) shall be provided if BG-11 is used",
            ));
        }
    }
}

fn br_56(invoice: &Invoice, report: &mut Report) {
    if let Some(tr) = invoice.tax_representative.as_ref()
        && tr.vat_identifier.is_none()
    {
        report.push(Finding::fatal(
            "BR-56",
            Path::term(BtId(63)),
            "Seller tax representative VAT identifier (BT-63) shall be provided if BG-11 is used",
        ));
    }
}

fn br_29(invoice: &Invoice, report: &mut Report) {
    if let Some(p) = invoice.period.as_ref()
        && let (Some(s), Some(e)) = (p.start, p.end)
        && e < s
    {
        report.push(Finding::fatal(
            "BR-29",
            Path::term(BtId(74)),
            "Invoicing period end date shall be on or after start date",
        ));
    }
}

fn br_30(invoice: &Invoice, report: &mut Report) {
    for (i, line) in invoice.lines.iter().enumerate() {
        if let Some(p) = line.period.as_ref()
            && let (Some(s), Some(e)) = (p.start, p.end)
            && e < s
        {
            report.push(Finding::fatal(
                "BR-30",
                Path::at_term(Group::Line, i, BtId(135)),
                "Invoice line period end date shall be on or after start date",
            ));
        }
    }
}

fn br_52(invoice: &Invoice, report: &mut Report) {
    for (i, d) in invoice.supporting_documents.iter().enumerate() {
        if d.id.as_str().trim().is_empty() {
            report.push(Finding::fatal(
                "BR-52",
                Path::at_term(Group::Attachment, i, BtId(122)),
                "Each additional supporting document shall contain a reference (BT-122)",
            ));
        }
    }
}

fn br_54(invoice: &Invoice, report: &mut Report) {
    for (i, line) in invoice.lines.iter().enumerate() {
        for a in &line.attributes {
            if a.name.trim().is_empty() || a.value.trim().is_empty() {
                report.push(Finding::fatal(
                    "BR-54",
                    Path::at_term(Group::Line, i, BtId(160)),
                    "Each item attribute (BG-32) shall contain name (BT-160) and value (BT-161)",
                ));
            }
        }
    }
}

fn br_55(invoice: &Invoice, report: &mut Report) {
    for (i, p) in invoice.preceding.iter().enumerate() {
        if p.reference.as_str().trim().is_empty() {
            report.push(Finding::fatal(
                "BR-55",
                Path::at_term(Group::Document, i, BtId(25)),
                "Each preceding invoice reference (BG-3) shall contain BT-25",
            ));
        }
    }
}

fn br_57(invoice: &Invoice, report: &mut Report) {
    let Some(d) = invoice.delivery.as_ref() else {
        return;
    };
    let Some(addr) = d.address.as_ref() else {
        return;
    };
    let cc = addr
        .country
        .as_ref()
        .map(|c| c.as_str().trim())
        .unwrap_or("");
    if cc.is_empty() {
        report.push(Finding::fatal(
            "BR-57",
            Path::term(BtId(80)),
            "Each deliver-to address (BG-15) shall contain country (BT-80)",
        ));
    }
}

fn br_62(invoice: &Invoice, report: &mut Report) {
    if let Some(ep) = invoice.seller.electronic_address.as_ref()
        && ep.scheme.as_deref().unwrap_or("").trim().is_empty()
    {
        report.push(Finding::fatal(
            "BR-62",
            Path::group_term(Group::Seller, BtId(34)),
            "Seller electronic address (BT-34) shall have a scheme",
        ));
    }
}

fn br_63(invoice: &Invoice, report: &mut Report) {
    if let Some(ep) = invoice.buyer.electronic_address.as_ref()
        && ep.scheme.as_deref().unwrap_or("").trim().is_empty()
    {
        report.push(Finding::fatal(
            "BR-63",
            Path::group_term(Group::Buyer, BtId(49)),
            "Buyer electronic address (BT-49) shall have a scheme",
        ));
    }
}

fn br_64(invoice: &Invoice, report: &mut Report) {
    for (i, line) in invoice.lines.iter().enumerate() {
        if let Some(id) = line.standard_id.as_ref()
            && id.scheme.as_deref().unwrap_or("").trim().is_empty()
        {
            report.push(Finding::fatal(
                "BR-64",
                Path::at_term(Group::Line, i, BtId(157)),
                "Item standard identifier (BT-157) shall have a scheme",
            ));
        }
    }
}

fn br_65(invoice: &Invoice, report: &mut Report) {
    for (i, line) in invoice.lines.iter().enumerate() {
        for cl in &line.classifications {
            if cl.scheme.as_deref().unwrap_or("").trim().is_empty() {
                report.push(Finding::fatal(
                    "BR-65",
                    Path::at_term(Group::Line, i, BtId(158)),
                    "Item classification identifier (BT-158) shall have a scheme (listID)",
                ));
            }
        }
    }
}

fn br_co_09(invoice: &Invoice, report: &mut Report) {
    // BR-CO-09: VAT ids have ISO 3166 prefix (Greece EL). Must not run on PINT-MY TIN.
    if invoice.profile == crate::profile::Profile::PintMy {
        return;
    }
    let ids = [
        invoice.seller.vat_identifier.as_ref(),
        invoice.buyer.vat_identifier.as_ref(),
        invoice
            .tax_representative
            .as_ref()
            .and_then(|t| t.vat_identifier.as_ref()),
    ];
    for id in ids.into_iter().flatten() {
        let v = id.value.trim();
        if v.len() < 2 {
            report.push(Finding::fatal(
                "BR-CO-09",
                Path::term(BtId(31)),
                "VAT identifier shall have an ISO 3166-1 alpha-2 prefix (Greece EL)",
            ));
            continue;
        }
        let prefix = &v[..2];
        let ok = prefix.eq_ignore_ascii_case("EL") || crate::codes::country(prefix);
        if !ok {
            report.push(Finding::fatal(
                "BR-CO-09",
                Path::term(BtId(31)),
                "VAT identifier shall have an ISO 3166-1 alpha-2 prefix (Greece EL)",
            ));
        }
    }
}

fn br_co_19(invoice: &Invoice, report: &mut Report) {
    if let Some(p) = invoice.period.as_ref()
        && p.start.is_none()
        && p.end.is_none()
        && invoice.tax_point_code.is_none()
    {
        report.push(Finding::fatal(
            "BR-CO-19",
            Path::term(BtId(73)),
            "If invoicing period (BG-14) is used, start or end shall be present",
        ));
    }
}

fn br_co_20(invoice: &Invoice, report: &mut Report) {
    for (i, line) in invoice.lines.iter().enumerate() {
        if let Some(p) = line.period.as_ref()
            && p.start.is_none()
            && p.end.is_none()
        {
            report.push(Finding::fatal(
                "BR-CO-20",
                Path::at_term(Group::Line, i, BtId(134)),
                "If invoice line period (BG-26) is used, start or end shall be present",
            ));
        }
    }
}

fn reason_or_code(reason: Option<&str>, code: Option<&crate::code::Code>) -> bool {
    reason.is_some_and(|s| !s.trim().is_empty())
        || code.is_some_and(|c| !c.as_str().trim().is_empty())
}

fn br_co_21(invoice: &Invoice, report: &mut Report) {
    for (i, a) in invoice.document_allowances.iter().enumerate() {
        if !reason_or_code(a.reason.as_deref(), a.reason_code.as_ref()) {
            report.push(Finding::fatal(
                "BR-CO-21",
                Path::at_term(Group::DocumentAllowance, i, BtId(97)),
                "Document level allowance shall have a reason or reason code",
            ));
        }
    }
}

fn br_co_22(invoice: &Invoice, report: &mut Report) {
    for (i, a) in invoice.document_charges.iter().enumerate() {
        if !reason_or_code(a.reason.as_deref(), a.reason_code.as_ref()) {
            report.push(Finding::fatal(
                "BR-CO-22",
                Path::at_term(Group::DocumentCharge, i, BtId(104)),
                "Document level charge shall have a reason or reason code",
            ));
        }
    }
}

fn br_co_23(invoice: &Invoice, report: &mut Report) {
    for (i, line) in invoice.lines.iter().enumerate() {
        for a in &line.allowances {
            if !reason_or_code(a.reason.as_deref(), a.reason_code.as_ref()) {
                report.push(Finding::fatal(
                    "BR-CO-23",
                    Path::at_term(Group::Line, i, BtId(139)),
                    "Invoice line allowance shall have a reason or reason code",
                ));
            }
        }
    }
}

fn br_co_24(invoice: &Invoice, report: &mut Report) {
    for (i, line) in invoice.lines.iter().enumerate() {
        for a in &line.charges {
            if !reason_or_code(a.reason.as_deref(), a.reason_code.as_ref()) {
                report.push(Finding::fatal(
                    "BR-CO-24",
                    Path::at_term(Group::Line, i, BtId(144)),
                    "Invoice line charge shall have a reason or reason code",
                ));
            }
        }
    }
}

fn br_12(invoice: &Invoice, report: &mut Report) {
    if invoice.totals.as_ref().and_then(|t| t.line_net).is_none() {
        report.push(Finding::fatal(
            "BR-12",
            Path::term(BtId(106)),
            "An Invoice shall have the Sum of Invoice line net amount (BT-106)",
        ));
    }
}

fn br_13(invoice: &Invoice, report: &mut Report) {
    if invoice
        .totals
        .as_ref()
        .and_then(|t| t.without_tax)
        .is_none()
    {
        report.push(Finding::fatal(
            "BR-13",
            Path::term(BtId(109)),
            "An Invoice shall have the Invoice total amount without VAT (BT-109)",
        ));
    }
}

fn br_14(invoice: &Invoice, report: &mut Report) {
    if invoice.totals.as_ref().and_then(|t| t.with_tax).is_none() {
        report.push(Finding::fatal(
            "BR-14",
            Path::term(BtId(112)),
            "An Invoice shall have the Invoice total amount with VAT (BT-112)",
        ));
    }
}

fn br_15(invoice: &Invoice, report: &mut Report) {
    // BR-15: PayableAmount (BT-115). Missing BG-22 or missing PayableAmount both fire.
    // A stated 0.00 is present; do not treat it as absent.
    if invoice.totals.as_ref().and_then(|t| t.payable).is_none() {
        report.push(Finding::fatal(
            "BR-15",
            Path::term(BtId(115)),
            "An Invoice shall have the Amount due for payment (BT-115)",
        ));
    }
}

fn br_19(invoice: &Invoice, report: &mut Report) {
    // BR-19: Seller tax representative postal address (BG-12) if BG-11 is used.
    if let Some(tr) = invoice.tax_representative.as_ref()
        && tr.address.is_none()
    {
        report.push(Finding::fatal(
            "BR-19",
            Path::term(BtId(64)),
            "The Seller tax representative postal address (BG-12) shall be provided if BG-11 is used",
        ));
    }
}

fn br_31(_invoice: &Invoice, _report: &mut Report) {
    // BR-31: AllowanceCharge.amount is not Option (BT-92); type-retired.
}

fn br_32(invoice: &Invoice, report: &mut Report) {
    for (i, a) in invoice.document_allowances.iter().enumerate() {
        if a.tax
            .as_ref()
            .map(|t| t.code.trim())
            .unwrap_or("")
            .is_empty()
        {
            report.push(Finding::fatal(
                "BR-32",
                Path::at_term(Group::DocumentAllowance, i, BtId(95)),
                "Each Document level allowance (BG-20) shall have a VAT category code (BT-95)",
            ));
        }
    }
}

fn br_33(invoice: &Invoice, report: &mut Report) {
    for (i, a) in invoice.document_allowances.iter().enumerate() {
        if !reason_or_code(a.reason.as_deref(), a.reason_code.as_ref()) {
            report.push(Finding::fatal(
                "BR-33",
                Path::at_term(Group::DocumentAllowance, i, BtId(97)),
                "Each Document level allowance (BG-20) shall have a reason (BT-97) or reason code (BT-98)",
            ));
        }
    }
}

fn br_36(_invoice: &Invoice, _report: &mut Report) {
    // BR-36: charge amount is not Option (BT-99); type-retired.
}

fn br_37(invoice: &Invoice, report: &mut Report) {
    for (i, a) in invoice.document_charges.iter().enumerate() {
        if a.tax
            .as_ref()
            .map(|t| t.code.trim())
            .unwrap_or("")
            .is_empty()
        {
            report.push(Finding::fatal(
                "BR-37",
                Path::at_term(Group::DocumentCharge, i, BtId(102)),
                "Each Document level charge (BG-21) shall have a VAT category code (BT-102)",
            ));
        }
    }
}

fn br_38(invoice: &Invoice, report: &mut Report) {
    for (i, a) in invoice.document_charges.iter().enumerate() {
        if !reason_or_code(a.reason.as_deref(), a.reason_code.as_ref()) {
            report.push(Finding::fatal(
                "BR-38",
                Path::at_term(Group::DocumentCharge, i, BtId(104)),
                "Each Document level charge (BG-21) shall have a reason (BT-104) or reason code (BT-105)",
            ));
        }
    }
}

fn br_41(_invoice: &Invoice, _report: &mut Report) {
    // BR-41: line allowance amount is not Option (BT-136); type-retired.
}

fn br_42(invoice: &Invoice, report: &mut Report) {
    for (i, line) in invoice.lines.iter().enumerate() {
        for a in &line.allowances {
            if !reason_or_code(a.reason.as_deref(), a.reason_code.as_ref()) {
                report.push(Finding::fatal(
                    "BR-42",
                    Path::at_term(Group::Line, i, BtId(139)),
                    "Each Invoice line allowance (BG-27) shall have a reason or reason code",
                ));
            }
        }
    }
}

fn br_43(_invoice: &Invoice, _report: &mut Report) {
    // BR-43: line charge amount is not Option (BT-141); type-retired.
}

fn br_44(invoice: &Invoice, report: &mut Report) {
    for (i, line) in invoice.lines.iter().enumerate() {
        for a in &line.charges {
            if !reason_or_code(a.reason.as_deref(), a.reason_code.as_ref()) {
                report.push(Finding::fatal(
                    "BR-44",
                    Path::at_term(Group::Line, i, BtId(144)),
                    "Each Invoice line charge shall have a reason or reason code",
                ));
            }
        }
    }
}

fn br_45(_invoice: &Invoice, _report: &mut Report) {
    // BR-45: TaxBreakdown.taxable is not Option (BT-116); type-retired.
}

fn br_46(_invoice: &Invoice, _report: &mut Report) {
    // BR-46: TaxBreakdown.tax is not Option (BT-117); type-retired.
}

fn br_47(invoice: &Invoice, report: &mut Report) {
    for (i, row) in invoice.tax_breakdown.iter().enumerate() {
        if row.category.as_str().trim().is_empty() {
            report.push(Finding::fatal(
                "BR-47",
                Path::at_term(Group::TaxBreakdown, i, BtId(118)),
                "Each VAT breakdown (BG-23) shall be defined through a VAT category code (BT-118)",
            ));
        }
    }
}

fn br_48(invoice: &Invoice, report: &mut Report) {
    for (i, row) in invoice.tax_breakdown.iter().enumerate() {
        let cat = row.category.as_str();
        // EN O has no BT-119. TTX has no IBT-119 (ALIGNED-IBRP-048).
        if cat == "O" || cat == "TTX" || row.scheme.eq_ignore_ascii_case("AAL") {
            continue;
        }
        if row.rate.is_none() {
            report.push(Finding::fatal(
                "BR-48",
                Path::at_term(Group::TaxBreakdown, i, BtId(119)),
                "Each VAT breakdown (BG-23) shall have a VAT category rate (BT-119), except if not subject to VAT",
            ));
        }
    }
}

fn br_49(invoice: &Invoice, report: &mut Report) {
    let Some(pay) = invoice.payment.as_ref() else {
        return;
    };
    if pay
        .means_code
        .as_ref()
        .map(|c| c.as_str().trim().is_empty())
        .unwrap_or(true)
    {
        report.push(Finding::fatal(
            "BR-49",
            Path::term(BtId(81)),
            "A Payment instruction (BG-16) shall specify the Payment means type code (BT-81)",
        ));
    }
}

fn br_50(invoice: &Invoice, report: &mut Report) {
    let Some(pay) = invoice.payment.as_ref() else {
        return;
    };
    let Some(crate::payment::PaymentMeans::CreditTransfer(accts)) = pay.means.as_ref() else {
        return;
    };
    if accts.is_empty() || accts.iter().any(|a| a.account_id.value.trim().is_empty()) {
        report.push(Finding::fatal(
            "BR-50",
            Path::term(BtId(84)),
            "A Payment account identifier (BT-84) shall be present if Credit transfer (BG-17) is used",
        ));
    }
}

fn br_61(invoice: &Invoice, report: &mut Report) {
    let Some(pay) = invoice.payment.as_ref() else {
        return;
    };
    let code = pay
        .means_code
        .as_ref()
        .map(|c| c.as_str().trim())
        .unwrap_or("");
    if code != "30" && code != "58" {
        return;
    }
    let has_account = matches!(
        pay.means.as_ref(),
        Some(crate::payment::PaymentMeans::CreditTransfer(a))
            if a.iter().any(|x| !x.account_id.value.trim().is_empty())
    );
    if !has_account {
        report.push(Finding::fatal(
            "BR-61",
            Path::term(BtId(84)),
            "If BT-81 is 30 or 58 (credit transfer), the Payment account identifier (BT-84) shall be present",
        ));
    }
}

fn br_co_26(invoice: &Invoice, report: &mut Report) {
    // BR-CO-26: BT-29 (not SEPA) and/or BT-30 and/or BT-31. Skip Pint/PintMy (IBR-02/04).
    if matches!(
        invoice.profile,
        crate::profile::Profile::Pint | crate::profile::Profile::PintMy
    ) {
        return;
    }
    let p = &invoice.seller;
    let vat = p
        .vat_identifier
        .as_ref()
        .is_some_and(|i| !i.value.trim().is_empty());
    let legal = p
        .legal_registration
        .as_ref()
        .is_some_and(|i| !i.value.trim().is_empty());
    let ident = p
        .identifiers
        .iter()
        .any(|i| i.scheme.as_deref() != Some("SEPA") && !i.value.trim().is_empty());
    if !(vat || legal || ident) {
        report.push(Finding::fatal(
            "BR-CO-26",
            Path::group_term(Group::Seller, BtId(29)),
            "Seller identifier (BT-29), legal registration (BT-30) and/or VAT identifier (BT-31) shall be present",
        ));
    }
}

fn br_26(invoice: &Invoice, report: &mut Report) {
    // BR-26: Item net price (BT-146) present (UBL Schematron).
    for (i, line) in invoice.lines.iter().enumerate() {
        if line.price.is_none() {
            report.push(Finding::fatal(
                "BR-26",
                Path::at_term(Group::Line, i, BtId(146)),
                "Each Invoice line shall contain the Item net price (BT-146)",
            ));
        }
    }
}

fn br_27(invoice: &Invoice, report: &mut Report) {
    // BR-27: Item net price (BT-146) shall NOT be negative.
    for (i, line) in invoice.lines.iter().enumerate() {
        if let Some(price) = line.price.as_ref()
            && price.net.raw().is_sign_negative()
        {
            report.push(Finding::fatal(
                "BR-27",
                Path::at_term(Group::Line, i, BtId(146)),
                "The Item net price (BT-146) shall NOT be negative",
            ));
        }
    }
}

fn br_28(invoice: &Invoice, report: &mut Report) {
    // BR-28: Item gross price (BT-148) shall NOT be negative.
    for (i, line) in invoice.lines.iter().enumerate() {
        if let Some(g) = line.price.as_ref().and_then(|p| p.gross)
            && g.raw().is_sign_negative()
        {
            report.push(Finding::fatal(
                "BR-28",
                Path::at_term(Group::Line, i, BtId(148)),
                "The Item gross price (BT-148) shall NOT be negative",
            ));
        }
    }
}

fn br_co_03(invoice: &Invoice, report: &mut Report) {
    // BR-CO-03: BT-7 and BT-8 are mutually exclusive.
    if invoice.tax_point_date.is_some() && invoice.tax_point_code.is_some() {
        report.push(Finding::fatal(
            "BR-CO-03",
            Path::term(BtId(7)),
            "Value added tax point date (BT-7) and Value added tax point date code (BT-8) are mutually exclusive",
        ));
    }
}

fn br_51(invoice: &Invoice, report: &mut Report) {
    // BR-51 is the sole core Warning: PAN (BT-87) at most 10 digits.
    let Some(crate::payment::PaymentMeans::Card(card)) =
        invoice.payment.as_ref().and_then(|p| p.means.as_ref())
    else {
        return;
    };
    if card.pan.chars().filter(|c| c.is_ascii_digit()).count() > 10 {
        report.push(Finding::warning(
            "BR-51",
            Path::term(BtId(87)),
            "An invoice should never include a full card primary account number (BT-87)",
        ));
    }
}

fn br_co_nlp(_invoice: &Invoice, _report: &mut Report) {
    // BR-CO-05…08: artefact test is true() (NLP). Do not invent a reason-code ontology.
}

fn br_09(invoice: &Invoice, report: &mut Report) {
    if invoice.seller.country().trim().is_empty() {
        report.push(Finding::fatal(
            "BR-09",
            Path::term(BtId(40)),
            "The Seller postal address shall contain a Seller country code (BT-40)",
        ));
    }
}

fn br_11(invoice: &Invoice, report: &mut Report) {
    if invoice.buyer.country().trim().is_empty() {
        report.push(Finding::fatal(
            "BR-11",
            Path::term(BtId(55)),
            "The Buyer postal address shall contain a Buyer country code (BT-55)",
        ));
    }
}

fn br_21(invoice: &Invoice, report: &mut Report) {
    for (i, line) in invoice.lines.iter().enumerate() {
        if line.id.trim().is_empty() {
            report.push(Finding::fatal(
                "BR-21",
                Path::at_term(Group::Line, i, BtId(126)),
                "Each Invoice line shall have an Invoice line identifier (BT-126)",
            ));
        }
    }
}

fn br_25(invoice: &Invoice, report: &mut Report) {
    for (i, line) in invoice.lines.iter().enumerate() {
        if line.name.trim().is_empty() {
            report.push(Finding::fatal(
                "BR-25",
                Path::at_term(Group::Line, i, BtId(153)),
                "Each Invoice line shall have an Item name (BT-153)",
            ));
        }
    }
}

fn br_02(invoice: &Invoice, report: &mut Report) {
    if invoice.number.trim().is_empty() {
        report.push(Finding::fatal(
            "BR-02",
            Path::term(BtId(1)),
            "Invoice number (BT-1) shall be present",
        ));
    }
}

fn br_05(invoice: &Invoice, report: &mut Report) {
    if invoice.currency.trim().is_empty() {
        report.push(Finding::fatal(
            "BR-05",
            Path::term(BtId(5)),
            "Invoice currency code (BT-5) shall be present",
        ));
    }
}

fn br_53(invoice: &Invoice, report: &mut Report) {
    // BR-53 artefact: every TaxCurrencyCode (BT-6) has a TaxTotal/TaxAmount @currencyID of that code.
    // When BT-6 equals BT-5, the document TaxTotal (BT-110) satisfies it. BT-111 is a second
    // TaxTotal only when the currencies differ. Never derive BT-111.
    let Some(tax_ccy) = invoice
        .tax_currency
        .as_ref()
        .map(|c| c.as_str())
        .filter(|c| !c.trim().is_empty())
    else {
        return;
    };
    let totals = invoice.totals.as_ref();
    let has_amount = if tax_ccy.eq_ignore_ascii_case(&invoice.currency) {
        totals.and_then(|t| t.tax_total).is_some()
    } else {
        totals.and_then(|t| t.tax_total_accounting).is_some()
    };
    if !has_amount {
        report.push(Finding::fatal(
            "BR-53",
            Path::term(BtId(111)),
            "If the VAT accounting currency code (BT-6) is present, then a TaxAmount in that currency shall be provided",
        ));
    }
}

fn br_06(invoice: &Invoice, report: &mut Report) {
    if invoice.seller.name.trim().is_empty() {
        report.push(Finding::fatal(
            "BR-06",
            Path::term(BtId(27)),
            "Seller name (BT-27) shall be present",
        ));
    }
}

fn br_07(invoice: &Invoice, report: &mut Report) {
    if invoice.buyer.name.trim().is_empty() {
        report.push(Finding::fatal(
            "BR-07",
            Path::term(BtId(44)),
            "Buyer name (BT-44) shall be present",
        ));
    }
}

fn br_16(invoice: &Invoice, report: &mut Report) {
    if invoice.lines.is_empty() {
        report.push(Finding::fatal(
            "BR-16",
            Path::group(Group::Line),
            "An invoice shall have at least one Invoice line (BG-25)",
        ));
    }
}

fn br_co_04(invoice: &Invoice, report: &mut Report) {
    // Missing BT-151 is a finding (BR-CO-04 / line tax presence), not category S.
    for (i, line) in invoice.lines.iter().enumerate() {
        if line.tax.code.trim().is_empty() {
            report.push(Finding::fatal(
                "BR-CO-04",
                Path::at_term(Group::Line, i, BtId(151)),
                "Invoiced item VAT category code (BT-151) shall be present",
            ));
        }
    }
}

// PINT-TAX: sibling profiles; PintMy.tax_systems is SST only (wire TaxScheme VAT/AAL).
fn pint_tax(invoice: &Invoice, report: &mut Report) {
    if matches!(invoice.profile, crate::profile::Profile::Unknown) {
        return;
    }
    for (i, line) in invoice.lines.iter().enumerate() {
        if line.tax.code.trim().is_empty() {
            continue;
        }
        if !invoice.profile.allows(line.tax.system) {
            report.push(Finding::fatal(
                "PINT-TAX",
                Path::at_term(Group::Line, i, BtId(151)),
                format!(
                    "Tax system {} is not allowed on profile {}",
                    line.tax.system.as_str(),
                    invoice.profile.slug()
                ),
            ));
        }
    }
}

fn pint_my_only(invoice: &Invoice) -> bool {
    invoice.profile == crate::profile::Profile::PintMy
}

fn ibr_02_my(invoice: &Invoice, report: &mut Report) {
    if !pint_my_only(invoice) {
        return;
    }
    if invoice.seller.legal_registration.is_none() {
        report.push(Finding::fatal(
            "IBR-02-MY",
            Path::term(BtId(30)),
            "Seller legal registration identifier (BRN) shall be present",
        ));
    }
}

fn ibr_03_my(invoice: &Invoice, report: &mut Report) {
    if !pint_my_only(invoice) {
        return;
    }
    if invoice.buyer.legal_registration.is_none() {
        report.push(Finding::fatal(
            "IBR-03-MY",
            Path::term(BtId(47)),
            "Buyer legal registration identifier (BRN) shall be present",
        ));
    }
}

fn ibr_04_my(invoice: &Invoice, report: &mut Report) {
    if !pint_my_only(invoice) {
        return;
    }
    if invoice.seller.tax_registration.is_none() {
        report.push(Finding::fatal(
            "IBR-04-MY",
            Path::term(BtId(32)),
            "Seller TIN (tax registration) shall be present",
        ));
    }
}

fn ibr_cl_05_my(invoice: &Invoice, report: &mut Report) {
    // IBR-CL-05-MY: BT-6 ⇒ MYR. Not BT-5. Not IRBM.
    if !pint_my_only(invoice) {
        return;
    }
    let Some(ccy) = invoice.tax_currency.as_ref() else {
        return;
    };
    if !ccy.as_str().eq_ignore_ascii_case("MYR") {
        report.push(Finding::fatal(
            "IBR-CL-05-MY",
            Path::term(BtId(6)),
            "If tax currency (BT-6 / IBT-006) is present it shall be MYR",
        ));
    }
}

fn aligned_ibrp_cl_01_my(invoice: &Invoice, report: &mut Report) {
    if !pint_my_only(invoice) {
        return;
    }
    for (i, line) in invoice.lines.iter().enumerate() {
        if line.tax.code.trim().is_empty() {
            continue;
        }
        if !crate::tax::pint_my_category(&line.tax.code) {
            report.push(Finding::fatal(
                "ALIGNED-IBRP-CL-01-MY",
                Path::at_term(Group::Line, i, BtId(151)),
                format!(
                    "Tax category {} is not a PINT-MY code (SA SE HVG LVG TTX E O)",
                    line.tax.code
                ),
            ));
        }
    }
}

fn totals_of(invoice: &Invoice) -> Option<&crate::invoice::DocumentTotals> {
    invoice.totals.as_ref()
}

fn overflow(report: &mut Report, id: &'static str, term: u16, label: &str) {
    report.push(Finding::fatal(
        id,
        Path::group_term(Group::Totals, BtId(term)),
        format!("{label} overflowed; amounts are not representable"),
    ));
}

fn br_co_10(invoice: &Invoice, report: &mut Report) {
    let Some(totals) = totals_of(invoice) else {
        return;
    };
    let Some(expected) =
        crate::amount::InvoiceAmount::checked_sum(invoice.lines.iter().map(|l| l.net))
    else {
        overflow(report, "BR-CO-10", 106, "BT-106");
        return;
    };
    match totals.line_net {
        Some(stated) if stated != expected => report.push(Finding::fatal(
            "BR-CO-10",
            Path::group_term(Group::Totals, BtId(106)),
            format!("BT-106 {stated} ≠ Σ BT-131 {expected}"),
        )),
        None => report.push(Finding::fatal(
            "BR-CO-10",
            Path::group_term(Group::Totals, BtId(106)),
            format!("BT-106 is absent; expected {expected}"),
        )),
        _ => {}
    }
}

fn br_co_11(invoice: &Invoice, report: &mut Report) {
    let Some(totals) = totals_of(invoice) else {
        return;
    };
    let path = Path::group_term(Group::Totals, BtId(107));
    let Some(expected) = crate::amount::InvoiceAmount::checked_sum(
        invoice.document_allowances.iter().map(|a| a.amount),
    ) else {
        overflow(report, "BR-CO-11", 107, "BT-107");
        return;
    };
    match (
        invoice.document_allowances.is_empty(),
        totals.allowance_total,
    ) {
        (true, None) => {}
        // BR-CO-11 artefact: present BT-107 = Σ BG-20. Empty sum is 0, so 0 with no BG-20 is valid.
        (true, Some(stated)) if stated.is_zero() => {}
        (true, Some(stated)) => report.push(Finding::fatal(
            "BR-CO-11",
            path,
            format!("BT-107 {stated} ≠ Σ BT-92 0.00 (no BG-20)"),
        )),
        (false, None) => report.push(Finding::fatal(
            "BR-CO-11",
            path,
            format!("BT-107 is absent; expected Σ BT-92 {expected}"),
        )),
        (false, Some(stated)) if stated != expected => report.push(Finding::fatal(
            "BR-CO-11",
            path,
            format!("BT-107 {stated} ≠ Σ BT-92 {expected}"),
        )),
        _ => {}
    }
}

fn br_co_12(invoice: &Invoice, report: &mut Report) {
    let Some(totals) = totals_of(invoice) else {
        return;
    };
    let path = Path::group_term(Group::Totals, BtId(108));
    let Some(expected) = crate::amount::InvoiceAmount::checked_sum(
        invoice.document_charges.iter().map(|c| c.amount),
    ) else {
        overflow(report, "BR-CO-12", 108, "BT-108");
        return;
    };
    match (invoice.document_charges.is_empty(), totals.charge_total) {
        (true, None) => {}
        // BR-CO-12 artefact: present BT-108 = Σ BG-21. Empty sum is 0.
        (true, Some(stated)) if stated.is_zero() => {}
        (true, Some(stated)) => report.push(Finding::fatal(
            "BR-CO-12",
            path,
            format!("BT-108 {stated} ≠ Σ BT-99 0.00 (no BG-21)"),
        )),
        (false, None) => report.push(Finding::fatal(
            "BR-CO-12",
            path,
            format!("BT-108 is absent; expected Σ BT-99 {expected}"),
        )),
        (false, Some(stated)) if stated != expected => report.push(Finding::fatal(
            "BR-CO-12",
            path,
            format!("BT-108 {stated} ≠ Σ BT-99 {expected}"),
        )),
        _ => {}
    }
}

fn br_co_13(invoice: &Invoice, report: &mut Report) {
    let Some(totals) = totals_of(invoice) else {
        return;
    };
    let Some(line_net) = totals.line_net else {
        return;
    };
    let expected = match (totals.allowance_total, totals.charge_total) {
        (None, None) => Some(line_net),
        (Some(a), None) => line_net.checked_sub(a),
        (None, Some(c)) => line_net.checked_add(c),
        (Some(a), Some(c)) => line_net.checked_sub(a).and_then(|v| v.checked_add(c)),
    };
    let Some(expected) = expected else {
        overflow(report, "BR-CO-13", 109, "BT-109");
        return;
    };
    match totals.without_tax {
        Some(stated) if stated != expected => report.push(Finding::fatal(
            "BR-CO-13",
            Path::group_term(Group::Totals, BtId(109)),
            format!("BT-109 {stated} ≠ BT-106 − BT-107 + BT-108 = {expected}"),
        )),
        None => report.push(Finding::fatal(
            "BR-CO-13",
            Path::group_term(Group::Totals, BtId(109)),
            format!("BT-109 is absent; expected {expected}"),
        )),
        _ => {}
    }
}

fn br_co_14(invoice: &Invoice, report: &mut Report) {
    let Some(totals) = totals_of(invoice) else {
        return;
    };
    let path = Path::group_term(Group::Totals, BtId(110));
    let rows = invoice
        .tax_breakdown
        .iter()
        .filter(|e| crate::reconcile::counts_toward_tax_total(invoice.profile, e));
    let Some(expected) = crate::amount::InvoiceAmount::checked_sum(rows.map(|e| e.tax)) else {
        overflow(report, "BR-CO-14", 110, "BT-110");
        return;
    };
    match totals.tax_total {
        Some(stated) if stated != expected => report.push(Finding::fatal(
            "BR-CO-14",
            path,
            format!("BT-110 {stated} ≠ Σ BT-117 {expected}"),
        )),
        None if !expected.is_zero() => report.push(Finding::fatal(
            "BR-CO-14",
            path,
            format!("BT-110 is absent; expected {expected}"),
        )),
        _ => {}
    }
}

fn br_co_15(invoice: &Invoice, report: &mut Report) {
    let Some(totals) = totals_of(invoice) else {
        return;
    };
    let Some(without) = totals.without_tax else {
        return;
    };
    let tax = totals
        .tax_total
        .unwrap_or(crate::amount::InvoiceAmount::ZERO);
    let Some(expected) = without.checked_add(tax) else {
        overflow(report, "BR-CO-15", 112, "BT-112");
        return;
    };
    match totals.with_tax {
        Some(stated) if stated != expected => report.push(Finding::fatal(
            "BR-CO-15",
            Path::group_term(Group::Totals, BtId(112)),
            format!("BT-112 {stated} ≠ BT-109 + BT-110 = {expected}"),
        )),
        None => report.push(Finding::fatal(
            "BR-CO-15",
            Path::group_term(Group::Totals, BtId(112)),
            format!("BT-112 is absent; expected {expected}"),
        )),
        _ => {}
    }
}

fn br_co_16(invoice: &Invoice, report: &mut Report) {
    let Some(totals) = totals_of(invoice) else {
        return;
    };
    let Some(with_tax) = totals.with_tax else {
        return;
    };
    let expected = match (totals.paid, totals.rounding) {
        (None, None) => Some(with_tax),
        (Some(p), None) => with_tax.checked_sub(p),
        (None, Some(r)) => with_tax.checked_add(r),
        (Some(p), Some(r)) => with_tax.checked_sub(p).and_then(|v| v.checked_add(r)),
    };
    let Some(expected) = expected else {
        overflow(report, "BR-CO-16", 115, "BT-115");
        return;
    };
    let Some(stated) = totals.payable else {
        return;
    };
    if stated != expected {
        report.push(Finding::fatal(
            "BR-CO-16",
            Path::group_term(Group::Totals, BtId(115)),
            format!("BT-115 {stated} ≠ BT-112 − BT-113 + BT-114 = {expected}"),
        ));
    }
}

fn br_co_17(invoice: &Invoice, report: &mut Report) {
    use crate::arith::{derived_vat, within_vat_tolerance, xpath_round};
    use rust_decimal::Decimal;
    for (i, e) in invoice.tax_breakdown.iter().enumerate() {
        if e.category.as_str().eq_ignore_ascii_case("TTX") {
            continue;
        }
        let path = Path::at_term(Group::TaxBreakdown, i, BtId(117));
        let rate = e.rate.map_or(Decimal::ZERO, Percentage::as_percent);
        if xpath_round(rate) == Decimal::ZERO {
            if xpath_round(e.tax.raw()) != Decimal::ZERO {
                report.push(Finding::fatal(
                    "BR-CO-17",
                    path,
                    format!("zero-rate group must have tax 0 (found {})", e.tax),
                ));
            }
            continue;
        }
        let Some(expected) = derived_vat(e.taxable.raw(), rate) else {
            continue;
        };
        let stated = e.tax.raw().abs();
        if !within_vat_tolerance(stated, expected) {
            report.push(Finding::fatal(
                "BR-CO-17",
                path,
                format!(
                    "BT-117 {} is not within ±1.00 exclusive of derived {expected}",
                    e.tax
                ),
            ));
        }
    }
}

/// Presence, co-occurrence, and totals rows in CORE (before category/codes/DEC).
pub static ALL: &[Rule] = &[
    Rule {
        id: "CORE-SPEC-01",
        severity: Severity::Fatal,
        text: "Unrecognised specification identifier (BT-24).",
        source: Source::Crate,
        eval: spec_lookup,
    },
    Rule {
        id: "CORE-PROCESS-01",
        severity: Severity::Fatal,
        text: "Self-billing (and other) process URNs are not validated as billing.",
        source: Source::Crate,
        eval: core_process_01,
    },
    Rule {
        id: "IBR-SR-63",
        severity: Severity::Fatal,
        text: "BT-24 must not contain '*'.",
        source: Source::Crate,
        eval: ibr_sr_63,
    },
    Rule {
        id: "BR-01",
        severity: Severity::Fatal,
        text: "An Invoice shall have a Specification identifier (BT-24).",
        source: Source::Both,
        eval: br_01,
    },
    Rule {
        id: "BR-02",
        severity: Severity::Fatal,
        text: "Invoice number (BT-1) shall be present.",
        source: Source::Both,
        eval: br_02,
    },
    Rule {
        id: "BR-03",
        severity: Severity::Fatal,
        text: "An Invoice shall have an Invoice issue date (BT-2).",
        source: Source::Both,
        eval: br_03,
    },
    Rule {
        id: "BR-04",
        severity: Severity::Fatal,
        text: "An Invoice shall have an Invoice type code (BT-3).",
        source: Source::Both,
        eval: br_04,
    },
    Rule {
        id: "BR-08",
        severity: Severity::Fatal,
        text: "The Seller shall have a Seller postal address (BG-5).",
        source: Source::Both,
        eval: br_08,
    },
    Rule {
        id: "BR-09",
        severity: Severity::Fatal,
        text: "The Seller postal address shall contain a Seller country code (BT-40).",
        source: Source::Both,
        eval: br_09,
    },
    Rule {
        id: "BR-10",
        severity: Severity::Fatal,
        text: "The Buyer shall have a Buyer postal address (BG-8).",
        source: Source::Both,
        eval: br_10,
    },
    Rule {
        id: "BR-11",
        severity: Severity::Fatal,
        text: "The Buyer postal address shall contain a Buyer country code (BT-55).",
        source: Source::Both,
        eval: br_11,
    },
    Rule {
        id: "BR-21",
        severity: Severity::Fatal,
        text: "Each Invoice line shall have an Invoice line identifier (BT-126).",
        source: Source::Both,
        eval: br_21,
    },
    Rule {
        id: "BR-25",
        severity: Severity::Fatal,
        text: "Each Invoice line shall have an Item name (BT-153).",
        source: Source::Both,
        eval: br_25,
    },
    Rule {
        id: "BR-22",
        severity: Severity::Fatal,
        text: "Each Invoice line shall have an Invoiced quantity (BT-129).",
        source: Source::Both,
        eval: br_22,
    },
    Rule {
        id: "BR-23",
        severity: Severity::Fatal,
        text: "An Invoice line shall have an Invoiced quantity unit of measure code (BT-130).",
        source: Source::Both,
        eval: br_23,
    },
    Rule {
        id: "BR-24",
        severity: Severity::Fatal,
        text: "Each Invoice line shall have an Invoice line net amount (BT-131).",
        source: Source::Both,
        eval: br_24,
    },
    Rule {
        id: "BR-26",
        severity: Severity::Fatal,
        text: "Each Invoice line shall contain the Item net price (BT-146).",
        source: Source::Both,
        eval: br_26,
    },
    Rule {
        id: "BR-27",
        severity: Severity::Fatal,
        text: "The Item net price (BT-146) shall NOT be negative.",
        source: Source::Both,
        eval: br_27,
    },
    Rule {
        id: "BR-28",
        severity: Severity::Fatal,
        text: "The Item gross price (BT-148) shall NOT be negative.",
        source: Source::Both,
        eval: br_28,
    },
    Rule {
        id: "BR-51",
        severity: Severity::Warning,
        text: "An invoice should never include a full card primary account number (BT-87).",
        source: Source::Both,
        eval: br_51,
    },
    Rule {
        id: "BR-17",
        severity: Severity::Fatal,
        text: "Payee name (BT-59) shall be provided if Payee (BG-10) is used.",
        source: Source::Both,
        eval: br_17,
    },
    Rule {
        id: "BR-18",
        severity: Severity::Fatal,
        text: "Seller tax representative name (BT-62) shall be provided if BG-11 is used.",
        source: Source::Both,
        eval: br_18,
    },
    Rule {
        id: "BR-20",
        severity: Severity::Fatal,
        text: "Tax representative country (BT-69) shall be provided if BG-11 is used.",
        source: Source::Both,
        eval: br_20,
    },
    Rule {
        id: "BR-56",
        severity: Severity::Fatal,
        text: "Seller tax representative VAT identifier (BT-63) shall be provided if BG-11 is used.",
        source: Source::Both,
        eval: br_56,
    },
    Rule {
        id: "BR-29",
        severity: Severity::Fatal,
        text: "Invoicing period end date shall be on or after start date.",
        source: Source::Both,
        eval: br_29,
    },
    Rule {
        id: "BR-30",
        severity: Severity::Fatal,
        text: "Invoice line period end date shall be on or after start date.",
        source: Source::Both,
        eval: br_30,
    },
    Rule {
        id: "BR-52",
        severity: Severity::Fatal,
        text: "Each additional supporting document shall contain a reference (BT-122).",
        source: Source::Both,
        eval: br_52,
    },
    Rule {
        id: "BR-54",
        severity: Severity::Fatal,
        text: "Each item attribute (BG-32) shall contain name (BT-160) and value (BT-161).",
        source: Source::Both,
        eval: br_54,
    },
    Rule {
        id: "BR-55",
        severity: Severity::Fatal,
        text: "Each preceding invoice reference (BG-3) shall contain BT-25.",
        source: Source::Both,
        eval: br_55,
    },
    Rule {
        id: "BR-57",
        severity: Severity::Fatal,
        text: "Each deliver-to address (BG-15) shall contain country (BT-80).",
        source: Source::Both,
        eval: br_57,
    },
    Rule {
        id: "BR-62",
        severity: Severity::Fatal,
        text: "Seller electronic address (BT-34) shall have a scheme.",
        source: Source::Both,
        eval: br_62,
    },
    Rule {
        id: "BR-63",
        severity: Severity::Fatal,
        text: "Buyer electronic address (BT-49) shall have a scheme.",
        source: Source::Both,
        eval: br_63,
    },
    Rule {
        id: "BR-64",
        severity: Severity::Fatal,
        text: "Item standard identifier (BT-157) shall have a scheme.",
        source: Source::Both,
        eval: br_64,
    },
    Rule {
        id: "BR-65",
        severity: Severity::Fatal,
        text: "Item classification identifier (BT-158) shall have a scheme (listID).",
        source: Source::Both,
        eval: br_65,
    },
    Rule {
        id: "BR-CO-09",
        severity: Severity::Fatal,
        text: "VAT identifiers shall have an ISO 3166-1 alpha-2 prefix (Greece EL). Not PINT-MY TIN.",
        source: Source::Both,
        eval: br_co_09,
    },
    Rule {
        id: "BR-CO-19",
        severity: Severity::Fatal,
        text: "If invoicing period (BG-14) is used, start or end shall be present.",
        source: Source::Both,
        eval: br_co_19,
    },
    Rule {
        id: "BR-CO-20",
        severity: Severity::Fatal,
        text: "If invoice line period (BG-26) is used, start or end shall be present.",
        source: Source::Both,
        eval: br_co_20,
    },
    Rule {
        id: "BR-CO-21",
        severity: Severity::Fatal,
        text: "Document level allowance shall have a reason or reason code.",
        source: Source::Both,
        eval: br_co_21,
    },
    Rule {
        id: "BR-CO-22",
        severity: Severity::Fatal,
        text: "Document level charge shall have a reason or reason code.",
        source: Source::Both,
        eval: br_co_22,
    },
    Rule {
        id: "BR-CO-23",
        severity: Severity::Fatal,
        text: "Invoice line allowance shall have a reason or reason code.",
        source: Source::Both,
        eval: br_co_23,
    },
    Rule {
        id: "BR-CO-24",
        severity: Severity::Fatal,
        text: "Invoice line charge shall have a reason or reason code.",
        source: Source::Both,
        eval: br_co_24,
    },
    Rule {
        id: "BR-12",
        severity: Severity::Fatal,
        text: "An Invoice shall have the Sum of Invoice line net amount (BT-106).",
        source: Source::Both,
        eval: br_12,
    },
    Rule {
        id: "BR-13",
        severity: Severity::Fatal,
        text: "An Invoice shall have the Invoice total amount without VAT (BT-109).",
        source: Source::Both,
        eval: br_13,
    },
    Rule {
        id: "BR-14",
        severity: Severity::Fatal,
        text: "An Invoice shall have the Invoice total amount with VAT (BT-112).",
        source: Source::Both,
        eval: br_14,
    },
    Rule {
        id: "BR-15",
        severity: Severity::Fatal,
        text: "An Invoice shall have the Amount due for payment (BT-115).",
        source: Source::Both,
        eval: br_15,
    },
    Rule {
        id: "BR-19",
        severity: Severity::Fatal,
        text: "The Seller tax representative postal address (BG-12) shall be provided if BG-11 is used.",
        source: Source::Both,
        eval: br_19,
    },
    Rule {
        id: "BR-31",
        severity: Severity::Fatal,
        text: "Each Document level allowance (BG-20) shall have a Document level allowance amount (BT-92).",
        source: Source::Both,
        eval: br_31,
    },
    Rule {
        id: "BR-32",
        severity: Severity::Fatal,
        text: "Each Document level allowance (BG-20) shall have a VAT category code (BT-95).",
        source: Source::Both,
        eval: br_32,
    },
    Rule {
        id: "BR-33",
        severity: Severity::Fatal,
        text: "Each Document level allowance (BG-20) shall have a reason (BT-97) or reason code (BT-98).",
        source: Source::Both,
        eval: br_33,
    },
    Rule {
        id: "BR-36",
        severity: Severity::Fatal,
        text: "Each Document level charge (BG-21) shall have a Document level charge amount (BT-99).",
        source: Source::Both,
        eval: br_36,
    },
    Rule {
        id: "BR-37",
        severity: Severity::Fatal,
        text: "Each Document level charge (BG-21) shall have a VAT category code (BT-102).",
        source: Source::Both,
        eval: br_37,
    },
    Rule {
        id: "BR-38",
        severity: Severity::Fatal,
        text: "Each Document level charge (BG-21) shall have a reason (BT-104) or reason code (BT-105).",
        source: Source::Both,
        eval: br_38,
    },
    Rule {
        id: "BR-41",
        severity: Severity::Fatal,
        text: "Each Invoice line allowance (BG-27) shall have an Invoice line allowance amount (BT-136).",
        source: Source::Both,
        eval: br_41,
    },
    Rule {
        id: "BR-42",
        severity: Severity::Fatal,
        text: "Each Invoice line allowance (BG-27) shall have a reason or reason code.",
        source: Source::Both,
        eval: br_42,
    },
    Rule {
        id: "BR-43",
        severity: Severity::Fatal,
        text: "Each Invoice line charge (BG-28) shall have an Invoice line charge amount (BT-141).",
        source: Source::Both,
        eval: br_43,
    },
    Rule {
        id: "BR-44",
        severity: Severity::Fatal,
        text: "Each Invoice line charge shall have a reason or reason code.",
        source: Source::Both,
        eval: br_44,
    },
    Rule {
        id: "BR-45",
        severity: Severity::Fatal,
        text: "Each VAT breakdown (BG-23) shall have a VAT category taxable amount (BT-116).",
        source: Source::Both,
        eval: br_45,
    },
    Rule {
        id: "BR-46",
        severity: Severity::Fatal,
        text: "Each VAT breakdown (BG-23) shall have a VAT category tax amount (BT-117).",
        source: Source::Both,
        eval: br_46,
    },
    Rule {
        id: "BR-47",
        severity: Severity::Fatal,
        text: "Each VAT breakdown (BG-23) shall be defined through a VAT category code (BT-118).",
        source: Source::Both,
        eval: br_47,
    },
    Rule {
        id: "BR-48",
        severity: Severity::Fatal,
        text: "Each VAT breakdown (BG-23) shall have a VAT category rate (BT-119), except if not subject to VAT.",
        source: Source::Both,
        eval: br_48,
    },
    Rule {
        id: "BR-49",
        severity: Severity::Fatal,
        text: "A Payment instruction (BG-16) shall specify the Payment means type code (BT-81).",
        source: Source::Both,
        eval: br_49,
    },
    Rule {
        id: "BR-50",
        severity: Severity::Fatal,
        text: "A Payment account identifier (BT-84) shall be present if Credit transfer (BG-17) is used.",
        source: Source::Both,
        eval: br_50,
    },
    Rule {
        id: "BR-61",
        severity: Severity::Fatal,
        text: "If BT-81 is 30 or 58, the Payment account identifier (BT-84) shall be present.",
        source: Source::Both,
        eval: br_61,
    },
    Rule {
        id: "BR-CO-26",
        severity: Severity::Fatal,
        text: "Seller identifier (BT-29), legal registration (BT-30) and/or VAT identifier (BT-31) shall be present.",
        source: Source::Both,
        eval: br_co_26,
    },
    Rule {
        id: "BR-CO-03",
        severity: Severity::Fatal,
        text: "Value added tax point date (BT-7) and Value added tax point date code (BT-8) are mutually exclusive.",
        source: Source::Both,
        eval: br_co_03,
    },
    Rule {
        id: "BR-CO-05",
        severity: Severity::Fatal,
        text: "Document level allowance reason code and reason shall indicate the same type of allowance. Artefact test is true() (NLP).",
        source: Source::ArtefactOnly,
        eval: br_co_nlp,
    },
    Rule {
        id: "BR-CO-06",
        severity: Severity::Fatal,
        text: "Document level charge reason code and reason shall indicate the same type of charge. Artefact test is true() (NLP).",
        source: Source::ArtefactOnly,
        eval: br_co_nlp,
    },
    Rule {
        id: "BR-CO-07",
        severity: Severity::Fatal,
        text: "Invoice line allowance reason code and reason shall indicate the same type. Artefact test is true() (NLP).",
        source: Source::ArtefactOnly,
        eval: br_co_nlp,
    },
    Rule {
        id: "BR-CO-08",
        severity: Severity::Fatal,
        text: "Invoice line charge reason code and reason shall indicate the same type. Artefact test is true() (NLP).",
        source: Source::ArtefactOnly,
        eval: br_co_nlp,
    },
    Rule {
        id: "BR-05",
        severity: Severity::Fatal,
        text: "Invoice currency code (BT-5) shall be present.",
        source: Source::Both,
        eval: br_05,
    },
    Rule {
        id: "BR-53",
        severity: Severity::Fatal,
        text: "If BT-6 is present, a TaxAmount in that currency shall exist (BT-110 when BT-6=BT-5, else BT-111). Never derived.",
        source: Source::Both,
        eval: br_53,
    },
    Rule {
        id: "BR-06",
        severity: Severity::Fatal,
        text: "Seller name (BT-27) shall be present.",
        source: Source::Both,
        eval: br_06,
    },
    Rule {
        id: "BR-07",
        severity: Severity::Fatal,
        text: "Buyer name (BT-44) shall be present.",
        source: Source::Both,
        eval: br_07,
    },
    Rule {
        id: "BR-16",
        severity: Severity::Fatal,
        text: "An Invoice shall have at least one Invoice line (BG-25).",
        source: Source::Both,
        eval: br_16,
    },
    Rule {
        id: "BR-CO-04",
        severity: Severity::Fatal,
        text: "Each Invoice line shall have an Invoiced item VAT category code (BT-151).",
        source: Source::Both,
        eval: br_co_04,
    },
    Rule {
        id: "BR-CO-10",
        severity: Severity::Fatal,
        text: "Sum of Invoice line net amount (BT-106) = Σ Invoice line net amount (BT-131).",
        source: Source::Both,
        eval: br_co_10,
    },
    Rule {
        id: "BR-CO-11",
        severity: Severity::Fatal,
        text: "Sum of allowances on document level (BT-107) = Σ Document level allowance amount (BT-92). Present 0 with no BG-20 is valid (empty sum).",
        source: Source::Both,
        eval: br_co_11,
    },
    Rule {
        id: "BR-CO-12",
        severity: Severity::Fatal,
        text: "Sum of charges on document level (BT-108) = Σ Document level charge amount (BT-99). Present 0 with no BG-21 is valid (empty sum).",
        source: Source::Both,
        eval: br_co_12,
    },
    Rule {
        id: "BR-CO-13",
        severity: Severity::Fatal,
        text: "Invoice total amount without VAT (BT-109) = BT-106 − BT-107 + BT-108 (four presence branches; absent ≠ 0).",
        source: Source::Both,
        eval: br_co_13,
    },
    Rule {
        id: "BR-CO-14",
        severity: Severity::Fatal,
        text: "Invoice total tax amount (BT-110) = Σ tax category tax amount (BT-117). Exact. PINT IBR-CO-14 sums every IBG-23 row, including TTX/AAL.",
        source: Source::Both,
        eval: br_co_14,
    },
    Rule {
        id: "BR-CO-15",
        severity: Severity::Fatal,
        text: "Invoice total amount with VAT (BT-112) = BT-109 + BT-110.",
        source: Source::Both,
        eval: br_co_15,
    },
    Rule {
        id: "BR-CO-16",
        severity: Severity::Fatal,
        text: "Amount due for payment (BT-115) = Invoice total amount with VAT (BT-112) − Paid amount (BT-113) + Rounding amount (BT-114).",
        source: Source::Both,
        eval: br_co_16,
    },
    Rule {
        id: "BR-CO-17",
        severity: Severity::Fatal,
        text: "VAT category tax amount (BT-117) = VAT category taxable amount (BT-116) × (VAT category rate (BT-119) / 100), rounded to two decimals. Artefact slack ±1.00 exclusive on abs; zero-rate branch has no slack.",
        source: Source::Both,
        eval: br_co_17,
    },
    Rule {
        id: "PINT-TAX",
        severity: Severity::Fatal,
        // PINT-TAX: sibling profiles; PintMy.tax_systems is SST only.
        text: "Tax system on a line must be allowed by the profile. EN 16931 / Peppol BIS 3.0: VAT only. PINT: VAT, GST, SST, consumption. PINT-MY: SST only.",
        source: Source::Crate,
        eval: pint_tax,
    },
    Rule {
        id: "IBR-02-MY",
        severity: Severity::Fatal,
        text: "Seller legal registration identifier (BRN / IBT-030) shall be present.",
        source: Source::Crate,
        eval: ibr_02_my,
    },
    Rule {
        id: "IBR-03-MY",
        severity: Severity::Fatal,
        text: "Buyer legal registration identifier (BRN / IBT-047) shall be present.",
        source: Source::Crate,
        eval: ibr_03_my,
    },
    Rule {
        id: "IBR-04-MY",
        severity: Severity::Fatal,
        text: "Seller TIN (IBT-032) shall be present.",
        source: Source::Crate,
        eval: ibr_04_my,
    },
    Rule {
        id: "IBR-CL-05-MY",
        severity: Severity::Fatal,
        text: "If tax accounting currency (IBT-006 / BT-6) is present, it shall be MYR. Invoice currency (BT-5) is not forced to MYR.",
        source: Source::Crate,
        eval: ibr_cl_05_my,
    },
    Rule {
        id: "ALIGNED-IBRP-CL-01-MY",
        severity: Severity::Fatal,
        text: "Malaysian invoice tax categories shall be SA, SE, HVG, LVG, TTX, E or O.",
        source: Source::Crate,
        eval: aligned_ibrp_cl_01_my,
    },
];

/// BR-DEC-* are Amount.Type. InvoiceAmount refuses a third digit; these rows exist so explain() resolves artefact ids.
fn br_dec_pass(_invoice: &Invoice, _report: &mut Report) {}

macro_rules! dec {
    ($id:literal, $text:literal) => {
        Rule {
            id: $id,
            severity: Severity::Fatal,
            text: $text,
            source: Source::Both,
            eval: br_dec_pass,
        }
    };
}

/// Amount.Type decimal rows. Eval is a no-op so [`explain`] resolves artefact ids.
pub static DEC: &[Rule] = &[
    dec!(
        "BR-DEC-01",
        "Document level allowance amount (BT-92) has at most 2 decimals."
    ),
    dec!(
        "BR-DEC-02",
        "Document level allowance base amount (BT-93) has at most 2 decimals."
    ),
    dec!(
        "BR-DEC-05",
        "Document level charge amount (BT-99) has at most 2 decimals."
    ),
    dec!(
        "BR-DEC-06",
        "Document level charge base amount (BT-100) has at most 2 decimals."
    ),
    dec!(
        "BR-DEC-09",
        "Sum of invoice line net amount (BT-106) has at most 2 decimals."
    ),
    dec!(
        "BR-DEC-10",
        "Sum of allowances on document level (BT-107) has at most 2 decimals."
    ),
    dec!(
        "BR-DEC-11",
        "Sum of charges on document level (BT-108) has at most 2 decimals."
    ),
    dec!(
        "BR-DEC-12",
        "Invoice total amount without VAT (BT-109) has at most 2 decimals."
    ),
    dec!(
        "BR-DEC-13",
        "Invoice total VAT amount (BT-110) has at most 2 decimals."
    ),
    dec!(
        "BR-DEC-14",
        "Invoice total amount with VAT (BT-112) has at most 2 decimals."
    ),
    dec!(
        "BR-DEC-15",
        "Invoice total VAT amount in accounting currency (BT-111) has at most 2 decimals."
    ),
    dec!("BR-DEC-16", "Paid amount (BT-113) has at most 2 decimals."),
    dec!(
        "BR-DEC-17",
        "Rounding amount (BT-114) has at most 2 decimals."
    ),
    dec!(
        "BR-DEC-18",
        "Amount due for payment (BT-115) has at most 2 decimals."
    ),
    dec!(
        "BR-DEC-19",
        "VAT category taxable amount (BT-116) has at most 2 decimals."
    ),
    dec!(
        "BR-DEC-20",
        "VAT category tax amount (BT-117) has at most 2 decimals."
    ),
    dec!(
        "BR-DEC-23",
        "Invoice line net amount (BT-131) has at most 2 decimals."
    ),
    dec!(
        "BR-DEC-24",
        "Invoice line allowance amount (BT-136) has at most 2 decimals."
    ),
    dec!(
        "BR-DEC-25",
        "Invoice line charge amount (BT-141) has at most 2 decimals."
    ),
    dec!(
        "BR-DEC-27",
        "Item net price (BT-146) — Amount.Type is two decimals on InvoiceAmount only; unit price is not this row."
    ),
    dec!(
        "BR-DEC-28",
        "Item gross price (BT-148) — Amount.Type is two decimals on InvoiceAmount only."
    ),
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::code::Code;

    #[test]
    fn matrix_lists_catalogue_ids() {
        let matrix = crate::conformance_matrix();
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/matrix.md");
        let on_disk = std::fs::read_to_string(&path).expect("docs/matrix.md");
        assert_eq!(
            on_disk, matrix,
            "docs/matrix.md is stale; replace it with core_invoice::conformance_matrix()"
        );
        for rule in catalogue() {
            assert!(
                matrix.contains(rule.id),
                "{} missing from generated matrix",
                rule.id
            );
        }
        assert!(matrix.contains("Not IRBM Valid"));
        assert!(matrix.contains("CORE"));
        assert!(
            matrix.contains("| id | en16931 | peppol | pint | pint-my |"),
            "{matrix}"
        );
        assert!(
            !matrix.contains("| xrechnung |"),
            "XRechnung is not a matrix column"
        );
        for id in ["BR-DE-1", "BR-DE-15", "BR-DE-18", "BR-TMP-2"] {
            assert!(
                !matrix.contains(&format!("| {id} |")),
                "{id} is overlay, not a catalogue row"
            );
        }
        assert_eq!(crate::profile::Profile::parse("xrechnung"), None);
        assert!(!crate::profile::Profile::known_slugs().contains("xrechnung"));
    }

    #[cfg(feature = "xrechnung")]
    #[test]
    fn explain_br_de_15_with_feature() {
        assert!(explain("BR-DE-15").is_some());
        assert!(explain("BR-DE-18").is_some());
        assert!(explain("BR-TMP-2").is_some());
    }

    #[cfg(not(feature = "xrechnung"))]
    #[test]
    fn explain_br_de_15_without_feature() {
        assert!(explain("BR-DE-15").is_none());
        assert!(explain("BR-DE-18").is_none());
        assert!(explain("BR-TMP-2").is_none());
    }

    #[test]
    fn padding_matches() {
        assert!(matches_id("BR-02", "br-2"));
        assert!(matches_id("BR-CO-16", "BR-CO-16"));
        assert!(explain("br-02").unwrap().contains("BT-1"));
        assert!(explain("nope").is_none());
        assert!(explain("BR-DEC-12").unwrap().contains("BT-109"));
        assert!(
            explain("BR-CO-16")
                .unwrap()
                .contains("BT-115) = Invoice total amount with VAT (BT-112)")
        );
        assert!(!explain("BR-CO-16").unwrap().contains("line net + tax"));
    }

    #[test]
    fn ibr_03_my_eval_fires_on_missing_buyer_brn() {
        let mut inv = crate::invoice::Invoice::blank(
            crate::profile::Profile::PintMy,
            "MY-1",
            "MYR",
            {
                let mut p = crate::invoice::Party::new("S", "MY");
                p.legal_registration = Some(crate::identifier::Identifier::new("2023010000001"));
                p.tax_registration = Some(crate::identifier::Identifier::new("C12345678901"));
                p
            },
            crate::invoice::Party::new("B", "MY"),
        );
        inv.issue_date = crate::date::Date::parse("2026-01-15").ok();
        inv.type_code = Some(Code::new("380"));
        let report = crate::validate::validate(&inv);
        assert!(
            report.findings.iter().any(|f| f.id == "IBR-03-MY"),
            "{report}"
        );
        let eval = catalogue()
            .iter()
            .find(|r| r.id == "IBR-03-MY")
            .unwrap()
            .eval;
        let mut from_eval = crate::report::Report {
            profile_slug: "pint-my",
            ..crate::report::Report::default()
        };
        eval(&inv, &mut from_eval);
        assert!(from_eval.findings.iter().any(|f| f.id == "IBR-03-MY"));
    }

    #[test]
    fn present_zero_allowance_total_without_bg20_is_not_br_co_11() {
        let mut inv = crate::invoice::Invoice::blank(
            crate::profile::Profile::En16931,
            "1",
            "EUR",
            {
                let mut p = crate::invoice::Party::new("S", "DE");
                p.vat_identifier = Some(crate::identifier::Identifier::new("DE1"));
                p
            },
            crate::invoice::Party::new("B", "FR"),
        );
        inv.issue_date = crate::date::Date::parse("2026-01-15").ok();
        inv.type_code = Some(Code::new("380"));
        inv.payment_terms = Some("Net 30".into());
        let mut line = crate::invoice::Line::new(
            "1",
            "A",
            crate::amount::InvoiceAmount::parse("100.00").unwrap(),
            crate::tax::TaxCategory::vat("S", rust_decimal::Decimal::from(19)),
        );
        line.quantity = Some(crate::numeric::Quantity::parse("1").unwrap());
        line.unit = Some(Code::new("C62"));
        line.price = Some(crate::invoice::Price {
            net: crate::amount::UnitPriceAmount::parse("100.00").unwrap(),
            discount: None,
            gross: None,
            base_qty: None,
            base_unit: None,
        });
        inv.lines = vec![line];
        crate::reconcile::reconcile(&mut inv).unwrap();
        let t = inv.totals.as_mut().unwrap();
        t.allowance_total = Some(crate::amount::InvoiceAmount::ZERO);
        let report = crate::validate::validate(&inv);
        assert!(
            report.findings.iter().all(|f| f.id != "BR-CO-11"),
            "{report}"
        );
    }

    #[test]
    fn catalogue_ids_are_tested_or_uncovered() {
        let uncovered = include_str!("../../../docs/UNCOVERED.md");
        let tests = [
            include_str!("rules.rs"),
            include_str!("peppol.rs"),
            include_str!("category.rs"),
            include_str!("codes.rs"),
            include_str!("xrechnung.rs"),
        ]
        .concat();
        for rule in catalogue() {
            let id = rule.id;
            let ok = tests.contains(id) || uncovered.contains(id) || id.starts_with("BR-DEC-");
            assert!(ok, "{id} is neither in tests nor UNCOVERED.md");
        }
    }

    #[test]
    fn br_23_fires_without_quantity() {
        let mut inv = crate::invoice::Invoice::blank(
            crate::profile::Profile::En16931,
            "1",
            "EUR",
            crate::invoice::Party::new("S", "DE"),
            crate::invoice::Party::new("B", "FR"),
        );
        inv.lines = vec![crate::invoice::Line::new(
            "1",
            "A",
            crate::amount::InvoiceAmount::parse("1.00").unwrap(),
            crate::tax::TaxCategory::vat("S", rust_decimal::Decimal::from(19)),
        )];
        let report = crate::validate::validate(&inv);
        assert!(report.findings.iter().any(|f| f.id == "BR-22"), "{report}");
        assert!(report.findings.iter().any(|f| f.id == "BR-23"), "{report}");
    }

    #[test]
    fn br_24_is_explainable() {
        assert!(crate::explain("BR-24").unwrap().contains("BT-131"));
    }

    #[test]
    fn ibr_cl_05_my_bt6_must_be_myr() {
        let mut inv = crate::invoice::Invoice::blank(
            crate::profile::Profile::PintMy,
            "MY-1",
            "MYR",
            {
                let mut p = crate::invoice::Party::new("S", "MY");
                p.legal_registration = Some(crate::identifier::Identifier::new("2023010000001"));
                p.tax_registration = Some(crate::identifier::Identifier::new("C12345678901"));
                p
            },
            {
                let mut b = crate::invoice::Party::new("B", "MY");
                b.legal_registration = Some(crate::identifier::Identifier::new("1999010000001"));
                b
            },
        );
        inv.issue_date = crate::date::Date::parse("2026-01-15").ok();
        inv.type_code = Some(Code::new("380"));
        inv.tax_currency = Some(Code::new("USD"));
        let report = crate::validate::validate(&inv);
        assert!(
            report.findings.iter().any(|f| f.id == "IBR-CL-05-MY"),
            "{report}"
        );
        inv.tax_currency = Some(Code::new("MYR"));
        assert!(
            crate::validate::validate(&inv)
                .findings
                .iter()
                .all(|f| f.id != "IBR-CL-05-MY")
        );
    }

    #[test]
    fn recargo_half_percent_does_not_take_zero_branch() {
        use crate::amount::InvoiceAmount;
        use crate::date::Date;
        use crate::invoice::{Invoice, Line, Party, TaxBreakdown};
        use crate::profile::Profile;
        use crate::tax::{TaxCategory, TaxSystem};
        use crate::validate;
        use rust_decimal::Decimal;
        use std::str::FromStr;

        let mut inv = Invoice::blank(
            Profile::En16931,
            "INV-R",
            "EUR",
            {
                let mut p = Party::new("S", "ES");
                p.vat_identifier = Some(crate::identifier::Identifier::new("ESA12345678"));
                p
            },
            Party::new("B", "ES"),
        );
        inv.issue_date = Date::parse("2026-01-15").ok();
        inv.type_code = Some(Code::new("380"));
        let rate = Percentage::new(Decimal::from_str("0.5").unwrap());
        inv.lines = vec![Line::new(
            "1",
            "Recargo",
            InvoiceAmount::parse("1000.00").unwrap(),
            TaxCategory::vat("S", rate),
        )];
        inv.tax_breakdown = vec![TaxBreakdown {
            system: TaxSystem::Vat,
            scheme: "VAT".into(),
            category: Code::new("S"),
            rate: Some(rate),
            taxable: InvoiceAmount::parse("1000.00").unwrap(),
            tax: InvoiceAmount::parse("5.00").unwrap(),
            exemption_reason: None,
            exemption_code: None,
        }];
        crate::reconcile::reconcile(&mut inv).unwrap();
        let report = validate(&inv);
        assert!(
            report.findings.iter().all(|f| f.id != "BR-CO-17"),
            "{report}"
        );
    }

    #[test]
    fn br_12_15_fire_when_totals_absent() {
        let inv = crate::invoice::Invoice::blank(
            crate::profile::Profile::En16931,
            "1",
            "EUR",
            {
                let mut p = crate::invoice::Party::new("S", "DE");
                p.vat_identifier = Some(crate::identifier::Identifier::new("DE123456789"));
                p
            },
            crate::invoice::Party::new("B", "FR"),
        );
        let report = crate::validate::validate(&inv);
        for id in ["BR-12", "BR-13", "BR-14", "BR-15"] {
            assert!(report.findings.iter().any(|f| f.id == id), "{id}: {report}");
            assert!(explain(id).is_some());
        }
    }

    #[test]
    fn br_19_tax_rep_needs_address() {
        let mut inv = crate::invoice::Invoice::blank(
            crate::profile::Profile::En16931,
            "1",
            "EUR",
            crate::invoice::Party::new("S", "DE"),
            crate::invoice::Party::new("B", "FR"),
        );
        inv.tax_representative = Some(crate::invoice::TaxRepresentative {
            name: "R".into(),
            vat_identifier: Some(crate::identifier::Identifier::new("DE1")),
            address: None,
        });
        let report = crate::validate::validate(&inv);
        assert!(report.findings.iter().any(|f| f.id == "BR-19"), "{report}");
    }

    #[test]
    fn br_32_33_on_document_allowance() {
        let mut inv = crate::invoice::Invoice::blank(
            crate::profile::Profile::En16931,
            "1",
            "EUR",
            crate::invoice::Party::new("S", "DE"),
            crate::invoice::Party::new("B", "FR"),
        );
        inv.document_allowances
            .push(crate::invoice::AllowanceCharge {
                amount: crate::amount::InvoiceAmount::parse("1.00").unwrap(),
                base: None,
                percent: None,
                reason: None,
                reason_code: None,
                tax: None,
            });
        let report = crate::validate::validate(&inv);
        assert!(report.findings.iter().any(|f| f.id == "BR-32"), "{report}");
        assert!(report.findings.iter().any(|f| f.id == "BR-33"), "{report}");
        assert!(explain("BR-31").unwrap().contains("BT-92"));
        assert!(explain("BR-36").unwrap().contains("BT-99"));
        assert!(explain("BR-41").unwrap().contains("BT-136"));
        assert!(explain("BR-43").unwrap().contains("BT-141"));
        assert!(explain("BR-45").unwrap().contains("BT-116"));
        assert!(explain("BR-46").unwrap().contains("BT-117"));
    }

    #[test]
    fn br_48_skips_o_and_ttx() {
        use crate::invoice::TaxBreakdown;
        let mut inv = crate::invoice::Invoice::blank(
            crate::profile::Profile::En16931,
            "1",
            "EUR",
            crate::invoice::Party::new("S", "DE"),
            crate::invoice::Party::new("B", "FR"),
        );
        inv.tax_breakdown.push(TaxBreakdown {
            system: crate::tax::TaxSystem::Vat,
            scheme: "VAT".into(),
            category: Code::new("S"),
            rate: None,
            taxable: crate::amount::InvoiceAmount::parse("1.00").unwrap(),
            tax: crate::amount::InvoiceAmount::ZERO,
            exemption_reason: None,
            exemption_code: None,
        });
        let report = crate::validate::validate(&inv);
        assert!(report.findings.iter().any(|f| f.id == "BR-48"), "{report}");
        inv.tax_breakdown[0].category = Code::new("O");
        assert!(
            crate::validate::validate(&inv)
                .findings
                .iter()
                .all(|f| f.id != "BR-48")
        );
        inv.tax_breakdown[0].category = Code::new("TTX");
        inv.tax_breakdown[0].scheme = "AAL".into();
        assert!(
            crate::validate::validate(&inv)
                .findings
                .iter()
                .all(|f| f.id != "BR-48")
        );
        inv.tax_breakdown[0].category = Code::new("");
        assert!(
            crate::validate::validate(&inv)
                .findings
                .iter()
                .any(|f| f.id == "BR-47")
        );
    }

    #[test]
    fn br_49_50_61_payment() {
        let mut inv = crate::invoice::Invoice::blank(
            crate::profile::Profile::En16931,
            "1",
            "EUR",
            crate::invoice::Party::new("S", "DE"),
            crate::invoice::Party::new("B", "FR"),
        );
        inv.payment = Some(crate::invoice::PaymentInstructions {
            means_code: None,
            means_text: None,
            remittance: None,
            means: None,
        });
        let report = crate::validate::validate(&inv);
        assert!(report.findings.iter().any(|f| f.id == "BR-49"), "{report}");
        inv.payment = Some(crate::invoice::PaymentInstructions {
            means_code: Some(Code::new("30")),
            means_text: None,
            remittance: None,
            means: Some(crate::payment::PaymentMeans::CreditTransfer(vec![
                crate::payment::CreditTransfer {
                    account_id: crate::identifier::Identifier::new(""),
                    account_name: None,
                    provider: None,
                },
            ])),
        });
        let report = crate::validate::validate(&inv);
        assert!(report.findings.iter().any(|f| f.id == "BR-50"), "{report}");
        assert!(report.findings.iter().any(|f| f.id == "BR-61"), "{report}");
        inv.payment = Some(crate::invoice::PaymentInstructions {
            means_code: Some(Code::new("48")),
            means_text: None,
            remittance: None,
            means: Some(crate::payment::PaymentMeans::Card(
                crate::payment::PaymentCard {
                    pan: "4111".into(),
                    holder: None,
                },
            )),
        });
        let report = crate::validate::validate(&inv);
        assert!(report.findings.iter().all(|f| f.id != "BR-50"), "{report}");
        assert!(report.findings.iter().all(|f| f.id != "BR-61"), "{report}");
    }

    #[test]
    fn br_co_26_seller_identifiable() {
        let mut inv = crate::invoice::Invoice::blank(
            crate::profile::Profile::En16931,
            "1",
            "EUR",
            crate::invoice::Party::new("S", "DE"),
            crate::invoice::Party::new("B", "FR"),
        );
        let report = crate::validate::validate(&inv);
        assert!(
            report.findings.iter().any(|f| f.id == "BR-CO-26"),
            "{report}"
        );
        inv.seller.vat_identifier = Some(crate::identifier::Identifier::new("DE123"));
        assert!(
            crate::validate::validate(&inv)
                .findings
                .iter()
                .all(|f| f.id != "BR-CO-26")
        );
        let my = crate::invoice::Invoice::blank(
            crate::profile::Profile::PintMy,
            "1",
            "MYR",
            crate::invoice::Party::new("S", "MY"),
            crate::invoice::Party::new("B", "MY"),
        );
        assert!(
            crate::validate::validate(&my)
                .findings
                .iter()
                .all(|f| f.id != "BR-CO-26")
        );
        assert!(explain("BR-CO-26").unwrap().contains("BT-29"));
        assert!(explain("BR-42").is_some());
        assert!(explain("BR-44").is_some());
        assert!(explain("BR-37").is_some());
        assert!(explain("BR-38").is_some());
    }

    #[test]
    fn br_42_44_line_ac_reason() {
        let mut inv = crate::invoice::Invoice::blank(
            crate::profile::Profile::En16931,
            "1",
            "EUR",
            crate::invoice::Party::new("S", "DE"),
            crate::invoice::Party::new("B", "FR"),
        );
        let mut line = crate::invoice::Line::new(
            "1",
            "A",
            crate::amount::InvoiceAmount::parse("1.00").unwrap(),
            crate::tax::TaxCategory::vat("S", rust_decimal::Decimal::from(19)),
        );
        line.allowances.push(crate::invoice::LineAllowanceCharge {
            amount: crate::amount::InvoiceAmount::parse("1.00").unwrap(),
            base: None,
            percent: None,
            reason: None,
            reason_code: None,
        });
        line.charges.push(crate::invoice::LineAllowanceCharge {
            amount: crate::amount::InvoiceAmount::parse("1.00").unwrap(),
            base: None,
            percent: None,
            reason: None,
            reason_code: None,
        });
        inv.lines = vec![line];
        let report = crate::validate::validate(&inv);
        assert!(report.findings.iter().any(|f| f.id == "BR-42"), "{report}");
        assert!(report.findings.iter().any(|f| f.id == "BR-44"), "{report}");
    }
}
