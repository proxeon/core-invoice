# core-invoice-cli

Offline binary `core-invoice`. No network. **0.1.x is a skeleton.** `inspect` prints fields, not a valid/invalid verdict.

| Exit | Meaning |
|---|---|
| **0** | valid / no semantic difference / explained / listed |
| **1** | invalid (findings on **stdout**) / documents differ (`diff`) |
| **2** | unreadable XML, I/O, unknown `explain` id, CII refused for PINT-MY |

`--profile` default is **`auto`** (BT-24 / CustomizationID). Named slugs: `en16931`, `peppol`, `pint`, `pint-my`. A named profile forces that rule set (“would this pass as Peppol?”).

```sh
cargo run -p core-invoice-cli -- validate invoice.xml
cargo run -p core-invoice-cli -- validate --profile pint-my invoice.xml
cargo run -p core-invoice-cli -- convert invoice.xml --to ubl
cargo run -p core-invoice-cli -- diff a.xml b.xml
cargo run -p core-invoice-cli -- explain BR-05
```

`validate --format json` prints `{ok, profile, findings}` on stdout (exit 0/1 as usual). `convert` proves first, then writes. `convert -o out.xml` writes the file and leaves stdout empty. Fatal findings: exit **1**, findings on stdout, **no XML**. Unreadable XML / CII refused for PINT-MY: exit **2**. `convert --to cii` writes a **subset** D16B envelope for EN/Peppol. On a **PINT-MY** invoice it exits **2** (`CiiNotForProfile`): PINT-MY is UBL-only. Unknown `explain` ids exit **2**. Broken pipe on convert stdout is swallowed.
