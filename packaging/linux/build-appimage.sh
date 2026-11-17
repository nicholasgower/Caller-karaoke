#!/usr/bin/env bash
# Build a simple AppImage from the release binary.
# Requires appimagetool (downloaded on demand) and a Linux release build.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
VERSION="${VERSION:-0.1.0}"
BIN="$ROOT/target/release/contra-karaoke"
OUT="$ROOT/dist/ContraKaraoke-${VERSION}-x86_64.AppImage"
APPDIR="$ROOT/dist/ContraKaraoke.AppDir"
ICON="$ROOT/assets/icons/contra-karaoke-256.png"
DESKTOP="$ROOT/packaging/linux/contra-karaoke.desktop"

if [ ! -x "$BIN" ]; then
  echo "Missing $BIN; run 'cargo build --release -p contra-app' first." >&2
  exit 1
fi

rm -rf "$APPDIR"
mkdir -p "$APPDIR/usr/bin" "$APPDIR/usr/share/applications" \
         "$APPDIR/usr/share/icons/hicolor/256x256/apps"
cp "$BIN" "$APPDIR/usr/bin/contra-karaoke"
cp "$DESKTOP" "$APPDIR/usr/share/applications/contra-karaoke.desktop"
cp "$ICON" "$APPDIR/usr/share/icons/hicolor/256x256/apps/contra-karaoke.png"
# appimagetool aborts without a top-level .desktop file and a PNG icon that
# matches its Icon= entry (an SVG icon is rejected), so both also live at the
# AppDir root.
cp "$DESKTOP" "$APPDIR/contra-karaoke.desktop"
cp "$ICON" "$APPDIR/contra-karaoke.png"
cp "$ICON" "$APPDIR/.DirIcon"

cat > "$APPDIR/AppRun" <<'EOF'
#!/bin/sh
HERE="$(dirname "$(readlink -f "$0")")"
exec "$HERE/usr/bin/contra-karaoke" "$@"
EOF
chmod +x "$APPDIR/AppRun"

TOOL="$ROOT/target/appimagetool-x86_64.AppImage"
if [ ! -x "$TOOL" ]; then
  curl -L --fail --retry 3 \
    -o "$TOOL" \
    https://github.com/AppImage/AppImageKit/releases/download/continuous/appimagetool-x86_64.AppImage
  chmod +x "$TOOL"
fi

mkdir -p "$ROOT/dist"
APPIMAGE_EXTRACT_AND_RUN=1 "$TOOL" "$APPDIR" "$OUT"
echo "Wrote $OUT"
