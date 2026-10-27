#!/bin/zsh
# Builds PawPal.app into ./build from the Rust sources (pass --install to copy it to /Applications,
# --universal to build one app for both Apple silicon and Intel Macs, as the release does).
# On Windows and Linux, `cargo build --release` is all you need; the binary is target/release/pawpal.
set -e
cd "$(dirname "$0")"
VERSION=$(sed -n 's/^version = "\(.*\)"/\1/p' Cargo.toml | head -1)
APP=build/PawPal.app
rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$APP/Contents/Resources"
if [[ " $* " == *" --universal "* ]]; then
  rustup target add aarch64-apple-darwin x86_64-apple-darwin >/dev/null
  cargo build --release --target aarch64-apple-darwin
  cargo build --release --target x86_64-apple-darwin
  lipo -create target/aarch64-apple-darwin/release/pawpal target/x86_64-apple-darwin/release/pawpal -output "$APP/Contents/MacOS/PawPal"
else
  cargo build --release
  cp target/release/pawpal "$APP/Contents/MacOS/PawPal"
fi

# App icon: every size macOS wants, from assets/icon.png.
ICONSET=build/AppIcon.iconset
rm -rf "$ICONSET" && mkdir -p "$ICONSET"
for size in 16 32 128 256 512; do
  sips -z $size $size assets/icon.png --out "$ICONSET/icon_${size}x${size}.png" >/dev/null
  sips -z $((size * 2)) $((size * 2)) assets/icon.png --out "$ICONSET/icon_${size}x${size}@2x.png" >/dev/null
done
iconutil -c icns "$ICONSET" -o "$APP/Contents/Resources/AppIcon.icns"
rm -rf "$ICONSET"
cat > "$APP/Contents/Info.plist" <<PLIST
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
  <key>CFBundleShortVersionString</key><string>$VERSION</string>
  <key>CFBundleVersion</key><string>$VERSION</string>
  <key>LSMinimumSystemVersion</key><string>13.0</string>
  <key>LSUIElement</key><true/>
  <key>NSHighResolutionCapable</key><true/>
  <key>NSCalendarsFullAccessUsageDescription</key><string>Your pet tells you 10, 5 and 1 minute before an event starts.</string>
  <key>NSCalendarsUsageDescription</key><string>Your pet tells you 10, 5 and 1 minute before an event starts.</string>
</dict>
</plist>
PLIST
codesign --force --sign - "$APP" >/dev/null
echo "Built $APP"
if [[ " $* " == *" --install "* ]]; then
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
