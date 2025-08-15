# Changelog

## 0.3.7 - 2025-08-15

### Changed

- Update dependencies and fix vulnerabilities.
- Update Rust to 1.89.0 with GitHub Actions runner image 20250810.1.0.

## 0.3.6 - 2025-07-25

### Changed

- Update dependencies and fix vulnerabilities.
- Update Rust to 1.88.0 with GitHub Actions runner image 20250720.1.0.

### Fixed

- **sylvia-iot-core-cli**: Fix client ID validation.

## 0.3.5 - 2025-06-30

### Fixed

- Fix CI bugs.

## 0.3.4 - 2025-06-30

### Fixed

- Fix CI bugs.

## 0.3.3 - 2025-06-30

### Fixed

- Fix CI bugs.

## 0.3.2 - 2025-06-29

### Fixed

- Fix CI bugs.

## 0.3.1 - 2025-06-29

### Changed

- Switched from GLIBC to MUSL for improved Linux distribution compatibility.
- Added pre-built binaries for the `arm64` architecture.
- Provided multi-architecture Docker images supporting both `amd64` and `arm64`.
- Update Rust to 1.87.0 with GitHub Actions runner image 20250622.1.0.

## 0.3.0 - 2025-06-20

### Changed

- **breaking**: Upgrade Rust to 2024 edition and modify MSRV to 1.85.1.
- Run rustfmt with 2024 edition.
- Update dependencies.
- Release using Ubuntu 24.04.

## 0.2.1 - 2025-01-10

### Fixed

- **sylvia-iot-auth**, **sylvia-iot-core**, **sylvia-router**: Replace `nest_service` with `fallback_service` for root path not support issue.

## 0.2.0 - 2025-01-10

### Changed

- **breaking**: Upgrade axum to 0.8, changing the path parameter syntax from `/:single` to `/{single}`.
- Update dependencies.

## 0.1.16 - 2024-12-20

### Changed

- Update Rust to 1.83.0 with GitHub Actions runner image 20241215.1.1.
- Update dependencies and fix vulnerabilities.

## 0.1.15 - 2024-09-28

### Changed

- Update dependencies for fixing crate bugs.

## 0.1.14 - 2024-09-20

### Changed

- Update dependencies for yanked crates.

## 0.1.13 - 2024-09-13

### Changed

- Update Rust to 1.81.0 with GitHub Actions runner image 20240908.1.1.
- Update dependencies for yanked crates.

## 0.1.12 - 2024-08-31

### Changed

- Update dependencies.

### Fixed

- **broker**: Fix cache cleaning when changing device's network or address.
- **broker**: Fix device profile information when sending network uplink data to the **data** module.

## 0.1.11 - 2024-08-30

### Changed

- Update Rust to 1.80.1 with GitHub Actions runner image 20240825.1.1.
- Update dependencies and fix vulnerabilities.

## 0.1.10 - 2024-08-16

### Changed

- Update dependencies.

## 0.1.9 - 2024-08-05

### Changed

- Separates publishing crates and releases into two GitHub Action jobs.

## 0.1.8 - 2024-08-04

### Changed

- Update Rust to 1.80.0 with GitHub Actions runner image 20240730.2.1.
- Update dependencies.

### Fixed

- Fix HTTPS error for upgrading `rustls` to v0.23.

## 0.1.7 - 2024-08-02

### Added

- Add SBOM (SPDX and CycloneDX JSON using `cargo-sbom`) for Releases. See `sbom.tar.xz` in Releases.

### Changed

- Update dependencies.

## 0.1.6 - 2024-07-30

### Fixed

- Publish `sylvia-iot-coremgr-cli` for building `sylvia-router`.

## 0.1.5 - 2024-07-30

### Changed

- Use release branches for tags to release crates instead of using `--allow-dirty`.
- Update dependencies.

## 0.1.4 - 2024-07-26

### Changed

- Update Rust to 1.79.0 with GitHub Actions runner image 20240721.1.1.
- Update dependencies.

### Fixed

- **broker**: Fix all device APIs by modify `profile` and `networkAddr` relative parameters to lowercase and case insensitive.
- **corelib**: Fix user account E-mail address validation.

## 0.1.3 - 2024-05-27

### Changed

- Update dependencies.

### Fixed

- **router**: Fix LAN leases start/end time.

## 0.1.2 - 2024-05-24

### Changed

- Update dependencies.

### Fixed

- **router**: Fix LAN leases API.

## 0.1.1 - 2024-05-18

### Changed

- Use `frolvlad/alpine-glibc:alpine-3.18_glibc-2.34` to solve segmentation fault from v0.0.36.
    - **Note**: This image has some CVE medium level issues and we will update when the segmentation fault is resolved.

### Fixed

- **coremgr**: Fix API bridge `Content-Length` header.
- **router**: Fix wireless LAN API.

## 0.1.0 - 2024-05-17

### Added

- **broker**: Add `code` query parameter for application/network `GET /count` and `GET /list` APIs.

### Changed

- Update Rust to 1.78.0 with GitHub Actions runner image 20240516.1.1.
- Replace Actix-Web with axum.
- Update CI tools.
- Update dependencies.
- Release using Ubuntu 22.04.

### Fixed

- Fix a bug that it cannot detect client disconnect when using `GET /list` APIs for long response.
    - This is solved by using axum.
- **general-mq**: Fix return `Err` with additional `Send + Sync`.
- **auth**: Remove tokens when modifying user passwords or client secrets.
- **auth**: Only remove the request access token when using `POST /logout`.
- **coremgr**: Check duplicate application/network before update (AMQP/MQTT) brokers' resources.

## 0.0.36 - 2024-02-16

### Changed

- Update Rust to 1.76.0 with GitHub Actions runner image 20240212.2.1.
- Update Docker images with alpine-3.19.1.
- Update dependencies.

## 0.0.35 - 2023-12-15

### Changed

- Update Rust to 1.74.1 with GitHub Actions runner image 20231204.1.1.
- Update Docker images with alpine-3.18.5.
- Update dependencies.

## 0.0.34 - 2023-10-20

### Changed

- Update Rust to 1.73.0 with GitHub Actions runner image 20231016.1.1.
- Update dependencies.

## 0.0.33 - 2023-09-08

### Changed

- Update Rust to 1.72.0 with GitHub Actions runner image 20230903.1.0.
- Update dependencies and fix vulnerabilities.

## 0.0.32 - 2023-09-01

### Changed

- Update dependencies and fix most of vulnerabilities.
- Add Node.js SDK GitHub repository in documentation.

## 0.0.31 - 2023-08-26

### Changed

- Update dependencies.

### Fixed

- Support listening both v4 and v6 addresses by binding `::` only instead of both `0.0.0.0` and `::`.

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
