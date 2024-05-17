# 跨平台編譯

Sylvia-IoT 主要是針對 x86-64 Linux 平台開發。由於 Rust 語言本身的跨平台特性，Sylvia-IoT 也同樣可以編譯成不同平台的可執行檔。
本章節將介紹筆者測試的幾個平台的編譯流程。

編譯出來的可執行檔應該可以執行於相容的環境下。比如 Windows 10 的可執行檔也可以執行在 Windows 7、Windows 11 上。

> 編譯環境都是基於 Ubuntu-22.04。

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
sudo apt -y install gcc-arm-linux-gnueabihf
echo -e "[target.armv7-unknown-linux-gnueabihf]\nlinker = \"arm-linux-gnueabihf-gcc\"\n" >> ~/.cargo/config
cargo build --target=armv7-unknown-linux-gnueabihf -p sylvia-iot-coremgr
```
