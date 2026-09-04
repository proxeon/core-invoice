# UNCOVERED

Every `catalogue()` id is either exercised by a `#[test]` or listed here with a reason.

## Oracle expected-unmatched

Ids the artefact may fire and we do not. `task svrl` loads **this fenced list only**. Prose elsewhere is not scanned.

```
BR-CO-25
```

`BR-CO-25` has no UBL assert in pin `validation-1.3.16`. UBL-CR/SR/DT (e.g. UBL-SR-43 on SST-as-EN) are ignored by the semantic compare, not listed here.

## Peppol BIS pin is Schematron-only

Peppol BIS **v3.0.20** in `refers/peppol-bis-invoice-3` is `.sch` (`rules/sch/PEPPOL-EN16931-UBL.sch`). This pin has **no compiled XSLT**. `task svrl` does **not** diff Peppol `@id`. SA-as-Peppol is **our** Fatal set only (must be non-empty). Compiling `.sch` ourselves is not OpenPEPPOL Valid. If a later pin ships XSLT, compare then.

## Code lists (generated; remaining holes)

- none of BR-CL-03 / BR-CL-08: BR-CL-03 is a formats wire walk (`@currencyID` ∈ ISO 4217). BR-CL-08 uses UNCL 4451 extracted from EN UBL preprocessed Schematron (no `.gc` in the PINT-MY zip).

## Peppol extra_rules not modelled (syntax-only or Option-at-most-one)

R006/R008/R043/R044/R053/R080/R100/CL007 are **registered constant-pass** so `explain` works; they do not walk XML. R051 is a formats wire walk (`@currencyID` = BT-5 except BT-111).

COMMON-R040–R050/R052/R053 ICD checksums evaluate (R048 is commented out in the artefact — not registered). R044–R047/R052/R053 are **warning**.

## VAT family rows still only for S-03/04/06/07

- Family `-03/-04/-06/-07` rows are registered (RateContext reuse). Remaining: none.

## Dummy / type-enforced (now explainable)

- `BR-24` constant-pass: `Line.net` is not `Option` (BT-131). `explain BR-24` works.
- `BR-DEC-*` constant-pass: `InvoiceAmount` refuses a third decimal. Holes 03,04,07,08,21,22,26,29,30 not invented (percent / unit price).
- `BR-CO-05`…`BR-CO-08` artefact `true()` (NLP). Owner: P11.05.
- `BR-CO-25` — UBL Schematron 1.3.16 has no BR-CO-25. EDIFACT wording is positive BT-115 → BT-9 or BT-20. Owner: P11.04.

## PINT-MY remaining from 1.3.0 zip

Registered+evaluated: `ibr-02-my`, `ibr-03-my`, `ibr-04-my`, `aligned-ibrp-cl-01-my`, SA/SE/HVG/LVG/E/TTX `-08/-09/-10`, `aligned-ibrp-o-11-my`, `aligned-ibrp-001-my` not separately registered (BT-24 lookup).
Registered remaining zip ids: `ALIGNED-IBRP-002`, `046/047/048`, `HVG-10`, `LVG-10`, `TTX-08`, `E-05/08`, `O-09`, `IBR-CL-05-MY`. `ALIGNED-IBRP-O-11-MY` is crate-only (not in zip).

CLASS: mapped as `Line.classifications` with `listID` (`CG` is CLASS in PINT-MY). `BR-CL-13` / UNCL 7143. Not an LHDN submit artefact.

## CEN semantic ids not in catalogue (pin validation-1.3.16)

Artefact union of `EN16931-model.sch` ∪ `EN16931-UBL-codes.sch`: **223** syntax-independent `BR-*`. Live `catalogue()` EN intersection: **223**. Listed holes: **0**. Do not invent `BR-CO-01/02/27`, `BR-CL-02/09/12` (`explain` stays None). `BR-CO-25` is not in the 223 (no UBL rule). UBL-CR/SR/DT (~756) are formats/unmapped, not CORE.

Family A/C rows and IC-11/12: in catalogue. Presence evals registered: BR-12…15, 17–20, 29–33, 36–38, 41–50, 52, 54–57, 61–65. BR-31/36/41/43/45/46 are type-retired (`explain` works). CL bound: BR-CL-03 (wire), BR-CL-08 (BT-21). CO registered: BR-CO-09/19–24/26 (CO-26 skipped on Pint/PintMy).

## Pint GST

- Pint international 1.1.2 zip has no official example XML (`refers/pint-billing-1.1.2/unpacked/trn-invoice/` is genericode + Schematron only). Authored `pint_gst_sr` is not an oracle.
- `pint_gst_category` helper exists; GST family table on `Profile::Pint` is not a full IBR set. Owner: P13.09.

## P14 / P18

- ConnectingEurope XSLT SVRL job: `task svrl` / `xtask/svrl_oracle.py`. Default EN corpus is TC434 examples 1–10 + credit note + guide 1–3 + `sample-discount-price` + BR-03 mutant. PINT-MY official Invoice/CreditNote samples SA/SE/HVG/LVG/TTX. BIS3/G2G/`issue116` and LHDN-shaped zip samples are named skips. v0.2.0 tag baseline (example1/5/discount+mutant+SA) was comparable. Peppol BIS pin is `.sch` only (see above).
- Mustang: only if `MUSTANG_JAR` is set; VAT CII; never default CI.
- nix flake: Later (P18.03 / P19).
- cargo-fuzz target `fuzz/fuzz_targets/formats_read.rs` is run locally (`cargo fuzz run formats_read`); not default CI. Unit smoke `random_bytes_do_not_panic` stays.
- `#![warn(missing_docs)]` on the model crate is deferred until a full rustdoc audit (CI `-D warnings` would fail closed). Public Table 2 fields cite BT numbers.
