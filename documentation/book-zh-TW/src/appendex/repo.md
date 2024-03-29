# 輔助專案

- [sylvia-router](https://github.com/woofdogtw/sylvia-iot-core/tree/main/sylvia-router)
    - 整合 auth/broker/coremgr/data 的基本路由器。
    - 支援多 WAN interface 和單一 LAN bridge。
    - （可選）支援 WiFi WAN 和 WiFi LAN。
- [stress-simple](https://github.com/woofdogtw/sylvia-iot-core/tree/main/stress-simple)
    - 簡易的壓力程式，可以測試 Broker 的轉發速度。
    - 提供最大、最小、平均值、P50/P80/P90/P95/P98/P99 的延遲數據。
- [sylvia-iot-examples](https://github.com/woofdogtw/sylvia-iot-examples)
    - 使用 SDK 實作的應用和網路範例。
    - **lora-ifroglab**
        - [iFrogLab LoRa USB Dongle](http://www.ifroglab.com/en/?p=6536)
        - 實作對應的網路服務並直接和裝置端對送。
    - **app-demo**: 接收 lora-ifroglab 裝置的感應器資料並顯示。有溫度、濕度、RSSI 等。
- [sylvia-iot-simple-ui](https://github.com/woofdogtw/sylvia-iot-simple-ui)
    - 提供簡易的 Sylvia-IoT UI。
    - **coremgr-cli** 提供完整的功能，UI 依據畫面排版提供必要的操作功能。
    - 除了 auth/broker/coremgr/data，還整合了 router 和 examples。
- [sylvia-iot-go](https://github.com/woofdogtw/sylvia-iot-go)
    - Go 實作的元件。
    - 含有 **general-mq**、**sdk** 等。
- [sylvia-iot-node](https://github.com/woofdogtw/sylvia-iot-node)
    - Node.js 實作的元件。
    - 含有 **general-mq**、**sdk** 等。
- [sylvia-iot-deployment](https://github.com/woofdogtw/sylvia-iot-deployment)
    - 提供部署的方案，如 K8S 等。
