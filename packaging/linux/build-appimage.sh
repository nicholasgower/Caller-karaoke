#!/usr/bin/env bash
# Build a simple AppImage from the release binary.
# Requires appimagetool (downloaded on demand) and a Linux release build.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
VERSION="${VERSION:-0.1.0}"
BIN="$ROOT/target/release/contra-karaoke"
OUT="$ROOT/dist/ContraKaraoke-${VERSION}-x86_64.AppImage"
APPDIR="$ROOT/dist/ContraKaraoke.AppDir"

if [ ! -x "$BIN" ]; then
  echo "Missing $BIN; run 'cargo build --release -p contra-app' first." >&2
  exit 1
fi

rm -rf "$APPDIR"
mkdir -p "$APPDIR/usr/bin" "$APPDIR/usr/share/applications" \
         "$APPDIR/usr/share/icons/hicolor/256x256/apps"
cp "$BIN" "$APPDIR/usr/bin/contra-karaoke"
cp "$ROOT/packaging/linux/contra-karaoke.desktop" \
   "$APPDIR/usr/share/applications/contra-karaoke.desktop"
cp "$ROOT/assets/icons/contra-karaoke-256.png" \
   "$APPDIR/usr/share/icons/hicolor/256x256/apps/contra-karaoke.png"
# The AppImage tools also look for the icon at the bundle root.
cp "$ROOT/assets/icons/contra-karaoke-256.png" "$APPDIR/contra-karaoke.png"
cp "$ROOT/assets/icons/contra-karaoke-256.png" "$APPDIR/.DirIcon"

cat > "$APPDIR/AppRun" <<'EOF'
#!/bin/sh
HERE="$(dirname "$(readlink -f "$0")")"
exec "$HERE/usr/bin/contra-karaoke" "$@"
EOF
chmod +x "$APPDIR/AppRun"

TOOL="$ROOT/dist/appimagetool-x86_64.AppImage"
if [ ! -x "$TOOL" ]; then
  curl -L --fail --retry 3 \
    -o "$TOOL" \
    https://github.com/AppImage/AppImageKit/releases/download/continuous/appimagetool-x86_64.AppImage
  chmod +x "$TOOL"
fi

mkdir -p "$ROOT/dist"
APPIMAGE_EXTRACT_AND_RUN=1 "$TOOL" "$APPDIR" "$OUT"
echo "Wrote $OUT"
