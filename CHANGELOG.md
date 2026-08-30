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

Nothing yet.

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
