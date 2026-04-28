#!/bin/bash
set -euo pipefail

REPO="torifo/cmd-mock-cli"
BIN_NAME="cmdock"
INSTALL_DIR="${HOME}/.local/bin"

OS=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)

case "${OS}" in
  darwin) OS_TAG="macos" ;;
  linux)  OS_TAG="linux" ;;
  *)
    echo "Unsupported OS: ${OS}" >&2
    exit 1
    ;;
esac

case "${ARCH}" in
  x86_64)        ARCH_TAG="x86_64" ;;
  aarch64|arm64) ARCH_TAG="aarch64" ;;
  *)
    echo "Unsupported architecture: ${ARCH}" >&2
    exit 1
    ;;
esac

ARCHIVE="${BIN_NAME}-${OS_TAG}-${ARCH_TAG}.tar.gz"

LATEST=$(curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" \
  | grep '"tag_name"' \
  | sed 's/.*"tag_name": *"\([^"]*\)".*/\1/')

if [ -z "${LATEST}" ]; then
  echo "Failed to fetch latest release tag" >&2
  exit 1
fi

URL="https://github.com/${REPO}/releases/download/${LATEST}/${ARCHIVE}"

TMP_DIR=$(mktemp -d)
trap 'rm -rf "${TMP_DIR}"' EXIT

echo "Downloading cmdock ${LATEST} (${OS_TAG}-${ARCH_TAG})..."
curl -fsSL "${URL}" -o "${TMP_DIR}/${ARCHIVE}"
tar -xzf "${TMP_DIR}/${ARCHIVE}" -C "${TMP_DIR}"

if [ ! -f "${TMP_DIR}/${BIN_NAME}" ]; then
  echo "Error: binary not found in archive" >&2
  exit 1
fi

mkdir -p "${INSTALL_DIR}"
install -m 755 "${TMP_DIR}/${BIN_NAME}" "${INSTALL_DIR}/${BIN_NAME}"

echo ""
echo "Installed: ${INSTALL_DIR}/${BIN_NAME}"
echo ""

if ! printf '%s\n' "${PATH//:/$'\n'}" | grep -qx "${INSTALL_DIR}"; then
  echo "NOTE: Add to your shell profile (~/.zshrc or ~/.bashrc):"
  echo "  export PATH=\"${INSTALL_DIR}:\${PATH}\""
  echo ""
fi

echo "Get started:"
echo "  cmdock"
