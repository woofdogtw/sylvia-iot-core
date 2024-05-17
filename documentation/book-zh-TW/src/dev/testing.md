# 撰寫測試

Sylvia-IoT 採用 BDD 模式撰寫整合測試，框架則是選擇仿照 [**Mocha**](https://mochajs.org/) 的 [**laboratory**](https://enokson.github.io/laboratory/)。

本章節將針對 libs、models、routes 描述撰寫測試時的原則和技巧。

## TestState

`TestState` 結構用來作為 `SpecContext()` 的參數。保存幾種變數：

- 長期存在，且只需初始化一次或很少次的。如 `runtime`、`mongodb` 等。
- 需要確保在 `after` 被釋放的資源。由於測試項目都可能在執行到一半的時候離開，一定要記得在 `after` 釋放。

## libs

- 簡單的函數可以直接測試輸入、輸出。
- 在測試前務必先啟動把所需的基礎設施，比如 RabbitMQ、EMQX 等。
- 需要架設服務的複雜場景，可以在 `before` 建立服務（比如佇列的連線），並於 `after` 釋放。

## models

- 在測試前務必先啟動 MongoDB、Redis 等資料庫。
- 撰寫測試的順序為 R、C、U、D。
    - **R**: 使用 `mongodb`、`sqlx` 等原生套件建立測試資料集，然後測試 model 的 get、count、list 的結果。
    - **C**: 使用 model 的 add、upsert 等函數建立資料，並且使用 get 驗證內容的正確性。
    - **U**: 使用 model 的 add、upsert 等函數建立測試資料集，接著用 update 修改資料，最後使用 get 來驗證結果。
    - **D**: 使用 model 的 add、upsert 等函數建立測試資料集，接著用 delete 修改資料，最後使用 get 來驗證結果。
    - 先測試 **R** 的功能，用意在撰寫 C、U、D 的時候可以使用統一的程式碼來撰寫測試項目，看看同一個邏輯是否可以在每一種資料庫引擎都是一樣的結果。之後引進新的引擎時，就可以寫最少的測試程式碼進行測試。
- 在 `after` 刪除的時候使用原生的套件進行。因為在測試前無法保證 D 相關的功能都已經正確被實作和測試。

## routes

- 雖然可以使用 axum 的 `TestServer::new()` 作為虛擬服務，但 middleware 或是 API bridge 背後所需要的服務，都需要先用 Tokio Task 啟動。
- 可以使用 model trait 介面進行測試資料集的初始化，以及作為 API 請求後的資料檢查。
- 可以使用 model delete 在 `after` 時候刪除測試資料。
