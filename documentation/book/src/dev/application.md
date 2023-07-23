# Application Services

This chapter provides a brief overview of key points in developing network services, including:

- Data Channel
- Using the [**SDK**](https://crates.io/crates/sylvia-iot-sdk) to connect channels in Rust

Before starting this chapter, please make sure you have read and understood the
[**Data Flow**](../arch/flow.md) section, and are familiar with the generation and consumption
timing of queues and related data.

## Queues and Data Formats

- [**This document**](https://github.com/woofdogtw/sylvia-iot-core/blob/main/sylvia-iot-broker/doc/message.md#between-broker-and-application) 
  defines the data content of the queues between the Broker and application services.
- Data channels use unicast mode.
    - AMQP properties:
        - durable: true
        - exclusive: false
        - auto-delete: false
        - ttl: determined when generating the application
        - max-length: determined when generating the application
    - MQTT properties:
        - QoS: 1 at the Broker side
        - clean session: true at the Broker side
- In the [**Data Flow**](../arch/flow.md) chapter, it is mentioned that when downlink data is sent
  to the Broker through the `dldata` queue, the Broker will immediately report the result.
    - The `correlationId` is recommended to be unique. If the application service simultaneously
      sends a large amount of downlink data, this correlation ID will be used to track whether each
      transmission has been correctly sent to the network service.
    - If the data is successfully processed, the `dataId` will be returned. The application service
      can use this data ID to track the processing status of this downlink data in the network
      service.
- In the downlink data, you can choose to specify the destination device using either the `deviceId`
  or the combination of `networkCode` and `networkAddr`.
    - If the device is on the **public network**, you must use the `deviceId`. Sylvia-IoT adopts
      this approach to prevent application services from sending data arbitrarily to devices that do
      not belong to their own unit.
- Currently, the control channel is not supported. Changes to devices must rely on application
  services to request the Sylvia-IoT HTTP APIs or manage the list of devices themselves.

## Rust and Using the SDK

For Rust developers, there is an SDK available to assist in developing application services. Usage
examples can be found in the [**Appendix**](../appendex/repo.md) chapter. Here are a few tips on how
to use it:

- Channel maintenance is handled in the `mq` module's `ApplicationMgr`.
- One `ApplicationMgr` corresponds to one application service.
- Only manage `ApplicationMgr`; no need to manually manage the connection status of all queues and
  AMQP/MQTT properties.
- Register an `EventHandler` to receive real-time updates when the queue status changes or data is
  delivered.
- You can use `send_dldata()` to send data to the Broker.
