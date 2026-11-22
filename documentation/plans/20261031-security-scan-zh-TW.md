# sylvia-iot 安全掃描報告

| 範圍 | 掃描日期 | 基準 |
|---|---|---|
| `general-mq/src`（10 個檔案，約 2,000 行） | 2026-10-31 | branch `20261023-update-dependencies`（`238dfc9f6`） |
| `sylvia-iot-corelib/src`（9 個檔案，約 1,100 行） | 2026-11-01 | `main`（`41a9a7f9a`）加上當時尚未 commit 的 general-mq 修改 |
| `sylvia-iot-auth/src`（55 個檔案，約 8,500 行） | 2026-11-01 | 同上 |
| `sylvia-iot-coremgr/src`（25 個檔案，約 8,000 行） | 2026-11-01 | 同上 |
| `sylvia-iot-broker/src`（74 個檔案，約 22,000 行） | 2026-11-01 | 同上 |
| `sylvia-iot-data/src`（53 個檔案，約 9,400 行） | 2026-11-01 | 同上 |
| `sylvia-iot-sdk/src`（10 個檔案，約 1,800 行） | 2026-11-01 | 同上 |
| `sylvia-iot-coremgr-cli/src`（21 個檔案，約 11,600 行） | 2026-11-01 | 同上 |
| `sylvia-router/src`（19 個檔案，約 3,700 行） | 2026-11-01 | 同上 |

- 不在範圍內：`stress-simple`。
- `sylvia-router` 是 IP 分享器的案子，和 IoT 本身沒有關聯，也不會用 K8S 部署，所以依使用者說明列為「視情況處理」，不列入主要的修正優先順序。
- 相依版本：`rumqttc` 0.25.1、`amqprs` 2.1.5、`lapin` 4.12.3、`url` 2.5.8、`tera` 2.4.0。
- 範圍不含 `tests/` 和 `examples/`。
- 各節標示的行號，都是掃描當時基準版本的行號。已修正的項目保留原本的缺陷說明，方便對照。
- 修正放在 branch `20261122-security-fixes`。這一輪的安全修正已經全部完成（2026-11-22），每個 crate 一個 commit：
  - `d1e5329f2`：general-mq（MQ-1、MQ-3、MQ-4），以及 SDK、broker 的封包大小處理。
  - `1a78a3c3f`：corelib（CORE-1～4），以及 auth 跟著修改的部分。
  - `1c7a4999c`：auth（AUTH-1～5、AUTH-7，AUTH-6 的時間差與雜湊數量限制）。
  - `c3d32c0ed`：broker（BRK-1、BRK-2，以及 CMGR-1 在 broker 端的修正）。
  - `a07542aa1`：coremgr（CMGR-2～7、CMGR-10）。
  - `a5cda738a`：data（DATA-1）。
  - `2d730dff6`：coremgr-cli（CLI-1、CLI-2，另外修正 `coremgr-cli.*` 命令列參數沒有註冊的問題）。
- **編號規則：** 依 crate 分組，前綴是 `MQ`（general-mq）、`CORE`（corelib）、`AUTH`、`CMGR`（coremgr）、`BRK`（broker）、`DATA`、`CLI`（coremgr-cli）、`RTR`（router）；同一個 crate 內依嚴重度排序；後來才新增的 AUTH-7、CMGR-10 接在最後，沒有依嚴重度排序。問題的根源在哪個 crate，就歸在哪個 crate。
- **權限模型：** 依使用者說明，unit 成員可以讀取和收發資料，管理操作（建立、修改、刪除）只有擁有者能做，admin 和 manager 可以做所有操作。成員只能由 admin 或 manager 加入，GUI 目前沒有成員管理；成員權限的細節留到 0.5 的 unit member management。
- **rumqttd：** 只用於 demo/PoC，不列入。

## 總覽

### 未修正（延後、視情況或不適用）

| # | 嚴重度 | 判定 | 問題 | 位置 | 狀態 |
|---|---|---|---|---|---|
| MQ-2 | 中 | CONFIRMED（程式碼追蹤） | MQTT 接收端沒有背壓，task 數量無上限增加 | general-mq | 延後 |
| RTR-1 | 中 | CONFIRMED（程式碼追蹤） | router 的管理 API 沒有角色檢查，任何使用者都能修改 WAN/LAN/WiFi 設定，並讀出 PPPoE 和 WiFi 的密碼 | router | 視情況 |
| RTR-2 | 中 | PLAUSIBLE（取決於主機的防火牆） | router 的所有服務都綁在 `[::]`，包含 WAN 網卡，而 repo 沒有任何防火牆設定或說明 | router | 視情況 |
| CMGR-8 | — | 不適用 | 內建的 rumqttd 允許匿名連線、沒有 ACL | coremgr | 不修正（rumqttd 只用於 demo/PoC） |
| CMGR-9 | — | 不適用 | unit 成員可以透過 API 發布 dldata 或 uldata | coremgr | 不修正（設計允許） |

### 已修正

| # | 嚴重度 | 問題 | 位置 | commit |
|---|---|---|---|---|
| CMGR-1 | 嚴重 | broker 的 API 把帶有 MQ 管理員帳密的 `hostUri` 回傳給 unit 成員 | coremgr、broker | `c3d32c0ed` |
| CMGR-2 | 嚴重 | coremgr 沒有套用 MQ hosts 白名單，管理帳密會送到租戶指定的主機 | coremgr | `a07542aa1` |
| DATA-1 | 嚴重 | data 服務的 unit 檢查可以用空字串 `?unit=` 繞過，任何登入的使用者都能讀取所有租戶的資料 | data | `a5cda738a` |
| MQ-1 | 高 | MQTT 訊息超過 10 KiB 會斷掉整條共用連線，可跨租戶 DoS | general-mq | `d1e5329f2` |
| AUTH-1 | 高 | 登入頁反射型 XSS：模板沒有做 HTML 跳脫 | auth | `1c7a4999c` |
| AUTH-2 | 高 | 在登入頁的 `state` 塞入 `allow=yes`，可以跳過同意頁，取得受害者的 token | auth | `1c7a4999c` |
| BRK-1 | 高 | range API 的範圍檢查有整數截斷，任何登入的使用者都能讓 broker 卡住並耗盡記憶體 | broker | `c3d32c0ed` |
| CMGR-4 | 高 | EMQX 上的租戶隔離依賴 EMQX 的安全設定，6.3 之前的預設等同允許所有存取 | coremgr、文件 | `a07542aa1`（文件） |
| AUTH-3 | 中高 | grant 頁的反射型與儲存型 XSS | auth | `1c7a4999c` |
| AUTH-4 | 中高 | 停用的使用者仍可繼續使用 token，並無限期 refresh | auth | `1c7a4999c` |
| CMGR-3 | 中 | unit 成員可以重設 application/network 的 MQ 密碼，接管它的 MQ 帳號（原為「高」，成員只能由 admin/manager 加入） | coremgr | `a07542aa1` |
| CMGR-10 | 中 | unit 成員可以透過 coremgr 的 DELETE API 清除 application、network，甚至整個 unit 的 MQ 資源（原為「高」，理由同 CMGR-3） | coremgr、broker | `a07542aa1` |
| CMGR-5 | 中 | 未登入就能送出超大 request body，讓 coremgr 的記憶體耗盡 | coremgr | `a07542aa1` |
| CORE-1 | 低中 | `random_id_sha()` 產生的安全憑證熵太低 | corelib、auth | `1a78a3c3f` |
| AUTH-5 | 低中 | SQLite 的 login session 永遠不會過期；按「拒絕」時 session 也不會刪除 | auth | `1c7a4999c` |
| MQ-3 | 低 | MQTT URI 使用 IPv6 位址時，主機被換成 `localhost` | general-mq | `d1e5329f2` |
| MQ-4 | 低 | URI 格式錯誤時，錯誤訊息包含明文密碼 | general-mq | `d1e5329f2` |
| CORE-2 | 低 | 密碼雜湊強度不足（PBKDF2 只跑 10000 rounds） | corelib | `1a78a3c3f` |
| CORE-3 | 低 | access log 會記下 `session_id` | corelib | `1a78a3c3f` |
| CORE-4 | 低 | log4j 格式沒有跳脫控制字元 | corelib | `1a78a3c3f` |
| AUTH-6 | 低 | 登入沒有次數限制，而且可以從回應時間判斷帳號是否存在 | auth | `1c7a4999c`（部分：次數限制未做） |
| AUTH-7 | 低 | login session 沒有綁定 client，可以拿去授權別的 client | auth | `1c7a4999c` |
| BRK-2 | 低 | dldata-result 沒有檢查這筆資料是否屬於回報的 network | broker | `c3d32c0ed` |
| CMGR-6 | 低 | coremgr 用 REST API 發布資料時沒有大小限制 | coremgr、文件 | `a07542aa1` |
| CMGR-7 | 低 | MQ 資源的清理程式沒有 `await`，從來沒有執行過 | coremgr | `a07542aa1` |
| CLI-1 | 低 | token 存放檔的權限通常是 `0644`，其他本機使用者都能讀 | coremgr-cli | `2d730dff6` |
| CLI-2 | 低 | 密碼只能從命令列參數 `--password` 傳入 | coremgr-cli | `2d730dff6` |

「嚴重」比「高」更高。

**目前狀態：** 所有需要修正的項目都已修正。剩下的 MQ-2 和 AUTH-6 的次數限制延後到 0.5，RTR-* 視情況處理，CMGR-8、CMGR-9 不適用。後續事項見報告最後的「修正結果與後續事項」。

## 威脅模型

- 來自網路的資料都不可信，包括 AMQP/MQTT 封包、payload、topic、queue 名稱和 HTTP 請求。
- 已登入的低權限使用者會嘗試跨越租戶（unit）的邊界。例如某個 unit 的 network 或 application 擁有者，或是有 DEV 角色、可以建立 client 的開發者。
- 設定值（URI、帳密、檔案路徑）可能來自使用者可以編輯的設定檔。
- 服務需要維持可用：panic、記憶體無上限成長、資源耗盡都算漏洞。

---

# 一、general-mq

## MQ-1.【高】MQTT 超過 10 KiB 的訊息會斷掉整條連線（跨租戶 DoS）

**判定：CONFIRMED（PoC 實測）　狀態：已修正（`d1e5329f2`）**

### 缺陷

`create_event_loop()` 用 `RumqttOption::new()` 建立連線，但沒有呼叫 `set_max_packet_size()`。所以收和送都用 `rumqttc` 的預設上限，只有 10 KiB（rumqttc `src/lib.rs:516-517`）。

- **入站：** 收到的封包超過上限時，`rumqttc` 在解析 fixed header 時就回傳 `PayloadSizeLimitExceeded`。general-mq 把這個錯誤吞掉，然後斷線重連。
- **出站：** `AsyncClient::publish()` 不檢查大小，所以 `send_msg()` 會回 `Ok(())`。等到 event loop 寫出封包時才遇到 `OutgoingPacketTooLarge`，整條連線斷掉，連線中排隊的訊息也一起遺失。

MQTT 的連線由多個 queue 共用，連線一斷，上面所有 queue 都要重新連線和訂閱。

### sylvia-iot 中的觸發路徑

