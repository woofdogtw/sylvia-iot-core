# Changelog

## 0.0.12 - 2023-05-21

### Changed

- **general-mq**: Rename Connection trait to `GmqConnection` and Queue trait to `GmqQueue`.
    - Let `Connection` and `Queue` to be the utility structures.
- Update dependencies.

### Fixed

- **general-mq**: Replace `lapin` by `amqprs`.
    - To avoid locking a connection when creating channels concurrently.
    - To fix memory leak when processing consumer messages.

## 0.0.11 - 2023-05-20

### Fixed

- **coremgr-cli**: Add device changing network or address that should be supported in v0.0.10.
- **coremgr-cli**: Fix login flow.

## 0.0.10 - 2023-05-19

### Added

- **broker**: Add profile for each device.
    - For application servers, they can quickly parse data by profiles instead of maintaining relationships of devices and data format themselves.
    - For the **data** module(s), it (they) can develop rule engines by refering to the profiles.
- **broker**: Support changing network and address for devices.
    - A device can be changed to another network or address like changing network module.

### Changed

- Update dependencies.

### Fixed

- **auth**: Fix authentication and authorization flow by replacing `user_id` with `session_id` to prevent force attack.

## 0.0.9 - 2023-05-12

### Added

- Add `GET /version` API for all services (auth, broker, coremgr, data, router).

### Changed

- Improve integration tests.
    - Merge common test cases of MongoDB/SQLite. This reduces more than 16,000 lines to compile integration tests faster.
    - Add test cases for configurations from command-line arguments (with clap) and environment variables.
- Remove more compiler warning messages.
- Update dependencies.
- **coremgr**: Modify CSV UTF-8 BOM generation.
- **coremgr**: Update `rumqttd` to 0.14.0.

### Fixed

- **broker**: Fix command-line argument that does not support `--broker.api-scopes`.
- **broker**: Fix environment variables prefix from `BROKER_MEMORY_` to `BROKER_CACHE_MEMORY_`
- **coremgr**: Fix integration test that middleware cases cannot be put before API cases.

## 0.0.8 - 2023-04-28

### Changed

- Update Rust to 1.69.0 with GitHub Actions runner image 20230426.1.
- Update dependencies.

## 0.0.7 - 2023-04-21

### Changed

- Update Rust to 1.68.2 with GitHub Actions runner image 20230417.1.
- Replace `async-lock` with `tokio::sync`.
- Update dependencies.

## 0.0.6 - 2023-03-31

### Changed

- Update Rust to 1.68.1 (v0.0.4 uses the old action runner with 1.68.0).
    - With GitHub Actions runner image 20230326.1.
- Update dependencies.

### Fixed

- Update Docker images with alpine-3.17.3.

## 0.0.5 - 2023-03-27

### Changed

- Update dependencies.

### Fixed

- Fix a bug that configurations cannot be modified by environment variables. This is caused by using clap `default_value()`.

## 0.0.4 - 2023-03-24

### Changed

- Update Rust to 1.68.1.
- Update dependencies.

## 0.0.3 - 2023-03-17

### Added

- Add `pull-request` event in the build-test workflow for Pull Requests.

### Changed

- Update dependencies.

## 0.0.2 - 2023-03-10

### Changed

- Rename `sylvia-*` to `sylvia-iot-*`.
- Update dependencies.

## 0.0.1 - 2023-03-05

### Added

- The first release.
