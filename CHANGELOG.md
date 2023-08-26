# Changelog

## 0.0.30 - 2023-08-26

### Changed

- Update dependencies.
- **sdk**: **INCOMPATIBLE** API modifications.
    - Modify the `roles` field type of user HTTP API.

## 0.0.29 - 2023-08-25

### Changed

- Update dependencies.
- Add more information in Cargo.toml for publishing crates.

### Fixed

- **sdk**: Fix middleware that the `token` field must be token, not Authorization header content.

## 0.0.28 - 2023-08-21

### Changed

- Update dependencies.
- **sdk**: **INCOMPATIBLE** API modifications.
    - Modify time-relative fields of user HTTP functions from `String` to `DateTime<Utc>` for convenience.

## 0.0.27 - 2023-08-20

### Changed

- **sdk**: **INCOMPATIBLE** API modifications.
    - Modify the `time` field of network control data from `String` to `DateTime<Utc>` for convenience.

## 0.0.26 - 2023-08-20

### Changed

- Update dependencies.
- **sdk**: **INCOMPATIBLE** API modifications.
    - Modify the `data` field from `String` to `Vec<u8>` for convenience.

## 0.0.25 - 2023-08-18

### Changed

- Update Rust to 1.71.1 with GitHub Actions runner image 20230814.1.0.
- Modify time format descriptions from ISO 8601 to RFC 3339.
- Update dependencies.
- **general-mq/broker/coremgr/data/sdk**: **INCOMPATIBLE** API modifications.
    - Separates errors, status, messages into three two handlers and three callback functions.

## 0.0.24 - 2023-08-12

### Changed

- Update dependencies.
- **general-mq/broker/coremgr/sdk**: Provides `persistent` options for AMQP.

## 0.0.23 - 2023-08-11

### Changed

- Update dependencies.
- **documentation**
    - Add the cross platform compilation chapter.
    - Add IDE configuration examples.
    - Add Go projects.

### Fixed

- Fix bugs that init flow does not apply default configurations for JSON5.
- **general-mq**: Use **persistent** delivery mode when sending data with reliable queues.
- **sdk**: Fix `mq` examples in docs.

## 0.0.22 - 2023-08-04

### Fixed

- **broker**: Fix the ApplicationMgr/NetworkMgr name from **code** instead of **name**.

## 0.0.21 - 2023-08-04

### Changed

- Update dependencies.
- **documentation**: Add `rustfmt` section.

### Fixed

- **broker**: Fix init flow for adding ApplicationMgr/NetworkMgr.
    - When adding a new instance, it causes other instances in the cluster to re-create ApplicationMgrs/NetworkMgrs.
    - The root cause is that it uses broadcast control channel to init ApplicationMgrs/NetworkMgrs.
- **broker**, **sdk**: Fix shared connection management for queues.
    - To lock the connection mutex after allocating `Connection`s.

## 0.0.20 - 2023-07-28

### Changed

- Update dependencies.
- **sdk**: Add example code in Rust doc for auth middleware.

## 0.0.19 - 2023-07-23

### Changed

- Update dependencies.

### Fixed

- **general-mq**: Fix bugs of establishing AMQP/MQTT connections after `connect()`.

## 0.0.18 - 2023-07-21

### Changed

- Update Rust to 1.71.0 with GitHub Actions runner image 20230716.1.1.
- Update action versions for GitHub CI.
- Update dependencies.
- **auth**: Update jquery and bootstrap version for default templates.

## 0.0.17 - 2023-07-14

### Changed

- Update dependencies.
- **coremgr-cli**: Print latency with stderr for using pipes to process JSON output more convinient.

### Fixed

- Fix bugs of the example JSON5 configuration file.

## 0.0.16 - 2023-07-07

### Changed

- Update dependencies.

### Fixed

- Update `sqlx` to 0.7.0 to solve CVE issues.
- Use Ubuntu-20.04 to release binary for GLIBC compatibility on older OS distributions.

## 0.0.15 - 2023-06-30

### Changed

- Update Rust to 1.70.0 with GitHub Actions runner image 20230619.1.0.
- Update Docker images with alpine-3.17.4.
- Update dependencies.
- **broker**: Force convert host name to lowercase when using `PATCH /appplication` or `PATCH /network`.
- **coremgr**: Modify test cases for RabbitMQ 3.12.

## 0.0.14 - 2023-06-04

### Added

- **broker**: Add network control messages for operating devices.
- **coremgr**: Add network control message queue support including permissions and stats.
- **sdk**: Add network control message support in `NetworkMgr`.
- **sdk**: Add HTTP client to handle request retrying when the access token is expired.
    - Currently we only provide user get/update API utility functions.

### Changed

- **simple-stress**: Add latency statistics for min, 50%, 80%, 90%, 95%, 98%, 99%, max.

### Fixed

- **coremgr**: To fix a bug that creating EMQX network queues with application permissions.

## 0.0.13 - 2023-05-26

### Changed

- Update dependencies.

### Fixed

- **general-mq**: Support **amqps**.

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
