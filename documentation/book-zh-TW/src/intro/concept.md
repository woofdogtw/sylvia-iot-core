# 概念

Sylvia-IoT 提供了 HTTP API 管理以下實體：

- 使用者帳號（User Account）
    - 透過使用者帳號可以存取 Sylvia-IoT 的管理介面。
    - 可以透過客戶端取得訪問令牌（access token）存取 HTTP API。
- 客戶端應用程式（Client）
    - 存取 HTTP API 的實體。
    - 第三方可以透過 HTTP API 開發 Sylvia-IoT 的管理功能。
    - 透過 OAuth2 讓使用者授權客戶端存取資源。
- 單位（Unit）
    - 每個單位可以指派一個擁有者和多個成員。
    - 每個單位可以管理自己的裝置、網路、應用。
- 裝置（Device）
    - 如感應器、追蹤器等物聯網的終端裝置。
- 應用（Application）
    - 依據需求，分析裝置的資料並呈現，比如智慧家電控制中心。
- 網路（Network）
    - 依裝置的通訊需求，連接不同的網路伺服器來收送裝置資料。
    - 常見的通訊協定有 LoRa、WiFi 等。也可以直接使用 TCP/IP。
    - 可以開發網路連接器（network adapter），將既有的網路伺服器（network server，如 TTN、ChirpStack）和 Sylvia-IoT 整合。
- 路由規則（Routing Rules）
    - 裝置和應用的關聯。
    - 可以將個別裝置透過網路位址（network address）綁定、或是將整個網路綁定到特定應用。
    - 多對多的關係。也就是多個裝置可以綁定一個應用，也可以一個裝置綁定多個應用。

## 傳輸協定

目前 Sylvia-IoT 支援以下協定，和應用與網路進行訊息傳輸：

- AMQP 0-9-1
- MQTT 3.1.1

只要能符合明確名稱（不含萬用字元）的消息佇列模式（message queuing model）都可以支援。比如 AMQP 1.0、Apache Kafka、NATS 等。
目前還不支援主題式發布訂閱（topic publish/subscribe）或廣播（broadcast）、多播（multicast）的模式。
