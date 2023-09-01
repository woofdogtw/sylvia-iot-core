# Supplementary Projects

- [sylvia-router](https://github.com/woofdogtw/sylvia-iot-core/tree/main/sylvia-router)
    - A basic routers that integrates auth/broker/coremgr/data components.
    - Supports multiple WAN interfaces and a single LAN bridge.
    - (Optional) Supports WiFi WAN and WiFi LAN.
- [stress-simple](https://github.com/woofdogtw/sylvia-iot-core/tree/main/stress-simple)
    - A simple stress program for testing the forwarding speed of the Broker.
    - Provides latency data for maximum, minimum, average, and P50/P80/P90/P95/P98/P99.
- [sylvia-iot-examples](https://github.com/woofdogtw/sylvia-iot-examples)
    - Contains applications and network examples implemented using the SDK.
    - **lora-ifroglab**
        - [iFrogLab LoRa USB Dongle](http://www.ifroglab.com/en/?p=6536)
        - Implements corresponding network services and communicates directly with device endpoints.
    - **app-demo**: Receives sensor data from the lora-ifroglab devices and displays temperature,
      humidity, RSSI, etc.
- [sylvia-iot-simple-ui](https://github.com/woofdogtw/sylvia-iot-simple-ui)
    - Provides a simple Sylvia-IoT UI.
    - **coremgr-cli** provides complete functionality, and the UI provides necessary operational
      functions based on the screen layout.
    - In addition to auth/broker/coremgr/data, it also integrates router and examples.
- [sylvia-iot-go](https://github.com/woofdogtw/sylvia-iot-go)
    - Components implemented in Go.
    - Includes **general-mq**, **sdk**, etc.
- [sylvia-iot-node](https://github.com/woofdogtw/sylvia-iot-node)
    - Components implemented in Node.js.
    - Includes **general-mq**, **sdk**, etc.
- [sylvia-iot-deployment](https://github.com/woofdogtw/sylvia-iot-deployment)
    - Provides deployment solutions, such as K8S, and more.
