# Changelog

All crates in this workspace — [`core-invoice`], [`core-invoice-formats`],
[`core-invoice-cli`], [`core-invoice-fixtures`], [`core-invoice-sys`] — share
one version and one entry per release.

The format is [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this
project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).
2.x may add APIs; breaking changes are 3.0. The C ABI is the 0/1/2 verbs, not
`Invoice` layout.

[`core-invoice`]: https://crates.io/crates/core-invoice
[`core-invoice-formats`]: https://crates.io/crates/core-invoice-formats
[`core-invoice-cli`]: https://crates.io/crates/core-invoice-cli
[`core-invoice-fixtures`]: https://crates.io/crates/core-invoice-fixtures
[`core-invoice-sys`]: https://crates.io/crates/core-invoice-sys

## [Unreleased]

## [2.0.3] — 2026-09-04

Additive. No Rust API break.

### Added

- `Finding::info` for overlay info findings (`BR-DE-TMP-32`).
- Optional `xrechnung`: Skonto `BR-DE-18`, IBAN warnings `BR-DE-19`/`20`, specification-id warning `BR-DE-21`, unique filenames `BR-DE-22`, `BR-DE-26`/`27`/`28` (warnings), `BR-DE-31`, `BR-DE-TMP-32` (info), `BR-TMP-2`. Still never default, never CORE, not KoSIT Valid. Extension / CVD stay unregistered.

### Changed

- Prohibition tables extract `not(elem/@attr)` and `not(//elem)` with context (UBL 693/696, CII 466/511). Remaining UNEXTRACTED are predicates, `ends-with`/`*` wildcards, or mutual exclusions. `scan_written` matches contextual attributes on both syntaxes and treats unparseable written XML as a hit. Still does not rewrite. Empty hits are not CEN Valid. PINT-MY TIN `schemeID="GST"` is a named `UBL-CR-652` hit.
- CII maps BT-15/16 despatch and receiving advice, BT-19 buyer accounting, BT-7/8 tax point on each header `ApplicableTradeTax`, line BG-26/27/28, BT-155…159, BT-132/133/128, and BT-147 inside gross price. `CII_DROPPED` shrinks to empty `PostalAddress` vs absent plus PINT-MY line extras (`lines.extra_tax`, `lines.tax_total`). Discount without gross is not written (no D16B home). PINT-MY stays UBL-only.
- Peppol syntax extras: formats `validate_xml` walks R006 (CII), R008 (UBL empty elements), R043, R044, R053, R080 (UBL CreditNote), R100, and CL007. Model evals stay constant-pass. Not OpenPEPPOL Valid.
- Optional `xrechnung`: `BR-DE-1` is BG-16 (was mislabelled seller contact). Seller contact is `BR-DE-2`. Added `BR-DE-3`–`11`, `BR-DE-14`, `BR-DE-17` (warning). `BR-DE-16` uses the listed VAT categories.
- crates.io README is the workspace page: members set `readme.workspace = true`. Crate-level `crates/*/README.md` removed so `cargo package` does not ship the short local copy. Honesty fences unchanged (not OpenPEPPOL Valid, not IRBM Valid, XRechnung not CORE). `testdata/` still not in the crate tarball.
- PINT GST honesty: `pint_gst_sr` stays authored MIT, not a 1.1.2 oracle. Helper rustdoc no longer claims the zip lists `SR`/`ZR`. UNCOVERED names the missing tax-category CL and removed `ibr-cl-27`. No official PINT international XML.
- `core-invoice` enables `missing_docs` (rustdoc on public items).
- Artefact tripwires: Peppol `rules/sch` panics under `CORE_INVOICE_REQUIRE_SPEC` if missing; PINT 1.1.2 pin must not grow official instance XML unnoticed; pin consts include PINT 1.1.2; Python `validate_xml` ctypes 0/1/2 in CI. convert/diff stay C/CLI. Fuzz and nix stay parked.

## [2.0.2] — 2026-09-04

Additive. No Rust API break.

### Added

