# Architecture

![Architecture](arch.svg)

Here is the diagram of Sylvia-IoT components. In this chapter, we will explain each one in detail.

## Sylvia-IoT Core Components

Abbreviated as ABCD (laughs &#x1F60A;)

### Auth (sylvia-iot-auth)

- Purpose
    - Provides the validity and information of access tokens for HTTP APIs, allowing APIs to
      determine whether to authorize access with the token.
    - Offers the authorization mechanism for OAuth2, currently supporting the following flows:
        - Authorization code grant flow
            - Clients need to use a webview to display login and authorization pages.
            - Currently used by coremgr CLI.
        - Client credentials grant flow
            - Currently reserved and not actively used.
- Managed entities
    - User accounts
        - User's basic information.
        - Permissions (roles).
    - Clients
        - Access permissions (scopes) for HTTP APIs.
- Dependencies
    - None. It can operate independently.

### Broker (sylvia-iot-broker)

- Purpose
    - Manages entities related to devices.
    - Binds devices and applications, forwards device data to applications, or receives data from
      applications to devices.
    - (Optional) Can send all traffic passing through networks and application data via the data
      channel to the **Data service** for storage or analysis.
- Managed entities
    - Units
        - Composed of one owner and multiple members.
        - Independently manage devices, applications, networks, and route (binding) rules.
    - Applications
        - Analyze data and present results based on device data.
    - Networks
        - Can use services directly connected to Sylvia-IoT or connect existing network services
          (e.g., [**The Things Network (TTN)**](https://www.thethingsnetwork.org/) or
          [**ChirpStack**](https://www.chirpstack.io/)) to Sylvia-IoT using adapters.
        - One network address can be used to transmit data from one device.
        - Administrators (admin role) can create public networks.
    - Devices
        - Each device represents an application on an endpoint, such as a tracker, meter, sensor,
          etc.
        - Devices need to be attached to a network address under a network to transmit data.
            - Devices can be attached to public networks, but it requires administrator accounts
              (admin/manager roles) to set up.
            - Each device has a unique identifier (device ID). If the application relies on this
              identifier, even if the network and address are changed, there is no need to change
              the application's management.
        - Each device can be assigned a device profile based on the data content.
            - With the profile, applications can quickly parse data without the need to create a
              mapping table for identifiers.
    - Route rules
        - Binds devices to applications.
            - Many-to-many relationships are supported.
        - Binds networks to applications, and all devices under that network will be routed, meaning
          there is no need to bind them one by one.
            - Many-to-many relationships are supported.
            - Public networks cannot be bound.
- Dependencies
    - Depends on Auth service.

### Coremgr (sylvia-iot-coremgr)

- Purpose
    - Coremgr, short for Core Manager, is responsible for managing the core components of
      Sylvia-IoT.
    - Provides the main HTTP APIs for external direct access.
        - The Auth service only exposes authentication/authorization APIs. User and client
          management still requires the use of coremgr API.
        - Uses bridging to indirectly access Auth and Broker HTTP APIs to manage various entities.
    - Creates queues and corresponding permissions using the management API for RabbitMQ/EMQX, and
      other message brokers.
        - Broker only manages associations between entities and AMQP/MQTT connections. The actual
          configuration of RabbitMQ/EMQX is performed by coremgr.
    - (Optional) Sends operation records, including additions, modifications, deletions, etc.,
      through the data channel to the **Data service** for storage or analysis.
- Managed entities
    - (None)
- Dependencies
    - Depends on Auth and Broker services.
    - Depends on the management API of the message broker.

### Coremgr CLI (sylvia-iot-coremgr-cli)

- Purpose
    - Provides a command-line interface (CLI) for users to configure Sylvia-IoT using commands.
- Dependencies
    - Depends on Auth and Coremgr services. Auth is only used for authentication/authorization.
    - Can depend on the Data service to read historical data.

### Control Channel vs. Data Channel

- The control channel is used to transmit messages related to entity management (users, units,
  devices, etc.). It can be categorized as follows:
    - Unicast: Each message has only one consumer and is used for Sylvia-IoT to push messages to
      networks or applications, which will be explained in the later [**Data Flow**](./flow.md)
      section.
    - Broadcast: Used to broadcast messages to various core processes within the Sylvia-IoT cluster,
      which will be explained in the later [**Cache**](./cache.md) section.
- The data channel is used to transmit device data or historical data.
    - It covers Application Data, Network Data, and Coremgr OP Data.
    - Currently, AMQP 0-9-1 and MQTT 3.1.1 protocols are implemented. Additionally, AMQP 1.0, Kafka,
      NATS, and other protocols can also be implemented.

#### general-mq

Sylvia-IoT utilizes [**general-mq**](https://crates.io/crates/general-mq) to implement unicast and
broadcast, abstracting the details of communication protocols.

By implementing unicast/broadcast modes for AMQP 1.0, Kafka, or other protocols in general-mq and
corresponding management APIs in coremgr, Sylvia-IoT can support more protocols.

### Data (sylvia-iot-data)

- Purpose
    - Record or analyze data from the data channel.

This module is unique in that it does not have a specific implementation. Currently,
**sylvia-iot-data** in Sylvia-IoT Core provides storage and retrieval of raw data.

Below are some possible scenarios for extension:

- Rule engine.
    - Since the data channel contains all network data, the Data module can be implemented as a
      common rule engine in IoT platforms.
- Stream processing.
    - The data channel can be implemented as a Kafka queue for stream processing.

## Message Brokers

> Here, message brokers refer to services like RabbitMQ and EMQX, not Sylvia-IoT Broker.
> Unless specifically mentioned, "Broker" in this document refers to Sylvia-IoT Broker.

Some important points:

- Since coremgr needs to configure queues through the management APIs, relevant implementations must
  be provided to support this feature. Currently, coremgr supports the following message brokers:
    - RabbitMQ
    - EMQX
    - In the future, Kafka or other protocols can be implemented to broaden the application scope of
      Sylvia-IoT.
- Sylvia-IoT has the following requirements:
    - Message queuing, which refers to the traditional message pattern (one message has only one
      consumer).
        - MQTT is implemented using shared subscription.
    - Publish/Subscribe, used for broadcasting control channel messages. This will be covered in the
      [**Cache**](./cache.md) section.
        - AMQP is implemented using fanout exchanges and temporary queues.

### rumqttd

In the [**Quick Start**](../guide/quick.md) section, we used **sylvia-iot-core** as an example. This
executable includes the complete Auth/Broker/Coremgr/Data and rumqttd.

To make it possible to run in resource-constrained environments, **sylvia-iot-core** contains the
[**rumqttd**](https://github.com/bytebeamio/rumqtt) MQTT broker.
By configuring it to use SQLite as the database and MQTT for message delivery, the sylvia-iot-core
achieves the full functionality of Sylvia-IoT in just two files.

> The "core" is an executable that contains all the complete functionalities, whereas "coremgr" only
  contains management functionalities and does not include rumqttd.

To accommodate this limited environment, Sylvia-IoT adopts rumqttd.
Currently, Sylvia-IoT does not have an implementation of rumqttd management APIs, so it is not
suitable for use in a cluster architecture. It is also not recommended to use this mode for queue
permission requirements.

## Third-Party Components

### Application Servers, Network Servers

In addition to using the data channel to send and receive device data, applications and networks can
also access Sylvia-IoT HTTP APIs and control channel messages to build their own management systems.

### Devices

Devices in Sylvia-IoT refer to narrow-definition terminal devices that only process the data
required by applications, and they are generally bound to network modules. The network module,
however, can be interchangeable.

Here's an example of replacing the network module: Suppose the device uses a Raspberry Pi to connect
sensors for specific application development. The network part can be changed to different protocols
at any time (e.g., switching from LoRa to WiFi or even using an Ethernet cable). In Sylvia-IoT, you
only need to modify the corresponding network and address settings for the device.
