#!/bin/zsh
# Builds PawPal.app into ./build from the Rust sources (pass --install to copy it to /Applications).
# On Windows and Linux, `cargo build --release` is all you need; the binary is target/release/pawpal.
set -e
cd "$(dirname "$0")"
cargo build --release
APP=build/PawPal.app
rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
cp target/release/pawpal "$APP/Contents/MacOS/PawPal"

# App icon: every size macOS wants, from assets/icon.png.
ICONSET=build/AppIcon.iconset
rm -rf "$ICONSET" && mkdir -p "$ICONSET"
for size in 16 32 128 256 512; do
  sips -z $size $size assets/icon.png --out "$ICONSET/icon_${size}x${size}.png" >/dev/null
  sips -z $((size * 2)) $((size * 2)) assets/icon.png --out "$ICONSET/icon_${size}x${size}@2x.png" >/dev/null
done
iconutil -c icns "$ICONSET" -o "$APP/Contents/Resources/AppIcon.icns"
rm -rf "$ICONSET"
cat > "$APP/Contents/Info.plist" <<'PLIST'
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>CFBundleName</key><string>PawPal</string>
  <key>CFBundleDisplayName</key><string>PawPal</string>
  <key>CFBundleIdentifier</key><string>id.my.rihanna.pawpal</string>
  <key>CFBundleExecutable</key><string>PawPal</string>
  <key>CFBundleIconFile</key><string>AppIcon</string>
  <key>CFBundlePackageType</key><string>APPL</string>
  <key>CFBundleShortVersionString</key><string>1.0</string>
  <key>CFBundleVersion</key><string>1</string>
  <key>LSMinimumSystemVersion</key><string>13.0</string>
  <key>LSUIElement</key><true/>
  <key>NSHighResolutionCapable</key><true/>
</dict>
</plist>
PLIST
codesign --force --sign - "$APP" >/dev/null
echo "Built $APP"
if [[ "$1" == "--install" ]]; then
  osascript -e 'quit app "PawPal"' 2>/dev/null || true
  sleep 1
  rm -rf /Applications/PawPal.app
  cp -R "$APP" /Applications/
  # Only the installed copy should be known to macOS, or System Settings may show the build copy (or no icon).
  LSREGISTER=/System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister
  "$LSREGISTER" -u "$APP" 2>/dev/null || true
  "$LSREGISTER" -f /Applications/PawPal.app
  echo "Installed to /Applications/PawPal.app"
fi
