#!/bin/bash
set -euo pipefail

if [[ "$(uname -s)" != "Darwin" ]]; then
    echo "This script requires macOS (sips, iconutil and codesign)." >&2
    exit 1
fi

profile="${1:-release}"
case "$profile" in
    debug) cargo_profile="dev" ;;
    release) cargo_profile="release" ;;
    *) echo "Usage: bash scripts/bundle-macos.sh [debug|release]" >&2; exit 1 ;;
esac

project_dir="$(cd "$(dirname "$0")/.." && pwd)"
cd "$project_dir"
cargo build --profile "$cargo_profile"

# Ask Cargo for the output directory, including any configured target-dir.
target_dir="$(cargo metadata --no-deps --format-version 1 | /usr/bin/plutil -extract target_directory raw -o - -)"
bundle_dir="$target_dir/bundle/Popcorn.app"
iconset_dir="$target_dir/bundle/Popcorn.iconset"
mkdir -p "$bundle_dir/Contents/MacOS" "$bundle_dir/Contents/Resources" "$iconset_dir"

for pixels in 16 32 128 256 512; do
    sips -z "$pixels" "$pixels" popcorn.png --out "$iconset_dir/icon_${pixels}x${pixels}.png" >/dev/null
    retina_pixels=$((pixels * 2))
    sips -z "$retina_pixels" "$retina_pixels" popcorn.png --out "$iconset_dir/icon_${pixels}x${pixels}@2x.png" >/dev/null
done
iconutil -c icns "$iconset_dir" -o "$bundle_dir/Contents/Resources/Popcorn.icns"
cp "$target_dir/$profile/popcorn" "$bundle_dir/Contents/MacOS/popcorn"
cp macos/Info.plist "$bundle_dir/Contents/Info.plist"
plutil -lint "$bundle_dir/Contents/Info.plist"
codesign --force --sign - "$bundle_dir"
echo "Built $bundle_dir"
