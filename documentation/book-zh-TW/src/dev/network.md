# 網路服務

本章節簡述開發網路服務的幾個要點，包含：

- 資料通道 (Data Channel)
- 控制通道 (Control Channel)
- Rust 使用 [**SDK**](https://crates.io/crates/sylvia-iot-sdk) 連接通道

在開始本章之前，請先確保已經研讀過 [**資料流**](../arch/flow.md) 章節並了解佇列和相關資料的產生與消費時機。

## 佇列與資料格式

- [**這份文件**](https://github.com/woofdogtw/sylvia-iot-core/blob/main/sylvia-iot-broker/doc/message.md#between-broker-and-network) 定義了 Broker 與網路服務佇列的資料內容。
- 無論資料通道或控制通道，都是使用單播模式（unicast）。特性如下：
    - AMQP 屬性：
        - durable: true
        - exclusive: false
        - auto-delete: false
        - ttl: 產生 network 時決定
        - max-length: 產生 network 時決定
    - MQTT 屬性：
        - QoS: Broker 端為 1
        - clean session: Broker 端為 true
- 在 [**資料流**](../arch/flow.md) 章節有提到，網路服務在處理下行資料時需要保留 `dataId`，以便後續回報結果。
    - 對於未回報的下行資料，對 Broker 沒有影響。
        - 目前只保留一天。如沒有回報則永遠呈現未回報狀態。
    - 應用服務可以自行決定太久沒有回報的下行資料該如何處理。
- 關於 `result` 的規則：
    - 小於 0 表示進行中。
        - -2: 表示資料正在往網路服務傳送中。由 Broker 在儲存到資料庫前設定。
        - -1: 表示網路服務已經接收到。須由網路服務透過 `result` 佇列回報 -1。
    - 0 或是正數表示處理完成。此時會從 dldata 資料庫中移除，之後的任何回報都無法再傳回應用端。
        - 都由網路服務回報。
        - 0: 成功發送到裝置端，或是裝置端回覆了成功。
        - 正數: 無法發送到裝置端，或是裝置端回覆了錯誤。

    > 由於目前是由網路服務定義結果，應用端仍需知道該裝置目前綁定到哪個網路。建議開發網路服務時，跟著以上的規則，可以讓應用端的呈現更加一致化。

## Rust 使用 SDK

針對 Rust 開發人員，目前提供了 SDK 協助開發人員開發網路服務，在 [**附錄**](../appendex/repo.md) 章節也有提供使用範例。這裡介紹幾個使用的技巧：

- 通道的維護都寫在 `mq` 模組中的 `NetworkMgr` 中。
- 一個 `NetworkMgr` 對應一個網路服務。
- 只要管理 `NetworkMgr` 即可，無需自行管理所有佇列的連線狀態和 AMQP/MQTT 的屬性。
- 註冊 `EventHandler` 可即時於佇列狀態改變或是資料送達時收到。
- 可以透過 `send_uldata()` 和 `send_dldata_result()` 傳送資料給 Broker。
