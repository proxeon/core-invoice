# core-invoice

[![crates.io](https://img.shields.io/crates/v/core-invoice.svg)](https://crates.io/crates/core-invoice)
[![docs.rs](https://docs.rs/core-invoice/badge.svg)](https://docs.rs/core-invoice)
[![MSRV](https://img.shields.io/badge/MSRV-1.88-blue)](rust-toolchain.toml)

Offline library for **what an e-invoice means**, not for sending it.

Europe’s semantic model is [CEN EN 16931](https://ec.europa.eu/digital-building-blocks/sites/display/DIGITAL/EN+16931+compliance). [Peppol PINT](https://docs.peppol.eu/poac/pint/pint/) is the international sibling: tax is **VAT, GST, SST, or consumption** — not VAT only. This library validates and converts **UBL 2.1** and **UN/CEFACT CII D16B** against those rules, with no network.

Other software can call it. It is not a Peppol Access Point, not LHDN / MyInvois submit, and not an accounting product.

```sh
cargo add core-invoice core-invoice-formats
cargo install core-invoice-cli
core-invoice validate invoice.xml
```

## How it works

XML is not the model. The library reads UBL or CII into a typed `Invoice`, runs profile rules on that value, and writes XML back from the same value.

```
UBL 2.1  or  CII D16B
        │
        ▼
   Invoice  (parties, lines, tax, totals)
        │
        ├── validate(profile)  →  Report   (rule ids such as BR-05, IBR-CL-01-MY)
        └── convert            →  UBL or CII
```

| Term | Meaning here |
|---|---|
| **EN 16931** | CEN core: the business terms (`BT-*`) and groups (`BG-*`) an invoice must carry. |
| **UBL 2.1** | OASIS XML syntax. Invoice and CreditNote. |
| **CII D16B** | UN/CEFACT XML syntax. A mapped subset for EN / Peppol; **not** for PINT-MY. |
| **Profile** | Which rule set to apply. Four shipped; they are **siblings**, not a ladder. |
| **BT-24** | `CustomizationID` — the URN that says which profile the document claims. CLI `--profile auto` reads this. |
| **CIUS** | A Core Invoice Usage Specification: extra rules on top of EN 16931. Peppol BIS 3.0 is the only CIUS shipped here. PINT is not a CIUS. |

Fatal rule ids are comparable to pinned ConnectingEurope EN 16931 `validation-1.3.16` and PINT-MY 1.3.0 (see [Evidence](#evidence)). `validate().ok()` is still **not** a legal attestation: not OpenPEPPOL Valid, not IRBM Valid.

## Profiles

`--profile auto` (the default) picks the rule set from BT-24. A named profile **forces** that set — “would this pass as Peppol?” even if BT-24 says otherwise.

You do not `widen()` PINT or PINT-MY into EN or Peppol BIS.

| Profile | CLI slug | What it is | Syntax |
|---|---|---|---|
| EN 16931 | `en16931` | Core semantic model (2017+A1). | UBL and CII |
| Peppol BIS Billing 3.0 | `peppol` | CIUS of EN. VAT-only. Pin **v3.0.20** (Schematron `.sch` only in this pin). | UBL and CII |
| PINT | `pint` | International. Tax is not only VAT. **Not** a CIUS. Pin Billing **1.1.2**. | UBL; CII subset |
| PINT-MY | `pint-my` | Specialises PINT (SST / TTx). Pin **1.3.0**. Wire TaxScheme is `VAT` / `AAL`, never `SST`. | **UBL only** |

A German XRechnung BT-24 is ingested as **EN 16931**. There is no `Profile::XRechnung`. Optional Cargo feature `xrechnung` adds some `BR-DE-*` rules; it is **off by default** and is not CORE.

## Crates

One version across the workspace.

| Crate | Role |
|---|---|
| [`core-invoice`](https://crates.io/crates/core-invoice) | Semantic model and rules. No XML, no I/O. |
| [`core-invoice-formats`](https://crates.io/crates/core-invoice-formats) | UBL 2.1 tree walk (Invoice and CreditNote). CII D16B subset for EN/Peppol. PINT-MY is UBL-only (`CiiNotForProfile`). |
| [`core-invoice-cli`](https://crates.io/crates/core-invoice-cli) | Binary `core-invoice`: validate, convert, diff, explain, rules, inspect. |
| [`core-invoice-fixtures`](https://crates.io/crates/core-invoice-fixtures) | In-memory PINT-MY SST and Peppol VAT samples (not the official CEN corpus). |
| [`core-invoice-sys`](https://crates.io/crates/core-invoice-sys) | C ABI: validate / convert / diff / version, exit 0/1/2. |

## Library

Validate XML. `None` as profile means “read BT-24”.

```rust
use core_invoice_formats::validate_xml;

let xml = std::fs::read_to_string("invoice.xml")?;
let report = validate_xml(&xml, None)?;
if report.ok() {
    println!("valid as {}", report.profile_slug);
} else {
    for f in &report.findings {
        println!("{}: {}", f.id, f.message);
    }
}
```

Build an invoice in memory and validate the model (no XML):

```rust
use core_invoice::{Invoice, Party, Profile, validate};

let mut invoice = Invoice::blank(
    Profile::PintMy,
    "INV-1",
    "MYR",
    Party::new("Seller", "MY"),
    Party::new("Buyer", "MY"),
);
// set lines, tax category, totals, issue date, …
let report = validate(&invoice);
if !report.ok() {
    println!("{report}");
}
```

`convert` proves the document first, then writes. Production write is `write_validated` (stamps BT-24 / BT-23 from the proved profile). CII write of a PINT-MY invoice is refused.

## Command line

```sh
core-invoice validate invoice.xml
core-invoice validate --profile pint-my invoice.xml
core-invoice validate --format json invoice.xml
core-invoice convert invoice.xml --to ubl -o out.xml
core-invoice diff a.xml b.xml
core-invoice explain BR-05
core-invoice rules --profile peppol
core-invoice inspect invoice.xml
```

From this repo, after clone:

```sh
cargo run -p core-invoice-cli -- validate testdata/peppol-bis-invoice-3/rules/examples/base-example.xml
```

| Exit | Meaning |
|---|---|
| **0** | Valid, no semantic difference, explained, or listed |
| **1** | Invalid (findings on **stdout**) or `diff` found a difference |
| **2** | Unreadable XML, I/O, unknown `explain` id, or CII refused for PINT-MY |

`inspect` prints fields. It does not give a valid/invalid verdict. Validity is never colour-only.

## C, Python, WebAssembly

- **C** — [`crates/core-invoice-sys`](crates/core-invoice-sys): `core_invoice_validate` / `_ubl`, convert, diff, version. Same 0/1/2 contract. Header: [`include/core_invoice.h`](crates/core-invoice-sys/include/core_invoice.h).
- **Python** — [`python/core_invoice.py`](python/core_invoice.py) via ctypes. Surface is `validate_xml` only (no PyPI wheel).
- **wasm** — the **model** crate: `cargo build -p core-invoice --target wasm32-unknown-unknown`. Not a browser Access Point. Formats/sys are not that job.

## Not this library

- Peppol Access Point, AS4, SMP, SML
- LHDN / MyInvois submit, IRBM Valid, digital certificates
- Accounting UI, general ledger, PDF letterhead
- XRechnung as a default / CORE profile
- ZUGFeRD / Factur-X PDF **write**
- EN 16931-1:2026 rules (`Edition::En2026` stays unimplemented until **its** artefacts exist)
- CII write of a PINT-MY invoice
- Compiling Peppol `.sch` ourselves and calling that OpenPEPPOL Valid

## Evidence

Pins, fetch, and licence fences: [`docs/spec.md`](docs/spec.md). Expected unmatched ids: [`docs/UNCOVERED.md`](docs/UNCOVERED.md).

| Corpus | Pin | Role |
|---|---|---|
| ConnectingEurope EN 16931 | `validation-1.3.16` | Fatal-id compare (`task svrl`) |
| Peppol BIS Billing 3.0 | `v3.0.20` | `.sch` only — no compiled OpenPEPPOL XSLT in this pin |
| Peppol PINT Billing | `1.1.2` | International profile; no official example XML in this pin |
| PINT-MY Billing | `1.3.0` | Malaysian specialisation (UBL) |

`task svrl` runs pinned Schematron (Saxon-HE, or Docker Compose Java) and diffs Fatal `Finding.id` against SVRL `@id`. That is an **oracle**, not a crate dependency.

**Test XML** lives in git-tracked [`testdata/`](testdata/) (~2 MB): CEN UBL unit tests and official EN / Peppol / PINT-MY samples, so `cargo test` on a fresh clone does not skip. EUPL / Peppol terms — [`testdata/NOTICE`](testdata/NOTICE). Not copied into `crates/`, so crates.io packages do not include it.

**Full artefacts** (Schematron, XSLT, XSD zips, clones) stay in gitignored [`refers/`](refers/). Fetch with `task spec`.

## Development

MSRV **1.88.0**, pinned by [`rust-toolchain.toml`](rust-toolchain.toml). Orchestrator: `brew install go-task`.

```sh
task                 # list
task test            # cargo test --workspace
task check           # fmt + clippy + test
task cli -- validate --profile auto path/to/invoice.xml
task spec            # fetch refers/ (needed for task svrl, not for cargo test)
task svrl            # compare Fatal ids to pinned ConnectingEurope / PINT-MY XSLT
```

## License and versions

MIT OR Apache-2.0. Releases: [CHANGELOG.md](CHANGELOG.md).

crates.io: [core-invoice 2.0.2](https://crates.io/crates/core-invoice). Git tag **v2.0.2**. crates.io **2.0.1** / **2.0.0** / **1.0.0** remain (not yanked). 2.x may add APIs; breaking changes are 3.0. The C ABI is the 0/1/2 verbs, not `Invoice` layout.
