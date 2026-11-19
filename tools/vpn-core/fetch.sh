#!/usr/bin/env sh
# Скачивает VPN-ядро sing-box (windows-amd64, официальный выпуск) и проверяет SHA-256.
# Для локальной работы без PowerShell (Linux/macOS/WSL): sh tools/vpn-core/fetch.sh
# Файлы кладутся в apps/desktop/src-tauri/resources/vpn-core/ и в git не попадают.
set -eu
VERSION="1.14.4"
URL="https://github.com/SagerNet/sing-box/releases/download/v${VERSION}/sing-box-${VERSION}-windows-amd64.zip"
ARCHIVE_HASH="99185d75d314a07dbba4a130c8ad986cbeb629a07927975636b341ca61f991df"
HASHES="sing-box.exe 8f30682ee198855440d2a7fe070d5d3653793b2d01bb8c85eea88f93bbe7c8b9
libcronet.dll 3217c6260fbca5f16072e0b79735742f40109a63bb0ff88fd6b96dd6b54a2928
LICENSE bb3805862b583aee73ad6f7805ec634747a37257a637a3069857843f05ea589c"

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
TARGET="$ROOT/apps/desktop/src-tauri/resources/vpn-core"
mkdir -p "$TARGET"

sha() { sha256sum "$1" 2>/dev/null | cut -d' ' -f1 || shasum -a 256 "$1" | cut -d' ' -f1; }

echo "$HASHES" | while read -r name hash; do
  [ -f "$TARGET/$name" ] && [ "$(sha "$TARGET/$name")" = "$hash" ] || exit 1
done && { echo "VPN-ядро sing-box $VERSION уже установлено, хэши совпали."; exit 0; } || true

WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT
echo "Скачиваем $URL"
curl -fsSL --proto '=https' --tlsv1.2 -o "$WORK/core.zip" "$URL"
actual="$(sha "$WORK/core.zip")"
[ "$actual" = "$ARCHIVE_HASH" ] || { echo "SHA-256 архива не совпал: $actual (ожидали $ARCHIVE_HASH). Файл не установлен." >&2; exit 1; }
mkdir -p "$WORK/out"
if command -v unzip >/dev/null 2>&1; then unzip -q "$WORK/core.zip" -d "$WORK/out"; else python3 -I -m zipfile -e "$WORK/core.zip" "$WORK/out"; fi
echo "$HASHES" | while read -r name hash; do
  f="$(find "$WORK/out" -type f -name "$name" | head -n 1)"
  [ -n "$f" ] || { echo "В архиве нет файла $name" >&2; exit 1; }
  [ "$(sha "$f")" = "$hash" ] || { echo "SHA-256 файла $name не совпал. Файлы не установлены." >&2; exit 1; }
done
echo "$HASHES" | while read -r name hash; do
  cp "$(find "$WORK/out" -type f -name "$name" | head -n 1)" "$TARGET/$name"
done
echo "Готово: sing-box $VERSION установлен в $TARGET"
