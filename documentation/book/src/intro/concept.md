# Concept

Sylvia-IoT provides HTTP APIs to manage the following entities:

- User Account:
    - Access to Sylvia-IoT's management interface is possible through user accounts.
    - Clients can obtain access tokens to access HTTP APIs.
- Client:
    - Represents entities that access HTTP APIs.
    - Third parties can develop management features for Sylvia-IoT through HTTP APIs.
    - Users authorize clients to access resources using OAuth2.
- Unit:
    - Each unit can have an owner and multiple members.
    - Units can manage their own devices, networks, and applications.
- Device:
    - Represents IoT terminal devices, such as sensors, trackers, and more.
- Application:
    - Analyzes device data and presents it based on requirements, such as a smart home control
      center.
- Network:
    - Connects different network servers to receive and send device data based on communication
      requirements.
    - Common communication protocols include LoRa, WiFi, and TCP/IP.
    - Network adapters can be developed to integrate existing network servers (e.g., TTN,
      ChirpStack) with Sylvia-IoT.
- Routing Rules:
    - Associate devices with applications.
    - Individual devices can be bound using network addresses or entire networks can be bound to
      specific applications.
    - Supports many-to-many relationships, allowing multiple devices to be bound to one application
      or vice versa.

## Communication Protocols

Currently, Sylvia-IoT supports the following protocols for message transmission between applications
and networks:

- AMQP 0-9-1
- MQTT 3.1.1

Any message queuing model with explicit names (excluding wildcards) can be supported, such as AMQP
1.0, Apache Kafka, NATS, etc. However, topic publish/subscribe, broadcast, and multicast modes are
currently not supported.
