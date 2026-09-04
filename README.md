# core-invoice

A memory-safe library for Europe’s e-invoice **meaning** (CEN EN 16931) and Peppol **PINT** — VAT, GST, SST, and consumption tax, not VAT only. Other software can **validate and convert** UBL 2.1 and UN/CEFACT CII D16B **offline**.

No LHDN, no Peppol Access Point, no accounting UI.

```sh
cargo add core-invoice
cargo install core-invoice-cli
core-invoice validate --profile auto path/to/invoice.xml
```

Fatal ids are comparable to pinned ConnectingEurope EN 16931 `validation-1.3.16` and PINT-MY 1.3.0 Schematron as evidenced by `task svrl` (Saxon-HE, or Docker Compose Java when the host has no JDK). Peppol BIS v3.0.20 in `refers/` is `.sch` (no compiled XSLT). `validate().ok()` is **not** OpenPEPPOL Valid.

`validate --profile pint-my` is PINT-MY Schematron-shaped. It is **not** IRBM Valid / MyInvois submit.

| Crate | Job |
|---|---|
| [`core-invoice`](crates/core-invoice) | Semantic model. Tax systems: VAT, GST, SST, consumption. Profiles are **siblings**: EN 16931, Peppol BIS 3.0 (the only CIUS of EN here), PINT (not a CIUS), PINT-MY. TaxScheme `SST` is never emitted. |
| [`core-invoice-formats`](crates/core-invoice-formats) | UBL 2.1 tree walk (Invoice **and** CreditNote). CII D16B three-part envelope for EN/Peppol (qty/price/payment/allowances/delivery subset). **PINT-MY is UBL-only.** |
| [`core-invoice-cli`](crates/core-invoice-cli) | `validate` / `convert` / `diff` / `explain` / `rules` / `inspect` (binary `core-invoice`). Exit 0 valid, 1 invalid, 2 unreadable. Default `--profile auto`. `validate --format json`, batch paths, `--quiet`, convert `-o`. |
| [`core-invoice-fixtures`](crates/core-invoice-fixtures) | In-memory PINT-MY SST, Peppol VAT, Pint GST samples |
| [`core-invoice-sys`](crates/core-invoice-sys) | C ABI `core_invoice_validate` / `_ubl` / convert / diff / version (0/1/2). Python: [`python/core_invoice.py`](python/core_invoice.py) via ctypes (`validate_xml` only). wasm is the **model** crate (`cargo build -p core-invoice --target wasm32-unknown-unknown`), not a browser AP. |

```sh
task                 # list
task test            # cargo test --workspace
task check           # fmt + clippy + test
task cli -- validate --profile auto path/to/invoice.xml
task invoice:test    # one crate (invoice|formats|bin|fixtures|sys)
task spec            # fetch official artefacts into gitignored refers/
task svrl            # compare Fatal ids to pinned ConnectingEurope / PINT-MY XSLT
```

Install the orchestrator: `brew install go-task`.

MSRV 1.88.0, pinned by [`rust-toolchain.toml`](rust-toolchain.toml).

## Profiles

Four shipped profiles, **siblings**, not a ladder. You do not `widen()` PINT or PINT-MY into EN or Peppol BIS. CLI `--profile` default is `auto` (BT-24 prefix).

| Profile | What it is |
|---|---|
| EN 16931 | Core semantic model. UBL and CII. |
| Peppol BIS Billing 3.0 | A **CIUS** of EN. VAT-only. Extra rules via `Profile::extra_rules`. Pin `v3.0.20` (Schematron only in this pin). |
| PINT | International. Tax is not only VAT. **Not** a CIUS. Pin Billing 1.1.2. |
| PINT-MY | Specialises PINT (SST / TTx). **UBL-only.** Pin 1.3.0. Wire TaxScheme is `VAT` / `AAL`, never `SST`. |

A German XRechnung BT-24 is ingested as **EN 16931**. There is no shipped `Profile::XRechnung` and no KoSIT `BR-DE-*` as CORE.

## Not this library

- Peppol Access Point, AS4, SMP, SML
- LHDN / MyInvois submit, IRBM Valid, digital certificates
- Accounting UI, general ledger, PDF letterhead
- XRechnung as a default / CORE profile (`BR-DE-*`)
- ZUGFeRD / Factur-X PDF **write**
- EN 16931-1:2026 rules (`Edition::En2026` stays unimplemented until **its** artefacts exist)
- CII write of a PINT-MY invoice

License: MIT OR Apache-2.0. Releases: [CHANGELOG.md](CHANGELOG.md). crates.io: [core-invoice 2.0.0](https://crates.io/crates/core-invoice). Git tag **v2.0.0**. crates.io **1.0.0** remains (not yanked).

Official artefacts (CEN / Peppol / PINT): [docs/spec.md](docs/spec.md) and [`refers/`](refers/) (`task spec`; clones and zips are gitignored).
