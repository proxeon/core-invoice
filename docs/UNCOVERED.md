# UNCOVERED

Every `catalogue()` id is either exercised by a `#[test]` or listed here with a reason.

## Code lists (generated; remaining holes)

- BR-CL-07, BR-CL-08, BR-CL-10, BR-CL-11, BR-CL-21, BR-CL-26 — ICD/UNCL 1153 scheme slots on specific identifier elements; lists generated (`ICD`) but not yet bound per BT. Owner: P10.
- BR-CL-03 `@currencyID` on wire amounts — model amounts inherit `Invoice.currency`; P8 R051 same reason.

## Peppol extra_rules not modelled (syntax-only or Option-at-most-one)

From `refers/peppol-bis-invoice-3` v3.0.20:

- PEPPOL-EN16931-R006 — CII-only “one invoiced object”; UBL is `Invoice.invoiced_object: Option`.
- PEPPOL-EN16931-R008 — empty XML elements (syntax walk, not model).
- PEPPOL-EN16931-R043 — ChargeIndicator true/false; model uses two vecs, writer emits the boolean.
- PEPPOL-EN16931-R044 — price-level charge forbidden; `Price` has discount only.
- PEPPOL-EN16931-R051 — `@currencyID` on every amount vs BT-5 except BT-111; wire-only.
- PEPPOL-EN16931-R053 — one TaxTotal with subtotals; model has one `tax_breakdown` vec.
- PEPPOL-EN16931-R080 — one project reference; `Invoice.project` is `Option`.
- PEPPOL-EN16931-R100 — at most one line DocumentReference; `Line.invoiced_object` is `Option`.
- PEPPOL-EN16931-CL007 — `@currencyID` ISO 4217; CORE BR-CL-04 covers BT-5.
- PEPPOL-COMMON-R041–R053 — ICD checksums besides GLN (R040 shipped). Owner: P8/P10.

## VAT family rows still only for S-03/04/06/07

- BR-Z-03/04/06/07, BR-E-03/04/06/07, BR-AE-03/04, BR-IC-03/04, BR-G-03/04, BR-AF-03/04, BR-AG-03/04 — artefacts have allowance/charge identifier rows; only the S family is uncollapsed. Owner: P12.

## Dummy / type-enforced (now explainable)

- `BR-DEC-*` constant-pass: `InvoiceAmount` refuses a third decimal. Holes 03,04,07,08,21,22,26,29,30 not invented (percent / unit price).
- `BR-CO-05`…`BR-CO-08` artefact `true()` (NLP). Owner: P11.05.
- `BR-CO-25` — UBL Schematron 1.3.16 has no BR-CO-25. EDIFACT wording is positive BT-115 → BT-9 or BT-20. Owner: P11.04.

## PINT-MY remaining from 1.3.0 zip

Registered+evaluated: `ibr-02-my`, `ibr-03-my`, `ibr-04-my`, `aligned-ibrp-cl-01-my`, SA/SE/HVG/LVG/E/TTX `-08/-09/-10`, `aligned-ibrp-o-11-my`, `aligned-ibrp-001-my` not separately registered (BT-24 lookup).
UNCOVERED: `aligned-ibrp-002`, `aligned-ibrp-046/047/048`, `aligned-ibrp-hvg-10`, `aligned-ibrp-lvg-10`, `aligned-ibrp-ttx-08`, `aligned-ibrp-e-05/08`, `aligned-ibrp-o-09`, `ibr-cl-05-my`. Owner: P9.03.

CLASS: mapped as `Line.classifications` with `listID` (`CG` is CLASS in PINT-MY). `BR-CL-13` / UNCL 7143. Not an LHDN submit artefact.

## Pint GST

- Pint international 1.1.2 zip has no official example XML (`refers/pint-billing-1.1.2/unpacked/trn-invoice/` is genericode + Schematron only). Authored `pint_gst_sr` is not an oracle.
- `pint_gst_category` helper exists; GST family table on `Profile::Pint` is not a full IBR set. Owner: P13.09.

## P14 / P18

- ConnectingEurope XSLT SVRL job: `task svrl` / `xtask/svrl_oracle.py` (Saxon). Mapping table `docs/svrl-id-map.md`.
- Mustang: only if `MUSTANG_JAR` is set; VAT CII; never default CI.
- nix flake: Later (P18.03 / P19).