- Git-tracked `testdata/` (~2 MB): CEN UBL unit-test XML and official EN/Peppol/PINT-MY samples so `cargo test` on a fresh clone does not skip. EUPL/Peppol terms in `testdata/NOTICE`. Not copied into `crates/`. Full Schematron/XSLT still `task spec`.

## [2.0.1] — 2026-09-04

Additive. No Rust API break.

### Added

- `write_drops` also lists CEN prohibition hits on the written bytes (does not strip). A model-built invoice should produce none.
- Optional `xrechnung`: `BR-DE-1`, `BR-DE-23-a`/`24-a`/`25-a`, type-retired `-b` (PaymentMeans enum), `BR-DE-30`.

### Changed

- CII maps buyer reference, endpoints, party ids, full postal (line2/subdivision), payee, tax representative, order/contract/project/tender/object refs, supporting docs, remittance, due date, payment terms, preceding invoices, allowance reasons, line note/description, gross price. `CII_DROPPED` shrinks to the remaining unmapped tail. Empty `PostalAddress` vs absent stays named.
- `Invoice` rustdoc: fields stay `pub` on 2.x; `#[non_exhaustive]` would be 3.0.

## [2.0.0] — 2026-09-04

**Breaking (Rust API only).** `DocumentTotals.payable` is `Option<InvoiceAmount>`. A missing PayableAmount is not `0.00`, so BR-15 can fire. `Invoice::payable()` was already `Option` (absent BG-22). Callers that wrote `totals.payable` as a required amount, or `DocumentTotals { payable: amt, .. }`, need `Some(amt)`. The C ABI is unchanged.

This is not a new product. crates.io 1.0.0 stays; 2.0 is the honest BT-115 type after 1.0 was already published.

### Added

- CEN UBL unit-test runner (`refers/en16931/test/{Invoice,CreditNote}-unit-UBL`). Ratchet: ≥1000 run, ≥1055 agreed, **0** unexplained disagreements. Six named divergences remain (empty `PostalAddress` / empty `BillingReference`). Skip unless `refers/` is present.
- Generated UBL/CII prohibition tables from preprocessed Schematron (`task prohibitions`). Context is kept; 32 UBL / 75 CII `not(…)` assertions stay `UNEXTRACTED`.
- Optional `xrechnung` Cargo feature (never default, never CORE): `BR-DE-15` / `BR-DE-16` overlay when BT-24 claims KoSIT/xeinkauf. Remaining KoSIT rules stay in UNCOVERED.

### Changed

- UBL `AllowanceCharge` without `Amount` is kept (amount 0) so BT-95/102 on CEN fragments still evaluate. Line A/C, empty PayeeFinancialAccount, empty item attributes, Price without `PriceAmount`, TaxSubtotal without category, and supporting documents without `ID` are kept so presence rules can fire.
- VAT family rows apply as soon as the category appears (not only once BG-22/BG-23 exist). Dual `TaxTotal` subtotals are concatenated. PartyTaxScheme `TAX`/`GST`/`AAL` vs `VAT` is distinguished.
- CII maps party street/city/postcode and contact (phone/email/name). `seller.contact` / `buyer.contact` leave `CII_DROPPED`.
- Peppol pin test: `rules/sch/` must not contain compiled Schematron XSLT. `stylesheet/stylesheet-ubl.xslt` is presentation, not OpenPEPPOL Valid.

## [1.0.0] — 2026-09-04

Fatal ids comparable to pinned ConnectingEurope EN 16931 `validation-1.3.16` and PINT-MY 1.3.0 as evidenced by `task svrl`. Peppol BIS v3.0.20 is `.sch` only — not OpenPEPPOL Valid. Still **not** IRBM Valid / MyInvois submit.

### Added

- SVRL default corpus: EN TC434 examples 1–10, credit note, guide 1–3, discount sample; PINT-MY official SA/SE/HVG/LVG/TTX invoices and credit notes. Named skips: BIS3/G2G/`issue116`, LHDN-shaped zip samples (not IRBM Valid).
- `docs/UNCOVERED.md` fenced **Oracle expected-unmatched** list (prose is not scanned). CEN artefact `BR-*` vs catalogue: 223/223, listed holes 0.

### Changed

