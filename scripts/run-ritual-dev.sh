#!/bin/sh
# Run the development binary under its macOS bundle identity.
set -eu
cd "$(dirname "$0")/.."
target_dir="${CARGO_TARGET_DIR:-target}"
cargo build --locked --bin ritual --features gpui/runtime_shaders --target-dir "$target_dir"
contents="$target_dir/debug/Ritual Dev.app/Contents"
binary="$target_dir/debug/ritual"
mkdir -p "$contents/MacOS"
cp packaging/macos/ritual-dev.plist "$contents/Info.plist"
if ! [ "$binary" -ef "$contents/MacOS/ritual" ]; then
    ln -f "$binary" "$contents/MacOS/ritual"
fi
exec "$contents/MacOS/ritual" "$@"
