#!/bin/bash
set -euo pipefail

REPO="torifo/cmd-mock-cli"
BIN_NAME="cmdock"
LEGACY_BIN_NAME="cmd-mock-cli"
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

LATEST=$(curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" \
  | grep '"tag_name"' \
  | sed 's/.*"tag_name": *"\([^"]*\)".*/\1/')

if [ -z "${LATEST}" ]; then
  echo "Failed to fetch latest release tag" >&2
  exit 1
fi

TMP_DIR=$(mktemp -d)
trap 'rm -rf "${TMP_DIR}"' EXIT

echo "Downloading cmdock ${LATEST} (${OS_TAG}-${ARCH_TAG})..."
ARCHIVES=(
  "${BIN_NAME}-${OS_TAG}-${ARCH_TAG}.tar.gz"
  "${LEGACY_BIN_NAME}-${OS_TAG}-${ARCH_TAG}.tar.gz"
)

DOWNLOADED_ARCHIVE=""
for ARCHIVE in "${ARCHIVES[@]}"; do
  URL="https://github.com/${REPO}/releases/download/${LATEST}/${ARCHIVE}"
  if curl -fsSL "${URL}" -o "${TMP_DIR}/${ARCHIVE}"; then
    DOWNLOADED_ARCHIVE="${ARCHIVE}"
    break
  fi
done

if [ -z "${DOWNLOADED_ARCHIVE}" ]; then
  echo "Error: no compatible archive found for ${OS_TAG}-${ARCH_TAG}" >&2
  exit 1
fi

tar -xzf "${TMP_DIR}/${DOWNLOADED_ARCHIVE}" -C "${TMP_DIR}"

SOURCE_BIN=""
if [ -f "${TMP_DIR}/${BIN_NAME}" ]; then
  SOURCE_BIN="${TMP_DIR}/${BIN_NAME}"
elif [ -f "${TMP_DIR}/${LEGACY_BIN_NAME}" ]; then
  SOURCE_BIN="${TMP_DIR}/${LEGACY_BIN_NAME}"
else
  echo "Error: binary not found in archive" >&2
  exit 1
fi

mkdir -p "${INSTALL_DIR}"
install -m 755 "${SOURCE_BIN}" "${INSTALL_DIR}/${BIN_NAME}"

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
