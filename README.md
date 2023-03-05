![GitHub Actions](https://github.com/woofdogtw/sylvia-iot-core/actions/workflows/build-test.yaml/badge.svg)

# sylvia-iot-core

This contains core modules of Sylvia-IoT platform.

# Milestone

## v0.1.0 (end of 2023)

To release v0.1.0, there are some tasks to do:

- [x] (Q1) build CI/CD with GitHub Actions for GitHub Releases, Docker Hub, and crates.io.
- [ ] (Q2) establish K8S environment to run v0.0.x Sylvia-IoT core.
- [ ] (Q3) **general-mq**: fix memory leak when processing received messages.
- [ ] (Q3) **general-mq**: refine AMQP `connect()` for integration test.
- [ ] (Q3) **broker**: provide control channel to publish events when changing devices for network (adapters) and application (adapters).
- [ ] (Q3) **sdk**: add a module for operating control channels.
- [ ] (Q3) **sdk**: add HTTP API functions for **Service** clients (of networks/applications) to refresh tokens automatically.
- [ ] (Q4) refine documents.
- [ ] (Q4) write Gitbook to introduce Sylvia-IoT platform.

## v0.2.0

- [ ] **general-mq**: use **mpsc** for AMQP/MQTT connection status handling.
- [ ] **corelib**: new logger middleware to replace `actix_web::middleware::Logger`.
- [ ] **broker**: messages with Protobuf.
