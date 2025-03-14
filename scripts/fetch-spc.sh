#!/usr/bin/env bash

set -euo pipefail

OS=${1:-$(uname -s | tr '[:upper:]' '[:lower:]')}
ARCH=${2:-$(uname -m)}

if [[ "$OS" == "darwin" ]]; then
  OS="macos"
fi

if [[ "$ARCH" == "arm64" ]]; then
  ARCH="aarch64"
elif [[ "$ARCH" == "amd64" ]]; then
  ARCH="x86_64"
fi

curl -fsSL -o spc https://dl.static-php.dev/static-php-cli/spc-bin/nightly/spc-${OS}-${ARCH}
chmod +x ./spc
./spc doctor --auto-fix