- Peppol BIS v3.0.20 documented as `.sch` only: no OpenPEPPOL `@id` compare in this pin.
- `CII_DROPPED` no longer names mapped delivery/payment/notes/document A/C amounts; sub-paths remain where the subset still drops.
- CI installs cbindgen and diffs `core_invoice.h` (no skip). Python 1.0 surface is `validate_xml` only.
- `refers/fetch.sh` unzips PINT / PINT-MY `resources.zip` into `unpacked/` so tag artefacts jobs see official samples.

## [0.2.0] — 2026-09-03

Honest meaning engine for embedders to **build against**. Still **not** a legal validator: `validate().ok()` is not ConnectingEurope / OpenPEPPOL / IRBM Valid. Do not publish this tree as 0.1.1.

This tag folds work that sat under Unreleased while the workspace version was already 0.2.0. crates.io **0.1.0** remains the published skeleton; this tree is not 0.1.1 and is not 1.0.0.

### Changed

- **Honesty (0.2.x development engine).** Do not treat `validate().ok()` as EN 16931 / Peppol / PINT compliance.
- **Parse.** Dispatch is the document element (not `contains("CrossIndustryInvoice")`). Unknown BT-24 is `Profile::Unknown` (`CORE-SPEC-01`), not En16931. Missing line tax is empty + `BR-CO-04`, not category `S`. A third money decimal is a parse error (CLI exit 2), not `0.00`. `Read { unmapped, malformed }` exists. `diff` walks the model (dates, parties, lines qty/price, totals). One DTD refuse; CII depth 64; 10 MiB input cap.
- **Breaking (0.1.x).** `Party.tax_id` / `id_scheme` removed. Country (BT-40 / BT-55) is `PostalAddress.country`; `Party::country()` reads it.
- **Tax.** PINT-MY `SE` is rated service tax, not a zero-tax family. `TaxCategory.percent` is `Option` (TTX and EN `O` are absent, not 0 %). TTX omits `cbc:Percent` and uses TaxScheme `AAL`. `VAT/CGST` is not VAT. PintMy tax systems are SST-only in memory. MY `-08` uses the same content as reconcile (lines + charges − allowances, exact).
- **Breaking (0.1.x).** `Invoice.payable` / `tax_total` fields removed; they are views of `DocumentTotals`. BR-CO-18 fires without reconcile. BR-53: BT-6 ⇔ BT-111, never derived.
- **Table 2.** BT-7/8 tax point, BT-11…19 document refs (BT-13 is purchase order, not BG-3), line period BG-26, line A/C BG-27/28 (already in BT-131), item ids BT-155…159. Peppol R003 is BT-10 or BT-13. R120 includes line charges − allowances.
- **UBL.** Invoice/CreditNote child order (DueDate before type on Invoice; none on CreditNote and reported). Notes `#CODE#` round-trip. PaymentMeans `@name` is BT-82. Credit transfer IBAN/BIC, card, mandate. Dual TaxTotal BT-110/BT-111. Attachments. PINT-MY TIN scheme GST.
- **Peppol extras** run only via `Profile::extra_rules` (not CORE): R001/R003/R004/R007, R010 buyer / R020 seller EndpointID (artefact ids), R002, R005/R055/R061, R041/R042, R046 exact, R054, R101, R110/R111, R120 ±0.02 inclusive, R130, P0100/P0101/P0112, VATEX P0104–P0111, CL001–CL003/CL006/CL008, F001, COMMON-R040 (GLN). Official `base-example.xml` is loaded from `refers/` when present. They do not run on EN core or PINT-MY.
- **Presence.** BR-08/10 address group; BR-22 quantity (BT-129); BR-23 unit (BT-130); BR-26 net price present (BT-146); BR-27 net price not negative; BR-28 gross not negative (BT-148). PINT-MY Z01/Z03–Z08 overlay BR-CL-16. `validate --format json`. convert `-o`. CII writes qty/price. `docs/UNCOVERED.md` lists remaining catalogue ids.
- **Convert.** CLI `convert` proves, then `write_validated`. Fatal → exit 1, findings on stdout, empty XML. `write` is renamed `write_unchecked` (tests only). `write_validated` overwrites BT-24 / BT-23 from the proved profile. Self-billing BT-24 cannot be re-stamped as billing (`CORE-PROCESS-01`).
- **CII (historical, crates.io 0.1.0 / 004 P0).** `convert --to cii` used to wrap UBL in `CrossIndustryInvoice` and was then refused as `CiiNotImplemented`. That wrapper is gone.
- **CII (current).** Three-part D16B **subset** for EN 16931 and Peppol BIS (lines before header, format 102). Qty, price, payment means, document A/C, and delivery date/address are mapped. Remaining CII drops are named in the cross-syntax test. **PINT-MY is UBL-only:** `FormatError::CiiNotForProfile`, CLI exit 2.
- **Lists.** ISO 4217 / 3166, EAS, UNCL 1001/4461/5189/7161/2005/7143, Rec 20, MIME generated from `refers/` genericode (`task lists`). `XXX` stays allowed.
- **Docs.** `Line.item_id` is BT-155 (`SellersItemIdentification`); `Line.standard_id` is BT-157 (`StandardItemIdentification`). PINT-TAX text: PintMy is SST only. Fixtures corpus path is `refers/` (not `/spec/`).
- **Families.** BR-S-03/04/06/07 are RateContext allowance/charge, not aliases of BR-S-02/05. BR-O-11 groups vs BR-O-12 lines. BR-DEC-* explainable constant-pass.
- **CLI.** `validate` batch paths, stdin `-`, `--quiet`. `rules --profile peppol`. `inspect` prints unmapped. `profiles` prints artefact pins.
- **C ABI.** `core_invoice_validate`, convert, diff, version. Python ctypes wrapper.
- **Official samples.** BR-CO-11/12 accept present `0` when there is no BG-20/21 (Schematron empty sum). EAS includes `EM` (EN BR-CL-25). PINT-MY BT-110 sums every IBG-23 row including TTX. TTX-09 uses line `TaxTotal`, not taxable=tax. Official SA/SE/HVG/LVG/TTX 1.3.0 samples `validate().ok()`.
- **BR-53** matches CEN: BT-6 requires a TaxAmount in that currency; when BT-6 equals BT-5 the document TaxTotal (BT-110) counts. CreditNote `DueDate` is stored as BT-9 and is not a parse error (UBL CN writer still omits it).
- **Breaking (0.1.x).** `FormatError::CiiNotImplemented` is renamed to `CiiNotForProfile` and is returned when writing CII for PINT-MY.
- **CLI.** `--profile` default is `auto` (BT-24). Invalid findings print on **stdout**. `explain` unknown ids exit 2. `explain` covers BR-06 and BR-07.
- **BR-05** is currency **presence**, not a 3-letter length check.
- **BR-CO-16** is no longer emitted for `payable = line net + tax total`. The CEN formula is documented by `explain` as not evaluated until document totals exist.
- **PINT-MY fixture** category is `SA` (not Singapore `SR`).
- **C ABI.** Unknown profile strings return 2 (not silent PINT). NULL profile is auto from BT-24. Error buffer truncation is byte-safe.
- **`/spec/`** is gitignored (CEN artefacts are EUPL-1.2). See `docs/spec.md`.
- `Invoice::payable()` / `tax_total()` return `Option` — absent BG-22 is not 0.00.
- UBL `LegalMonetaryTotal` child order matches XSD `MonetaryTotalType`. Price BT-147/148 round-trip as unit amounts. Unparseable dates and duplicate singletons are reported. Nested Item/Party unknowns appear in `unmapped`. CreditNote DueDate omit is named (`write_drops`).
- CII: TypeCode credit-note set (not 381-only); `@format` not 102 is malformed; one PaymentMeans per IBAN; missing line tax is not SST; notes Content round-trips; delivery ShipTo before ActualDelivery; settlement tax before allowance/charge.
- BR-23 fires when unit is absent even if quantity is absent (artefact `@unitCode` independent of qty).
- Peppol R061 also applies to means code 59, not only 49 / DirectDebit.
- CLI `inspect` uses the document element, not a `CrossIndustryInvoice` substring.
- CLI `rules --profile en16931` lists CORE only (not Peppol extras).
- UBL reads Payee / TaxRepresentative / Delivery; TaxRepresentativeParty has no extra `cac:Party`.

