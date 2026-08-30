# core-invoice-cli

Offline binary `core-invoice`. No network. **0.1.x is a skeleton.**

Exit codes (validate): **0** valid, **1** invalid (findings on stdout), **2** unreadable / I/O.

`--profile` default is **`auto`** (BT-24 / CustomizationID). Named slugs: `en16931`, `peppol`, `pint`, `pint-my`. A named profile forces that rule set (“would this pass as Peppol?”).

```sh
cargo run -p core-invoice-cli -- validate invoice.xml
cargo run -p core-invoice-cli -- validate --profile pint-my invoice.xml
cargo run -p core-invoice-cli -- convert invoice.xml --to ubl
cargo run -p core-invoice-cli -- diff a.xml b.xml
cargo run -p core-invoice-cli -- explain BR-05
```

`convert --to cii` exits **2** until UN/CEFACT CII D16B exists. Unknown `explain` ids exit **2**.
