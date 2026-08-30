# core-invoice

**0.1.x is a skeleton. Do not validate legal invoices with it yet.**

Destination: a memory-safe library that implements Europe’s e-invoice meaning
(EN 16931) and Peppol PINT — including SST/GST, not only VAT — so software can
validate and convert UBL/CII offline, without talking to LHDN, Peppol, or any
accounting UI.

| Crate | Job |
|---|---|
| [`core-invoice`](crates/core-invoice) | Semantic model. Tax systems: VAT, GST, SST, consumption. Profiles are **siblings**: EN 16931, Peppol BIS 3.0 (the only CIUS of EN here), PINT (not a CIUS), PINT-MY. TaxScheme `SST` is never emitted. |
| [`core-invoice-formats`](crates/core-invoice-formats) | UBL 2.1 tree walk (Invoice **and** CreditNote) — a **lossy subset** until remaining EN 16931 terms are mapped on the wire. CII D16B three-part envelope for EN/Peppol only (qty/price/payment still incomplete). **PINT-MY is UBL-only.** |
| [`core-invoice-cli`](crates/core-invoice-cli) | `validate` / `convert` / `diff` / `explain` / `rules` / `inspect` (binary `core-invoice`). Exit 0 valid, 1 invalid, 2 unreadable. Default `--profile auto`. |
| [`core-invoice-fixtures`](crates/core-invoice-fixtures) | In-memory PINT-MY SST, Peppol VAT, Pint GST samples |
| [`core-invoice-sys`](crates/core-invoice-sys) | C ABI `core_invoice_validate_ubl` (0/1/2). Python is not implemented; the model crate builds for `wasm32-unknown-unknown`. |

```sh
task                 # list
task test            # cargo test --workspace
task check           # fmt + clippy + test
task cli -- validate --profile auto path/to/invoice.xml
task invoice:test    # one crate (invoice|formats|bin|fixtures|sys)
```

Install the orchestrator: `brew install go-task`.

License: MIT OR Apache-2.0. Releases: [CHANGELOG.md](CHANGELOG.md).
Official artefacts (CEN / Peppol / PINT): [docs/spec.md](docs/spec.md) and [`refers/`](refers/) (`task spec`; clones and zips are gitignored).
