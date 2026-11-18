#!/usr/bin/env sh
# Скачивает исходники ClamAV 1.4.6 из зеркала Debian и проверяет SHA-256.
# Исходники и сборка не коммитятся (см. .gitignore в этой папке).
set -eu
VERSION=1.4.6
URL="https://deb.debian.org/debian/pool/main/c/clamav/clamav_${VERSION}+dfsg.orig.tar.xz"
SHA256=09823124e9bae5d602699c2a2fe5d4876298283cf3e7f8982973571843572468
HERE="$(cd "$(dirname "$0")" && pwd)"
mkdir -p "$HERE/src"
curl -sSL -o "$HERE/clamav.orig.tar.xz" "$URL"
echo "$SHA256  $HERE/clamav.orig.tar.xz" | sha256sum -c -
tar -xJf "$HERE/clamav.orig.tar.xz" -C "$HERE/src" --strip-components=1
echo "Исходники готовы: $HERE/src"
