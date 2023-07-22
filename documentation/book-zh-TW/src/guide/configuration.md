# 設定檔

本章節描述 Sylvia-IoT 的設定格式和用途。

Sylvia-IoT 的設定支援四種來源，優先權依序為（高到低）：

- JSON5 設定檔
- 命令列參數
- 環境變數
- 內部預設值（不一定存在，如為必填且不提供則會有錯誤訊息）

JSON5 可以參考範例檔案，該檔案提供了完整的設定項目。
本章節將會提供對應的說明。以下為設定的慣例：

- JSON5 的巢狀形式會以 `.` 來表示。
- 命令列參數遇到 JSON5 的巢狀會以 `.` 來表示。
- 命令列參數遇到 JSON5 為駝峰的情形，會以全小寫或是 `-` 尾隨小寫來表示。以下為例子：
    - JSON5 的 `server.httpPort` 對應 `--server.httpport`。
    - JSON5 的 `broker.mqChannels` 對應 `--broker.mq-channels`。
- 環境變數全大寫。
- 環境變數遇到 JSON5 的巢狀會以 `_` 來表示。
- 環境變數遇到 JSON5 為駝峰的情形，會以全大寫或是 `_` 分開來表示。以下為例子：
    - JSON5 的 `server.httpPort` 對應 `SERVER_HTTP_PORT`
    - JSON5 的 `broker.mqChannels` 對應 `BROKER_MQCHANNELS`

接下來是完整的表格說明。

> 如有標示 **參照範例** 表示範例的 JSON5 有提供，或是可以使用 CLI **help** 指令查看支援的選項。

## 共同設定

| JSON5             | 命令列參數        | 環境變數              | 預設值    | 說明 |
| -                 | -                 | -                     | -         | - |
| log.level         | log.level         | LOG_LEVEL             | info      | Log 等級。參照範例 |
| log.style         | log.style         | LOG_STYLE             | json      | Log 樣式。參照範例 |
| server.httpPort   | log.httpport      | SERVER_HTTP_PORT      | 1080      | HTTP 監聽連接埠 |
| server.httpsPort  | log.httpsport     | SERVER_HTTPS_PORT     | 1443      | HTTPS 監聽連接埠 |
| server.cacertFile | log.cacertfile    | SERVER_CACERT_FILE    |           | HTTPS 根憑證擋位置 |
| server.certFile   | log.certfile      | SERVER_CERT_FILE      |           | HTTPS 憑證擋位置 |
| server.keyFile    | log.keyfile       | SERVER_KEY_FILE       |           | HTTPS 私鑰位置 |
| server.staticPath | log.static        | SERVER_STATIC_PATH    |           | 靜態檔案目錄位置 |

### 詳細說明

- 目前還未使用根憑證。
- 必須同時使用憑證和私鑰才能啟用 HTTPS 服務。

## API Scopes

所有的 API 都需要透過在系統註冊的客戶端（client）以及訪問令牌（access token）才能存取。每一個令牌會記錄所屬的客戶端，只有經過授權的客戶端才能存取 API。

當某個 API 對應的 `apiScopes` 的設定被開啟時，除非客戶端在註冊的時候有啟用這些 scope 並被使用者授權，取得的令牌才能存取該 API。

命令列參數和環境變數都要是 JSON string，舉例：
```
--auth.api-scopes='{"auth.tokeninfo.get":[]}'
```

您可以自行定義 scope 名稱，並套用到各個 API scope 中。可以參考下面**認證服務**的範例。

## 認證服務（auth）

| JSON5                     | 命令列參數                | 環境變數                  | 預設值                    | 說明 |
| -                         | -                         | -                         | -                         | - |
| auth.db.engine            | auth.db.engine            | AUTH_DB_ENGINE            | sqlite                    | 使用的資料庫種類 |
| auth.db.mongodb.url       | auth.db.mongodb.url       | AUTH_DB_MONGODB_URL       | mongodb://localhost:27017 | MongoDB 連線的 URL |
| auth.db.mongodb.database  | auth.db.mongodb.database  | AUTH_DB_MONGODB_DATABASE  | auth                      | MongoDB 資料庫名稱 |
| auth.db.mongodb.poolSize  | auth.db.mongodb.poolsize  | AUTH_DB_MONGODB_POOLSIZE  |                           | MongoDB 最大連線數量 |
| auth.db.sqlite.path       | auth.db.sqlite.path       | AUTH_DB_SQLITE_PATH       | auth.db                   | SQLite 檔案位置 |
| auth.db.templates.login   | auth.db.templates         | AUTH_TEMPLATES            |                           | 登入頁面樣板位址 |
| auth.db.templates.grant   | auth.db.templates         | AUTH_TEMPLATES            |                           | 授權頁面樣板位址 |
| auth.db.apiScopes         | auth.api-scopes           | AUTH_API_SCOPES           |                           | API 權限設定 |

