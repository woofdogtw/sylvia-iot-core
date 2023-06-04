![CI](https://github.com/woofdogtw/sylvia-iot-core/actions/workflows/build-test.yaml/badge.svg)
[![Docker](https://img.shields.io/docker/v/woofdogtw/sylvia-iot-core?label=docker&logo=docker)](https://hub.docker.com/r/woofdogtw/sylvia-iot-core)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

# sylvia-iot-core

This contains core modules of Sylvia-IoT platform.

# Milestone

## v0.1.0 (end of 2023)

To release v0.1.0, there are some tasks to do:

- [x] (Q1) build CI/CD with GitHub Actions for GitHub Releases, Docker Hub, and crates.io.
- [X] (Q2) establish K8S environment to run v0.0.x Sylvia-IoT core.
- [X] (Q3) **general-mq**: fix memory leak when processing received messages.
- [ ] (Q3) **general-mq**: refine AMQP `connect()` for integration test.
- [X] (Q3) **broker**: provide control channel to publish events when changing devices for network (adapters) and application (adapters).
- [X] (Q3) **sdk**: add a module for operating control channels.
- [X] (Q3) **sdk**: add HTTP API functions for **Service** clients (of networks/applications) to refresh tokens automatically.
- [ ] (Q4) refine documents.
- [ ] (Q4) write mdBook to introduce Sylvia-IoT platform.

## v0.2.0

- [ ] **general-mq**: use **mpsc** for AMQP/MQTT connection status handling.
- [ ] **corelib**: new logger middleware to replace `actix_web::middleware::Logger`.
- [ ] **broker**: messages with Protobuf.
