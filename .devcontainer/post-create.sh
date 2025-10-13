#!/usr/bin/env bash
set -euo pipefail

sudo apt-get update
sudo apt-get install -y curl unzip ca-certificates

if ! sudo apt-get install -y openjdk-21-jre; then
  sudo apt-get install -y openjdk-17-jre
fi

if ! command -v typst >/dev/null 2>&1; then
  echo "Installing Typst binary..."
  wget -qO typst.tar.xz https://github.com/typst/typst/releases/latest/download/typst-x86_64-unknown-linux-musl.tar.xz
  sudo tar xf typst.tar.xz --strip-components=1 -C /usr/local/bin typst-x86_64-unknown-linux-musl/typst
  rm -f typst.tar.xz
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
