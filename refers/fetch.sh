#!/usr/bin/env bash
# Populate refers/ with pinned official artefacts. Clones and zips are gitignored.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")" && pwd)"
cd "$ROOT"

EN_TAG="${EN_TAG:-validation-1.3.16}"
PEPPOL_TAG="${PEPPOL_TAG:-v3.0.20}"
PINT_VER="${PINT_VER:-1.1.2}"
PINT_MY_VER="${PINT_MY_VER:-1.3.0}"

EN_URL="https://github.com/ConnectingEurope/eInvoicing-EN16931.git"
PEPPOL_URL="https://github.com/OpenPEPPOL/peppol-bis-invoice-3.git"
UBL_ZIP_URL="https://docs.oasis-open.org/ubl/os-UBL-2.1/UBL-2.1.zip"
CII_ZIP_URL="https://unece.org/fileadmin/DAM/cefact/xml_schemas/D16B_SCRDM__Subset__CII.zip"
PINT_ZIP_URL="https://docs.peppol.eu/poac/pint/v${PINT_VER}/pint/resources.zip"
PINT_MY_ZIP_URL="https://docs.peppol.eu/poac/my/pint-my/resources.zip"

download() {
  local url="$1" dest="$2"
  if [[ -f "$dest" ]]; then
    echo "have $dest"
    return 0
  fi
  echo "GET $url"
  if command -v curl >/dev/null 2>&1; then
    curl -fL --retry 3 --retry-delay 2 -o "$dest" "$url"
  else
    wget -O "$dest" "$url"
  fi
}

clone_tag() {
  local url="$1" dir="$2" tag="$3"
  if [[ -d "$dir/.git" ]]; then
    echo "have $dir"
    git -C "$dir" fetch --depth 1 origin "refs/tags/${tag}:refs/tags/${tag}" || true
    git -C "$dir" checkout -q "refs/tags/${tag}" || git -C "$dir" checkout -q "$tag"
    return 0
  fi
  echo "clone $url @ refs/tags/$tag"
  git clone --depth 1 --branch "$tag" "$url" "$dir"
  git -C "$dir" fetch --depth 1 origin "refs/tags/${tag}" || true
  git -C "$dir" checkout -q FETCH_HEAD 2>/dev/null || git -C "$dir" checkout -q "$tag"
}

echo "== EN 16931 artefacts ($EN_TAG)"
clone_tag "$EN_URL" "$ROOT/en16931" "$EN_TAG"

echo "== Peppol BIS Billing 3.0 ($PEPPOL_TAG)"
clone_tag "$PEPPOL_URL" "$ROOT/peppol-bis-invoice-3" "$PEPPOL_TAG"

echo "== UBL 2.1 XSD zip"
mkdir -p "$ROOT/ubl-2.1"
download "$UBL_ZIP_URL" "$ROOT/ubl-2.1/UBL-2.1.zip"

echo "== CII D16B XSD zip"
mkdir -p "$ROOT/cii-d16b"
if ! download "$CII_ZIP_URL" "$ROOT/cii-d16b/D16B_SCRDM__Subset__CII.zip"; then
  echo "WARN: UNECE CII zip 403/moved. Using CEN artefact copy at en16931/cii/schema."
  ln -sfn "../en16931/cii/schema" "$ROOT/cii-d16b/from-cen-artefacts"
fi

echo "== PINT Billing $PINT_VER resources.zip"
mkdir -p "$ROOT/pint-billing-${PINT_VER}"
download "$PINT_ZIP_URL" "$ROOT/pint-billing-${PINT_VER}/resources.zip"

echo "== PINT-MY $PINT_MY_VER resources.zip"
mkdir -p "$ROOT/pint-my-${PINT_MY_VER}"
download "$PINT_MY_ZIP_URL" "$ROOT/pint-my-${PINT_MY_VER}/resources.zip"

SHAPE="/Users/akmalfirdaus/Code/lazuar/en16931"
if [[ -d "$SHAPE" && ! -e "$ROOT/shape-en16931" ]]; then
  ln -s "$SHAPE" "$ROOT/shape-en16931"
  echo "symlink shape-en16931 -> $SHAPE"
fi

{
  echo "# sha256 of downloaded files (not git clones)"
  echo "# generated $(date -u +%Y-%m-%dT%H:%M:%SZ)"
  find "$ROOT" -type f \( -name '*.zip' \) -print0 | sort -z | xargs -0 shasum -a 256
} >"$ROOT/PINS.sha256"

echo
echo "done. bulky trees are gitignored."
echo "pins: EN $EN_TAG  Peppol $PEPPOL_TAG  PINT $PINT_VER  PINT-MY $PINT_MY_VER"
ls -la "$ROOT"