### Added

- SVRL oracle: official EN examples, BR-03 mutant, PINT-MY SA, SST-as-EN three-way. Docker Compose Java fallback.
- BR-CL-03 (`@currencyID` ISO 4217 wire walk), BR-CL-08 (UNCL 4451 from EN Schematron). Peppol COMMON-R041–R047/R049/R050/R052/R053 ICD checksums; R051 `@currencyID` = BT-5 except BT-111.
- Remaining CEN presence/CO: BR-12…15, BR-19, BR-31–33, BR-36–38, BR-41–50, BR-61, BR-CO-26 (skipped on Pint/PintMy).
- Named CII drop list `CII_DROPPED`; UBL↔CII model diff must stay inside it.
- `docs/matrix.md` generated from `catalogue()` × profile.
- cargo-fuzz target on `formats::read` (local). `task tracked` greps `crates/` for TODO/FIXME.
- Line BT-132 (`order_line`), BT-133 (`accounting_reference`), BT-156 (`buyer_id`), BG-32 (`attributes` BT-160/161). UBL read/write. CII still named-dropped.
- `IBR-CL-05-MY` (BT-6 ⇒ MYR). `explain BR-24` (type-retired BT-131).
- BR-CL-07/10/11/21/26 bound to UNTDID 1153 / ICD. Generated `UNCL_1153`. EN type list includes 326/384/389 (Peppol P0100 still forbids 389).
- VAT family `-03/-04/-06/-07` rows beyond S; BR-IC-11/12; BR-B-01 is Italian-domestic.
- PINT-MY zip leftovers: ALIGNED-IBRP-002, 046–048, HVG/LVG-10, TTX-08, E-05/08, O-09.
- Presence/co-occurrence: BR-17/18/20/29/30/52/54/55/56/57/62–65, BR-CO-09/19–24.
- CII `Read.unmapped` lists unmapped `SupplyChainTradeTransaction` children. Official CEN CII example test when `refers/` present.

