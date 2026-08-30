# Spec artefacts (not in git)

CEN EN 16931 validation artefacts are **EUPL-1.2**. Peppol and PINT carry their own terms. They must not enter this MIT OR Apache-2.0 crate graph.

**Local cache:** [`refers/`](../refers/) — links, pins, and `fetch.sh`. Clones and zips are gitignored; the README is tracked.

```
task spec
# or
./refers/fetch.sh
```

| Corpus | Pin | Where after fetch |
|---|---|---|
| ConnectingEurope/eInvoicing-EN16931 | tag `validation-1.3.16` (git commit `b6c9e06`) | `refers/en16931/` |
| OpenPEPPOL/peppol-bis-invoice-3 | tag `v3.0.20` | `refers/peppol-bis-invoice-3/` |
| OASIS UBL 2.1 XSD | `UBL-2.1.zip` | `refers/ubl-2.1/` |
| UN/CEFACT CII D16B XSD | `D16B_SCRDM__Subset__CII.zip` | `refers/cii-d16b/` |
| Peppol PINT Billing | **1.1.2** `resources.zip` | `refers/pint-billing-1.1.2/` |
| PINT-MY Billing | **1.3.0** (2025-12-08) `resources.zip` | `refers/pint-my-1.3.0/` |

The hupe1980 crate at `/Users/akmalfirdaus/Code/lazuar/en16931` is a **shape** reference only (`refers/shape-en16931` after fetch). Schematron + CEN text win if they disagree.

Do not copy CEN example XML into `crates/core-invoice-fixtures/data/`. Synthetic samples we author may be MIT OR Apache-2.0. Never `git add refers/*.zip` or the clones.

Optional CI job `artefacts` (`CORE_INVOICE_REQUIRE_SPEC=1`) fetches `refers/` then runs official-sample tests. Default PR/`check` skips when artefacts are absent. **Tag skip is not OK:** pushes of `v*` tags run `artefacts`.

SVRL oracle: `task svrl` (set `SVRL_ORACLE=1` or `CORE_INVOICE_REQUIRE_SPEC=1`). Needs Saxon-HE (`saxon` on PATH or `SAXON_JAR` pointing at the jar). The MIT crate does **not** vendor Java. Mapping: [`svrl-id-map.md`](svrl-id-map.md). Not the default `task test`. Self-check of the diff (no Saxon): `python3 xtask/svrl_oracle.py --self-test`. Mustang (`MUSTANG_JAR`) is VAT CII only and never default CI.

`task lists` regenerates `crates/core-invoice/src/generated_codes.rs` from PINT-MY genericode (code points only).