1. coremgr 的 `transfer_host_uri()`（`sylvia-iot-coremgr/src/routes/v1/mod.rs:549`）會把每個 MQTT host URI 都填入同一組 EMQX 管理帳密。所以同一台 MQTT broker 的 URI 字串完全相同。
2. broker 的連線池用完整的 URI 字串當 key，所以**同一台 MQTT broker 上所有租戶共用一條 MQTT 連線**。AMQP 用 vhost 區分，每個租戶是獨立連線，不受影響。
3. **入站攻擊：** 租戶 A 往自己的 `uldata` topic 發布一則大於 10 KiB 的訊息。EMQX 預設上限是 1 MB，所以會照常轉送給 broker，broker 的共用連線就斷掉了。只要持續送大訊息，就能讓這條連線一直斷線。
4. **出站攻擊：** 應用端送出大的 downlink，broker 轉送給 network 時，就會斷掉自己的連線。

就算沒有惡意，資料包上 JSON 並轉成 hex 之後大小會變兩倍，所以原始資料只要約 5 KB 就會超過上限。

### PoC

測試環境：當時 Docker Hub 連不上，所以改用 `rabbitmq:4.4.0-management-alpine`，並開啟 `rabbitmq_mqtt` plugin。

**入站：** 在 general-mq 的一條共用連線上掛兩個接收 queue，再用一個不受 10 KiB 限制的 `rumqttc` client 送出 11 KiB：

```
[shared] conn status: Connected
--- tenant-a publisher (non-general-mq client) sends 11 KiB to its own topic
[shared] conn status: Disconnected
raw publisher: 11264 bytes to tenant-a.uldata acked by broker
  queue tenant-a.uldata status: Connecting
  queue tenant-b.uldata status: Connecting
```

另外用預設 10 KiB 上限的 subscriber 確認斷線原因：`MqttState(Deserialization(PayloadSizeLimitExceeded(11283)))`。

**出站：** `send_msg` 回傳 `Ok(())`，但連線隨即斷開。

### 修正內容

**general-mq**
- `MqttConnectionOptions` 新增 `max_packet_size`，預設 1 MiB，收和送都套用。
- `MqttQueue::send_msg()` 送出前先計算封包大小，超過就回傳 `Error::PacketTooLarge`，不讓連線斷掉。
- 新增同步的 `Queue::check_msg_size()`，讓呼叫端在 `task::spawn` 之前就能檢查。

**SDK 和 broker：兩層檢查，API 都維持同步**
- **第一層：** SDK 的 `send_uldata()`、`send_dldata()`、`send_dldata_result()`，以及 broker 的 `send_uldata()`、`send_dldata()`，都在 spawn 之前呼叫 `check_msg_size()`。超量時直接同步回錯，不再默默丟掉。
- **第二層：** broker 收到訊息時，用 `MAX_DATA_MSG_SIZE = 1 MiB - 16 KiB` 檢查。
  - 計算的是正規化並重新序列化後的大小，因為 extension 裡像 `1e15` 這樣的數字，重新序列化後會變長。
  - dldata 超量：回 dldata-resp，錯誤碼 `err_param`，訊息 `"data too large"`。
  - uldata 超量：寫 warn log 後丟棄。
  - 預留的 16 KiB 是給 broker 轉送時加上的欄位用的：固定的部分約 300 B，其餘是給 code、profile、network addr 的緩衝，因為這些欄位沒有長度上限。

**文件**
- 大小上限已寫進 `sylvia-iot-broker/doc/message.md`。
- 中英文 book 的 `dev/application.md` 和 `dev/network.md` 也已補上。

**測試**
- general-mq、SDK、broker 的整合測試都通過。

**殘留風險**
- code、profile 這些欄位沒有長度上限，所以 16 KiB 是實務上夠用，不是保證。
- 如果 data channel 送出時撞到 `PacketTooLarge`，程式仍會 nack。application 或 network 那段是 AMQP 時，會無限重送。

### 相容性

這次的修法依使用者決定，新增了 `pub` 欄位 `MqttConnectionOptions::max_packet_size` 和 `Error::PacketTooLarge`。

- 這兩個 struct 和 enum 都沒有 `#[non_exhaustive]`，所以對外屬於 **breaking change**：
  - 外部程式如果用 struct literal 建立 options、又沒寫 `..Default::default()`，會編譯失敗。
  - 外部程式如果對 `Error` 做 exhaustive `match`，也會編譯失敗。
- repo 內的 crate 都用 `..Default::default()`，不受影響。
- 發佈到 crates.io 的版本號需要另外決定。

## MQ-2.【中】MQTT 接收端沒有背壓，task 數量無上限增加

**判定：CONFIRMED（程式碼追蹤，未實測記憶體耗盡）　狀態：延後**

### 缺陷

- `on_publish()` 每收到一則 PUBLISH 就 `task::spawn` 一個 task，沒有數量限制。
- `rumqttc` 預設會自動回 PUBACK，所以 broker 會持續送訊息過來，不會等 client 處理完。
- MQTT 的 `ack()` 和 `nack()` 什麼都不做。

### 觸發路徑

租戶高頻率發布小訊息。如果 handler 處理一則訊息（要查 DB 做路由）的時間比訊息進來的間隔還長，task 就會一直累積，最後耗盡記憶體。

AMQP 沒有這個問題，因為 `prefetch` 會限制尚未 ack 的訊息數。

另外，MQTT 在處理之前就已經 ack，所以 `reliable: true` 做不到「至少送達一次」。

### 修正方向

- 改用 `set_manual_acks(true)`，在 `ack()` 時才送出 PUBACK。這樣可以同時解決背壓和可靠性問題。
- 但這會改變行為：不呼叫 `ack()` 的話，就收不到新訊息。
- 不建議用 semaphore 在 `on_publish()` 裡等待，因為會卡住整條連線的 event loop。

## MQ-3.【低】MQTT URI 使用 IPv6 位址時，主機被換成 `localhost`

**判定：CONFIRMED（實測 `url` crate 行為）　狀態：已修正（`d1e5329f2`）**

### 缺陷

`url.domain()` 遇到 IPv6 會回傳 `None`，程式就 fallback 成 `"localhost"`。

例如設定 `mqtt://user:pass@[2001:db8::5]:1883`，實際會連到 `localhost`，而且用明文送出帳密。

### 修正內容

改用 `url.host()`，依類型分別處理：

- **MQTT：** IPv6 位址加上方括號給 TCP 用。
- **`mqtts` 搭配 IPv6：** 在 `new()` 直接回錯。原因是 rumqttc 用同一個字串當 TCP 位址和 TLS server name，前者需要方括號，後者不能有。
- **AMQP：** `AMQPUri` 對 IPv6 有同樣的問題，所以一併修正，改從 `Url` 取主機。

### 測試

新增 `connect_ipv6`。

## MQ-4.【低】URI 格式錯誤時，錯誤訊息包含明文密碼

**判定：CONFIRMED（測試）　狀態：已修正（`d1e5329f2`）**

### 缺陷

URI 是 cannot-be-a-base 的格式時（例如 `mqtt:user:secret@host`），錯誤訊息會帶出原始 URI。

- MQTT 的問題在自己的 `uri.rs`。
- AMQP 的問題在第三方的 `amq-protocol-uri`（lib.rs:189）。

### 修正內容

- MQTT 改成只回 `Invalid URL`。
- AMQP 改成先用 `Url` 檢查 cannot-be-a-base。沒有用比對錯誤字串的方式，因為那樣太脆弱。

### 測試

`new_wrong_opts` 會確認錯誤訊息裡不含密碼。

## 其他 general-mq 修正（一併完成）

- MQTT 和 AMQP 的連線逾時修正：rumqttc 的 timeout 單位是秒，AMQP 則補上 `time::timeout`。
- `lib.rs` 的 doctest 失敗修正：shell 指令改用 ```` ```sh ```` 區塊。

## 不算安全漏洞，但值得注意（延後）

- **AMQP 的 `reliable` 沒有等待 publisher confirm：** broker 拒收或無法路由時，`send_msg()` 仍然回 `Ok(())`。
- **MQTT 無法把 PUBACK 對應到某一次 `send_msg()`：** `rumqttc` 的 `publish()` 不回傳 packet id。
- **RabbitMQ MQTT plugin 不相容：** 它會把 topic 裡的 `.` 和 `/` 互換，跟 general-mq 的 queue 名稱對不上。sylvia-iot 用的是 EMQX 或 rumqttd，不受影響。

## general-mq 檢查過、沒有發現問題的部分

- **Topic 和 queue 名稱注入：** 名稱的 regex 不允許 `+`、`#`、`/`、`$`。
- **TLS：** AMQP 和 MQTT 都有驗證伺服器憑證。
- **Mutex：** 沒有 guard 被持有跨過 `.await`，lock 順序一致。
- **panic：** 網路上收到的資料走不到任何會 panic 的路徑。沒有 `unsafe`。

## 介面問題（0.5 再處理）

- **options struct 無法擴充：** 欄位全部是 `pub`，也沒有 `#[non_exhaustive]`。只加 `#[non_exhaustive]` 的話，外部連 `..Default::default()` 都不能用，必須搭配 builder 一起改。
- **`Error` 無法擴充：** 沒有 `#[non_exhaustive]`。
- **實作沒有做到文件的語意：**
  - MQTT 的 `ack()` 什麼都不做。
  - `send_msg()` 回 `Ok` 不代表送達。

---

# 二、corelib

## CORE-1.【低中】`random_id_sha()` 產生的安全憑證熵太低

**判定：CONFIRMED（熵）／線上暴力破解目前不實際　狀態：已修正（`1a78a3c3f`）**

### 缺陷

`random_id_sha()` 的產生方式是 `sha256("{毫秒時間戳}-{len 個英數字}")`（`sylvia-iot-corelib/src/strings.rs:79`）。

SHA-256 不會增加熵，時間戳又大致可以推測，所以真正的熵只有隨機那一段。

| 用途 | 位置 | 隨機長度 | 熵 | 有效時間 |
|---|---|---|---|---|
| login session_id | `auth/src/routes/oauth2/api.rs:201` | 4 | 約 2^24 | 60 秒 |
| authorization code | `auth/src/routes/oauth2/primitive.rs:44` | 4 | 約 2^24 | 30 秒 |
| access token | `primitive.rs:128` | 8 | 約 2^48 | 1 小時 |
| refresh token | `primitive.rs:126` | 8 | 約 2^48 | 14 天 |

### 觸發路徑

1. 猜中一個有效的 session_id，就能用自己的 client 呼叫 `POST /authorize`，拿到受害者帳號的 authorization code。
2. 即使毫秒時間戳完全猜中，60 秒內也要送出約 1,480 萬次請求。
3. 時間戳只要有 1 秒的誤差，請求次數就要再乘上 1000 倍。

### 修正內容

- auth 的 authorization code、access token、refresh token 和 login session_id，都改用 `randomstring(64)`。
  - `randomstring()` 用的 `rand::rng()` 是 ChaCha12，屬於 CSPRNG。
  - 每個字元有 62 種可能，64 個字元約 381 bits。
- `random_id_sha()` 保留，但在 rustdoc 註明「不要用來產生安全憑證」。

