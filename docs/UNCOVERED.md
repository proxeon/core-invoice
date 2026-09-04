# UNCOVERED

Every `catalogue()` id is either exercised by a `#[test]` or listed here with a reason.

## Oracle expected-unmatched

Ids the artefact may fire and we do not. `task svrl` loads **this fenced list only**. Prose elsewhere is not scanned.

```
BR-CO-25
```

`BR-CO-25` has no UBL assert in pin `validation-1.3.16`. UBL-CR/SR/DT (e.g. UBL-SR-43 on SST-as-EN) are ignored by the semantic compare, not listed here.

## Peppol BIS pin is Schematron-only

Peppol BIS **v3.0.20** in `refers/peppol-bis-invoice-3` is `.sch` (`rules/sch/PEPPOL-EN16931-UBL.sch`). This pin has **no compiled Schematron XSLT** under `rules/sch/` (`stylesheet/stylesheet-ubl.xslt` is a presentation stylesheet). `task svrl` does **not** diff Peppol `@id`. SA-as-Peppol is **our** Fatal set only (must be non-empty). Compiling `.sch` ourselves is not OpenPEPPOL Valid. A formats test fails if a later pin drops compiled XSLT into `rules/sch/` so we notice.

## Code lists (generated; remaining holes)

- none of BR-CL-03 / BR-CL-08: BR-CL-03 is a formats wire walk (`@currencyID` ∈ ISO 4217). BR-CL-08 uses UNCL 4451 extracted from EN UBL preprocessed Schematron (no `.gc` in the PINT-MY zip).

## Peppol extra_rules (syntax walks vs Option-at-most-one)

Model evals stay `syntax_or_option_pass` (`explain` works; `validate(&Invoice)` has no XML). Formats `validate_xml` walks Peppol BIS only:

- **Walked:** R008 (UBL empty elements), R043 (`ChargeIndicator`/`Indicator` must be `true`/`false`), R044 (price-level charge), R051 (`@currencyID` = BT-5 except BT-111), R053 (exactly one TaxTotal with subtotals / CII document-currency `TaxTotalAmount`), R006 (CII TypeCode 130 cardinality), R080 (UBL CreditNote TypeCode 50), R100 (per-line DocumentReference / CII TypeCode 130), CL007 (Peppol `@currencyID` ∈ ISO 4217, dual with CORE BR-CL-03).
- **Still constant-pass / named:** CII R080 (artefact XPath counts TypeCode 50; we map 50 as tender BT-17, project as `SpecifiedProcuringProject`). R006 is not asserted on UBL (removed in BIS v3.0.16). ISO 4217 `STD` vs `STN` list-pin mismatch. Compiling `.sch` is not OpenPEPPOL Valid.

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

- Pint international **1.1.2** zip has **no official invoice XML** (genericode + Schematron + preprocessed XSLT; **no** `example/` instance XML). Authored MIT fixture `pint_gst_sr` is **not an oracle** and must not be billed as one.
- `pint_gst_category` helper exists; GST family table on `Profile::Pint` is not a full IBR set. Official instance XML is artefact-blocked.

## EN 16931-1:2026

Classified `Edition::En2026`. `is_implemented()` is **false**. `Profile::edition()` is always `En2017A1`. Pin `validation-1.3.16` is 2017+A1. Do not invent rules from drafts or news. Not an oracle unmatched id — do not put `En2026` in the fenced list.

## CEN UBL unit tests (pin validation-1.3.16)

`cargo test -p core-invoice-formats --test cen_conformance` uses git-tracked `testdata/en16931/test` (falls back to `refers/` after `task spec`). Floor **1055** agreed / **1000** run; ceiling **0** unexplained disagreements. Named divergences (6): empty `PostalAddress`, empty `BillingReference`. Raise the floor when a skipped unevaluated rule starts agreeing.

## Syntax prohibition tables

Generated from preprocessed ConnectingEurope Schematron (`task prohibitions`). UBL: 693/696 `not(…)` assertions (3 UNEXTRACTED: `UBL-CR-665`/`666`/`673` predicates on invoiced-object vs supporting-document). CII: 466/511 (45 UNEXTRACTED: `ends-with`/`ram:*` contexts, `CII-DT-031` `@currencyID` on non-`TaxTotalAmount`, mutual exclusions `CII-SR-465`/`466`). `not(x) or y` is not a `not(…)` assertion. `not(elem/@attr)` and `not(//elem)` are extracted with context — not as `//@schemeID`. `write_drops` / `scan_written` report hits on written XML; they do not rewrite the tree. Empty hits are not CEN Valid, not KoSIT Valid, not OpenPEPPOL Valid.

## XRechnung (optional feature)

`core-invoice` feature `xrechnung` is **off** by default and is **not CORE**. Evaluated when BT-24 claims KoSIT/xeinkauf: `BR-DE-1` (BG-16), `BR-DE-2` (BG-6), `BR-DE-3`/`4` seller city/postcode, `BR-DE-5`/`6`/`7` contact children, `BR-DE-8`/`9` buyer city/postcode, `BR-DE-10`/`11` deliver-to when BG-15 present, `BR-DE-14` (BT-119 on every BG-23 row including `O`), `BR-DE-15`, `BR-DE-16` (listed VAT categories), `BR-DE-17` (warning), Skonto `BR-DE-18`, IBAN warnings `BR-DE-19`/`20`, specification-id warning `BR-DE-21` (exact 3.0 CIUS / Extension / CVD URNs), `BR-DE-22` unique filenames, `BR-DE-23-a`/`24-a`/`25-a`, type-retired `-b`, `BR-DE-26` (warning, type 384 cites BG-3), `BR-DE-27`/`28` (phone digits / email shape), `BR-DE-30`/`31`, `BR-DE-TMP-32` (info), `BR-TMP-2` (absolute URL). Remaining KoSIT: withdrawn `BR-DE-12`/`13`/`29`; UBL lexical `BR-TMP-6`; Extension `BR-DEX-01`…`14` (sub-lines, third-party payments, DiGA); CVD `BR-DE-CVD-*` / `BR-TMP-CVD-01` (second CIUS, not hung on `claimed()`); Peppol-into-XR merge including HUF 0.5; KoSIT `customLevel` on CEN `BR-CL-21`/`23`; UBL BT-90 `schemeID='SEPA'` hop (formats). Do not treat `validate().ok()` on an XRechnung claim as KoSIT Valid. No KoSIT pin in `refers/`. No `Profile::XRechnung`.

## P14 / P18

- ConnectingEurope XSLT SVRL job: `task svrl` / `xtask/svrl_oracle.py`. Default EN corpus is TC434 examples 1–10 + credit note + guide 1–3 + `sample-discount-price` + BR-03 mutant. PINT-MY official Invoice/CreditNote samples SA/SE/HVG/LVG/TTX. BIS3/G2G/`issue116` and LHDN-shaped zip samples are named skips. v0.2.0 tag baseline (example1/5/discount+mutant+SA) was comparable. Peppol BIS pin is `.sch` only (see above).
- Mustang: only if `MUSTANG_JAR` is set; VAT CII; never default CI.
- nix flake: Later (P18.03 / P19).
- cargo-fuzz target `fuzz/fuzz_targets/formats_read.rs` is run locally (`cargo fuzz run formats_read`); not default CI. Unit smoke `random_bytes_do_not_panic` stays.
- `#![warn(missing_docs)]` is on the model crate. Public Table 2 fields cite BT numbers.
