# SVRL `@id` mapping

ConnectingEurope XSLT 1.3.16 / Peppol / PINT-MY Schematron SVRL `@id` compared to `Finding.id`.

**Policy.** Compare is case-insensitive. `ALIGNED-IBRP-*-MY` matches zip `aligned-ibrp-*` (the `-MY` suffix is crate spelling). Warning-only findings (e.g. BR-51) are not required to match `failed-assert` unless listed here as Fatal. Ids in `docs/UNCOVERED.md` are expected unmatched when **they** fire and **we** do not. An extra Fatal we emit that is not in SVRL and not in this table fails `task svrl`.

| Artefact `@id` | Our `Finding.id` | Notes |
|---|---|---|
| BR-CO-17 | BR-CO-17 | Artefact slack ±1.00 exclusive on abs; mapped 1:1 |
| BR-03 | BR-03 | Issue date presence (mutant) |
| BR-CO-26 | BR-CO-26 | Seller identifiable (mutant) |
| BR-S-02 | BR-S-02 | Standard rate line VAT (mutant) |
| PEPPOL-EN16931-R010 | PEPPOL-EN16931-R010 | Buyer EndpointID |
| PEPPOL-EN16931-R020 | PEPPOL-EN16931-R020 | Seller EndpointID |
| PEPPOL-EN16931-R004 | PEPPOL-EN16931-R004 | BT-24 Peppol prefix (SA-as-Peppol) |
| PEPPOL-EN16931-R007 | PEPPOL-EN16931-R007 | Peppol process id |
| PINT-TAX | PINT-TAX | Crate id; SST on Peppol |
| BR-CL-17 | BR-CL-17 | UNCL 5305 on VAT profiles |
| BR-CL-18 | PINT-TAX | SST line category is not UNCL 5305; we emit PINT-TAX instead of BR-CL-18 (EN artefact on SA) |
| CORE-SPEC-01 | — | Crate-only; expected extra on unknown BT-24, not on official EN examples |
| CORE-PROCESS-01 | — | Crate-only; self-billing URN |
| ids in `docs/UNCOVERED.md` fenced list | — | Expected unmatched when **they** fire and **we** do not. Prose in that file is not loaded. |

UBL-CR / UBL-SR / UBL-DT failed-asserts are **syntax**; the oracle drops them from the semantic compare (e.g. UBL-SR-43 on SST-as-EN).

Peppol-only Fatal ids (`PEPPOL-*`) compared to ConnectingEurope EN XSLT are not unexpected extras (EN cannot emit them). Docker Compose `saxon` (eclipse-temurin:21-jre + Saxon-HE 10.9) is an oracle runner when the host has no JDK.

`task svrl` (`xtask/svrl_oracle.py`) reads this table. Saxon is an oracle, not a crate dependency. Peppol BIS v3.0.20 in `refers/` is Schematron (`.sch`), not compiled XSLT — EN and PINT-MY XSLT are the live compares.