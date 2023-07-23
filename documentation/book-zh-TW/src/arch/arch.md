# 架構

![Architecture](arch.svg)

上面是 Sylvia-IoT 元件的示意圖。這章節我們將逐一解釋。

## Sylvia-IoT 核心元件

簡稱 ABCD（笑 &#x1F60A;）

### Auth (sylvia-iot-auth)

- 用途
    - 為 HTTP API 提供令牌（access token）的合法性和資訊，讓 API 可以決定是否授權此令牌存取。
    - 提供 OAuth2 的授權機制，目前支援下列流程：
        - Authorization code grant flow
            - 客戶端需使用 webview 顯示登入和授權頁面。
            - 目前 coremgr CLI 使用此流程。
        - Client credentials grant flow
            - 目前保留此功能。
- 管理的實體
    - 使用者帳號
        - 使用者基本資料。
        - 權限（role）。
    - 客戶端
        - HTTP API 的存取權限（scope）。
- 相依性
    - 無。可獨立運作。

### Broker (sylvia-iot-broker)

- 用途
    - 管理和裝置相關的實體。
    - 綁定裝置和應用、轉發裝置資料給應用，或是接收應用發出的資料給裝置。
    - （可選）將流經的網路、應用資料全部透過資料通道（data channel）送給 **Data 服務** 儲存或分析。
