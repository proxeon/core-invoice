# testdata

Git-tracked **official XML slice** so `cargo test` on a fresh clone runs CEN
unit tests and official samples. ~2 MB. Not the crate licence — see [NOTICE](NOTICE).

Layout mirrors `refers/` paths tests already used:

```
testdata/en16931/test/{Invoice,CreditNote}-unit-UBL/
testdata/en16931/ubl/examples/
testdata/en16931/cii/examples/
testdata/peppol-bis-invoice-3/rules/examples/
testdata/pint-my-1.3.0/unpacked/trn-invoice/example/
```

Tests call `core_invoice_formats::corpus("…")`, which prefers this tree and
falls back to `refers/` after `task spec`.

Not vendored here (still `task spec`): Schematron, XSLT, UBL/CII XSD zips,
full Peppol clone. Those are for `task svrl` / the artefacts CI job.

Do not put these files under `crates/`. crates.io packages do not include this
directory.