- Semantic types: `InvoiceAmount` (refuses a third decimal), `UnitPriceAmount`,
  `Quantity`, `Percentage` (per cent, not fraction), `Date`, `Code`,
  `Identifier`, `DocumentReference`, `Attachment`, `Path`/`BtId`/`Group`,
  `DocumentKind`, exclusive `PaymentMeans`. `Amount` is an alias of
  `InvoiceAmount`.
- Rule registry: `validate` and `explain` share one table. Findings include a
  BT path (`BG-25[0]/BT-151`). `catalogue()` lists shipped ids.
- BT-24 is stored on the invoice. Profile lookup is prefix-based (`starts-with`),
  not `contains("pint")`. Self-billing URNs are a wrong process. Edition 2017+A1
  vs classified 2026. Peppol BIS is the only conformant CIUS of EN 16931.
- Table 2 field set on `Invoice`: dates, type code, notes, preceding invoices,
  split party identifiers, payee/tax representative/delivery, payment,
  allowances/charges, tax breakdown, document totals, line quantity/price.
  `to_credit_note()` copies amounts without negation.
- PINT-MY: `IBR-02-MY` / `IBR-03-MY` / `IBR-04-MY` on BRN and TIN fields;
  categories restricted to SA/SE/HVG/LVG/TTX/E/O. `wire_scheme()` maps SST to
  TaxScheme `VAT` (never `SST` on the wire).
- Presence: BR-01, BR-03, BR-04, BR-09, BR-11, BR-21, BR-25.
- CII D16B three-part **subset** (lines before header, format 102). Not a UBL
  wrapper. `--to cii` is enabled for EN/Peppol only; PINT-MY is refused.
- Peppol extra_rules slot (historical Added): R001/R003/R004/R007, R120, R046.
  The full extra_rules set is under Changed.
- CLI: `rules --format json`, `inspect` (no verdict), `profiles`. `diff`
  exits 1 when documents differ. Process tests cover 0/1/2.
