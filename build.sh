#!/bin/sh

set -xeu

HOST_ARCH=${HOST_ARCH:-darwin-x86_64}
SKIP_ANDROID_BUILD=${SKIP_ANDROID_BUILD:-unset}
SKIP_PLATFORM_BUILD=${SKIP_PLATFORM_BUILD:-unset}


if [ x"$SKIP_PLATFORM_BUILD" != "x1" ]; then
	echo "Building for current platform..."
	cargo build --release
fi

if [ x"$SKIP_ANDROID_BUILD" != "x1" ]; then
	echo "Cross-compiling for Android (binaries can be ran under Termux)..."
	export AR=$ANDROID_NDK/toolchains/llvm/prebuilt/$HOST_ARCH/bin/llvm-ar
	export CC=$ANDROID_NDK/toolchains/llvm/prebuilt/$HOST_ARCH/bin/aarch64-linux-android23-clang
	export CARGO_TARGET_AARCH64_LINUX_ANDROID_AR=$AR
	export CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER=$CC
	cargo build --target=aarch64-linux-android --release
fi