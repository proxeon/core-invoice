# Spec artefacts (not in git)

CEN EN 16931 validation artefacts are **EUPL-1.2**. Peppol and PINT carry their own terms. They must not enter this MIT OR Apache-2.0 crate graph.

Clone into `/spec/` (gitignored) when you need the official corpora:

| Corpus | Pin | Notes |
|---|---|---|
| ConnectingEurope/eInvoicing-EN16931 | tag `validation-1.3.16` | Use a fully-qualified `refs/tags/…` ref. The name is both a tag and a branch at different commits. |
| OpenPEPPOL/peppol-bis-invoice-3 | tag `v3.0.20` | Peppol BIS Billing 3.0 |
| Peppol PINT Billing | docs.peppol.eu 1.1.x resources zip | Record zip hash in a pin constant when fetch lands |
| PINT-MY Billing | **1.3.0** (2025-12-08) | docs.peppol.eu/poac/my/pint-my/ |

Do not copy CEN example XML into `crates/core-invoice-fixtures/data/`. Synthetic samples we author may be MIT OR Apache-2.0.

Fetch command will be `task spec` once that recipe exists. Until then, do not `git add spec/`.
