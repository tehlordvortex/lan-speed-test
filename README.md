# LAN Speed Test

A tiny tool for testing how fast your WiFi/Ethernet is, You need two devices for that. Alternatively, you can either put `speedtestd` on a server somewhere, or tunnel it through `ngrok` or similar, for an actual internet speed test.
Note that this is not production grade or anything like that, I'm building this as I learn Rust.

## Building

You can build for your current platform, and also build an Android binary for use in Termux, for example.

Building for your current platform is easy, just run `SKIP_ANDROID_BUILD=1 ./build.sh` or `cargo build --release`.

In order to build for Android, you need to have the Android NDK set up, enable the `aarch64-android-linux` target in rustup (assuming your Android device is 64bit, all recent ones should be. But if it isn't, the target triple you need to enable will be different, and you'll need to modify build.sh).
You must set `ANDROID_NDK` to the path to the install side-by-side NDK (usually something like `$ANDROID_HOME/ndk/[VERSION]`) and `HOST_ARCH` to the host architecture (e.g. `darwin-x86_64` on MacOS, run `ls $ANDROID_NDK/toolchains/llvm/prebuilt` to check yours). For example:

```shell
you@YourPC:~ ANDROID_NDK=$ANDROID_HOME/ndk/[VERSION] HOST_ARCH=darwin-x86_64 SKIP_PLATFORM_BUILD=1 ./build.sh
```

## Running

There are two binaries built: `lan-speed-test` and `speedtestd`.

- `speedtestd` is the server and runs on the provided port (default `30000`) and exposes a `/stream` endpoint that streams `/dev/zero` to the client. (e.g. `speedtestd 42069`).
- `lan-speed-test` is the client, which connects to the server and measures fast the data comes in. You can invoke it with either an IP:PORT combo (e.g. `lan-speed-test 127.0.0.1:42069`), or a URL (which may or may not have the `/stream` path, e.g. `lan-speed-test https://my-server-url.some-provider.cloud`)

## TODO

- [] Have `lan-speed-test` automatically try and find a running `speedtestd` on the available network interfaces, if one isn't provided.
- [] Make this work on Windows, by generating the data to stream to the client somehow.

## Troubleshooting

- If you're having clang complain that it can't find `-lgcc`, follow the steps here: https://github.com/rust-lang/rust/pull/85806#issuecomment-1096266946 (You likely only have to do it for the particular arch you're targeting.)
