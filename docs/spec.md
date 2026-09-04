# Spec artefacts (not in git)

CEN EN 16931 validation artefacts are **EUPL-1.2**. Peppol and PINT carry their own terms. They must not enter this MIT OR Apache-2.0 crate graph.

**Local cache:** [`refers/`](../refers/) — links, pins, and `fetch.sh`. Clones and zips are gitignored; the README is tracked.

**Test XML slice:** [`testdata/`](../testdata/) (~2 MB, tracked). Official examples and CEN unit-test XML so `cargo test` on a fresh clone does not skip. EUPL/Peppol terms — [testdata/NOTICE](../testdata/NOTICE). Not copied into `crates/`. Full Schematron/XSLT still need `task spec`.

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

`Edition::En2026` stays unimplemented until **its** ConnectingEurope artefacts exist. Pin `validation-1.3.16` is EN 16931-1:2017+A1. Do not invent 2026 rules from drafts or news.

PINT Billing 1.1.2 is genericode + Schematron + preprocessed XSLT. **No** official invoice/credit-note instance XML in this pin. Authored `pint_gst_sr` is not an oracle. `task svrl` does not compare international PINT.

The hupe1980 crate at `/Users/akmalfirdaus/Code/lazuar/en16931` is a **shape** reference only (`refers/shape-en16931` after fetch). Schematron + CEN text win if they disagree.

Do not copy CEN example XML into `crates/core-invoice-fixtures/data/`. The git-tracked slice is [`testdata/`](../testdata/) (NOTICE, not crate licence). Synthetic samples we author may be MIT OR Apache-2.0. Never `git add refers/*.zip` or the clones.

Default `cargo test` loads `testdata/`. Optional CI job `artefacts` (`CORE_INVOICE_REQUIRE_SPEC=1`) still fetches full `refers/` for SVRL/XSLT. **Tag skip is not OK:** pushes of `v*` tags run `artefacts`.

SVRL oracle: `task svrl` (sets `SVRL_ORACLE=1`; `CORE_INVOICE_REQUIRE_SPEC=1` also works). Saxon-HE is an **oracle runner**, not a crate dependency. Resolution order: `saxon` on PATH, then `SAXON_JAR` + a working `java`, then **Docker Compose** `eclipse-temurin:21-jre` (`docker compose run --rm saxon …`) with the jar fetched to `xtask/.saxon/` (gitignored). Mapping: [`svrl-id-map.md`](svrl-id-map.md). Expected unmatched: the fenced list in [`UNCOVERED.md`](UNCOVERED.md) (prose is not scanned). UBL-CR/SR/DT are ignored in the semantic compare. Peppol BIS **v3.0.20** is `.sch` only — no OpenPEPPOL `@id` compare in this pin. Not the default `task test`. Self-check of the diff (no Saxon): `python3 xtask/svrl_oracle.py --self-test`. Mustang (`MUSTANG_JAR`) is VAT CII only and never default CI.

`task lists` regenerates `crates/core-invoice/src/generated_codes.rs` from PINT-MY genericode (code points only).
