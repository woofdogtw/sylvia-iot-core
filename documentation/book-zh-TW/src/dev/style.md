# 程式碼風格

## 使用 rustfmt

所有檔案請 **一定** 要使用 `rustfmt` 格式化。這邊建議使用 VSCode 搭配 **rust-analyzer 擴充** 來撰寫程式碼。

## MVC vs. 微服務

本人習慣 bottom-up 的開發模式。使用像是 MVC 這樣將資料庫設計為底層的通用介面、並且由 API 上層依據其所需呼叫來實現各種功能，比較符合本人習慣的風格。
這就是 `models`、`routes` 的由來。

不過在設計整個 Sylvia-IoT 平台時，也是盡可能朝向模組化的方向進行，於是採用了微服務的方式設計（就是 ABCD），並且嚴格遵守樹狀相依的原則。

即使是微服務的架構，如前一章節 [**目錄結構**](dir.md) 所述，只要 main.rs 引用了需要的 routes，依舊可以編譯成單一可執行檔並放在一台機器中執行。
這樣的設計優點在於部署的方式可以很靈活，比如：

- 單體：單一機器執行單一 all-in-one 可執行檔。
- 微服務叢集：將各個元件獨立運行在各個機器中，且每個元件可以自己架設叢集。
- 單體叢集：將 all-in-one 運作在多台機器中，形成叢集的架構。

Sylvia-IoT 就是集 MVC 與微服務於一身的設計 &#x1F60A;。

## 檔案內容編排

每一個 rs 的檔案內容會以如下的方式編排，每一個區塊之間要有空白行隔開：

```rust
use rust_builtin_modules;

use 3rd_party_modules;

use sylvia_iot_modules;

use crate_modules;

pub struct PubStructEnums {}

struct PrvStructEnums {}

pub const PUB_CONSTANTS;

const PRV_CONSTANTS;

pub pub_static_vars;

static prv_static_vars;

impl PubStructEnums {}

pub fn pub_funcs {}

impl PrvStructEnums {}

fn prv_funcs {}
```

大致上的順序就是：

- 引用模組
- 結構
- 常數
- 變數
- 函數（包含結構的函數實作）

而其中又以 `pub` 放在 private 前面。

## Model

Model 層必須提供統一的 struct 以及 trait 介面。
Sylvia-IoT 設計理念中，「任意抽換」是一個相當重視的概念。要盡可能讓使用者於不同的場景下選擇合適的實作。

### 資料庫設計

在提供 CRUD 的順序須遵守以下規則：

- count
- list
- get
- add
- upsert
- update
- del

幾個注意事項：

- **count** 與 **list** 需提供一致的參數，讓 API 和 UI 呈現的時候可以用一致的方式呼叫 count 和 list。
- 不可以在 model 中使用 logger，需要回傳 error 由上層來印出訊息。
    - 當有多個 API 呼叫同一個 model，無法從 model 中印出的錯誤訊息判斷是由誰呼叫的。
- 當取不到資料的時候，要回 `None` 或是空的 `Vec`，而不是 `Error`。
- 只要能滿足「複雜查詢」條件的資料庫，都應該要可以使用相同的 trait 介面實作。
    - SQL、MongoDB 等皆符合此要求。
    - Redis 無法設計為資料庫形式。

### 快取設計

- 能滿足低複雜度的 **key-value** 讀寫者，都要可以使用相同的 trait 介面實作。
    - Redis、程式語言的 map 都符合此要求。
    - SQL、MongoDB 等也可以透過查詢單一條件來實現。當系統不想安裝太多種工具時，使用 SQL、MongoDB 的快取介面實作也是允許的。

## Routes（HTTP API）

這邊提供 API 的文件和實作上需要遵守的規則。

### 動詞順序

- POST
- GET /count
- GET /list
- GET
- PUT
- PATCH
- DELETE

### 路徑

- `/[project]/api/v[version]/[function]`
- `/[project]/api/v[version]/[function]/[op]`
- `/[project]/api/v[version]/[function]/{id}`

上面有個歧義處：`[op]` 和 `{id}`。前者是固定的行為，後者是會變動的對象 ID。設計 ID 的時候要盡量避免與行為的名稱衝突。

> 使用 Actix Web 掛載路由的時候，須將固定的 `[op]` 放在變數 `{id}` 的前面。

這邊舉 Broker 的 [**Device API**](https://github.com/woofdogtw/sylvia-iot-core/blob/main/sylvia-iot-broker/doc/api.md#contents) 為例子：

```
- Device APIs
    - POST /broker/api/v1/device                Create device
    - POST /broker/api/v1/device/bulk           Bulk creating devices
    - POST /broker/api/v1/device/bulk-delete    Bulk deleting devices
    - GET  /broker/api/v1/device/count          Device count
    - GET  /broker/api/v1/device/list           Device list
    - GET  /broker/api/v1/device/{deviceId}     Get device information
```

可以看見 POST 同時有建立單一、建立多筆、刪除多筆的行為，其中 **bulk**、**bulk-delete**、**count**、**list** 就是前述的 `[op]`。
而裝置 ID 的設計上要避免和 **count** 和 **list** 衝突。

### 函數命名

`api.rs` 的函數命名方式如下：

```
fn [method]_[function]_[op]() {}
```

一樣舉剛才的 device API 為例子，函數會以下面的方式命名：

```
fn post_device() {}
fn post_device_bulk() {}
fn post_device_bulk_del() {}
fn get_device_count() {}
fn get_device_list() {}
fn get_device() {}
```

### 請求與回應命名

路徑變數、query、 request body 定義在 `request.rs` 中；response body 則是定義在 `response.rs` 中。命名如下（注意大小寫）：

```
struct [Id]Path {}
struct [Method][Function]Body {}
struct Get[Function]Query {}
```

舉例如下：

```
struct DeviceIdPath {}      // /device/{deviceId}
struct PostDeviceBody {}
struct GetDeviceListQuery {}
```
