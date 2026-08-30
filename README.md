# core-invoice

A memory-safe library that implements Europe’s e-invoice meaning (EN 16931) and
Peppol PINT — including SST/GST, not only VAT — so software can validate and
convert UBL/CII offline, without talking to LHDN, Peppol, or any accounting UI.

| Crate | Job |
|---|---|
| [`core-invoice`](crates/core-invoice) | Semantic model. Tax systems: VAT, GST, SST, consumption. Profiles: EN 16931, Peppol BIS 3.0, PINT, PINT-MY. |
| [`core-invoice-formats`](crates/core-invoice-formats) | UBL 2.1 ↔ UN/CEFACT CII |
| [`core-invoice-cli`](crates/core-invoice-cli) | `validate` / `convert` / `diff` / `explain` |
| [`core-invoice-fixtures`](crates/core-invoice-fixtures) | Public pass/fail corpus |
| [`core-invoice-sys`](crates/core-invoice-sys) | C, Python, WASM bindings |

```sh
task                 # list
task test            # cargo test --workspace
task check           # fmt + clippy + test
task cli -- validate --profile pint-my path/to/invoice.xml
task invoice:test    # one crate (invoice|formats|bin|fixtures|sys)
```

Install the orchestrator: `brew install go-task`.

License: MIT OR Apache-2.0. Releases: [CHANGELOG.md](CHANGELOG.md).
