# core-invoice

Semantic model for EN 16931 and Peppol PINT. Tax systems: VAT, GST, SST, consumption.
Profiles are **siblings**: EN 16931, Peppol BIS 3.0 (the only CIUS of EN here), PINT (not a CIUS), PINT-MY.
No XML, no I/O — codecs live in `core-invoice-formats`.

```sh
cargo add core-invoice
```

Fatal ids are comparable to pinned ConnectingEurope `validation-1.3.16` and PINT-MY 1.3.0 as evidenced by `task svrl`. Peppol BIS v3.0.20 is `.sch` only. Not OpenPEPPOL Valid.

`validate --profile pint-my` (via the formats/CLI crates) is PINT-MY Schematron-shaped
(Peppol PINT-MY 1.3.0). It is **not** IRBM Valid / MyInvois submit. This crate never talks to LHDN.

An XRechnung BT-24 is ingested as EN 16931. There is no `Profile::XRechnung`. Optional Cargo feature `xrechnung` (`BR-DE-1`…`11`, `14`–`17`, `23-a`/`24-a`/`25-a`, type-retired `-b`, `30`) is off by default and is not CORE.

Full guide: <https://github.com/proxeon/core-invoice>
