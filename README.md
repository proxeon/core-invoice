# core-invoice

**0.1.x is a skeleton. Do not validate legal invoices with it yet.**

Destination: a memory-safe library that implements Europe’s e-invoice meaning
(EN 16931) and Peppol PINT — including SST/GST, not only VAT — so software can
validate and convert UBL/CII offline, without talking to LHDN, Peppol, or any
accounting UI.

| Crate | Job |
|---|---|
| [`core-invoice`](crates/core-invoice) | Semantic model. Tax systems: VAT, GST, SST, consumption. Profiles: EN 16931, Peppol BIS 3.0, PINT, PINT-MY. |
| [`core-invoice-formats`](crates/core-invoice-formats) | UBL 2.1 subset scrape. **CII D16B is refused** until a real mapping exists. |
| [`core-invoice-cli`](crates/core-invoice-cli) | `validate` / `convert` / `diff` / `explain` (binary name `core-invoice`) |
| [`core-invoice-fixtures`](crates/core-invoice-fixtures) | In-memory PINT-MY SST and Peppol VAT samples |
| [`core-invoice-sys`](crates/core-invoice-sys) | C `validate` only. Python and WASM are not implemented. |

```sh
task                 # list
task test            # cargo test --workspace
task check           # fmt + clippy + test
task cli -- validate --profile auto path/to/invoice.xml
task invoice:test    # one crate (invoice|formats|bin|fixtures|sys)
```

Install the orchestrator: `brew install go-task`.

License: MIT OR Apache-2.0. Releases: [CHANGELOG.md](CHANGELOG.md).
 Spec artefacts: [docs/spec.md](docs/spec.md).
