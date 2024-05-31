![CI](https://github.com/woofdogtw/sylvia-iot-core/actions/workflows/build-test.yaml/badge.svg)
[![Docker](https://img.shields.io/docker/v/woofdogtw/sylvia-iot-core?label=docker&logo=docker)](https://hub.docker.com/r/woofdogtw/sylvia-iot-core)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

# Sylvia-IoT: An Open Source IoT Platform

**Sylvia-IoT** is an open source IoT (Internet of Things) platform primarily designed to forward device messages
to applications or enable applications to send commands to devices.

![Introduction](documentation/book/src/intro/intro.svg)

This repository contains the following modules:

- The core modules:
    - [Auth](sylvia-iot-auth)
    - [Broker](sylvia-iot-broker)
    - [Coremgr](sylvia-iot-coremgr) with a [CLI](sylvia-iot-coremgr-cli) tool.
    - [Data](sylvia-iot-data)
- [SDK](sylvia-iot-sdk)
- A [router](sylvia-router) for demonstration.
- A [simple stress test](stress-simple) tool.

Please refer to the documentation for more detailed information.

# Documentation

- [Documentation](https://woofdogtw.github.io/sylvia-iot-core)
    - [中文版](https://woofdogtw.github.io/sylvia-iot-core/book-zh-TW)
