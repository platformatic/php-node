#!/usr/bin/env bash

set -euo pipefail

EXTENSIONS=${1:-$(source ./scripts/extensions.sh)}

OS=${2:-$(uname -s | tr '[:upper:]' '[:lower:]')}
ARCH=${3:-$(uname -m)}

if [[ "$ARCH" == "arm64" ]]; then
  ARCH="aarch64"
elif [[ "$ARCH" == "amd64" ]]; then
  ARCH="x86_64"
fi

if [[ "$OS" == "darwin" ]]; then
  OS="macos"
  # export MACOSX_DEPLOYMENT_TARGET=$(rustc --target ${ARCH}-apple-darwin --print deployment-target)
elif [[ "$OS" == "linux" ]]; then
  export PATH="/usr/local/musl/bin:$PATH"
  export CC="${ARCH}-linux-musl-gcc"
  export AR="${ARCH}-linux-musl-ar"
  if [[ "$(ldd --version 2>&1)" != *"musl"* ]]; then
    export SPC_LIBC="glibc"
  else
    export SPC_LIBC="musl"
  fi
fi

# Ensure it is built with debug symbols when DEBUG is set
if [[ -n "${DEBUG:-}" ]]; then
  SPC_CMD_PREFIX_PHP_CONFIGURE="./configure --prefix= --with-valgrind=no --enable-shared=no --enable-static=yes --disable-all --disable-cgi --disable-phpdbg --enable-debug"
fi

./spc build ${EXTENSIONS} --build-embed --enable-zts --no-strip --debug
