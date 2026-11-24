#!/bin/bash
# librela.so for 64-bit ARM phones (Android 15+), and its Kotlin bindings, into the app's sources (git-ignored).
set -euo pipefail
cd "$(dirname "$0")/.."
export ANDROID_NDK_HOME=${ANDROID_NDK_HOME:-$HOME/Android/Sdk/ndk/30.0.16248370}
# symbols kept for the bindings generator, which reads them from the .so; stripped after it
CARGO_PROFILE_RELEASE_STRIP=debuginfo cargo ndk -t arm64-v8a -P 35 -o android/app/src/main/jniLibs build --release -p rela-android
cargo run -q -p uniffi-bindgen -- generate --library android/app/src/main/jniLibs/arm64-v8a/librela.so \
    --language kotlin --no-format --out-dir android/app/src/main/kotlin
"$ANDROID_NDK_HOME"/toolchains/llvm/prebuilt/linux-x86_64/bin/llvm-strip android/app/src/main/jniLibs/arm64-v8a/librela.so
