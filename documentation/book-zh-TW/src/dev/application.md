# 應用服務

本章節簡述開發應用服務的幾個要點，包含：

- 資料通道 (Data Channel)
- Rust 使用 [**SDK**](https://crates.io/crates/sylvia-iot-sdk) 連接通道

在開始本章之前，請先確保已經研讀過 [**資料流**](../arch/flow.md) 章節並了解佇列和相關資料的產生與消費時機。

## 佇列與資料格式

- [**這份文件**](https://github.com/woofdogtw/sylvia-iot-core/blob/main/sylvia-iot-broker/doc/message.md#between-broker-and-application) 定義了 Broker 與應用服務佇列的資料內容。
- 資料通道使用單播模式（unicast）。特性如下：
    - AMQP 屬性：
        - durable: true
        - exclusive: false
        - auto-delete: false
        - ttl: 產生 network 時決定
        - max-length: 產生 network 時決定
    - MQTT 屬性：
        - QoS: Broker 端為 1
        - clean session: Broker 端為 true
- 在 [**資料流**](../arch/flow.md) 章節有提到，下行資料透過 `dldata` 佇列傳送給 Broker 後，Broker 會立即回報結果。
    - `correlationId` 建議唯一。如果應用服務同時發送大量的下行資料，就要靠此 correlation ID 才能追蹤每一筆的傳輸是否有被正確送往網路服務。
    - 如果有成功被處理，將會回應 `dataId`。應用服務可以透過資料 ID 追蹤這筆下行資料在網路服務的處理情形。
- 下行資料中，可以選擇使用 `deviceId` 或是 `networkCode`+`networkAddr` 的方式指定目的裝置。
    - 如果裝置是在 **公用網路** 上，一定要使用 `deviceId`。Sylvia-IoT 用這方式避免應用服務任意傳輸資料到不屬於他們單位的裝置。
- 目前還沒有支援控制通道。裝置的變更就要依靠應用服務自行請求 Sylvia-IoT HTTP API，或是自己維護裝置的列表。

## Rust 使用 SDK

針對 Rust 開發人員，目前提供了 SDK 協助開發人員開發應用服務，在 [**附錄**](../appendex/repo.md) 章節也有提供使用範例。這裡介紹幾個使用的技巧：

- 通道的維護都寫在 `mq` 模組中的 `ApplicationMgr` 中。
- 一個 `ApplicationMgr` 對應一個應用服務。
- 只要管理 `ApplicationMgr` 即可，無需自行管理所有佇列的連線狀態和 AMQP/MQTT 的屬性。
- 註冊 `EventHandler` 可即時於佇列狀態改變或是資料送達時收到。
- 可以透過 `send_dldata()` 傳送資料給 Broker。
