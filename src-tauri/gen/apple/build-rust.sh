#!/bin/sh
# Xcode "Build Rust Code" phase.
#
# When `bun tauri ios dev` / `bun tauri ios build` is running, hand over to the
# Tauri CLI as usual. When the app is built straight from Xcode (Product > Run
# onto the iPad), build the web UI and the Rust library here instead, so
# Xcode works on its own.
set -eu

export PATH="$HOME/.cargo/bin:$HOME/.bun/bin:/opt/homebrew/bin:/usr/local/bin:$PATH"

if [ -f "$SRCROOT/.tauri/cli-options-server.json" ]; then
  exec bun tauri ios xcode-script -v --platform "${PLATFORM_DISPLAY_NAME:?}" --sdk-root "${SDKROOT:?}" \
    --framework-search-paths "${FRAMEWORK_SEARCH_PATHS:?}" --header-search-paths "${HEADER_SEARCH_PATHS:?}" \
    --gcc-preprocessor-definitions "${GCC_PREPROCESSOR_DEFINITIONS:-}" --configuration "${CONFIGURATION:?}" \
    ${FORCE_COLOR:-} ${ARCHS:?}
fi

ROOT="$SRCROOT/../../.."
case "${PLATFORM_NAME:?}" in
  iphonesimulator) TARGET=aarch64-apple-ios-sim ;;
  iphoneos) TARGET=aarch64-apple-ios ;;
  *) echo "error: unsupported platform $PLATFORM_NAME" >&2; exit 1 ;;
esac
case "${CONFIGURATION:?}" in
  release) PROFILE=release; FLAG=--release ;;
  *) PROFILE=debug; FLAG= ;;
esac

echo "Building web UI"
(cd "$ROOT" && bun install --frozen-lockfile >/dev/null && bun run build)

echo "Building Rust library for $TARGET ($PROFILE)"
# Xcode's environment points cargo at the wrong SDK for build scripts that
# run on the Mac; clear it so host builds use the macOS SDK.
env -u SDKROOT -u IPHONEOS_DEPLOYMENT_TARGET -u MACOSX_DEPLOYMENT_TARGET \
  cargo build --manifest-path "$ROOT/Cargo.toml" -p apexdiag --lib --target "$TARGET" $FLAG \
  --features tauri/custom-protocol

OUT="$SRCROOT/Externals/arm64/$CONFIGURATION"
mkdir -p "$OUT"
cp "$ROOT/target/$TARGET/$PROFILE/libapexdiag_lib.a" "$OUT/libapp.a"
