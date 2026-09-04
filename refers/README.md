# Official references for `core-invoice`

This directory is the **local cache of sources of truth**. The Rust clone
`/Users/akmalfirdaus/Code/lazuar/en16931` is a **shape** reference (how to
structure a meaning engine). It is **not** the spec.

Clones and zips are **gitignored**. This README and `fetch.sh` are tracked.
The XML slice tests need is git-tracked under [`testdata/`](../testdata/) (not this cache).

```
task spec          # same as: refers/fetch.sh
./refers/fetch.sh  # populate clones + zips
```

Licence reminder: CEN artefacts are **EUPL-1.2**. Do not copy `.sch` / official
example XML into `crates/` (MIT OR Apache-2.0). Extract code-point lists into
generated MIT files only after a licence check.

---

## Pins (constants)

Must match `crates/core-invoice/src/codes.rs`:

| Constant | Value |
|---|---|
| `ARTEFACT_VERSION` | `validation-1.3.16` |
| `PEPPOL_BIS_VERSION` | `v3.0.20` |
| `PINT_MY_VERSION` | `1.3.0` |
| PINT Billing (this cache) | `1.1.2` (mandatory 2026-03-09) |

EN 16931 git ref **must** be fully qualified `refs/tags/validation-1.3.16`.
The name is both a tag and a branch at different commits.

---

## Layout after fetch

```
refers/
  README.md                 this file (tracked)
  fetch.sh                  populate (tracked)
  en16931/                  ConnectingEurope/eInvoicing-EN16931 @ tag
  peppol-bis-invoice-3/     OpenPEPPOL/peppol-bis-invoice-3 @ v3.0.20
  ubl-2.1/
    UBL-2.1.zip
    unpacked/               optional unzip
  cii-d16b/
    D16B_SCRDM__Subset__CII.zip
  pint-billing-1.1.2/
    resources.zip
  pint-my-1.3.0/
    resources.zip
  PINS.sha256               written by fetch.sh (gitignored)
  shape-en16931             symlink to ../../en16931 if that clone exists
```

---

## Layer 1 — Semantic standard (paywalled PDF)

Not downloadable here. Keep a personal copy off-tree.

