#!/usr/bin/env bash
# Скачивает 3D-эмодзи Fluent Emoji (MIT, microsoft/fluentui-emoji) в promo/assets/emoji.
set -euo pipefail
cd "$(dirname "$0")/emoji"
B=https://raw.githubusercontent.com/microsoft/fluentui-emoji/main/assets
get() { # папка файл
  curl -fsS -m 30 -o "$2.png" "$B/${1// /%20}/3D/$(echo "$1" | tr 'A-Z ' 'a-z_')_3d.png" || { echo "не скачался: $1" >&2; exit 1; }
}
get Broom broom; get Wastebasket wastebasket; get Shield shield
get Rocket rocket; get "High voltage" lightning; get Sparkles sparkles
