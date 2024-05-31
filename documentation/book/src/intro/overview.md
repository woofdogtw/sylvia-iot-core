# What is Sylvia-IoT?

**Sylvia-IoT** is an IoT (Internet of Things) platform primarily designed to forward device messages
to applications or enable applications to send commands to devices.

![Introduction](intro.svg)

The diagram above provides a simple explanation. Devices (such as sensors) are bound to specific
communication modules and transmit data through network gateways or servers. Sylvia-IoT acts as a
message broker, allowing each application to subscribe to devices they are interested in for data
analysis or data transmission to the devices.

## Features

Using the Sylvia-IoT platform provides several benefits to different providers:

- Device Providers:
    - Easier module changes from network providers without altering applications.
- Network Providers:
    - Focus on developing network communication protocols for device usage.
    - Develop adapters to connect with the Sylvia-IoT platform.
- Application Providers:
    - Specify any number of applications to receive data from the same device.
    - Through Sylvia-IoT's communication protocol isolation, devices' network providers can be
      changed without rewriting code.