## CORE-2.【低】密碼雜湊強度不足

**判定：CONFIRMED　狀態：已修正（`1a78a3c3f`）**

### 缺陷

- PBKDF2-HMAC-SHA256 只跑 10000 rounds（`strings.rs:14`）。OWASP 建議 600000 rounds。
- salt 只有 8 個字元（`auth/src/routes/v1/user/api.rs:40`）。

### 影響

只有在資料庫外洩後才有影響：離線破解大約快 60 倍。

### 修正內容

- **新格式：** `$pbkdf2-sha256$i=600000$<hex>`。沒有前綴的就是舊格式（10000 rounds），照樣可以驗證。
- **新函式：** corelib 新增 `password_verify()` 和 `password_need_rehash()`。auth 在使用者用舊格式登入成功後，會重新雜湊並寫回資料庫。
- **改用 aws-lc-rs：** 原本的 `pbkdf2` crate 是泛型實作，debug build 下每次要約 3.5 秒；aws-lc-rs 約 0.33 秒。所有相依 corelib 的 crate 原本就會編譯 aws-lc-rs，所以不增加編譯成本。
- **不卡住 worker：** auth 的雜湊都改在 `spawn_blocking` 裡執行。
- 帳號不存在時也做一次雜湊、限制同時進行的雜湊數量，這兩項在 AUTH-6 處理（`1c7a4999c`）。

## CORE-3.【低】access log 會記下 `session_id`

**判定：CONFIRMED　狀態：已修正（`1a78a3c3f`）**

### 缺陷

`LoggerMiddleware` 會記錄完整的 URI，包含 query string（`logger.rs:139,146-148`）。

### 觸發路徑

1. 登入後，瀏覽器會 GET `/auth/oauth2/authorize?…&session_id=…`。
2. 這個 URI 符合 log 的過濾條件 `/auth/oauth2/`，所以會被記下來。
3. 能讀 log 的人只要在 60 秒內，就能用這個 session_id 替受害者授權。

### 修正內容

- path 含有 `/oauth2/` 時，query 只保留參數名稱、去掉值。例如 `?response_type&session_id`。
- 只看 path，不看前綴，所以 auth 掛在其他路徑時也有效，內建的 `/auth/oauth2/redirect?code=…` 也一併涵蓋。
- `FILTER_ONLY` 從 `"/auth/oauth2/"` 改成 `"/oauth2/"`。
- `/api/` 的 query 完整保留，例如 limit、offset 這些除錯需要的資訊。

## CORE-4.【低】log4j 格式沒有跳脫控制字元

**判定：CONFIRMED　狀態：已修正（`1a78a3c3f`）**

### 缺陷

`logger.rs:214` 只把 `\n` 替換掉，`\r` 和 ESC（`\x1b`）會原樣寫出。

### 觸發路徑

1. 送出 `GET /auth/oauth2/login?state=%1b[…`。
2. 解析失敗時，`auth/src/routes/oauth2/api.rs:108` 會用 `warn!` 記下原始的 `state`。
3. 管理者在終端機查看 log 時，ESC 序列會被終端機執行。可以用來偽造或隱藏 log 內容。

預設的 JSON 格式會把控制字元跳脫，所以只有設定成 log4j 格式時才會受影響。

### 修正內容

log4j 格式改用 `str::escape_debug()`：
- 控制字元、雙向文字控制字元（例如 `U+202E`）、零寬空格和 BOM 都會被跳脫，而且結果一定是單行。
- 中文、帶重音的字母和 emoji 保留原樣。

## corelib 檢查過、沒有發現問題的部分

- **`ErrResp::Custom` 的 `StatusCode::from_u16().unwrap()`：** 所有呼叫端傳入的都是 `ErrReq` 常數，不會 panic。
- **`escape_regex_str()`：** 已涵蓋 PCRE 在字元集合外的所有特殊字元。
  - 輸入含 NUL 時 `CString` 會失敗，「包含」條件被略過。但查詢仍限定在原本的 unit 範圍，不會越權。
- **`is_uri()` 接受 `javascript:`：** redirect_uri 只出現在 302 的 Location（瀏覽器會忽略 `javascript:`），以及 grant 頁的 hidden input。所以它本身不構成攻擊。
- **`hex_addr_to_u128()` 接受 `+` 開頭：** 後續都會經過 `u128_to_addr()` 正規化。
- **其他：**
  - 正規表示式用的是 Rust 的 regex crate，執行時間是線性的，不會有 ReDoS。
  - `Json` 解析錯誤回應的是 JSON，不會造成 XSS。
  - `server_config` 的 port 範圍已檢查。
  - `role.rs` 和 `version.rs` 沒有問題。

---

# 三、auth

## AUTH-1.【高】登入頁反射型 XSS：模板沒有做 HTML 跳脫

**判定：CONFIRMED（程式碼追蹤，未用瀏覽器實測）　狀態：已修正（`1c7a4999c`）**

### 缺陷

- tera 2.4.0 只有名稱結尾是 `.html`、`.htm`、`.xml` 的模板會自動跳脫。這是 `finalize_templates()` 裡的 `set_templates_auto_escape()` 決定的，已在原始碼確認。
- auth 註冊模板的名稱是 `login` 和 `grant`（`sylvia-iot-auth/src/routes/oauth2/mod.rs:20,30`、`api.rs:45-46`），所以**兩個模板都沒有跳脫**。
- 透過設定檔換成自訂模板檔也一樣，因為註冊名稱沒變。
- `get_login()` 把 URL 參數 `state` 的原始字串直接放進 `value="{{state}}"`（`api.rs:125`、`template.rs:40`）。

### 觸發路徑

1. 攻擊者準備內層字串：`response_type=code&client_id=<任一公開 client>&redirect_uri=<該 client 已註冊的 URI>&state="><script>…</script>`。
2. 把這段字串 URL 編碼後放進 `GET /auth/oauth2/login?state=…`。
3. `serde_urlencoded::from_str::<GetAuthRequest>` 解析成功，因為 `"`、`<`、`>` 在 form 編碼裡就是一般字元。
4. `check_auth_params()` 只檢查 client、redirect_uri、scope 是否有效。這些值都可以從任何 app 的登入連結取得，所以檢查通過。
5. 原始字串被渲染進 HTML，`"` 跳出屬性，script 在 auth 的網域執行。

### 影響

- 不需要登入，只要受害者點開連結。
- script 跑在真正的登入頁上，可以在送出時讀取帳號密碼並外送。網域是官方的，受害者很難察覺。

### 修正內容

- 模板改名為 `login.html`、`grant.html`，tera 會自動跳脫，自訂模板也一併涵蓋。
- 測試：`GET /authorize` 的 grant 頁輸出中，`state` 的 `"><script>` 會被跳脫成 `&quot;&gt;&lt;script&gt;`。

## AUTH-2.【高】在登入頁的 `state` 塞入 `allow=yes`，可以跳過同意頁，取得受害者的 token

**判定：CONFIRMED（程式碼追蹤，未實測）　狀態：已修正（`1c7a4999c`）**

### 缺陷

- `GET /authorize` 用 `Query::<AuthorizationRequest>` 解析參數，而這個 struct 包含 `allow` 欄位（`request.rs`）。所以 GET 請求帶上 `allow=yes`，就會**直接授權**，不會顯示 grant 頁（`api.rs` 的 `authorize()`）。
- `get_login()` 和 `post_login()` 會用 `GetAuthRequest` 解析 `state` 來驗證，但多出來的未知欄位不會被拒絕。驗證通過後，程式使用的仍是**原始字串**：
  - `get_login()` 把原始字串放進登入頁的 hidden input。
  - `post_login()` 把原始字串直接接進轉址網址：`/authorize?{state}&session_id=…`（`api.rs:210-213`）。
- login session 沒有綁定 client。不論 session 是在哪個 client 的登入流程中建立的，都可以拿來授權任何 client。

### 觸發路徑

1. 攻擊者有 DEV 角色，註冊一個自己的 client EVIL，redirect_uri 指向自己的伺服器，scopes 隨意設定。
2. 攻擊者讓受害者開啟 `GET /auth/oauth2/login?state=<URL 編碼的 response_type=code&client_id=EVIL&redirect_uri=<EVIL 的 URI>&allow=yes>`。
   - 網址在官方網域上，登入頁也不會顯示是哪個 client。
3. 受害者登入。`post_login()` 轉址到 `/authorize?response_type=code&client_id=EVIL&…&allow=yes&session_id=S`。
4. `authorize()` 驗證 client 和 redirect_uri 都通過，而 `allowed()` 是 `Some(true)`，所以直接發出 authorization code，轉址到攻擊者的伺服器。
5. 攻擊者用自己的 client 換到受害者的 access token 和 refresh token。

### 影響

- 預設的 `api_scopes` 是空的，所以 token 擁有受害者角色的**完整 API 權限**。受害者如果是 admin，攻擊者就拿到 admin 的權限。
- 其他服務也都透過 tokeninfo 驗證這個 token，所以 coremgr、broker、data 都能用。
- **修正 AUTH-1（模板跳脫）不會修掉這一項。** 跳脫只影響 HTML 的顯示，送出表單時，`state` 仍是同一個原始字串。

### 修正內容

- `AuthorizationRequest` 在 GET 時一律把 `allow` 設為 `None`，只有 grant 頁的 POST 能授權。
- 新增 `check_login_state()`：解析並驗證 `state` 後，用 `GetAuthRequest` 重新序列化。`get_login()` 和 `post_login()` 都使用重新序列化的字串，未知欄位（例如 `allow`）會被丟掉。
- login session 綁定 client，見 AUTH-7。
- 測試：登入頁與 `post_login()` 的轉址網址都不含 `allow=`；GET 帶 `allow` 仍然回 grant 頁（200）。

## AUTH-3.【中高】grant 頁的反射型與儲存型 XSS

**判定：CONFIRMED（程式碼追蹤）　狀態：已修正（`1c7a4999c`）**

### 缺陷

grant 頁的 `client_name`、`scope`、`state` 都沒有跳脫（`api.rs:319,327,331`、`template.rs:93-103`）。

### 觸發路徑

**a. OAuth `state` 參數（反射型）**
1. 走正常的 `/oauth2/auth?…&state="><script>…` 入口。這一步會正確編碼，所以登入頁是安全的。
2. 受害者登入後，`post_login()` 轉址到 `/authorize?{state}&session_id=…`。
3. GET 解碼後，`state` 又變回原始字串，在 grant 頁沒有跳脫地輸出。

**b. client 名稱（儲存型）**
1. client 的 `name` 完全沒有驗證。有 DEV 角色的使用者可以註冊一個名稱含 script 的 client。
2. 受害者對這個 client 走授權流程時，script 就會執行。
3. script 可以讀到頁面裡的 `session_id`，替受害者自動按下同意。

### 修正內容

- 模板改名後，grant 頁的 `client_name`、`scope`、`state` 都會跳脫（同 AUTH-1）。
- client `name` 的字元限制沒有做，跳脫已經足夠。

## AUTH-4.【中高】停用的使用者仍可繼續使用 token，並無限期 refresh

**判定：CONFIRMED（程式碼追蹤）　狀態：已修正（`1c7a4999c`）**

