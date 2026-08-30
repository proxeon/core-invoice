# core-invoice

Semantic model for EN 16931 and Peppol PINT. Tax systems: VAT, GST, SST, consumption.
Profiles are **siblings**: EN 16931, Peppol BIS 3.0 (the only CIUS of EN here), PINT (not a CIUS), PINT-MY.
No XML, no I/O — codecs live in `core-invoice-formats`.

**0.1.x is not a legal validator.** Do not validate legal invoices with it yet.

`validate --profile pint-my` (via the formats/CLI crates) is PINT-MY Schematron-shaped
(Peppol PINT-MY 1.3.0). It is **not** IRBM Valid / MyInvois submit. This crate never talks to LHDN.