- 管理的實體
    - 單位
        - 由一個擁有者和多個成員組成。
        - 可以獨立管理裝置、應用、網路、路由（綁定）規則。
    - 應用
        - 解析裝置資料並依據需求分析數據和呈現結果。
    - 網路
        - 可以使用直接與 Sylvia-IoT 介面連接的服務，也可以使用連接器（adapter）將目前既有的網路服務（如 [**The Things Network (TTN)**](https://www.thethingsnetwork.org/) 或是 [**ChirpStack**](https://www.chirpstack.io/) 等）連接到 Slvia-IoT。
        - 一個網路位址可以用來傳輸一個裝置的資料。
        - 管理員（admin role）可以建立公用的網路。
    - 裝置
        - 每一個裝置表示一種終端的應用，比如追蹤器、電錶、感應器等。
        - 裝置需依附於一個網路下的網路位址才能傳輸資料。
            - 可以將裝置依附於公用網路中，但需要透過管理員帳號（admin/manager roles）才能設定。
            - 每一個裝置有唯一的識別碼（device ID），應用如果依靠的是此識別碼，即使變更網路和位址，都可以無需變更應用的管理。
        - 每一個裝置可以依據資料內容，給定一個裝置設定檔（profile）。
            - 透過 profile，應用就可以快速解析資料而無需建立識別碼的對應表了。
    - 路由規則
        - 將裝置綁定到應用。
            - 可以多對多。
        - 將網路綁定到應用，所有該網路下的裝置都會被路由，亦即無須一台一台綁定。
            - 可以多對多。
            - 不可以綁定公用網路。
- 相依性
    - 需依賴 Auth 服務

### Coremgr (sylvia-iot-coremgr)

- 用途
    - coremgr 即 core manager，負責管理 Sylvia-IoT 核心的元件。
    - 提供主要的 HTTP API 給外部直接存取。
        - Auth 僅開放認證／授權 API。使用者和客戶端管理仍需使用 coremgr API。
        - 透過橋接的方式間接使用 Auth、Broker HTTP API 來管理各種實體。
    - 透過 management API 設定 RabbitMQ/EMQX 等訊息代理來建立佇列和對應的權限。
        - Broker 只負責管理實體間的關聯和 AMQP/MQTT 的連線，實際的 RabbitMQ/EMQX 的設定是靠 coremgr 進行設定。
    - （可選）將操作紀錄，包含新增、修改、刪除等，透過資料通道（data channel）送給 **Data 服務** 儲存或分析。
- 管理的實體
    - （無）
- 相依性
    - 需依賴 Auth、Broker 服務。
    - 需依賴訊息代理的 management API。

### Coremgr CLI (sylvia-iot-coremgr-cli)

- 用途
    - 提供命令列介面（CLI）給使用者透過指令來設定 Sylvia-IoT。
- 相依性
    - 需依賴 Auth、Coremgr 服務。Auth 僅使用認證／授權部分。
    - 可以相依 Data 服務來讀取歷史資料。

### 控制通道 (Control Channal) vs. 資料通道 (Data Channel)

- 控制通道用來傳輸實體（使用者、單位、裝置等）管理的訊息。一般分為：
    - Unicast（單播），一個訊息只有一個消費者，用於 Sylvia-IoT 推送訊息給網路或應用的時候。這個會在稍後的 [**資料流**](./flow.md) 章節說明。
    - Broadcast（廣播），用來廣播給 Sylvia-IoT 叢集中的各個核心程序。這個會在稍後的 [**快取**](./cache.md) 章節說明。
- 資料通道用來傳輸裝置資料或歷史資料。
    - 泛指 Application Data、Network Data、Coremgr OP Data。
    - 目前實作了 AMQP 0-9-1、MQTT 3.1.1 協議。也可以實作 AMQP 1.0、Kafka、NATS 等。

#### general-mq

Sylvia-IoT 使用 [**general-mq**](https://crates.io/crates/general-mq) 實現 unicast 和 broadcast，並隱藏了通訊協議的細節。

只要在 general-mq 實作 AMQP 1.0、Kafka 等協議的 unicast/broadcast 模式，並在 coremgr 實作對應的 management API，即可讓 Sylvia-IoT 支援更多協議。

### Data (sylvia-iot-data)

- 用途
    - 紀錄或分析資料通道中的資料。

這個模組比較特別的地方在於沒有特定的實作。目前 Sylvia-IoT Core 的 **sylvia-iot-data** 提供了原始資料的儲存和讀取。

以下列出可以延伸的場景：

- 規則引擎（rule engine）。
    - 由於資料通道含有所有網路資料，可以將 Data 模組實作為一般 IoT 平台常見的規則引擎。
- 流處理（stream processing）。
    - 可以將資料通道實作成 Kafka 佇列，進行流處理。

## 訊息代理服務 (Message Brokers)

> 這裡的訊息代理指的是 RabbitMQ、EMQX 等服務，不是 Sylvia-IoT Broker。
> 除非特別強調 RabbitMQ、EMQX 等，本文件的「Broker」泛指 Sylvia-IoT Broker。

幾個注意事項：

- 由於 coremgr 需要透過 management API 設定佇列，必須提供相關的實作才能支援。目前 coremgr 支援的訊息代理服務如下：
    - RabbitMQ
    - EMQX
    - 未來可以實作 Kafka 或其他協議，讓 Sylvia-IoT 的應用可以更廣泛。
- Sylvia-IoT 的需求有以下模式：
    - Message queuing，即傳統的訊息模式（一個訊息只有一個消費者）。
        - MQTT 透過 shared subscription 實作。
    - Publish/Subscribe（推播／訂閱），用作控制通道（control channel）的廣播訊息。在 [**快取**](./cache.md) 的章節會介紹。
        - AMQP 透過 fanout exchage 和 temorary queue 實作。

### rumqttd

在 [**快速開始**](../guide/quick.md) 章節中，我們使用了 **sylvia-iot-core** 作為範例。這個可執行檔本身包含了完整的 Auth/Broker/Coremgr/Data，以及 rumqttd。

為了在小容量的環境也可以執行，**sylvia-iot-core** 內含了 [**rumqttd**](https://github.com/bytebeamio/rumqtt) MQTT broker。
只要配合設定，就可以用 SQLite 作為資料庫，搭配 MQTT 傳遞訊息，以兩個檔案的規模，實現完整的 Sylvia-IoT 的功能。

> core 是集結完整功能的可執行檔。而 coremgr 只有管理部分，且不含 rumqttd。

為了因應這個受限的環境，Sylvia-IoT 才採用了 rumqttd。
Sylvia-IoT 目前沒有實作 rumqttd management API，請勿使用於叢集架構（cluster）。有佇列權限需求者也不建議使用這模式。

## 第三方元件 (3rd Party Components)

### Application Servers, Network Servers

應用和網路除了使用資料通道的訊息收送裝置資料，也可以透過客戶端存取 Sylvia-IoT HTTP API 和控制通道的訊息來打造自己的管理系統。

### Devices

裝置一般都和網路模組綁定；而 Sylvia-IoT 的「裝置」是指狹義的終端裝置，只處理應用所需要的資料。至於網路模組的部分是可以抽換的。

舉個抽換網路模組的例子，假如裝置是使用樹莓派連接感應器進行特定的應用開發，網路的部分可以用 USB 隨時變更成不同的協議（LoRa 換成 WiFi，甚至是接網路線）。
在 Sylvia-IoT 只需要修改裝置對應的網路和位址即可。
