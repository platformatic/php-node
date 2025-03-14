#!/usr/bin/env bash

set -euo pipefail

PHP_VERSION=${1:-8.4}
EXTENSIONS=${2:-$(source ./scripts/extensions.sh)}

./spc download --prefer-pre-built --with-php=${PHP_VERSION} --retry=10 --for-extensions=${EXTENSIONS}
