# UNCOVERED

Every `catalogue()` id is either exercised by a `#[test]` or listed here with a reason.

## Code lists (hand subsets)

- ISO 4217 / ISO 3166 remaining codes — generated lists are P10.01–P10.02; current hand subsets live in `codes.rs`.
- UNCL 1001 vs artefact — BR-CL-01 is a split subset; remainder P10.04.
- UNCL 4461 remainder; Rec 20 units — P10.04.
- EAS vs full CEF list (9 codes shipped) — P10.03.
- VATEX vs full list — P8.05 implemented pairing P0104–P0111; membership list P10.03.

## CEN BR-CL not yet membership-complete

- BR-CL-06, 07, 08, 10, 11, 13, 15, 19, 20, 21, 26 — need generated lists (P10).

## VAT family rows still aliased

- BR-S-03/04/06/07 currently share identifier/rate helpers with BR-S-02 — uncollapse P12.02.

## Dummy / type-enforced

- `CORE-PROCESS-01` eval is empty; finding is emitted from `spec_lookup` — P12.03.
- `IBR-SR-63` eval empty; emitted from `spec_lookup` — P12.03.
- `BR-DEC-12` is a constant-pass: `InvoiceAmount` already refuses a third decimal.

## Peppol extra_rules not yet implemented

From `refers/peppol-bis-invoice-3` v3.0.20, not in `peppol::RULES`:

- PEPPOL-EN16931-R002, R041, R042, R043, R044, R051, R053, R054, R080, R100, R101, R110, R111, R130
- PEPPOL-EN16931-CL001, CL002, CL003, CL006, CL007, CL008, F001
- PEPPOL-EN16931-P0101, P0112
- PEPPOL-COMMON-* (EAS/ICD) — P10 lists

## PINT-MY

- Remaining IBR-* / ALIGNED-IBRP-* from the 1.3.0 zip not in `category.rs` — P9.03.
- CLASS on line (BG-32) mapping to official CLASS samples — P9.04.
- Official samples may still fire presence/list ids listed above until P10/P11 complete.

## `pint_gst_category`

- Helper exists; GST family table on `Profile::Pint` is not a full IBR set — P12/P9.
