# Changelog

All crates in this workspace — [`core-invoice`], [`core-invoice-formats`],
[`core-invoice-cli`], [`core-invoice-fixtures`], [`core-invoice-sys`] — share
one version and one entry per release.

The format is [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this
project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).
Before 1.0, a minor bump may break: the promise is that a break is **written
down here**, not that there are none.

[`core-invoice`]: https://crates.io/crates/core-invoice
[`core-invoice-formats`]: https://crates.io/crates/core-invoice-formats
[`core-invoice-cli`]: https://crates.io/crates/core-invoice-cli
[`core-invoice-fixtures`]: https://crates.io/crates/core-invoice-fixtures
[`core-invoice-sys`]: https://crates.io/crates/core-invoice-sys

## [Unreleased]

### Changed

- **Honesty (0.1.x skeleton).** Do not treat `validate().ok()` as EN 16931 / Peppol / PINT compliance.
- **Parse.** Dispatch is the document element (not `contains("CrossIndustryInvoice")`). Unknown BT-24 is `Profile::Unknown` (`CORE-SPEC-01`), not En16931. Missing line tax is empty + `BR-CO-04`, not category `S`. A third money decimal is a parse error (CLI exit 2), not `0.00`. `Read { unmapped, malformed }` exists. `diff` walks the model (dates, parties, lines qty/price, totals). One DTD refuse; CII depth 64; 10 MiB input cap.
- **Breaking (0.1.x).** `Party.tax_id` / `id_scheme` removed. Country (BT-40 / BT-55) is `PostalAddress.country`; `Party::country()` reads it.
- **Tax.** PINT-MY `SE` is rated service tax, not a zero-tax family. `TaxCategory.percent` is `Option` (TTX and EN `O` are absent, not 0 %). TTX omits `cbc:Percent` and uses TaxScheme `AAL`. `VAT/CGST` is not VAT. PintMy tax systems are SST-only in memory. MY `-08` uses the same content as reconcile (lines + charges − allowances, exact).
- **Breaking (0.1.x).** `Invoice.payable` / `tax_total` fields removed; they are views of `DocumentTotals`. BR-CO-18 fires without reconcile. BR-53: BT-6 ⇔ BT-111, never derived.
- **Convert.** CLI `convert` proves, then `write_validated`. Fatal → exit 1, findings on stdout, empty XML. `write` is renamed `write_unchecked` (tests only). `write_validated` overwrites BT-24 / BT-23 from the proved profile. Self-billing BT-24 cannot be re-stamped as billing (`CORE-PROCESS-01`).
- **CII (historical, crates.io 0.1.0 / 004 P0).** `convert --to cii` used to wrap UBL in `CrossIndustryInvoice` and was then refused as `CiiNotImplemented`. That wrapper is gone.
- **CII (current).** Three-part D16B **subset** for EN 16931 and Peppol BIS (lines before header, format 102). Qty, price, payment, allowances, and delivery are not mapped yet. **PINT-MY is UBL-only:** `FormatError::CiiNotForProfile`, CLI exit 2.
- **Breaking (0.1.x).** `FormatError::CiiNotImplemented` is renamed to `CiiNotForProfile` and is returned when writing CII for PINT-MY.
- **CLI.** `--profile` default is `auto` (BT-24). Invalid findings print on **stdout**. `explain` unknown ids exit 2. `explain` covers BR-06 and BR-07.
- **BR-05** is currency **presence**, not a 3-letter length check.
- **BR-CO-16** is no longer emitted for `payable = line net + tax total`. The CEN formula is documented by `explain` as not evaluated until document totals exist.
- **PINT-MY fixture** category is `SA` (not Singapore `SR`).
- **C ABI.** Unknown profile strings return 2 (not silent PINT). NULL profile is auto from BT-24. Error buffer truncation is byte-safe.
- **`/spec/`** is gitignored (CEN artefacts are EUPL-1.2). See `docs/spec.md`.

### Added

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
- Peppol extra_rules: R001/R003/R004/R007, R120 ±0.02 inclusive, R046 exact.
  They do not run on EN core or PINT-MY.
- CLI: `rules --format json`, `inspect` (no verdict), `profiles`. `diff`
  exits 1 when documents differ. Process tests cover 0/1/2.
- C ABI snippet in `core-invoice-sys`. CI: fmt, clippy `-D warnings`, tests,
  wasm32 model build. `publish.sh` no longer `--allow-dirty` on a real publish.
- UBL 2.1 codec is a tree walk (`roxmltree`): Invoice and CreditNote roots,
  no first-tag-wins, no invented EUR/XX/S. DTD refused. Writer emits IssueDate,
  type code, EndpointID, TaxSubtotal, LegalMonetaryTotal children, quantity/price.
  PINT-MY TaxScheme is `VAT` (never `SST`). `write_validated` stamps from a proof.
- Code lists (hand-curated subsets): `BR-CL-01` UNTDID 1001 split by
  `DocumentKind`, `BR-CL-04` ISO 4217 (not length-3; `XXX` allowed),
  `BR-CL-14` ISO 3166, `BR-CL-17`/`BR-CL-18` UNCL 5305 on VAT profiles only.
  Artefact pins `validation-1.3.16` / Peppol `v3.0.20` / PINT-MY `1.3.0`.
  `task spec` fetches into gitignored `/spec/`.
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

[Unreleased]: https://github.com/proxeon/core-invoice/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/proxeon/core-invoice/releases/tag/v0.1.0
