#!/usr/bin/env bash
set -euo pipefail

TYPST_VERSION="0.11.0"
ARCHIVE="typst-x86_64-unknown-linux-gnu.tar.xz"
TMP_DIR="$(mktemp -d)"

if ! command -v typst >/dev/null 2>&1; then
  echo "Installing Typst ${TYPST_VERSION}..."
  curl -sSL "https://github.com/typst/typst/releases/download/v${TYPST_VERSION}/${ARCHIVE}" -o "${TMP_DIR}/${ARCHIVE}"
  tar -C "${TMP_DIR}" -xf "${TMP_DIR}/${ARCHIVE}"
  sudo mv "${TMP_DIR}/typst-x86_64-unknown-linux-gnu/typst" /usr/local/bin/typst
  sudo chmod +x /usr/local/bin/typst
else
  echo "Typst already installed; skipping."
fi

rm -rf "${TMP_DIR}"

BALLERINA_VERSION="2201.10.4"
BALLERINA_DIST="ballerina-${BALLERINA_VERSION}-swan-lake-linux-x64"
BALLERINA_ARCHIVE="${BALLERINA_DIST}.zip"
BALLERINA_URL="https://dist.ballerina.io/downloads/${BALLERINA_VERSION}/${BALLERINA_ARCHIVE}"

if ! command -v bal >/dev/null 2>&1; then
  echo "Installing Ballerina ${BALLERINA_VERSION}..."
  sudo apt-get update
  sudo apt-get install -y unzip

  BALLERINA_TMP="$(mktemp -d)"
  curl -sSL "${BALLERINA_URL}" -o "${BALLERINA_TMP}/${BALLERINA_ARCHIVE}"
  sudo unzip -q "${BALLERINA_TMP}/${BALLERINA_ARCHIVE}" -d /usr/local/
  sudo ln -sf "/usr/local/${BALLERINA_DIST}/bin/bal" /usr/local/bin/bal
  sudo ln -sf "/usr/local/${BALLERINA_DIST}/bin/ballerina" /usr/local/bin/ballerina
  rm -rf "${BALLERINA_TMP}"
else
  echo "Ballerina already installed; skipping."
fi

# Install cargo utilities required by the CI/release pipeline
if ! cargo install --list | grep -q "toml-cli"; then
  cargo install toml-cli
else
  echo "toml-cli already installed; skipping."
fi

# Prime the cargo cache for faster builds inside the container
cargo fetch
