#!/usr/bin/env bash
# Reproducible command sequence; only native runner logs count as acceptance.
set -euo pipefail
SDK="${1:-iphonesimulator}"
case "${SDK}" in
  iphoneos) RUST_TARGET=aarch64-apple-ios ;;
  iphonesimulator)
    case "$(uname -m)" in
      arm64) RUST_TARGET=aarch64-apple-ios-sim ;;
      x86_64) RUST_TARGET=x86_64-apple-ios ;;
      *) echo "Unsupported macOS host arch" >&2; exit 2 ;;
    esac
    ;;
  *) echo "Usage: bash apps/ios/build.sh [iphoneos|iphonesimulator]" >&2; exit 2 ;;
esac
if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "iOS requires a physical macOS/Xcode build host" >&2
  exit 2
fi
for tool in cargo rustup xcodegen xcodebuild; do
  command -v "$tool" >/dev/null || { echo "Missing $tool" >&2; exit 2; }
done
rustup target list --installed | grep -qx "$RUST_TARGET" || {
  echo "Missing Rust target $RUST_TARGET" >&2; exit 2;
}
ROOT="$(CDPATH= cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd -P)"
APP="$ROOT/apps/ios"
FFI="$ROOT/crates/ocaf-builder-mobile-ffi/Cargo.toml"
export CARGO_TARGET_DIR="$APP/.cargo-target"
mkdir -p "$APP/Generated" "$APP/NativeLibraries/$SDK"
cargo test --manifest-path "$FFI"
cargo build --release --manifest-path "$FFI"
cargo run --release --manifest-path "$FFI" --features bindgen --bin uniffi-bindgen -- \
  generate --library "$CARGO_TARGET_DIR/release/libocaf_builder_mobile_ffi.dylib" \
  --language swift --out-dir "$APP/Generated"
test -s "$APP/Generated/ocaf_builder_mobile_ffiFFI.modulemap"
cargo build --release --target "$RUST_TARGET" --manifest-path "$FFI"
cp "$CARGO_TARGET_DIR/$RUST_TARGET/release/libocaf_builder_mobile_ffi.a" \
   "$APP/NativeLibraries/$SDK/libocaf_builder_mobile_ffi.a"
(cd "$APP" && xcodegen generate)
xcodebuild -project "$APP/OCAFAgentsBuilder.xcodeproj" \
  -scheme OCAFAgentsBuilder -sdk "$SDK" -configuration Debug build
