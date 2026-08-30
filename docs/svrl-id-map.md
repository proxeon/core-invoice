# SVRL `@id` mapping (P14)

ConnectingEurope XSLT 1.3.16 / Peppol / PINT-MY Schematron SVRL `@id` compared to `Finding.id`.

| Artefact `@id` | Our `Finding.id` | Notes |
|---|---|---|
| BR-CO-17 | BR-CO-17 | Artefact slack ±1.00 exclusive on abs; mapped 1:1 |
| PEPPOL-EN16931-R010 | PEPPOL-EN16931-R010 | Buyer EndpointID |
| PEPPOL-EN16931-R020 | PEPPOL-EN16931-R020 | Seller EndpointID |
| ids in `docs/UNCOVERED.md` | — | Not implemented; SVRL hit is expected unmatched |

Run the oracle when `refers/en16931` XSLT is present (P14.01). Until that job exists, this table is the contract.