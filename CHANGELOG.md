# Changelog

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
