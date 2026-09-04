# core-invoice-formats

UBL 2.1 **tree walk** (`roxmltree`): Invoice **and** CreditNote roots, DTD refused,
depth 64. This is a **lossy subset** of EN 16931 — not first-tag-wins, not yet
lossless. `read_with_trace` reports unmapped children and malformed amounts.
`write_unchecked` is for tests; production write is `write_validated`
(stamps BT-24 / BT-23 from the proved profile). `convert` proves, then writes.

**CII D16B** is a three-part envelope (lines before header, format 102) for
**EN 16931 and Peppol BIS**. `Profile::Pint` (international) may emit the same
subset. Header BT-15/16/19 and BT-7/8, line identifiers/A/C/period/object, and
BT-147-in-gross are mapped. Empty `PostalAddress` vs absent stays a named drop.
**PINT-MY is UBL-only** — `write_unchecked`/`convert` to CII on that profile
returns `FormatError::CiiNotForProfile`. Do not wrap UBL in a
`CrossIndustryInvoice` costume.

Generated CEN syntax prohibitions (`task prohibitions`) feed `write_drops` / `scan_written`: they report forbidden children and attributes on the written bytes. They do not rewrite the tree and they are not a Schematron engine. Empty hits are not CEN Valid, not KoSIT Valid, not OpenPEPPOL Valid.

Full guide: <https://github.com/proxeon/core-invoice>
