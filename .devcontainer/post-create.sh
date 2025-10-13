#!/usr/bin/env bash
set -euo pipefail

sudo apt-get update
sudo apt-get install -y curl unzip ca-certificates

if ! sudo apt-get install -y openjdk-21-jre; then
  sudo apt-get install -y openjdk-17-jre
fi

if ! cargo install --list | grep -q "typst-cli"; then
  echo "Installing Typst via cargo..."
  cargo install --locked typst-cli
else
  echo "Typst already installed via cargo; skipping."
fi

if ! command -v typst >/dev/null 2>&1; then
  echo "Linking typst binary into /usr/local/bin"
  sudo ln -sf "${CARGO_HOME:-$HOME/.cargo}/bin/typst" /usr/local/bin/typst
fi

BALLERINA_VERSION="2201.12.10"
BALLERINA_DEB="ballerina-${BALLERINA_VERSION}-swan-lake-linux-x64.deb"

if ! command -v bal >/dev/null 2>&1; then
  echo "Installing Ballerina ${BALLERINA_VERSION}..."
  BALLERINA_TMP="$(mktemp -d)"
  pushd "${BALLERINA_TMP}" >/dev/null
  curl -sSLO "https://dist.ballerina.io/downloads/${BALLERINA_VERSION}/${BALLERINA_DEB}"
  if ! sudo dpkg -i "${BALLERINA_DEB}"; then
    sudo apt-get install -f -y
    sudo dpkg -i "${BALLERINA_DEB}"
  fi
  popd >/dev/null
  rm -rf "${BALLERINA_TMP}"
else
  echo "Ballerina already installed; skipping."
fi

# Prime the cargo cache for faster builds inside the container
cargo fetch
