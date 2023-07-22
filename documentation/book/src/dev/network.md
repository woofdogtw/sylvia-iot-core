# Network Services

This chapter provides a brief overview of key points in developing network services, including:

- Data Channel
- Control Channel
- Using the [**SDK**](https://crates.io/crates/sylvia-iot-sdk) to connect channels in Rust

Before starting this chapter, please make sure you have read and understood the
[**Data Flow**](../arch/flow.md) section, and are familiar with the generation and consumption
timing of queues and related data.

## Queues and Data Formats

- [**This document**](https://github.com/woofdogtw/sylvia-iot-core/blob/main/sylvia-iot-broker/doc/message.md#between-broker-and-network)
  defines the data content of the queues between the Broker and network services.
- Both data and control channels use unicast mode.
    - AMQP properties:
        - durable: true
        - exclusive: false
        - auto-delete: false
        - ttl: determined when generating the network
        - max-length: determined when generating the network
    - MQTT properties:
        - QoS: 1 at the Broker side
        - clean session: true at the Broker side
- In the [**Data Flow**](../arch/flow.md) section, it is mentioned that network services need to
  retain `dataId` while processing downlink data for subsequent result reporting.
    - For unreported downlink data, it has no impact on the Broker.
        - Currently retained for one day. If not reported, it will always be marked as "unreported".
    - The application service can decide how to handle downlink data that has not been reported for
      too long.
- Rules regarding `result`:
    - Less than 0 indicates the ongoing process.
        - -2: Indicates that the data is being sent to the network service. Set by the Broker before
          storing in the database.
        - -1: Indicates that the network service has received it. Must be reported back as -1 via
          the `result` queue by the network service.
    - 0 or positive values indicate the completion of processing. At this point, it will be removed
      from the dldata database, and any further reports cannot be sent back to the application side.
        - All reported by the network service.
        - 0: Successfully sent to the device or the device responded successfully.
        - Positive values: Unable to send to the device or the device responded with an error.

    > As the results are currently defined by the network service, the application side still needs
      to know which network the device is currently bound to. It is recommended to follow the above
      rules when developing network services to make the presentation on the application side more
      consistent.

## Rust and Using the SDK

For Rust developers, there is an SDK available to assist in developing network services. Usage
examples can be found in the [**Appendix**](../appendex/repo.md) chapter. Here are a few tips on how
to use it:

- Channel maintenance is handled in the `mq` module's `NetworkMgr`.
- One `NetworkMgr` corresponds to one network service.
- Only manage `NetworkMgr`; no need to manually manage the connection status of all queues and
  AMQP/MQTT properties.
- Register an `EventHandler` to receive real-time updates when the queue status changes or data is
  delivered.
- You can use `send_uldata()` and `send_dldata_result()` to send data to the Broker.