### 缺陷

- **停用時不撤銷 token：** `patch_admin_user()` 處理 `disable: true` 時只設定 `disabled_at`。只有改密碼時才會呼叫 `remove_tokens()`（`routes/v1/user/api.rs:384`）。
- **驗證時不檢查狀態：** middleware 會查詢使用者，但只檢查角色和 scope，不檢查 `disabled_at` 和 `expired_at`（`routes/oauth2/middleware.rs:313-353`）。
- **其他服務也看不到狀態：** tokeninfo 回傳的資料不含停用狀態（`routes/v1/auth/api.rs`）。
- **refresh 不檢查使用者：** `recover_refresh()` 和 `issue()` 完全不查使用者，每次 refresh 都會發一個新的、效期 14 天的 refresh token（`routes/oauth2/primitive.rs`）。

### 觸發路徑

1. admin 停用帳號 X，例如離職員工或被盜的帳號。
2. X 手上原本的 access token 仍然可以通過 middleware 和 tokeninfo，所以所有服務都還能用。
3. X 定期呼叫 `/oauth2/refresh`，就能**無限期**持有存取權。

`expired_at` 也一樣：帳號到期後不能再登入，但已經發出去的 token 不受影響。

### 修正內容

- `patch_admin_user()` 在改密碼或停用（`disabled_at` 設為有值）時，都會呼叫 `remove_tokens()`。
- middleware 查到使用者後，檢查 `disabled_at` 和 `expired_at`，不符合就回 401。tokeninfo 也走這個 middleware。
- `recover_refresh()` 查詢使用者，使用者不存在、停用或到期時，refresh 回 `invalid_grant`。
- 測試：停用或到期時 tokeninfo 回 401、refresh 回 `invalid_grant`，重新啟用後恢復；透過 API 停用後，舊 token 在重新啟用後仍然無效（已撤銷）。

## AUTH-5.【低中】SQLite 的 login session 永遠不會過期；按「拒絕」時 session 也不會刪除

**判定：CONFIRMED（程式碼追蹤）　狀態：已修正（`1c7a4999c`）**

### 缺陷

- **不檢查效期：** SQLite 的 `login_session.get()` 不檢查 `expires_at`（`models/sqlite/login_session.rs:52-74`），`authorize()` 也不檢查。SQLite 沒有 TTL 機制，所以 session 只有被使用時才會刪除。
- **拒絕時不刪除：** 使用者按「拒絕」時走 `pending.deny()`，session 不會被刪掉。只顯示 grant 頁、但沒有送出的 session，也一樣會留著。
- MongoDB 有 `createdAt` 的 TTL index，大約 2–3 分鐘後會清除，不受影響。

### 影響

在 SQLite 部署下，以下兩個問題的時間窗口從 60 秒變成**無限期**：

- CORE-3：能讀 access log 的人可以拿到 `session_id`。
- CORE-1：`session_id` 只有約 2^24 的熵，而且未使用的 session 會一直累積。

### 修正內容

- `authorize()` 取出 session 後先刪除，再檢查 `expires_at`，過期就拒絕。
- 按「拒絕」時也會先刪除 session。
- SQLite 的 `login_session.add()` 會先刪掉已過期的 session，不需要另外的排程。
- **相容性：** SQLite 的 `login_session` 表新增了兩個 `NOT NULL` 欄位（見 AUTH-7），`init()` 只做 `CREATE TABLE IF NOT EXISTS`。既有的 SQLite 資料庫要手動刪除 `login_session` 表。

## AUTH-6.【低】登入沒有次數限制，而且可以從回應時間判斷帳號是否存在

**判定：CONFIRMED（程式碼追蹤）　狀態：部分修正（`1c7a4999c`，次數限制未做）**

### 缺陷

- `post_login()` 沒有任何次數限制或鎖定機制，可以線上暴力猜密碼。
- 帳號不存在時直接回錯；帳號存在時，會先跑 PBKDF2 才回錯（`routes/oauth2/api.rs`）。兩者的回應時間不同，所以可以從時間判斷帳號是否存在。
- **CORE-2 修正後差距更大：** rounds 從 10000 改成 600000 後，時間差從幾毫秒放大到約上百毫秒，判斷帳號是否存在變得更容易。
- **CPU 用量：** 雜湊已經改在 `spawn_blocking` 裡執行，不會卡住 tokio 的 worker。但大量登入請求同時湧進時，blocking 池會平行跑很多個雜湊，把 CPU 吃滿。

### 修正內容

- 新增 `libs/password.rs`：
  - `hash()`、`verify()` 都在 `spawn_blocking` 裡執行，並用 `Semaphore` 把同時進行的雜湊數量限制在 CPU 數量。
  - 帳號不存在時，用 dummy hash 驗證一次，讓回應時間一致。
- `post_login()` 先驗證密碼，再檢查停用與到期，所以這些狀態也不會從回應時間看出來。
- **沒有處理的部分：** 依帳號或 IP 的登入次數限制。

## AUTH-7.【低】login session 沒有綁定 client，可以拿去授權別的 client

**判定：CONFIRMED（程式碼追蹤）　狀態：已修正（`1c7a4999c`）**

### 缺陷

- login session 只記錄 `user_id`。`authorize()` 只確認 session 存在，不確認它是在哪個 client 的登入流程中建立的。

### 觸發路徑

1. 攻擊者取得受害者在 client A 登入流程中產生的 `session_id`。例如透過 CORE-3 的 access log，或 grant 頁上的 XSS（AUTH-3）。
2. 攻擊者在 session 效期內，用這個 `session_id` 對自己的 client EVIL 呼叫 `POST /authorize`，並帶上 `allow=yes`。
3. 拿到受害者對 EVIL 的授權。

### 修正內容

- `LoginSession` 新增 `client_id`、`redirect_uri`，在 `post_login()` 時記錄。
- `authorize()` 比對請求的 `client_id`、`redirect_uri` 和 session 是否相同，不同就拒絕。
- 測試：公開 client 的 session 不能用在私有 client；同一個 session 只能用一次。

## auth 檢查過、沒有發現問題的部分

- **client API：** 非 admin 只能查、改、刪自己的 client，存取別人的會回 404。
- **user API 的權限：** manager 不能修改 admin，不能指派 admin 或 service 角色，也不能停用 admin 或 manager。刪除使用者只有 admin 可以，而且不能刪自己。
- **SQL：** `sql_builder::quote()` 會把單引號加倍，這是 SQLite 正確的跳脫方式。LIKE 的 `\`、`%`、`_` 也有跳脫，排序欄位是固定的 enum。MongoDB 的查詢都用有型別的 `doc!`。
- **Basic auth 解析：** 先確認有 `"Basic "` 前綴，切片 `[6..]` 才是安全的。
- **轉址：**
  - `Location` header 的值會經過 `HeaderValue` 驗證，不能塞入 CRLF。
  - `post_login()` 的轉址網址固定以 `scope_path` 開頭，不能轉到外部網域。
  - 其他轉址用的 redirect_uri 都必須是 client 已註冊的值。
- **authorization code：** 有檢查效期，取出後就刪除。理論上兩個同時送出的請求可能都換到 token，但前提是攻擊者已經有那個 code。

---

# 四、coremgr

CMGR-1 的根源在 coremgr（把帳密寫進 `hostUri`），但外洩發生在 broker 的 API。CMGR-4 則同時牽涉 coremgr 的 ACL 和文件。

## CMGR-1.【嚴重】broker 的 API 把帶有 MQ 管理員帳密的 `hostUri` 回傳給 unit 成員

**判定：CONFIRMED（程式碼追蹤，未實測）　狀態：已修正（`c3d32c0ed`，broker 端）**

### 缺陷

1. coremgr 建立或修改 application、network 時，`transfer_host_uri()`（`sylvia-iot-coremgr/src/routes/v1/mod.rs:549-567`）會把以下帳密填進 `hostUri`，再送給 broker 存進資料庫：
   - RabbitMQ 的**管理員**帳號和密碼。
   - EMQX 的 API key 和 secret。這組 key 同時也被建成 MQTT superuser（`mod.rs:430-438`）。
2. broker 的 `application_transform()` 和 `network_transform()` 會把 `host_uri` 原樣放進回應（`sylvia-iot-broker/src/routes/v1/application/api.rs:974`、`network/api.rs:1031`）。
3. broker 的讀取權限是 `only_owner=false`，所以 unit 成員就能讀（`broker/src/routes/v1/lib.rs` 的 `check_application()` 和 `check_network()`）。
4. coremgr 自己的回應會用 `trunc_host_uri()` 移除帳密，但 broker 的 API 在 `sylvia-iot-core`（`coremgr/src/bin/sylvia-iot-core.rs:104`）和 `sylvia-router`（`sylvia-router/src/bin/sylvia-router.rs:123`）裡，都和 coremgr 掛在同一個對外的 HTTP 服務上。

### 觸發路徑

1. 任何 unit 成員，用自己的 token 呼叫 `GET /broker/api/v1/application/list` 或 `/network/list`。
2. 回應的 `hostUri` 裡就有 `amqp://<管理員帳號>:<管理員密碼>@host/...`。
3. 拿到管理員帳密後，可以：
   - 登入 RabbitMQ 的管理介面，或直接以 superuser 身分連上 EMQX。
   - 讀取、發布**所有租戶**的資料。
   - 刪除其他租戶的 vhost 或使用者。

### 修正內容

- broker 的 `application_transform()` 和 `network_transform()` 用 `hide_host_uri_credentials()` 去掉 `hostUri` 的帳號和密碼，GET 和 list 都涵蓋，admin 也一樣。path（AMQP 的 vhost）保留。
- PATCH 用 `restore_host_uri_credentials()`：送進來的 `hostUri` 沒帶帳密、而且其他部分（scheme、host、port、path）都和 DB 相同時，沿用 DB 的帳密，視為沒有變更。host 或 vhost 不同時不沿用，避免把管理員帳密送到使用者指定的主機或 vhost。
- coremgr 不受影響：它只讀 broker 回應的 scheme、host、port，換主機時用自己設定檔的帳密組出完整的 URI。
- 測試：3 種角色的 GET 和 list，以及 6 種 PATCH 情況。

### 補充

- 依使用者說明，實際部署時前面會再加一層 reverse proxy，不會把 broker API 直接對外開放。這項修正是讓 broker API 即使被直接存取，也不會洩漏管理員帳密。
- **沒有處理的部分：** 長期來說，broker 連線 MQ 的帳密應該放在 broker 自己的設定檔，不要存在每筆資料裡。

## CMGR-2.【嚴重】沒有套用 MQ hosts 白名單，管理帳密會送到租戶指定的主機

**判定：CONFIRMED（程式碼追蹤，未做 PoC）　狀態：已修正（`a07542aa1`）**

### 缺陷

- 設定檔有 `coremgr.mq.rabbitmq.hosts` 和 `coremgr.mq.emqx.hosts`（`sylvia-iot-coremgr/src/libs/config.rs:59-60,72-73`），文件寫著「可用的主機」，但**程式完全沒有使用**。
- 建立或修改 application、network 時，`create_queue_rsc()`（`routes/v1/mod.rs:365`）會對租戶在 `hostUri` 指定的主機呼叫 MQ 管理 API：
  - RabbitMQ：`libs/mq/rabbitmq.rs:110-113` 用 `http://{hostname}:15672`，並帶上**管理員的 Basic auth**。
  - EMQX：同樣會帶上 api_key 和 secret。
