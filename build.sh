#!/bin/bash
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
OUTPUT_DIR="$SCRIPT_DIR/target/syncthing"

rm -rf "$OUTPUT_DIR"
mkdir -p "$OUTPUT_DIR/backend"

cp "$SCRIPT_DIR/src/manifest.json" "$OUTPUT_DIR/"
cp "$SCRIPT_DIR/src/icon.png" "$OUTPUT_DIR/"
cp "$SCRIPT_DIR/src/config.sample.json" "$OUTPUT_DIR/"

QT_BIN_DIR="${QT_BIN_DIR:-$(command -v qtpaths >/dev/null 2>&1 && qtpaths --binaries-dir || true)}"

if [[ -n "${QT_BIN_DIR:-}" && -x "$QT_BIN_DIR/rcc" ]]; then
    RCC_BIN="$QT_BIN_DIR/rcc"
elif [[ -x "/opt/homebrew/opt/qt/bin/rcc" ]]; then
    RCC_BIN="/opt/homebrew/opt/qt/bin/rcc"
elif [[ -x "/opt/homebrew/opt/qt@5/bin/rcc" ]]; then
    RCC_BIN="/opt/homebrew/opt/qt@5/bin/rcc"
elif command -v rcc >/dev/null 2>&1; then
    RCC_BIN="$(command -v rcc)"
else
    echo "error: Qt rcc not found. Install Qt and ensure qtpaths/rcc are on PATH (or set QT_BIN_DIR)." >&2
    exit 1
fi

"$RCC_BIN" --binary -o "$OUTPUT_DIR/resources.rcc" "$SCRIPT_DIR/src/application.qrc"

TARGET_TRIPLE="aarch64-unknown-linux-gnu"

(cd "$SCRIPT_DIR/src/backend" && cargo build --release --target "$TARGET_TRIPLE" --message-format=short)
cp "$SCRIPT_DIR/src/backend/target/$TARGET_TRIPLE/release/syncthing-monitor-backend" "$OUTPUT_DIR/backend/entry"

echo "Build completed. App output is in $OUTPUT_DIR"