### 詳細說明

- templates（樣板）
    - 使用 OAuth2 authorization code 授權流程需要使用的網頁。**sylvia-iot-auth** 有提供預設的頁面，而 Sylvia-IoT 允許您自訂符合自己風格的網頁。
    - 樣板使用 Jinja2 格式（相依 [**tera**](https://tera.netlify.app/) 套件）。
    - 命令列參數和環境變數都要用 JSON string，比如
        ```
        --auth.templates='{"login":"xxx"}'
        ```
    - 詳細說明請參考 [**OAuth2 認證**](../dev/oauth2.md)。
- API scopes
    - **auth** 模組提供了以下幾個 scope 可供設定給對應的 API，限制 client 可以存取的範圍：
        - `auth.tokeninfo.get`: 授權 client 讀取令牌的資料。
            - `GET /api/v1/auth/tokeninfo`
        - `auth.logout.post`: 授權 client 將令牌登出。
            - `POST /auth/api/v1/auth/logout`
        - `user.get`: 授權 client 存取目前使用者的個人資料。
            - `GET /api/v1/user`
        - `user.path`: 授權 client 修改目前使用者的個人資料。
            - `PATCH /api/v1/user`
        - `user.get.admin`: 授權 client 取得系統所有使用者的資料。
            - `GET /api/v1/user/count`
            - `GET /api/v1/user/list`
            - `GET /api/v1/user/{userId}`
        - `user.post.admin`: 授權 client 在系統建立新的使用者。
            - `POST /api/v1/user`
        - `user.patch.admin`: 授權 client 修改系統任意使用者的資料。
            - `PATCH /api/v1/user/{userId}`
        - `user.delete.admin`: 授權 client 刪除系統任意使用者的資料。
            - `DELETE /api/v1/user/{userId}`
        - `client.get`: 授權 client 存取得系統所有客戶端資料。
            - `GET /api/v1/client/count`
            - `GET /api/v1/client/list`
            - `GET /api/v1/client/{clientId}`
        - `client.post`: 授權 client 在系統建立新的客戶端。
            - `POST /api/v1/client`
        - `client.patch`: 授權 client 修改系統任意客戶端的資料。
            - `PATCH /api/v1/client/{clientId}`
        - `client.delete`: 授權 client 刪除系統任意客戶端的資料。
            - `DELETE /api/v1/client/{clientId}`
        - `client.delete.user`: 授權 client 刪除系統任意使用者的所有客戶端。
            - `DELETE /api/v1/client/user/{userId}`
    - 舉例，在您的服務定義如下的 scope：
        - `api.admin`: 僅能授權移除使用者的所有客戶端。
        - `api.rw`: 可以讀寫除了 `DELETE /api/v1/client/user/{userId}` 的所有 API。
        - `api.readonly`： 只能存取 GET API。
        - 取得令牌資料和登出，這兩個動作開放所有客戶端可以使用。
        ```
        "auth": {
            ...
            "apiScopes": {
                "auth.tokeninfo.get": [],
                "auth.logout.post": [],
                "user.get": ["api.rw", "api.readonly"],
                "user.patch": ["api.rw"],
                "user.post.admin": ["api.rw"],
                "user.get.admin": ["api.rw", "api.readonly"],
                "user.patch.admin": ["api.rw"],
                "user.delete.admin": ["api.rw"],
                "client.post": ["api.rw"],
                "client.get": ["api.rw", "api.readonly"],
                "client.patch": ["api.rw"],
                "client.delete": ["api.rw"],
                "client.delete.user": ["api.admin"],
            },
            ...
        }
        ```
        - 以這個例子，註冊的客戶端可以任意勾選這三種 scope。之後使用者會在授權頁面得知這些訊息，並且決定是否授權客戶端。

## 消息代理服務（broker）

| JSON5                                     | 命令列參數                                | 環境變數                                  | 預設值                        | 說明 |
| -                                         | -                                         | -                                         | -                             | - |
| broker.auth                               | broker.auth                               | BROKER_AUTH                               | http://localhost:1080/auth    | 認證服務位址 |
| broker.db.engine                          | broker.db.engine                          | BROKER_DB_ENGINE                          | sqlite                        | 使用的資料庫種類 |
| broker.db.mongodb.url                     | broker.db.mongodb.url                     | BROKER_DB_MONGODB_URL                     | mongodb://localhost:27017     | MongoDB 連線的 URL |
| broker.db.mongodb.database                | broker.db.mongodb.database                | BROKER_DB_MONGODB_DATABASE                | auth                          | MongoDB 資料庫名稱 |
| broker.db.mongodb.poolSize                | broker.db.mongodb.poolsize                | BROKER_DB_MONGODB_POOLSIZE                |                               | MongoDB 最大連線數量 |
| broker.db.sqlite.path                     | broker.db.sqlite.path                     | BROKER_DB_SQLITE_PATH                     | auth.db                       | SQLite 檔案位置 |
| broker.cache.engine                       | broker.cache.engine                       | BROKER_CACHE_ENGINE                       | none                          | 使用的快取種類 |
| broker.cache.memory.device                | broker.cache.memory.device                | BROKER_CACHE_MEMORY_DEVICE                | 1,000,000                     | Memory 對裝置的快取數量 |
| broker.cache.memory.deviceRoute           | broker.cache.memory.device-route          | BROKER_CACHE_MEMORY_DEVICE_ROUTE          | 1,000,000                     | Memory 對裝置路由的快取數量 |
| broker.cache.memory.networkRoute          | broker.cache.memory.network-route         | BROKER_CACHE_MEMORY_NETWORK_ROUTE         | 1,000,000                     | Memory 對網路路由的快取數量 |
| broker.mq.prefetch                        | broker.mq.prefetch                        | BROKER_MQ_PREFETCH                        | 100                           | AMQP 消費者最大同時消費的數量 |
| broker.mq.sharedPrefix                    | broker.mq.sharedprefix                    | BROKER_MQ_SHAREDPREFIX                    | $share/sylvia-iot-broker/     | MQTT shared subscription 的前綴 |
| broker.mqChannels.unit.url                | broker.mq-channels.unit.url               | BROKER_MQCHANNELS_UNIT_URL                | amqp://localhost              | 單位的控制訊息位址 |
| broker.mqChannels.unit.prefetch           | broker.mq-channels.unit.prefetch          | BROKER_MQCHANNELS_UNIT_PREFETCH           | 100                           | 單位的控制訊息 AMQP 消費者最大同時消費的數量 |
| broker.mqChannels.application.url         | broker.mq-channels.application.url        | BROKER_MQCHANNELS_APPLICATION_URL         | amqp://localhost              | 應用的控制訊息位址 |
| broker.mqChannels.application.prefetch    | broker.mq-channels.application.prefetch   | BROKER_MQCHANNELS_APPLICATION_PREFETCH    | 100                           | 應用的控制訊息 AMQP 消費者最大同時消費的數量 |
| broker.mqChannels.network.url             | broker.mq-channels.network.url            | BROKER_MQCHANNELS_NETWORK_URL             | amqp://localhost              | 單位的控制訊息位址 |
| broker.mqChannels.network.prefetch        | broker.mq-channels.network.prefetch       | BROKER_MQCHANNELS_NETWORK_PREFETCH        | 100                           | 單位的控制訊息 AMQP 消費者最大同時消費的數量 |
| broker.mqChannels.device.url              | broker.mq-channels.device.url             | BROKER_MQCHANNELS_DEVICE_URL              | amqp://localhost              | 裝置的控制訊息位址 |
| broker.mqChannels.device.prefetch         | broker.mq-channels.device.prefetch        | BROKER_MQCHANNELS_DEVICE_PREFETCH         | 100                           | 裝置的控制訊息 AMQP 消費者最大同時消費的數量 |
| broker.mqChannels.deviceRoute.url         | broker.mq-channels.device-route.url       | BROKER_MQCHANNELS_DEVICE_ROUTE_URL        | amqp://localhost              | 裝置路由的控制訊息位址 |
| broker.mqChannels.deviceRoute.prefetch    | broker.mq-channels.device-route.prefetch  | BROKER_MQCHANNELS_DEVICE_ROUTE_PREFETCH   | 100                           | 裝置路由的控制訊息 AMQP 消費者最大同時消費的數量 |
| broker.mqChannels.networkRoute.url        | broker.mq-channels.network-route.url      | BROKER_MQCHANNELS_NETWORK_ROUTE_URL       | amqp://localhost              | 網路路由的控制訊息位址 |
| broker.mqChannels.networkRoute.prefetch   | broker.mq-channels.network-route.prefetch | BROKER_MQCHANNELS_NETWORK_ROUTE_PREFETCH  | 100                           | 網路路由的控制訊息 AMQP 消費者最大同時消費的數量 |
| broker.mqChannels.data.url                | broker.mq-channels.data.url               | BROKER_MQCHANNELS_DATA_URL                |                               | 資料訊息位址 |
| broker.db.apiScopes                       | broker.api-scopes                         | BROKER_API_SCOPES                         |                               | API 權限設定 |

### 詳細說明

- 指定認證服務位址（`broker.auth`）的用意在檢查呼叫 API 的令牌的合法性，包含使用者帳號與客戶端。

- MQ channels:
    - 由於 Sylvia-IoT 訊息代理服務是決定效能的關鍵模組，會將許多設定放在記憶體中。這些設定需要透過 **控制訊息（control channel message）** 將 API 變更的內容透過訊息佇列傳遞到叢集的各個執行個體中。
        - 相關訊息請見 [**快取**](../arch/cache.md) 章節。
    - **data** 是 **資料訊息（data channel message）**，將所有資料記錄到 **sylvia-iot-data** 模組中。
        - 不指定參數（或是 JSON5 設定 **null**）就會不儲存任何資料。
        - 相關訊息請見 [**資料流**](../arch/flow.md) 章節。

- API scopes: 請參考**認證服務**的說明。

## 核心管理服務（coremgr）

| JSON5                             | 命令列參數                        | 環境變數                          | 預設值                        | 說明 |
| -                                 | -                                 | -                                 | -                             | - |
| coremgr.auth                      | coremgr.auth                      | COREMGR_AUTH                      | http://localhost:1080/auth    | 認證服務位址 |
| coremgr.broker                    | coremgr.broker                    | COREMGR_BROKER                    | http://localhost:2080/broker  | 訊息代理服務位址 |
| coremgr.mq.engine.amqp            | coremgr.mq.engine.amqp            | COREMGR_MQ_ENGINE_AMQP            | rabbitmq                      | AMQP 種類 |
| coremgr.mq.engine.mqtt            | coremgr.mq.engine.mqtt            | COREMGR_MQ_ENGINE_MQTT            | emqx                          | MQTT 種類 |
| coremgr.mq.rabbitmq.username      | coremgr.mq.rabbitmq.username      | COREMGR_MQ_RABBITMQ_USERNAME      | guest                         | RabbitMQ 管理者帳號 |
| coremgr.mq.rabbitmq.password      | coremgr.mq.rabbitmq.password      | COREMGR_MQ_RABBITMQ_PASSWORD      | guest                         | RabbitMQ 管理者密碼 |
| coremgr.mq.rabbitmq.ttl           | coremgr.mq.rabbitmq.ttl           | COREMGR_MQ_RABBITMQ_TTL           |                               | RabbitMQ 預設佇列訊息存活長度（秒） |
| coremgr.mq.rabbitmq.length        | coremgr.mq.rabbitmq.length        | COREMGR_MQ_RABBITMQ_LENGTH        |                               | RabbitMQ 預設佇列訊息最多個數 |
| coremgr.mq.rabbitmq.hosts         | coremgr.mq.rabbitmq.hosts         | COREMGR_MQ_RABBITMQ_HOSTS         |                               | （保留） |
| coremgr.mq.emqx.apiKey            | coremgr.mq.emqx.apikey            | COREMGR_MQ_EMQX_APIKEY            |                               | EMQX 管理 API key |
| coremgr.mq.emqx.apiSecret         | coremgr.mq.emqx.apisecret         | COREMGR_MQ_EMQX_APISECRET         |                               | EMQX 管理 API secret |
| coremgr.mq.emqx.hosts             | coremgr.mq.emqx.hosts             | COREMGR_MQ_EMQX_HOSTS             |                               | （保留） |
| coremgr.mq.rumqttd.mqttPort       | coremgr.mq.rumqttd.mqtt-port      | COREMGR_MQ_RUMQTTD_MQTT_PORT      | 1883                          | rumqttd MQTT 連接埠 |
| coremgr.mq.rumqttd.mqttsPort      | coremgr.mq.rumqttd.mqtts-port     | COREMGR_MQ_RUMQTTD_MQTTS_PORT     | 8883                          | rumqttd MQTTS 連接埠 |
| coremgr.mq.rumqttd.consolePort    | coremgr.mq.rumqttd.console-port   | COREMGR_MQ_RUMQTTD_CONSOLE_PORT   | 18083                         | rumqttd 管理 API 連接埠 |
| coremgr.mqChannels.data.url       | coremgr.mq-channels.data.url      | COREMGR_MQCHANNELS_DATA_URL       |                               | 資料訊息位址 |

### 詳細說明

- MQ channels:
    - **data** 是 **資料訊息（data channel message）**
        - 目前 coremgr 支援紀錄 GET API 以外的 HTTP 請求內容，開啟資料通道即可紀錄 API 使用歷程。
        - 不指定參數（或是 JSON5 設定 **null**）就會不儲存任何資料。

## 核心管理服務管理介面（coremgr-cli）

| JSON5                     | 命令列參數                | 環境變數                  | 預設值                        | 說明 |
| -                         | -                         | -                         | -                             | - |
| coremgrCli.auth           | coremgr-cli.auth          | COREMGRCLI_AUTH           | http://localhost:1080/auth    | 認證服務位址 |
| coremgrCli.coremgr        | coremgr-cli.coremgr       | COREMGRCLI_COREMGR        | http://localhost:3080/coremgr | 核心管理服務位址 |
| coremgrCli.data           | coremgr-cli.data          | COREMGRCLI_DATA           | http://localhost:4080/data    | 資料服務位址 |
| coremgrCli.clientId       | coremgr-cli.client-id     | COREMGRCLI_CLIENT_ID      |                               | 命令列客戶端 ID |
| coremgrCli.redirectUri    | coremgr-cli.redirect-uri  | COREMGRCLI_REDIRECT_URI   |                               | 命令列客戶端重轉向網址 |

## 資料服務（data）

| JSON5                                 | 命令列參數                            | 環境變數                              | 預設值                        | 說明 |
| -                                     | -                                     | -                                     | -                             | - |
| data.auth                             | data.auth                             | DATA_AUTH                             | http://localhost:1080/auth    | 認證服務位址 |
| data.broker                           | data.broker                           | DATA_BROKER                           | http://localhost:2080/broker  | 訊息代理服務位址 |
| data.db.engine                        | data.db.engine                        | DATA_DB_ENGINE                        | sqlite                        | 使用的資料庫種類 |
| data.db.mongodb.url                   | data.db.mongodb.url                   | DATA_DB_MONGODB_URL                   | mongodb://localhost:27017     | MongoDB 連線的 URL |
| data.db.mongodb.database              | data.db.mongodb.database              | DATA_DB_MONGODB_DATABASE              | data                          | MongoDB 資料庫名稱 |
| data.db.mongodb.poolSize              | data.db.mongodb.poolsize              | DATA_DB_MONGODB_POOLSIZE              |                               | MongoDB 最大連線數量 |
| data.db.sqlite.path                   | data.db.sqlite.path                   | DATA_DB_SQLITE_PATH                   | data.db                       | SQLite 檔案位置 |
| data.mqChannels.broker.url            | data.mq-channels.broker.url           | DATA_MQCHANNELS_BROKER_URL            | amqp://localhost              | 資料訊息位址 |
| data.mqChannels.broker.prefetch       | data.mq-channels.broker.prefetch      | DATA_MQCHANNELS_BROKER_PREFETCH       | 100                           | 資料訊息 AMQP 消費者最大同時消費的數量 |
| data.mqChannels.broker.sharedPrefix   | data.mq-channels.broker.sharedprefix  | DATA_MQCHANNELS_BROKER_SHAREDPREFIX   | $share/sylvia-iot-data/       | MQTT shared subscription 的前綴 |
| data.mqChannels.coremgr.url           | data.mq-channels.coremgr.url          | DATA_MQCHANNELS_COREMGR_URL           | amqp://localhost              | 資料訊息位址 |
| data.mqChannels.coremgr.prefetch      | data.mq-channels.coremgr.prefetch     | DATA_MQCHANNELS_COREMGR_PREFETCH      | 100                           | 資料訊息 AMQP 消費者最大同時消費的數量 |
| data.mqChannels.coremgr.sharedPrefix  | data.mq-channels.coremgr.sharedprefix | DATA_MQCHANNELS_COREMGR_SHAREDPREFIX  | $share/sylvia-iot-data/       | MQTT shared subscription 的前綴 |
