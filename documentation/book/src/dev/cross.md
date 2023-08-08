# Cross-Platform Compilation

Sylvia-IoT is primarily developed for the x86-64 Linux platform. However, thanks to Rust's inherent
cross-platform capabilities, Sylvia-IoT can also be compiled into executable binaries for different
platforms. This chapter will introduce the compilation process for several platforms that the author
has tested.

The compiled executable should be able to run on compatible environments. For example, a Windows 10
executable should also be executable on Windows 7 or Windows 11.

> The compilation environment is based on Ubuntu-20.04.

## Windows 10 64-bit

```shell
rustup target add x86_64-pc-windows-gnu
rustup toolchain install stable-x86_64-pc-windows-gnu
sudo apt -y install mingw-w64
echo -e "[target.x86_64-pc-windows-gnu]\nlinker = \"/usr/bin/x86_64-w64-mingw32-gcc\"\nar = \"/usr/bin/x86_64-w64-mingw32-ar\"\n" >> ~/.cargo/config
cargo build --target=x86_64-pc-windows-gnu -p sylvia-iot-coremgr
```

## Raspberry Pi OS 64-bit

```shell
rustup target add aarch64-unknown-linux-gnu
sudo apt -y install gcc-aarch64-linux-gnu
echo -e "[target.aarch64-unknown-linux-gnu]\nlinker = \"/usr/bin/aarch64-linux-gnu-gcc\"\n" >> ~/.cargo/config
cargo build --target=aarch64-unknown-linux-gnu -p sylvia-iot-coremgr
```

## Raspberry Pi OS 32-bit

```shell
rustup target add armv7-unknown-linux-gnueabihf
sudo apt install gcc-arm-linux-gnueabihf
echo -e "[target.armv7-unknown-linux-gnueabihf]\nlinker = \"arm-linux-gnueabihf-gcc\"\n" > ~/.cargo/config
cargo build --target=armv7-unknown-linux-gnueabihf -p sylvia-iot-coremgr
```
