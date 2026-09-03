# core-invoice

**0.2.x is a development engine. Do not validate legal invoices with it yet.**

Fatal ids can be compared to pinned ConnectingEurope EN 16931 `validation-1.3.16` and PINT-MY 1.3.0 Schematron via `task svrl` (Saxon-HE, or Docker Compose Java when the host has no JDK). Peppol BIS v3.0.20 in `refers/` is `.sch` (no compiled XSLT). `validate().ok()` is **not** ConnectingEurope / OpenPEPPOL / IRBM Valid.

`validate --profile pint-my` is PINT-MY Schematron-shaped. It is **not** IRBM Valid / MyInvois submit. This crate never talks to LHDN, a Peppol AP, or an accounting UI.

Destination: a memory-safe library that implements Europe’s e-invoice meaning
(EN 16931) and Peppol PINT — including SST/GST, not only VAT — so software can
validate and convert UBL/CII offline, without talking to LHDN, Peppol, or any
accounting UI.

| Crate | Job |
|---|---|
| [`core-invoice`](crates/core-invoice) | Semantic model. Tax systems: VAT, GST, SST, consumption. Profiles are **siblings**: EN 16931, Peppol BIS 3.0 (the only CIUS of EN here), PINT (not a CIUS), PINT-MY. TaxScheme `SST` is never emitted. |
| [`core-invoice-formats`](crates/core-invoice-formats) | UBL 2.1 tree walk (Invoice **and** CreditNote). CII D16B three-part envelope for EN/Peppol (qty/price/payment/allowances/delivery subset). **PINT-MY is UBL-only.** |
| [`core-invoice-cli`](crates/core-invoice-cli) | `validate` / `convert` / `diff` / `explain` / `rules` / `inspect` (binary `core-invoice`). Exit 0 valid, 1 invalid, 2 unreadable. Default `--profile auto`. `validate --format json`, batch paths, `--quiet`, convert `-o`. |
| [`core-invoice-fixtures`](crates/core-invoice-fixtures) | In-memory PINT-MY SST, Peppol VAT, Pint GST samples |
| [`core-invoice-sys`](crates/core-invoice-sys) | C ABI `core_invoice_validate` / `_ubl` / convert / diff / version (0/1/2). Python: [`python/core_invoice.py`](python/core_invoice.py) via ctypes. wasm is the **model** crate (`cargo build -p core-invoice --target wasm32-unknown-unknown`), not a browser AP. |

```sh
task                 # list
task test            # cargo test --workspace
task check           # fmt + clippy + test
task cli -- validate --profile auto path/to/invoice.xml
task invoice:test    # one crate (invoice|formats|bin|fixtures|sys)
```

Install the orchestrator: `brew install go-task`.

MSRV 1.88.0, pinned by [`rust-toolchain.toml`](rust-toolchain.toml).

License: MIT OR Apache-2.0. Releases: [CHANGELOG.md](CHANGELOG.md).
Official artefacts (CEN / Peppol / PINT): [docs/spec.md](docs/spec.md) and [`refers/`](refers/) (`task spec`; clones and zips are gitignored).
