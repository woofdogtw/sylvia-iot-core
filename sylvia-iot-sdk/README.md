[![crates.io](https://img.shields.io/crates/v/sylvia-iot-sdk)](https://crates.io/crates/sylvia-iot-sdk)
[![Documentation](https://docs.rs/sylvia-iot-sdk/badge.svg)](https://docs.rs/sylvia-iot-sdk)
![CI](https://github.com/woofdogtw/sylvia-iot-core/actions/workflows/build-test.yaml/badge.svg)
[![Coverage](https://raw.githubusercontent.com/woofdogtw/sylvia-iot-core/gh-pages/docs/coverage/sylvia-iot-sdk/badges/flat.svg)](https://woofdogtw.github.io/sylvia-iot-core/coverage/sylvia-iot-sdk/)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

# sylvia-iot-sdk

SDK for developing networks (adapters) and applications on Sylvia-IoT. The SDK contains:

- `api`: utilities for accessing Sylvia-IoT **coremgr** APIs.
- `middlewares`: middlewares.
    - `auth`: token authentication.
- `mq`: managers for managing network/application connections/queues by using `general-mq`.
- `util`: utilities functions.
