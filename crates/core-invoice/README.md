# core-invoice

Semantic model for EN 16931 and Peppol PINT. Tax systems: VAT, GST, SST, consumption.
Profiles are **siblings**: EN 16931, Peppol BIS 3.0 (the only CIUS of EN here), PINT (not a CIUS), PINT-MY.
No XML, no I/O — codecs live in `core-invoice-formats`.

Fatal ids are comparable to pinned ConnectingEurope / PINT-MY Schematron as evidenced by `task svrl`. Peppol BIS pin is `.sch`. Not IRBM Valid / MyInvois submit. This crate never talks to LHDN.