- `transfer_host_uri()`（`routes/v1/mod.rs:549`）也會把管理帳密填進 URI。

### 觸發路徑

1. unit 的擁有者用 `POST` 或 `PATCH` 建立或修改 application/network，把 `hostUri` 指向自己控制的主機。
2. coremgr 用明文 HTTP，把 MQ 管理員帳密送到這台主機。
3. 攻擊者拿到帳密後，就能管理整台 MQ，影響所有租戶。

### 修正建議

- 在 `POST` 和 `PATCH` 檢查 `hostUri` 的主機有沒有在 `hosts` 白名單裡。
- 白名單沒設定時要怎麼處理（不限制，或改成只允許預設主機），需要另外決定。

### 修正內容

- `hosts` 的每一筆是 `{name, host, external, active}`：
  - `host` 是 coremgr 內部連線用的主機，`external` 是給 unit 的 client 連線用的主機，`name` 只用於顯示。
  - `rabbitmq.hosts` 套用到 amqp/amqps，`emqx.hosts` 套用到 mqtt/mqtts。rumqttd 不支援。
- API 的 `hostUri` 收發的都是 `external`。coremgr 轉成內部的 `host` 之後，才交給 broker 和 MQ 管理 API；不在清單裡的主機一律回 `err_param`。
- `active: false` 的主機不能再拿來建立 application/network，也不能把 host 改過去，但現有的資源保留。
- 新增 `GET /coremgr/api/v1/mq/hosts`：列出 `type`、`name`、`external`、`active`，不會露出內部主機。
- 啟動時驗證設定：`name` 不能是空的；`host`、`external` 只能是主機名稱（不含 scheme、port、path），會轉成小寫，也不能重複；IPv6 會補上中括號。設定錯誤時服務無法啟動，CLI 參數或環境變數的 JSON 格式錯誤也一樣。
- 白名單沒設定（空陣列）時不限制，但會記錄 warning log。依使用者決定，這是營運方自己選擇開放。
- 已知限制：port 和 scheme 內外共用，只有主機名稱會轉換。
- 搭配 CMGR-3 的擁有者檢查，unit 成員已經無法觸發。

### 補充：只需要成員權限就能觸發

- `post_application()` 呼叫 `get_unit_inner()`，只要求讀取權限。
- `patch_application()` 呼叫 `get_application_inner()`，也只要求讀取權限。

這兩個函式都在 broker 檢查寫入權限**之前**，就呼叫了 `create_queue_rsc()`，所以 unit 成員就能觸發 CMGR-2，不需要是擁有者。

## CMGR-3.【中】unit 成員可以重設 application/network 的 MQ 密碼，接管它的 MQ 帳號

**判定：CONFIRMED（程式碼追蹤，跨 coremgr 和 broker）　狀態：已修正（`a07542aa1`）**

### 缺陷

`patch_application()` 收到的 body 只有 `password`、`ttl` 或 `length` 時：

1. 只呼叫 `get_application_inner()`，這是 broker 的 GET，只要求成員權限。
2. 完全**不呼叫** broker 的 PATCH（PATCH 才會檢查擁有者權限）。
3. 直接用管理員帳密執行 `rabbitmq::put_user()` 或 `emqx::put_user()`，把 MQ 密碼改成請求指定的值（`sylvia-iot-coremgr/src/routes/v1/application.rs:680-720`）。

`patch_network()` 也是同樣的寫法（`network.rs:700-750`）。

### 觸發路徑

1. unit 成員送出 `PATCH /coremgr/api/v1/application/{id}`，body 是 `{"data":{"password":"attacker"}}`。
2. MQ 帳號 `application.<unit>.<code>` 的密碼被改掉。
3. 攻擊者用新密碼連上 MQ，可以：
   - 讀取這個 application 的 uplink 資料。
   - 對裝置發送 downlink。
4. 原本的 application 用舊密碼，從此連不上。

### 補充

依使用者說明：管理類的 API 只有 unit 擁有者能使用。重設 MQ 密碼屬於管理操作，所以這一項成立。成員該怎麼管理，目前還沒有規劃。

### 修正建議

- 修改密碼、ttl、length 之前，也要先通過 broker 的擁有者檢查。例如先呼叫 broker 的 PATCH，或新增一個檢查權限的 API。
- 所有呼叫 `create_queue_rsc()` 的地方，都要在權限檢查之後才執行。

### 修正內容

- coremgr 新增 `check_unit_owner()`：admin、manager 和 unit 擁有者可以管理 MQ 資源；公開 network（沒有 unit）只有 admin 和 manager 可以。
- application/network 的 POST 和 PATCH 都在建立 MQ 資源、修改密碼之前檢查。非擁有者的成員回 400 `err_param`（訊息 `the user is not the unit owner`）。unit 不存在、或使用者不是成員時，回應不變（POST 是 `err_broker_unit_not_exist`，PATCH 是 404）。
- 測試：成員的 POST 會使用沒有 MQ 的主機，確認錯誤是在建立資源之前回傳的；PATCH 測試則包含公開 network。

### 嚴重度調整與延後事項

- **嚴重度從「高」調降為「中」：** broker 只允許 admin 和 manager 修改 `memberIds`（`sylvia-iot-broker/src/routes/v1/unit/api.rs` 的 `get_updates()`），擁有者和 GUI 都無法加入成員。所以前提是「admin 主動加入的成員濫用權限」。一旦成立，仍然可以接管 MQ 帳號，所以不降到「低」。
- **成員權限的語意延後到 0.5 的 unit member management：** 非擁有者暫時回 `err_param`，程式中標註 `TODO: member management`。broker 寫入 API 對非擁有者的回應不一致（POST 回 `*_not_exist`、PATCH 回 404、DELETE 回 204 但什麼都沒刪），也留到那時一起處理。

## CMGR-4.【高】EMQX 上的租戶隔離依賴 EMQX 的安全設定，6.3 之前的預設等同允許所有存取

**判定：PLAUSIBLE（EMQX 官方設定檔的說明支持；未在 legacy 模式下實測）　狀態：已修正（`a07542aa1`，文件）**

### 補充：查看 EMQX 6.3.0 容器的結果（2026-11-01，唯讀查詢）

- `authorization.no_match` 是 `deny`。授權來源依序是 `file` 和 `built_in_database`。
- EMQX 預設的 `acl.conf` 最後一條規則是 `{allow, {security_profile, legacy}}`。官方的註解說明：
  - `EMQX_SECURITY_PROFILE=legacy`（**EMQX 6.3 之前的預設**）時，這條規則生效，效果等同允許所有存取。
  - `hardened`（6.3 以上的預設）時，這條規則不生效，改用 `no_match`。
- 註解也寫著：正式部署時要把最後一條改成 `{deny, all}`，並設定 `no_match = deny`。
- 所以這個問題的影響範圍是：
  - EMQX 6.3 以上、維持預設的 hardened：安全。
  - EMQX 6.3 之前，或明確設定為 legacy：任何租戶都能存取所有 topic。
- 使用者文件 `quick.md` 指定的版本是 **`EMQX_VER=6.0.1`**，正好落在有問題的範圍。

不需要認證就能測：EMQX 的授權會依照 client 自帶的 username 判斷。不過要重現有問題的情況，必須把容器改成 legacy 模式，所以沒有在使用者的容器上實測。

### 原本的分析

### 缺陷

- coremgr 替每個 application 或 network 建立的 EMQX ACL 只有 **allow** 規則，沒有 deny 規則（`sylvia-iot-coremgr/src/libs/mq/emqx.rs:238-283`）。
- 所以「租戶不能存取別人的 topic」這件事，完全依賴 EMQX 的全域設定 `authorization.no_match = deny`，而且不能有更寬鬆的 ACL 來源。
- CI 用的 `.github/workflows/files/emqx.conf:58` 有設定 `no_match = deny`。
- 但使用者文件（`documentation/book/src/guide/quick.md`、`configuration.md`）完全沒有提到這個設定。quick.md 用的還是 EMQX 4 的環境變數。
- EMQX 5/6 的預設是 `no_match = allow`。預設的 `acl.conf` 最後一條是 `{allow, all}`，只禁止訂閱字面上的 `#`。

### 觸發路徑（待 PoC 確認）

1. 營運方照著文件部署 EMQX，沒有改 `no_match`。
2. 租戶用自己的 MQ 帳號連上 EMQX，訂閱 `+`。sylvia-iot 的 topic 都是單層名稱（用 `.` 分隔、不含 `/`），所以 `+` 會匹配所有 topic。
3. 租戶就能收到所有租戶的 uldata 和 dldata。也可以往別人的 topic 發布訊息，冒充其他 network 或 application。

### 修正建議

- coremgr 在每個使用者的規則最後，加上一條 deny all（`topic: "#"`、`action: "all"`、`permission: "deny"`）。這樣就不依賴全域設定。
- 文件註明 EMQX 必須設定 `authorization.no_match = deny`，並更新成 EMQX 5/6 的設定方式。

### 修正內容（只改文件）

- `quick.md`（中英文）改用 `EMQX_VER=6.3.3`，並註明建議使用 6.3.0 以上。
- `configuration.md`（中英文）的 coremgr 詳細說明：
  - 建議 EMQX 6.3.0 以上，使用預設的 hardened security profile。
  - 較早的版本或 legacy 模式，必須設定 `authorization.no_match = deny`，並把 `acl.conf` 最後一條改成 `{deny, all}`。
- CHANGELOG 加上這個建議。

### 為什麼沒有採用「coremgr 加上 deny all 規則」

EMQX 會依序評估授權來源，`file`（`acl.conf`）排在 `built_in_database` 之前。legacy 模式下，`acl.conf` 的 `{allow, ...}` 會先匹配，coremgr 寫在 built-in database 的 deny 規則根本輪不到，所以只改程式沒有效果。

## CMGR-5.【中】未登入就能送出超大 request body，讓 coremgr 的記憶體耗盡

**判定：CONFIRMED（程式碼追蹤，未實測）　狀態：已修正（`a07542aa1`）**

### 缺陷

- `api_bridge()` 用 `body::to_bytes(body, usize::MAX)` 把整個 body 讀進記憶體，然後才轉發給 auth 或 broker（`routes/v1/mod.rs:77`）。
- 這發生在任何驗證**之前**，所有直接轉發的 API 都受影響，例如 `PATCH /coremgr/api/v1/user`、`POST /auth/logout`。
- 有設定 data channel 時，外層的 `LogMiddleware` 對所有 DELETE、PATCH、POST、PUT 請求也做同樣的事（`routes/middleware.rs:409`）。
- axum 預設的 2 MB 上限只對 extractor 有效，對這種讀法沒有作用。bin 也只有 60 秒的 `TimeoutLayer`，沒有 body 大小限制。

### 觸發路徑

不帶 token，同時開多條連線，各自送出 GB 等級的 body。60 秒內就能把記憶體耗盡。

### 修正建議