| Document | What it decides | Link |
|---|---|---|
| **EN 16931-1:2017+A1:2019** | Table 2 (~164 BTs), §6.4 BR-\* / BR-CO-\* / BR-S-\* wording | [CEN](https://standards.cencenelec.eu/) / national body |
| **CEN/TS 16931-2** | UBL + CII are the mandated syntaxes | CEN |
| **CEN/TS 16931-3-2** | BT → UBL element | CEN |
| **CEN/TS 16931-3-3** | BT → CII element | CEN |
| EN 16931-1:2026 | Next edition; `Edition::En2026` stays unimplemented until **its** artefacts exist | CEN |

Rule **ids and `explain()` text** come from EN 16931-1. Slack does **not**.

---

## Layer 2 — Syntax XSD (well-formed XML)

| Artefact | Pin / file | Link |
|---|---|---|
| OASIS **UBL 2.1** (ISO/IEC 19845:2015) | `UBL-2.1.zip` | https://docs.oasis-open.org/ubl/os-UBL-2.1/UBL-2.1.zip |
| UBL 2.1 HTML | — | https://docs.oasis-open.org/ubl/os-UBL-2.1/UBL-2.1.html |
| UBL 2.1 directory | — | https://docs.oasis-open.org/ubl/os-UBL-2.1/ |
| UN/CEFACT **CII D16B** subset | UNECE zip (often 403) **or** `en16931/cii/schema` | https://unece.org/fileadmin/DAM/cefact/xml_schemas/D16B_SCRDM__Subset__CII.zip — local fallback `refers/cii-d16b/from-cen-artefacts` |
| UNECE XML schemas index | — | https://unece.org/trade/uncefact/xml-schemas |

---

## Layer 3 — Validation artefacts (Schematron) = deployed truth

Fatal `id` + XPath + artefact slack live here. EUPL-1.2 (CEN) / Peppol terms.

| Corpus | Pin | Git / zip | Notes |
|---|---|---|---|
| CEN EN 16931 UBL+CII | **`validation-1.3.16`** (2026-04-10, commit `b6c9e06`) | https://github.com/ConnectingEurope/eInvoicing-EN16931/releases/tag/validation-1.3.16 | `.sch`, `.xslt`, `examples/`. Tag/branch collide — use `refs/tags/`. |
| CEF registry (same drop) | 1.3.16 | https://ec.europa.eu/digital-building-blocks/sites/display/DIGITAL/Registry+of+supporting+artefacts+to+implement+EN16931 | Distribution DIGITAL publishes |
| CEF validations explainer | — | https://ec.europa.eu/digital-building-blocks/sites/display/DIGITAL/Validations | Artefacts are **not** the standard; they express it |
| Peppol BIS Billing 3.0 | **`v3.0.20`** | https://github.com/OpenPEPPOL/peppol-bis-invoice-3/releases/tag/v3.0.20 | `PEPPOL-EN16931-R*`, P0100, examples |
| BIS Billing 3.0 HTML | current docs | https://docs.peppol.eu/poacc/billing/3.0/ | Syntax binding + rules pages |
| PINT Billing | **1.1.2** | https://docs.peppol.eu/poac/pint/v1.1.2/pint/ | [resources.zip](https://docs.peppol.eu/poac/pint/v1.1.2/pint/resources.zip) |
| PINT-MY Billing | **1.3.0** (2025-12-08) | https://docs.peppol.eu/poac/my/pint-my/ | [resources.zip](https://docs.peppol.eu/poac/my/pint-my/resources.zip) |
| PINT-MY release notes | 1.3.0 | https://docs.peppol.eu/poac/my/pint-my/specialized-release-notes/ | TTx `AAL`, SA/SE/HVG/LVG, IBR-SR-63 |
| OpenPeppol post-award index | — | https://peppol.org/documentation/technical-documentation/post-award-documentation/ | Mandatory dates per spec |

Oracle: same XML through official XSLT (Saxon / [easybill/en16931-validator](https://github.com/easybill/en16931-validator)) **and** `core-invoice`. Diffs are bugs or documented artefact-only slack.

---

## Layer 4 — Code lists

Prefer packs inside the CEN / Peppol / PINT zips after fetch. Canonical homes:

| List | Used for | Home |
|---|---|---|
| ISO 4217 | BT-5 / BR-CL-04 | ISO; also in EN artefacts |
| ISO 3166-1 alpha-2 | BT-40 / BR-CL-14 | ISO |
| UNTDID 1001 | BT-3 / BR-CL-01 (invoice vs credit-note **two lists**) | UNECE + EN artefacts |
| UNCL 5305 | BT-118/151 **VAT profiles only** | UNECE |
| UNCL 4461 | BT-81 | UNECE; MY `Z0x` are profile extras |
| EAS | Endpoint `@schemeID` | CEF / Peppol |
| ICD (ISO 6523) | Party identifiers | ISO / Peppol |
| VATEX | BT-121 | CEF |
| Rec 20 / Rec 21 | BT-130 | UNECE |
| PINT-MY TaxCat | SA SE HVG LVG TTX E O | **PINT-MY 1.3.0 code list**, not UNCL 5305 |

---

## Layer 5 — Shape only (not truth)

| Path | Role |
|---|---|
| `/Users/akmalfirdaus/Code/lazuar/en16931` | hupe1980 crate: copy **structure** (`xpath_round`, `Reconciler`, `Validated<P>`). Do not copy VAT-only types, XRechnung as shipped profile, ZUGFeRD write, EUPL in git. |
| `refers/shape-en16931` | Symlink to that clone after fetch, if present. |

If this crate and Schematron disagree, **Schematron + CEN text win**.

---

## Not sources of truth

- Our own writer’s round-trip XML
- LHDN MyInvois SDK / portal JSON
- Mustang / ZUGFeRD examples as EN core
- crates.io `en16931`
- Blog BR tables
- PINT-MY **1.3.1 upcoming** until the pin constant bumps
- Wikipedia

---

## How layers compose

1. **Meaning / rule id / formula** → EN 16931-1 (IBT-\* from PINT / PINT-MY semantic pages).
2. **XPath, slack, list membership, example pass/fail** → pinned Schematron + `examples/`.
3. **Element names, order, CreditNote schema** → UBL 2.1 / CII D16B XSD.
4. **Rust structure** → `en16931` crate.
5. **CI oracle** → official XSLT vs `core-invoice` on the same file.