- C ABI snippet in `core-invoice-sys`. CI: fmt, clippy `-D warnings`, tests,
  wasm32 model build. `publish.sh` no longer `--allow-dirty` on a real publish.
- UBL 2.1 codec is a tree walk (`roxmltree`): Invoice and CreditNote roots,
  no first-tag-wins, no invented EUR/XX/S. DTD refused. Writer emits IssueDate,
  type code, EndpointID, TaxSubtotal, LegalMonetaryTotal children, quantity/price.
  PINT-MY TaxScheme is `VAT` (never `SST`). `write_validated` stamps from a proof.
- Code lists were hand-curated subsets at 0.1.0 / meaning-engine (historical).
  Current lists are generated from `refers/` genericode (`task lists`; Rec 20, not Rec 21).
  Artefact pins `validation-1.3.16` / Peppol `v3.0.20` / PINT-MY `1.3.0`.
  `task spec` fetches into gitignored `refers/`.
- `Validated<P>` proof type. `Check::without` cannot `prove`. Peppol proof may
  widen to EN 16931 (CIUS); Pint/PintMy do not.
- Category families: real `BR-S-*` / `BR-Z-*` / `BR-E-*` / `BR-AE-*` /
  `BR-IC-*` / `BR-G-*` / `BR-O-*` / `BR-AF-*` / `BR-AG-*` / `BR-B-*` ids on VAT
  profiles; PINT-MY uses `ALIGNED-IBRP-*-MY` (never `BR-S-08` for SST). GST `SR`
  fixture on `Pint` (fails PintMy CL). `BR-CO-18` when totals exist.
- Totals: `xpath_round` (`floor(x+0.5)`), real `BR-CO-10`…`BR-CO-16` on
  `DocumentTotals` (absent ≠ 0, four presence branches, overflow is a finding),
  `BR-CO-17` with artefact ±1.00 exclusive on abs. `reconcile()` fills BG-22/23;
  empty document A/C leave BT-107/108 absent. Prepaid 250 on 137.50 gross yields
  negative BT-115. Slack is per-rule: no HUF branch, no 5-sen slack, R046 exact
  called out as a trap.

## [0.1.0] — 2026-08-30

First crates.io release. Semantic model treats tax as VAT, GST, SST, or
consumption — not VAT only — and names EN 16931, Peppol BIS 3.0, PINT, and
PINT-MY as profiles.

### Added

- **`core-invoice`** — invoice, lines, parties, `Amount` (`rust_decimal`, two
  decimals). Profiles: `en16931`, `peppol`, `pint`, `pint-my`. Tax systems:
  VAT, GST, SST, consumption. `validate()` covers BR-02, BR-05, BR-06, BR-07,
  BR-16, BR-CO-16, `PINT-TAX`, `PINT-MY-ID`. Peppol BIS rejects SST; PINT-MY
  accepts it.
- **`core-invoice-formats`** — UBL 2.1 write/read; CII write wraps the UBL
  payload; `convert`, `diff`, `validate_xml`.
- **`core-invoice-cli`** — `core-invoice validate|convert|diff|explain`.
- **`core-invoice-fixtures`** — in-crate PINT-MY SST and Peppol VAT samples;
  UBL round-trip test.
- **`core-invoice-sys`** — C ABI `core_invoice_validate_ubl` and
  `include/core_invoice.h`.

[Unreleased]: https://github.com/proxeon/core-invoice/compare/v2.0.3...HEAD
[2.0.3]: https://github.com/proxeon/core-invoice/compare/v2.0.2...v2.0.3
[2.0.2]: https://github.com/proxeon/core-invoice/compare/v2.0.1...v2.0.2
[2.0.1]: https://github.com/proxeon/core-invoice/compare/v2.0.0...v2.0.1
[2.0.0]: https://github.com/proxeon/core-invoice/compare/v1.0.0...v2.0.0
[1.0.0]: https://github.com/proxeon/core-invoice/compare/v0.2.0...v1.0.0
[0.2.0]: https://github.com/proxeon/core-invoice/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/proxeon/core-invoice/releases/tag/v0.1.0
