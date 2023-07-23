# OAuth2 認證

Sylvia-IoT 的 HTTP API 需要透過 OAuth2 取得令牌（access token）來存取。以下列出幾種需要使用 OAuth2 認證和取得授權令牌的場景：

- 存取 Sylvia-IoT HTTP API
- 開發網路、應用需要整合 **sylvia-iot-auth** 的使用者帳號以及令牌認證。

**sylvia-iot-auth** 提供了基本的登入和授權頁面，本章也會描述如何開發自己需要的頁面。

## 開始之前

在這之前您需要先建立第一個帳號和客戶端。在 [**快速開始**](../guide/quick.md) 我們建立了如下的資源：

- 使用者帳號：名稱是 **admin**，密碼是 **admin**。
- 客戶端：ID 是 **public**，重新轉向位址是 **http://localhost:1080/auth/oauth2/redirect**。

使用 **coremgr-cli** 可以使用上述的資訊取得令牌，如果您想建立自己的帳號和客戶端，除了使用 CLI，以下將說明帳號的細節。

- 帳號的部分，密碼是結合了 **鹽**（salt）和 [**PBKDF2**](https://en.wikipedia.org/wiki/PBKDF2) 加密的結果，迭代次數 **10000**。
把 `salt` 和 `password` 替換成您指定的鹽和雜湊運算後的密碼即可。其他欄位也可以替換成您指定的。
- 客戶端的部分，替換 `clientId` 和 `redirectUri`。重新導向網址（redirect URI）的部分需要帶入客戶端的位址。
假如您的服務需要透過 **http://localhost** 或是 **https://network.example.com** 存取，且接收 authorization code 的路徑在 **/network/redirect**，那內容可以填入 `["http://localhost/network/redirect","https://network.example.com/network/redirect"]`。

## 使用瀏覽器和 curl

這裡我們介紹如何登入帳號密碼，並取得會話 ID（session ID）來進入授權頁面取取得令牌，以下範例採用 [**快速開始**](../guide/quick.md) 建立的帳號和客戶端。

打開瀏覽器，網址輸入 `http://localhost:1080/auth/oauth2/auth?response_type=code&redirect_uri=http%3A%2F%2Flocalhost%3A1080%2Fauth%2Foauth2%2Fredirect&client_id=public`

輸入帳號密碼，如果能導向到授權畫面就表示登入成功。此時畫面會印出此客戶端需要使用的 API scope，如果同意就按下 **Accept** 按鈕，接著瀏覽器的網址會呈現如下的樣子（您的內容會有些不同）：

```
http://localhost:1080/auth/oauth2/redirect?code=62a801a7d6ceaf2d1018cbac60a6b3d1744295016214bfec6214397d73368278
```

其中 `code` 就是 authorization code，請於 30 秒內使用 curl 指令取得令牌：

```shell
curl -X POST http://localhost:1080/auth/oauth2/token -d 'grant_type=authorization_code&code=62a801a7d6ceaf2d1018cbac60a6b3d1744295016214bfec6214397d73368278&redirect_uri=http%3A%2F%2Flocalhost%3A1080%2Fauth%2Foauth2%2Fredirect&client_id=public'
```

看到以下畫面表示取得令牌（您的內容會有些不同）：

```
{"access_token":"fecc5af17e254e6c5a561b7acc900c8f0449a42e77f07a19261c2e6cff518ec8","refresh_token":"5905fc23f65ca7ed92bc7be74e33fc3e79cd8bce2c9ef2ef1bb368caaf6c07f0","token_type":"bearer","expires_in":3599,"scope":""}
```

## 使用 curl

如果您要使用 curl 指令來輔助程式開發，可以使用下面的幾個步驟。先使用以下指令登入並取得 session ID：

```shell
curl -v -X POST http://localhost:1080/auth/oauth2/login -d 'state=response_type%3Dcode%26client_id%3Dpublic%26redirect_uri%3Dhttp%253A%252F%252Flocalhost%253A1080%252Fauth%252Foauth2%252Fredirect&account=admin&password=admin'
```

看到如下的回應即表示成功（您的內容會有些不同）：

```
< HTTP/1.1 302 Found
< content-length: 0
< access-control-allow-credentials: true
< location: /auth/oauth2/authorize?response_type=code&client_id=public&redirect_uri=http%3A%2F%2Flocalhost%3A1080%2Fauth%2Foauth2%2Fredirect&session_id=6643a450b4d678f7d0223fde9e118a2733f1958aa3fc55d616ec278e83d7a06a
< vary: Origin, Access-Control-Request-Method, Access-Control-Request-Headers
< access-control-expose-headers: location
< date: Sat, 15 Jul 2023 04:25:21 GMT
```

將 **location** 中的 `session_id` 內容保留，並於 60 秒內帶入下一個 HTTP 請求：

```shell
curl -v -X POST http://localhost:1080/auth/oauth2/authorize -d 'allow=yes&session_id=6643a450b4d678f7d0223fde9e118a2733f1958aa3fc55d616ec278e83d7a06a&client_id=public&response_type=code&redirect_uri=http%3A%2F%2Flocalhost%3A1080%2Fauth%2Foauth2%2Fredirect'
```

看到如下的回應即表示成功（您的內容會有些不同）：

```
< HTTP/1.1 302 Found
< content-length: 0
< access-control-allow-credentials: true
< location: http://localhost:1080/auth/oauth2/redirect?code=eee02ae34b6c93f955ebf244bccec2b7e6534e1a8dc451a2ed92a790be7b14bb
< vary: Origin, Access-Control-Request-Method, Access-Control-Request-Headers
< access-control-expose-headers: location
< date: Sat, 15 Jul 2023 04:40:36 GMT
```

其中 **location** 中的 `code` 就是 authorization code，請於 30 秒內使用 curl 指令取得令牌：

```shell
curl -X POST http://localhost:1080/auth/oauth2/token -d 'grant_type=authorization_code&code=eee02ae34b6c93f955ebf244bccec2b7e6534e1a8dc451a2ed92a790be7b14bb&redirect_uri=http%3A%2F%2Flocalhost%3A1080%2Fauth%2Foauth2%2Fredirect&client_id=public'
```

看到以下畫面表示取得令牌（您的內容會有些不同）：

```
{"access_token":"6994982614dc9f6f2bff08169f7636873531686c34c02fbd6bb45655c8f24b13","refresh_token":"387822850a8fa9a474c413b62a17d9f218204ddcaad51ca475448827b83972fe","token_type":"bearer","expires_in":3599,"scope":""}
```

## 認證流程的端點（endpoint）

- `GET /auth/oauth2/auth`
    - 驗證客戶端的基本訊息，成功就重新導向至下一個 endpoint。
    - Query 參數：
        - `response_type`: 必須為 `code`。
        - `client_id`: 客戶端識別碼。
        - `redirect_uri`: 重新導向位址。此位址將是接收 authorization code 的 endpoint。
        - `scope`: (**可選**) 希望存取的權限範圍。
        - `state`: (**可選**) 會在拿到 authorization code 的時候附上。一般用來保留前一次所在的頁面訊息，供登入後返回用。
- `GET /auth/oauth2/login`
    - 顯示帳號登入畫面。
    - Query 參數會在前一個步驟後自動帶入。
        - `state`: (自動產生)
    - 按下登入按鈕需要觸發下一步的 HTTP 請求。
- `POST /auth/oauth2/login`
    - 登入帳號密碼，成功就重新導向至下一個 endpoint。
    - HTTP body 參數：
        - `account`: 帳號名稱。
        - `password`: 密碼。
            - 由於使用明碼，建議使用 HTTPS 以及可信任的瀏覽器元件（webview）。
        - `state`: 前一個步驟中的 state 內容。
- `GET /auth/oauth2/authorize`
    - 認證客戶端參數與 session ID，然後顯示客戶端的權限需求。
    - Query 參數會在前一個步驟後自動帶入。
        - (同 `GET /auth/oauth2/auth`)
        - `session_id`: 此次登入流程的 session ID。目前保留 60 秒。
    - 按下允許或拒絕按鈕需要觸發下一步的 HTTP 請求。
- `POST /auth/oauth2/authorize`
    - 認證客戶端並產生 authorization code。成功或錯誤都會重新導向至客戶端指定的位址。
    - HTTP body 參數：
        - (同 `GET /auth/oauth2/authorize` query)
        - `allow`: `yes` 表示允許，其餘表示拒絕。
    - 重新導向的參數：
        - `code`: authorization code。需在 30 秒內將此內容帶入下一步的 HTTP 請求中。
- `POST /auth/oauth2/token`
    - 認證客戶端資訊以及 authorization code，並產生令牌。
    - HTTP body 參數：
        - `grant_type`: 必須為 `authorization_code`。
        - `code`: authorization code 的值。
        - `redirect_uri`: 客戶端得重新導向位址。
        - `client_id`: 客戶端識別碼。
    - 回傳內容：
        - `access_token`: 令牌。可存取 Sylvia-IoT HTTP API。
        - `refresh_token`: 當令牌失效時，以此重新取得令牌。
        - `token_type`: `bearer`。
        - `expires_in`: 過期時間（秒）。
        - `scope`: 存取權限範圍。
- `POST /auth/oauth2/refresh`
    - 重新取得令牌。
    - HTTP body 參數：
        - `grant_type`: 必須為 `refresh_token`。
        - `refresh_token`: refresh token 的值。
        - `scope`: (**可選**) 存取權限範圍。
        - `client_id`: (**可選**) 客戶端識別碼。
    - 回傳內容：同 `POST /auth/oauth2/token` 的回傳內容。

## 開發自己的樣板

參考 [**原始版本**](https://github.com/woofdogtw/sylvia-iot-core/blob/main/sylvia-iot-auth/src/routes/oauth2/template.rs) 並注意要預留的 Jinja2 變數 `{{ }}`。

在帳號登入畫面，請預留以下的變數：

- `scope_path`: 這個用來決定按下「登入」按鈕時要發送 `POST /login` 請求的位址。
    - Sylvia-IoT 的預設是 `SCHEME://SERVER_HOST/auth`。`SCHEME://SERVER_HOST` 是 `GET /auth` 時候的資訊。
- `state`: 當 `GET /auth` 成功時，**sylvia-iot-auth** 會產生 state 內容並帶入樣板中。

在客戶端授權畫面，請預留以下的變數：

- `scope_path`: 這個用來決定按下「登入」按鈕時要發送 `POST /authorize` 請求的位址。
    - Sylvia-IoT 的預設是 `SCHEME://SERVER_HOST/auth`。`SCHEME://SERVER_HOST` 是 `POST /login` 時候的資訊。
- 其餘參數請參考前面 endpoint 中 `GET /auth/oauth2/authorize` 的敘述。

您可以只實作登入或是授權網頁的內容，並且提供 [**設定檔**](../guide/configuration.md) 中的下列參數：

- `auth.db.templates.login`: 登入頁面樣板的路徑。
- `auth.db.templates.grant`: 授權頁面樣板的路徑。