- `to_bytes()` 改成合理的上限，例如 2 MB。
- 或是在 bin 加上 `RequestBodyLimitLayer`。

### 修正內容

- `api_bridge()` 和 `LogMiddleware` 的 `to_bytes()` 都改用 `BODY_LIMIT`（2 MiB），和 axum extractor 的預設上限一致。
- 超過上限時回 400 `err_param`，和 corelib 的 `Json` extractor 一致（axum 原生是 413，但 corelib 會把所有 extractor 錯誤轉成 `err_param`）。
- 測試：middleware 測 2 MiB 通過、2 MiB + 1 被拒；bridge 的 API 用 `POST /auth/logout` 測。

## CMGR-6.【低】用 REST API 發布資料時沒有大小限制

**判定：CONFIRMED（PoC，EMQX 6.3.0，2026-11-01）　狀態：已修正（`a07542aa1`）**

### PoC 結果

測試方式：模仿 coremgr 的 `emqx::publish_message()`，呼叫 `POST /api/v5/publish`（base64、QoS 0），同時用一個 MQTT 3.1.1 的 QoS 1 訂閱端接收。測試腳本是一次性的，沒有放進 repo。

| payload 大小 | REST 回應 | 訂閱端 |
|---|---|---|
| 512 KiB | 200 | 收到 |
| 1 MiB - 1 KiB | 200 | 收到 |
| 1 MiB + 1 KiB | **400 `packet_too_large`** | 沒收到 |
| 1.5 MiB | **400 `packet_too_large`** | 沒收到 |

EMQX 從 REST API 發布時，**也會**檢查 `mqtt.max_packet_size`（預設 1 MB），所以原本擔心的「REST API 可以繞過大小限制」不成立。只要 EMQX 的 `max_packet_size` ≤ 1 MiB，broker 的共用連線就不會被超大訊息打斷。

### 剩下的風險

- 營運方如果把 EMQX 的 `max_packet_size` 調到超過 1 MiB，超大訊息就會被轉送給 broker，讓共用連線斷線重連。
- 文件仍需要註明 EMQX 的 `max_packet_size` 必須 ≤ 1 MiB，位置在 `configuration.md` 的 coremgr 詳細說明。

### 原本的分析（保留作為紀錄）

### 缺陷

`post_application_dldata()`（`routes/v1/application.rs:958`）和 `post_network_uldata()`（`routes/v1/network.rs:948`）只檢查 `data` 是不是合法的 hex，然後就透過 EMQX 或 RabbitMQ 的 REST API 發布。axum 預設允許約 2 MB 的 body。

### 待驗證

EMQX 從 REST API 發布的訊息，可能不受 MQTT 入站 `max_packet_size` 的限制。需要確認：EMQX 是否會把超過 1 MiB 的訊息轉送給 MQTT 3.1.1 的 subscriber。

### 影響

如果會轉送：

- 修正前：會觸發 MQ-1，造成跨租戶斷線。
- 修正後：入站上限是 1 MiB。超過 1 MiB 的封包仍會讓共用連線斷線重連，因為 rumqttc 收到超過入站上限的封包時會斷線。所以這仍然是跨租戶 DoS，只是門檻從 10 KiB 提高到 1 MiB。

### 修正建議

- coremgr 在發布前檢查 payload 長度，上限和 broker 的 `MAX_DATA_MSG_SIZE` 一致。
- 文件註明 EMQX 的 `mqtt.max_packet_size` 必須 ≤ 1MB，寫在 `configuration.md` 的 coremgr 詳細說明。

### 修正內容

- `POST /application/{id}/dldata` 和 `POST /network/{id}/uldata` 的 JSON payload 超過 broker 的 `MAX_DATA_MSG_SIZE`（1 MiB - 16 KiB）時，回 400 `err_param`（`data too large, ...`）。
- `configuration.md`（中英文）註明 EMQX 的 `mqtt.max_packet_size` 必須 ≤ 1 MiB。

## CMGR-7.【低】MQ 資源的清理程式沒有 `await`，從來沒有執行過

**判定：CONFIRMED（程式碼追蹤）　狀態：已修正（`a07542aa1`）**

### 缺陷

- `clear_queue_rsc()` 是 async 函式，但以下 12 處都寫成 `let _ = clear_queue_rsc(...)`，future 建立後就被丟掉，從來沒有執行：
  - `routes/v1/mod.rs:389,396,411,450,457,543`
  - `application.rs:238,247,272`
  - `network.rs:252,261,286`
- 另外，`post_application()` 和 `post_network()` 在 broker 回傳非 200 時，完全沒有呼叫清理程式。

### 影響

建立或修改失敗時，MQ 上會留下沒用的使用者、vhost 和 ACL。如果搭配 CMGR-2，攻擊者指定的主機上也會留下這些資源。

### 修正建議

- 補上 `.await`。
- broker 回傳非 200 時也要清理。

### 修正內容

- 12 處都補上 `.await`。
- `post_application()` 和 `post_network()` 在 broker 回傳非 200 時，也會清除剛建立的資源。

### 補充

- 使用者提出：原本不 await，可能是怕清除太久導致 API timeout。不過 future 不 await 就完全不會執行，並不是在背景清除。清除只有 2～3 個 MQ 管理 API 呼叫，比同一個請求裡已經 await 的建立（最多 9 個）還少，而且只出現在錯誤路徑，所以維持 `.await`。
- coremgr 的 reqwest client 沒有設定 timeout，MQ 管理 API 卡住時請求會一直掛著。這已經列在待辦清單的「(0.5) (HTTP) client timeout」，做的時候要用 `connect_timeout` 加 `read_timeout`，不能用 `timeout()`，否則 CSV/list 的串流轉發會被切斷。

## CMGR-8.【不適用】內建的 rumqttd 允許匿名連線、沒有 ACL

**判定：不適用　狀態：不修正**

> 依使用者說明：rumqttd 只用於 demo/PoC，不能用於正式部署，sylvia-iot 不為此負責。只有整合版的 bin（`sylvia-iot-core`）內建 rumqttd，個別服務的 bin 刻意不包含。正式部署只考慮 EMQX 等正式的 MQTT broker。以下內容只作為紀錄保留。

### 缺陷

- rumqttd 的 listener 綁在 `0.0.0.0`，設定是 `auth: None`、`external_auth: None`（`sylvia-iot-coremgr/src/libs/mq/rumqttd.rs:45-61,89-96`）。
- coremgr 在 rumqttd 模式下，`create_queue_rsc()` 和 `clear_queue_rsc()` 什麼都不做（`routes/v1/mod.rs:462,521`），不會建立任何帳號或 ACL。
- console 也綁在 `0.0.0.0:18083`。

### 觸發路徑

任何連得到 1883 port 的人，都可以匿名連上 rumqttd，然後：

- 訂閱 `#`，讀取所有租戶的 uplink 和 downlink。
- 往 `broker.network.<unit>.<code>.uldata` 發布資料，冒充任何 network 送假的 uplink。
- 往 application 的 dldata topic 發布資料，冒充任何 application 對裝置下指令。

### 修正建議

- 如果 rumqttd 只打算用於單一租戶的邊緣部署（例如 sylvia-router），至少預設只綁 `127.0.0.1`，並在文件註明不能用於多租戶。
- 否則要啟用 rumqttd 的帳密驗證。

## CMGR-9.【不適用】unit 成員可以透過 API 發布 dldata 或 uldata

**判定：不適用　狀態：不修正**

> 依使用者說明：unit 成員本來就可以直接透過 MQ，或間接透過 uldata/dldata API 發送資料，這是設計允許的。只有管理類的 API（建立、修改、刪除，包含重設 MQ 密碼）限定 unit 擁有者。以下內容只作為紀錄保留。

### 缺陷

`post_application_dldata()`（`application.rs:958`）和 `post_network_uldata()`（`network.rs:948`）都只檢查讀取權限（`get_*_inner()`），就用管理員帳密，透過 MQ 的 REST API 發布訊息。

### 觸發路徑

1. 擁有者建立 application 時，coremgr 只把 MQ 密碼回傳給擁有者，所以正常情況下只有擁有者能用這個 application 送資料。
2. 但只有讀取權限的 unit 成員，可以透過這兩個 API：
   - 以 application 的身分，對 unit 內的裝置下 downlink。
   - 以 network 的身分，送出假的 uplink。

如果「成員可以發送資料」本來就是設計上允許的，這一項可以降級。需要你確認。

### 修正建議

改用擁有者權限檢查。

## CMGR-10.【中】unit 成員可以透過 DELETE API 清除 application、network，甚至整個 unit 的 MQ 資源

**判定：CONFIRMED（程式碼追蹤，跨 coremgr 和 broker）　狀態：已修正（`a07542aa1`）**

> coremgr 那一輪（2026-11-21）實作 CMGR-3 時發現。行號以 `c3d32c0ed` 為準。

### 缺陷

`delete_application()`（`sylvia-iot-coremgr/src/routes/v1/application.rs:738-780`）和 `delete_network()`（`network.rs:765-810`）的處理順序：

1. 用使用者的 token 呼叫 `get_*_inner()`，這是 broker 的 GET，只要求成員權限。
2. 轉送 broker 的 DELETE，只要回應 `is_success()`，就執行 `clear_queue_rsc()`。
3. broker 的 DELETE 對非擁有者的處理是：`check_application(..., true)` 找不到資料，就**直接回 204，什麼也沒刪**（`sylvia-iot-broker/src/routes/v1/application/api.rs:787`，network 也一樣）。
4. coremgr 把 204 當成成功，刪掉這個 application 或 network 的 MQ 帳號、vhost 和 ACL。broker 的資料還在，但連線帳號已經不存在。

`delete_unit()`（`unit.rs:201-245`）更嚴重：它在轉送 broker **之前**，就用成員的 token 列出整個 unit 的 application 和 network（`delete_application_resources()`、`delete_network_resources()`，`unit.rs:234-241`），逐一刪除它們的 MQ 資源，之後 broker 才回 204。

### 觸發路徑

1. unit 成員送出 `DELETE /coremgr/api/v1/unit/{unitId}`。
2. 這個 unit 所有 application 和 network 的 MQ 帳號都被刪除。
3. 所有裝置和應用都斷線。broker 的資料不變，要等擁有者逐一重設密碼或重建，才能恢復。

只刪單一 application 或 network 時，用 `DELETE /coremgr/api/v1/application/{id}` 或 `/network/{id}` 即可。

### 修正建議

- 清除 MQ 資源之前，先用 `check_unit_owner()` 確認使用者是擁有者、admin 或 manager。
- 非擁有者的請求只轉送給 broker，回應由 broker 決定，coremgr 不處理 MQ 資源。
- broker 的寫入 API 對非擁有者的回應不一致：POST 回 `*_not_exist`、PATCH 回 404、DELETE 回 204 但什麼也沒刪。DELETE 的靜默 204 是這一項的根本原因之一。這部分和成員權限的語意一起討論（見 0.5 的 member management），不在這一項處理。

### 修正內容

- `delete_application()`、`delete_network()`、`delete_unit()` 在清除 MQ 資源之前，先用 `check_unit_owner()` 檢查。
- 非擁有者的請求照樣轉送給 broker，回應由 broker 決定（目前是 204），coremgr 不會碰 MQ 資源。之後 broker 的語意改了，coremgr 會自動跟上。
- 測試：成員對 application、network、unit 送出 DELETE 之後，用該帳號登入 RabbitMQ，確認資源還在；擁有者再刪除，回 204。公開 network 的部分，成員會拿到 404，manager 可以刪除。

### 嚴重度調整與延後事項

嚴重度從「高」調降為「中」，理由同 CMGR-3：成員只能由 admin 或 manager 透過 API 加入。成員權限的語意延後到 0.5 的 unit member management。

## coremgr 檢查過、沒有發現問題的部分

- **刪除順序：** 刪除 application、network 時，先讓 broker 刪除（含擁有者檢查），成功後才清除 MQ 資源。
- **MQ 帳號名稱：** 格式是 `{type}.{unit_code}.{code}`。code 不能包含 `.`，所以不同的 application 或 network 不會產生相同的名稱。
- **管理 API 的 URL：** 組 URL 用的使用者名稱來自 code，已受 regex 限制。主機名稱的問題已列在 CMGR-2。
- **operation log：** 送進 data channel 前，`data.password` 會先被清空。

---

# 五、broker

## BRK-1.【高】range API 的範圍檢查有整數截斷，任何登入的使用者都能讓 broker 卡住並耗盡記憶體

**判定：CONFIRMED（程式碼追蹤，未實測）　狀態：已修正（`c3d32c0ed`）**

### 缺陷

範圍檢查寫成 `(end_addr - start_addr) as usize >= BULK_MAX`。`u128` 轉成 `usize` 時，在 64 位元系統上高位元會被截掉。所以差值是 `2^64 + 5` 時會變成 `5`，通過檢查。

同樣的寫法有 4 處：

- `sylvia-iot-broker/src/routes/v1/device/api.rs:717`（`post_device_range`）
- `device/api.rs:854`（`post_device_range_del`）
- `device_route/api.rs:616`（`post_device_route_range`）
- `device_route/api.rs:758`（`post_device_route_range_del`）

後面的 `for addr in start_addr..end_addr` 迴圈，每一圈都把一個字串 push 進 `Vec`，中間沒有 `.await`。所以它會一直占用一個 tokio worker，直到記憶體耗盡。

### 觸發路徑

1. 任何登入的使用者，都能用 `POST /broker/api/v1/unit` 建立自己的 unit（`unit.post` 沒有角色限制），再建立一個 network。
2. 呼叫 `POST /broker/api/v1/device/range`：
   - `startAddr` 是 `00000000000000000000000000000000`。
   - `endAddr` 是 `00000000000000010000000000000005`。
3. 檢查通過後，迴圈要跑 2^64 次以上。broker 的 CPU 被占住，記憶體也持續增加到耗盡。所有租戶的 API 和資料轉送都會受影響。

另外，`endAddr` 如果是全 `f`，`end_addr += 1` 會溢位：debug build 會 panic，release build 會繞回 0。

### 修正內容

- `BULK_MAX` 改成 `u128`，範圍檢查直接比較 `end_addr - start_addr >= BULK_MAX`，bulk API 的筆數檢查改成 `len() as u128`。
- 拿掉 `end_addr += 1`，迴圈改成 `start_addr..=end_addr`，結尾位址全是 `f` 時不會溢位。
- 測試：4 個 range API 都加了超過 1024 個位址、2^64+5 的案例；也加了建立和刪除 `ff…fe`～`ff…ff` 的測試。

## BRK-2.【低】dldata-result 沒有檢查這筆資料是否屬於回報的 network

**判定：PLAUSIBLE（缺少檢查已確認，但實際上猜不到 dataId）　狀態：已修正（`c3d32c0ed`）**

### 缺陷

`on_dldata_result()` 用 `dataId` 取得 dldata buffer 後，沒有比對 buffer 的 `network_id` 是不是回報的這個 network（`sylvia-iot-broker/src/routes/v1/network/api.rs:1778`）。

### 觸發路徑

1. network B 送出 `dataId` 屬於 network A 的 dldata-result。
2. broker 就會把偽造的結果轉給 application，並刪除這筆 buffer。

`dataId` 是毫秒時間戳加上 12 個隨機英數字（約 2^71 種組合），沒有外洩的話實際上猜不到。

### 修正內容

- `on_dldata_result()` 比對 `dldata.network_id` 和回報的 network，不同就寫 warn log 並忽略：不轉給 application，也不刪除 buffer。
- 送到 data channel 的紀錄照樣送出，留作稽核。
- 測試：routing 測試中讓 public network 用 manager 的 dataId 回報，application 不會收到，之後 manager network 回報仍然正常。

## broker 檢查過、沒有發現問題的部分

- **auth middleware：** 每個請求都向 auth 查 tokeninfo，沒有快取。角色和 scope 的檢查與 auth 一致。
- **權限矩陣：** application、network、device、device-route、network-route、dldata-buffer：
  - 讀取用 `only_owner=false`。
  - 建立、修改、刪除用 `only_owner=true`。
  - unit 的擁有者和成員只有 admin/manager 能改。
  - 非 admin 查 list 時一定要帶 `unit`。
  - public network 只有 admin/manager 能用。
- **unit 一致性：**
  - device 的 `check_network()` 要求 private network 必須屬於同一個 unit。
  - `post_device_route()` 和 `post_network_route()` 都會檢查 unit 是否相同。
  - 批次和 range 的 device route 沒有檢查 application 和 network 是否在同一個 unit。但非 admin 必須同時是兩個 unit 的擁有者，所以不會跨租戶，只是資料一致性的問題。
- **dldata 的授權：** application 只能對自己 unit 內的裝置發送。用 deviceId 時以 `unit_id` 查詢，用 network code 加 addr 時以 `unit_code` 查詢。
- **SQL：** 所有值都經過 `quote()`，`IN` 清單也有逐項 quote。排序欄位是固定的 enum。MongoDB 的查詢都用 `doc!`。

---

# 六、data

## DATA-1.【嚴重】data 服務的 unit 檢查可以用空字串 `?unit=` 繞過，任何登入的使用者都能讀取所有租戶的資料

**判定：CONFIRMED（程式碼追蹤，未實測）　狀態：已修正（`a5cda738a`）**

### 缺陷

`get_unit_cond()`（`sylvia-iot-data/src/routes/v1/mod.rs`）只有在 `unit` 參數**完全沒帶**（`None`）時，才會檢查使用者是不是 admin/manager。

`unit` 是空字串時，會走到 `0 => Ok(None)`，回傳「不篩選 unit」的條件，完全沒經過角色檢查：

```rust
match query_unit {
    None => { if !ADMIN && !MANAGER { return Err("missing `unit`") } Ok(None) }
    Some(unit_id) => match unit_id.len() {
        0 => Ok(None),          // ← 非 admin 也會走到這裡
        _ => { /* 用 broker 檢查 unit 讀取權限 */ }
    },
}
```

broker 的同類檢查（例如 `device/api.rs` 的 list）會同時擋下 `None` 和空字串，data 服務只擋了 `None`。

### 影響範圍

`application-uldata`、`application-dldata`、`network-uldata`、`network-dldata` 這 4 組的 `/count` 和 `/list` API，都使用 `get_unit_cond()`。這些路由只要求有效的 token，**沒有任何角色限制**。

### 觸發路徑

1. 任何登入的使用者，例如剛註冊的一般使用者，呼叫 `GET /data/api/v1/network-uldata/list?unit=&limit=0`。
2. `serde_urlencoded` 會把 `unit=` 解析成 `Some("")`，於是 `unit_cond` 是 `None`，查詢條件不篩選 `unit_id`。
3. 回傳**所有租戶**的資料紀錄，包含：
   - 裝置的 uplink 和 downlink payload（`data`）。
   - `extension`。
   - unit、network、device 的 ID 和 code。

### 修正建議

空字串也要套用和 `None` 相同的檢查：非 admin 就回錯。

可以參考 broker 的寫法：先檢查角色，再處理 `None` 和空字串。

### 修正內容

- `get_unit_cond()` 先用 `query_unit.filter(|u| u.len() > 0)` 把空字串視為沒帶 `unit`，再檢查角色：只有 admin 和 manager 可以查所有 unit，一般使用者回 400 `err_param`（`missing unit`）。符合 api.md 的「`unit`: required for normal user」與「manager acts as admin to view all data」。
- 帶 unit ID 時不變，仍用使用者的 token 呼叫 broker 檢查讀取權限，成員可以讀。
- 測試：4 組 uldata/dldata 的 count 和 list，各加上擁有者和成員帶 `unit=` 被拒的情況；manager 帶 `unit=` 查全部的既有測試照常通過。

## data 檢查過、沒有發現問題的部分

- **coremgr operation log：** 非 admin 一律強制只能查自己的 `user_id`，沒有繞過的方式。
- **unit 讀取權限：** 帶入一般的 unit ID 時，會用使用者自己的 token 呼叫 broker 的 `GET /unit/{id}` 檢查，所以成員可以讀，非成員會被擋下，符合權限模型。
- **`unit` 參數被接進 URL 路徑：**
  - `unit` 沒有編碼就直接放進 broker 的 URL，例如 `../application/x` 或 `mine?x`。URL 正規化後確實會打到別的 API。
  - 但資料庫的篩選條件用的是原始字串，做完全比對，所以查不到任何資料，不會越權。
- **data channel 的接收端：** 時間解析都用 `match` 處理，沒有會 panic 的路徑。
  - 誰能發布到 data channel，取決於 MQ 的隔離。EMQX 的部分見 CMGR-4。
- **SQL：** 字串值都經過 `quote()`，時間只用整數，排序欄位是固定的 enum。

---

# 七、sdk

沒有發現問題。

## sdk 檢查過、沒有發現問題的部分

- **MQ 接收端：** 收到 broker 的 dldata、ctrl、uldata、dldata-resp、dldata-result 時，hex 和時間都用 `match` 解析，沒有會 panic 的路徑。`unwrap()` 只用在 lock 上。
- **送出：** `send_*()` 已經加上 `check_msg_size()` 的同步檢查（見 MQ-1）。
- **HTTP client：** 用 client credentials 取得 token。收到 401 時只重試一次，不會無限重試。沿用 reqwest 預設的 TLS 驗證。
- **auth middleware：** 每次都向 auth 查 tokeninfo，沒有快取。

---

# 八、coremgr-cli

這兩項都只影響執行 CLI 的那台機器。

## CLI-1.【低】token 存放檔的權限通常是 `0644`，其他本機使用者都能讀

**判定：CONFIRMED（程式碼追蹤）　狀態：已修正（`2d730dff6`）**

### 缺陷

`write_storage()` 用 `fs::write()` 寫入 `~/.sylvia-iot-coremgr-cli.json`（`sylvia-iot-coremgr-cli/src/libs/cli/config.rs:23-26`）。這個檔案存著 access token 和 refresh token（效期 14 天）。

`fs::write()` 建立檔案時，權限是 `0666` 再套用 umask。常見的 umask 是 `022`，所以檔案權限會是 `0644`。

### 觸發路徑

在多人共用的機器上，其他本機使用者可以讀這個檔案，拿 refresh token 持續換新的 access token，冒用 CLI 使用者的身分。

### 修正建議

在 Unix 上，用 `OpenOptions` 搭配 `mode(0o600)` 建立檔案。

### 修正內容

- `write_storage()` 改用 `write_private_file()`：Unix 上用 `OpenOptions` 加 `mode(0o600)` 建立檔案，寫入前再 `set_permissions(0o600)`。`mode` 只對新檔案有效，所以舊版留下的 `0644` 檔案下次寫入時也會修正。非 Unix 維持 `fs::write()`。
- 驗證：在 umask 022 下，新檔案是 `600`，原本 `644` 的檔案寫入後變成 `600`。coremgr-cli 沒有測試，CI 只做 build，所以沒有覆蓋率報告。

## CLI-2.【低】密碼只能從命令列參數 `--password` 傳入

**判定：CONFIRMED（程式碼追蹤）　狀態：已修正（`2d730dff6`）**

### 缺陷

`login` 子命令的 `--password` 是必填參數，沒有其他輸入方式，例如互動式輸入或環境變數（`sylvia-iot-coremgr-cli/src/libs/cli/login.rs:64-70`）。

### 觸發路徑

密碼會出現在 shell 的歷史紀錄裡。執行期間，其他本機使用者也能用 `ps` 看到。

### 修正建議

- `--password` 改成選填。
- 沒帶的時候，改用不顯示字元的互動式輸入（例如 `rpassword`）。

### 修正內容

- `--password` 改成選填。沒帶的時候用 `rpassword::prompt_password()` 從終端機讀取，不顯示字元。新增依賴 `rpassword` 7.5.5。
- `-p` 仍然可以用，文件裡的 `login -a admin -p admin` 和現有腳本不受影響。

## 其他 coremgr-cli 修正（一併完成）

- **`coremgr-cli.*` 命令列參數沒有註冊：** `read_args()` 會讀 `--coremgr-cli.auth` 等參數（文件也有列出），但從第一個 commit 開始就沒有註冊到 clap。debug build 不帶 `-f` 時，clap 的 debug 檢查會 panic；release build 則是命令列參數無效，只能用環境變數。現在補上 `reg_args()`。
- **移除沒有用到的 `serde_with` 依賴：** 從第一個 commit 就存在，但 coremgr-cli 從來沒用過，可能是從 auth 複製依賴清單時帶進來的。

## coremgr-cli 檢查過、沒有發現問題的部分

- **登入流程：** 從伺服器的 Location 取出 code 後，換 token 的請求一律送往設定好的 auth 位址，不會跟著伺服器的 Location 跑。
- **CSV 輸出：** 檔名來自使用者自己的參數。
- **URL 設定：** auth、coremgr、data 的 URL 都經過 `is_uri()` 檢查。

---

# 九、router（視情況處理）

依使用者說明，router 是 IP 分享器的案子，和 IoT 本身沒有關聯，不會用 K8S 部署，所以這一節的問題要不要修，視情況決定。

## RTR-1.【中】router 的管理 API 沒有角色檢查，任何使用者都能修改 WAN/LAN/WiFi 設定，並讀出 PPPoE 和 WiFi 的密碼

**判定：CONFIRMED（程式碼追蹤）　狀態：視情況**

### 缺陷

`routes/v1/net/mod.rs` 和 `routes/v1/sys/mod.rs` 的所有路由，都只用 SDK 的 `AuthService`。它只檢查 token 是否有效，沒有任何角色或 scope 的檢查。

### 觸發路徑

任何持有有效 token 的使用者，不需要是 admin，可以：

- `GET /router/api/v1/net/wan`：回應的 `conf.pppoe` 包含 PPPoE 的**帳號和密碼**。
- `GET /router/api/v1/net/wlan`：回應包含 WiFi 的**密碼**。
- `PUT /router/api/v1/net/wan/{id}`、`PUT /lan`、`PUT /wlan`、`PUT /wwan`：直接修改網路設定，例如改掉 PPPoE 帳密讓整台 router 斷線，或改 LAN 的 DNS 和 gateway。

### 修正建議

- 這些路由比照 auth 和 broker，加上 admin 的角色檢查。
- 或者至少把 PUT 和含密碼的 GET 限定給 admin。

## RTR-2.【中】router 的所有服務都綁在 `[::]`，包含 WAN 網卡，而 repo 沒有任何防火牆設定或說明

**判定：PLAUSIBLE（程式碼已確認；實際影響取決於主機的防火牆）　狀態：視情況**

### 缺陷

`bin/sylvia-router.rs` 把 HTTP 和 HTTPS 服務都綁在 `[::]`，也就是所有網卡（dual-stack）。這個 bin 內含 auth、broker、coremgr、data 和 router 的管理 API。

repo 的程式和文件都沒有設定防火牆（iptables、nftables、ufw），而 Ubuntu 預設也不啟用防火牆。

### 觸發路徑

照預設部署時，WAN 那一端（也就是網際網路）可以直接連到：

- OAuth2 登入頁：AUTH-1、AUTH-2 修正前可以從外部攻擊（已修正）。
- 管理 API：搭配 RTR-1，任何帳號只要密碼外洩，就能從外部改掉 router 的設定。
- broker、data 的 API：DATA-1、BRK-1 等問題修正前也能從外部觸發（都已修正）。

### 修正建議

- 預設只綁 LAN 的位址。
- 或者在文件說明，WAN 端必須用防火牆擋掉服務用的 port。

## router 檢查過、沒有發現問題的部分

- **指令注入：** `nmcli` 和 `systemctl` 都用 `Command::args()` 逐一傳入參數，沒有經過 shell，所以不會有 shell 注入。每個值都是 nmcli 的 key 後面那個「值」，無法塞入額外的屬性。
- **dhcpd 設定檔：** `dhcpStart`、`dhcpEnd` 會解析成 `Ipv4Addr`，`address` 會解析成 `Ipv4Net`，`leaseTime` 是 `usize`，所以 `/etc/dhcp/dhcpd.conf` 不能被注入換行或額外的設定。
- **靜態 IP：** 位址、gateway、DNS 都會解析成 IPv4 位址，而且排除 broadcast、loopback 等特殊位址。
- **WiFi：** channel 限制在 1–11。SSID 和密碼是用 nmcli 的參數傳入，不會被當成指令解析。

## 附錄：Ubuntu 22.04 上 PPPoE 常斷線重撥（非安全問題，未實測）

router 本身**沒有**任何定期操作網路的程式，只有呼叫 API 時才會刪除再重建 WAN 連線。所以斷線不是 router 的程式主動造成的，比較可能是 NetworkManager（NM）的設定或系統環境。

`set_wan_conf()` 建立 PPPoE 連線的方式是 `nmcli c add type pppoe con-name <wan_id> ifname <實體網卡> username … password …`（`libs/network.rs`），沒有設定其他參數。依可能性排序的推測如下：

1. **同一張網卡上還有其他自動連線的 ethernet profile。**
   - 情況：`ifname` 直接指定實體網卡，是 NM 的舊式（legacy）PPPoE 模式。這時 PPPoE 和網卡本身的生命週期綁在一起。
   - 原因：`set_wan_conf()` 只刪除名為 `wan_id` 的 profile。如果系統裡還有 `Wired connection 1` 或 netplan 產生的 `netplan-<網卡>` 等 ethernet profile，而且開著自動連線，NM 可能在 PPPoE 短暫中斷，或網卡 carrier 變化時，改去啟用那個 profile（跑 DHCP），把 PPPoE 擠掉，之後再重撥。
   - 檢查：`nmcli -f NAME,TYPE,DEVICE,AUTOCONNECT c show`。
2. **LCP echo 逾時。**
   - 情況：NM 的 PPPoE 預設會定期送 LCP echo，連續沒有回應就判定斷線。如果 ISP 的設備回應慢，或線路偶爾掉封包，pppd 就會主動斷線再重撥。
   - 檢查：在 `journalctl -u NetworkManager -b` 裡找 `No response to N echo-requests`、`LCP terminated` 之類的訊息。
   - 緩解：調整 `ppp.lcp-echo-failure` 和 `ppp.lcp-echo-interval`。
3. **netplan 和 systemd-networkd 也在管這張網卡。**
   - 情況：Ubuntu Server 22.04 預設用 netplan 加 networkd。如果 `/etc/netplan/*.yaml` 還有設定 WAN 網卡（例如 `dhcp4: true`），networkd 和 NM 可能同時操作同一張網卡，互相干擾。
   - 檢查：`networkctl list`，看 WAN 網卡是不是 `configured`；再看 netplan 的設定。
4. **ISP 端定時斷線，或 MTU。**
   - ISP 定時強制重撥：有些業者會定期中斷 session，這是正常行為，可以從斷線的時間間隔是否固定來判斷。
   - MTU：PPPoE 的 MTU 必須 ≤ 1492。設定錯誤通常是「連著但部分網站卡住」，比較不會造成斷線。

### 程式面可以考慮的調整

- 改用 NM 的新式寫法：`pppoe.parent <實體網卡>` 搭配獨立的 `ifname`（例如 `ppp-<wan_id>`），讓 PPPoE 和網卡上其他 profile 脫鉤。
- 建立 PPPoE 時，順便把同一張網卡上其他 profile 的自動連線關掉。
- 明確設定 `ppp.lcp-echo-*` 和 `ppp.mtu`。
- `put_wan` 會把 PPPoE 帳號轉成小寫（`routes/v1/net/api.rs:539`）。這和間歇斷線無關，因為帳號錯了會完全連不上，但如果 ISP 的帳號區分大小寫就會有問題。

---

# 修正結果與後續事項

這一輪的安全修正依 crate 分成 8 個 commit，全部在 branch `20261122-security-fixes`：general-mq → corelib → coverage CI → auth → broker → coremgr → data → coremgr-cli。

**已完成：** MQ-1、MQ-3、MQ-4（`d1e5329f2`），CORE-1～4（`1a78a3c3f`），AUTH-1～5、AUTH-7、AUTH-6 部分（`1c7a4999c`），BRK-1、BRK-2、CMGR-1 的 broker 端（`c3d32c0ed`），CMGR-2～7、CMGR-10（`a07542aa1`），DATA-1（`a5cda738a`），CLI-1、CLI-2（`2d730dff6`）。另外 `034e00b1b` 修正了 CI 覆蓋率報告的頁面。

**延後到 0.5：**
- **MQ-2：** MQTT manual ack 和背壓。
- **AMQP publisher confirm：** `send_msg()` 等待 broker 確認（見 general-mq 的「不算安全漏洞，但值得注意」）。
- **AUTH-6 的登入次數限制。**
- **unit member management：** 非擁有者的錯誤碼（目前暫用 `err_param`，程式中標註 `TODO: member management`），以及 broker 寫入 API 對非擁有者的回應一致性。見 CMGR-3、CMGR-10。
- **HTTP client timeout：** 要用 `connect_timeout` 加 `read_timeout`，不能用 `timeout()`。見 CMGR-7。

**視情況處理：** router 的 RTR-1、RTR-2。

**不修正：** CMGR-8（rumqttd 只用於 demo/PoC）、CMGR-9（成員收發資料是設計允許的）。
