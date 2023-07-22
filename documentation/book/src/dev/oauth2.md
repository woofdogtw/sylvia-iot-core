# OAuth2 Authentication

Sylvia-IoT HTTP APIs require obtaining an access token through OAuth2 for access. The following
scenarios require using OAuth2 authentication and obtaining access tokens:

- Accessing Sylvia-IoT HTTP API.
- Developing network and application services that need to integrate with **sylvia-iot-auth** for
  user account and token authentication.

**sylvia-iot-auth** provides basic login and authorization pages, and this chapter will also
describe how to develop custom pages as needed.

## Before Getting Started

Before proceeding, you need to create the first user account and client. In the
[**Quick Start**](../guide/quick.md) guide, we created the following resources:

- User account: name is **admin**, and the password is **admin**.
- Client: ID is **public**, and the redirect URI is **http://localhost:1080/auth/oauth2/redirect**.

You can use **coremgr-cli** to obtain the token using the above information. If you want to create
your own user account and client, you can do so using the CLI, or you can follow the details below.

- For user accounts, the password is hashed using a combination of **salt** and
  [**PBKDF2**](https://en.wikipedia.org/wiki/PBKDF2) encryption, with **10000** iterations.
  Replace salt and password with your specified salt and hashed password, respectively. Other fields
  can also be replaced with your specified values.
- For clients, replace `clientId` and `redirectUri`. The redirect URI should be set to the client's
  address.
  If your service needs to be accessed through **http://localhost** or
  **https://network.example.com**, and it receives the authorization code at the path
  **/network/redirect**, you can set the redirect URI as
  `["http://localhost/network/redirect","https://network.example.com/network/redirect"]`.

## Using Browser and Curl

Here, we will explain how to log in with your account credentials and obtain a session ID to access
the authorization page and obtain the token. The following examples use the user account and client
created in the [**Quick Start**](../guide/quick.md) guide.

Open your browser and enter the URL `http://localhost:1080/auth/oauth2/auth?response_type=code&redirect_uri=http%3A%2F%2Flocalhost%3A1080%2Fauth%2Foauth2%2Fredirect&client_id=public`

Enter your account credentials. If you are redirected to the authorization page, it means you have
successfully logged in. The page will display the API scopes required by this client. If you agree,
click the **Accept** button. After that, the URL in the browser will look like the following (your
content will be slightly different):

```
http://localhost:1080/auth/oauth2/redirect?code=62a801a7d6ceaf2d1018cbac60a6b3d1744295016214bfec6214397d73368278
```

The `code` in the URL is the authorization code. You need to use the curl command within 30 seconds
to obtain the token:

```shell
curl -X POST http://localhost:1080/auth/oauth2/token -d 'grant_type=authorization_code&code=62a801a7d6ceaf2d1018cbac60a6b3d1744295016214bfec6214397d73368278&redirect_uri=http%3A%2F%2Flocalhost%3A1080%2Fauth%2Foauth2%2Fredirect&client_id=public'
```

If you see the following message, it means you have obtained the token (your content will be
slightly different):

```
{"access_token":"fecc5af17e254e6c5a561b7acc900c8f0449a42e77f07a19261c2e6cff518ec8","refresh_token":"5905fc23f65ca7ed92bc7be74e33fc3e79cd8bce2c9ef2ef1bb368caaf6c07f0","token_type":"bearer","expires_in":3599,"scope":""}
```

## Using Curl

If you want to use the curl command to assist with your program development, you can follow these
steps. First, use the following command to log in and obtain the session ID:

```shell
curl -v -X POST http://localhost:1080/auth/oauth2/login -d 'state=response_type%3Dcode%26client_id%3Dpublic%26redirect_uri%3Dhttp%253A%252F%252Flocalhost%253A1080%252Fauth%252Foauth2%252Fredirect&account=admin&password=admin'
```

If you see the response like this (your content will be slightly different):

```
< HTTP/1.1 302 Found
< content-length: 0
< access-control-allow-credentials: true
< location: /auth/oauth2/authorize?response_type=code&client_id=public&redirect_uri=http%3A%2F%2Flocalhost%3A1080%2Fauth%2Foauth2%2Fredirect&session_id=6643a450b4d678f7d0223fde9e118a2733f1958aa3fc55d616ec278e83d7a06a
< vary: Origin, Access-Control-Request-Method, Access-Control-Request-Headers
< access-control-expose-headers: location
< date: Sat, 15 Jul 2023 04:25:21 GMT
```

Keep the content of `session_id` from the **location** field and use it in the next HTTP request
within 60 seconds:

```shell
curl -v -X POST http://localhost:1080/auth/oauth2/authorize -d 'allow=yes&session_id=6643a450b4d678f7d0223fde9e118a2733f1958aa3fc55d616ec278e83d7a06a&client_id=public&response_type=code&redirect_uri=http%3A%2F%2Flocalhost%3A1080%2Fauth%2Foauth2%2Fredirect'
```

If you see the response like this (your content will be slightly different):

```
< HTTP/1.1 302 Found
< content-length: 0
< access-control-allow-credentials: true
< location: http://localhost:1080/auth/oauth2/redirect?code=eee02ae34b6c93f955ebf244bccec2b7e6534e1a8dc451a2ed92a790be7b14bb
< vary: Origin, Access-Control-Request-Method, Access-Control-Request-Headers
< access-control-expose-headers: location
< date: Sat, 15 Jul 2023 04:40:36 GMT
```

The `code` in the **location** field is the authorization code. You need to use the curl command
within 30 seconds to obtain the token:

```shell
curl -X POST http://localhost:1080/auth/oauth2/token -d 'grant_type=authorization_code&code=eee02ae34b6c93f955ebf244bccec2b7e6534e1a8dc451a2ed92a790be7b14bb&redirect_uri=http%3A%2F%2Flocalhost%3A1080%2Fauth%2Foauth2%2Fredirect&client_id=public'
```

If you see the following message, it means you have obtained the token (your content will be
slightly different):

```
{"access_token":"6994982614dc9f6f2bff08169f7636873531686c34c02fbd6bb45655c8f24b13","refresh_token":"387822850a8fa9a474c413b62a17d9f218204ddcaad51ca475448827b83972fe","token_type":"bearer","expires_in":3599,"scope":""}
```

## Authentication Flow Endpoints

Here are the endpoints involved in the OAuth2 authentication flow:

- `GET /auth/oauth2/auth`
    - Verifies the client's basic information and redirects to the next endpoint if successful.
    - Query parameters:
        - `response_type`: Must be `code`.
        - `client_id`: Client identifier.
        - `redirect_uri`: The redirect URI where the authorization code will be received.
        - `scope`: (**Optional**) The requested scope of access.
        - `state`: (**Optional**) Will be included when receiving the authorization code. Generally
          used to retain the previous page information for returning after login.
- `GET /auth/oauth2/login`
    - Displays the account login page.
    - Query parameters will be automatically populated from the previous step.
        - `state`: (Auto-generated)
    - Pressing the login button will trigger the next HTTP request.
- `POST /auth/oauth2/login`
    - Logs in with the account username and password and redirects to the next endpoint if
      successful.
    - HTTP body parameters:
        - `account`: Account username.
        - `password`: Password.
            - Since plaintext is used, it is recommended to use HTTPS and a trusted browser
              component (webview).
        - `state`: Content of the state from the previous step.
- `GET /auth/oauth2/authorize`
    - Authenticates the client parameters and session ID, and displays the client's permission
      requirements.
    - Query parameters will be automatically populated from the previous step.
        - (Same as `GET /auth/oauth2/auth`)
        - `session_id`: The session ID for the current login process. Currently reserved for 60
          seconds.
    - Pressing the Allow or Deny button will trigger the next HTTP request.
- `POST /auth/oauth2/authorize`
    - Authenticates the client and generates the authorization code. The endpoint will redirect to
      the address specified by the client whether successful or failed.
    - HTTP body parameters:
        - (Same as `GET /auth/oauth2/authorize` query)
        - `allow`: `yes` indicates approval, while others indicate rejection.
    - Redirect parameters:
        - `code`: The authorization code. This content must be used in the next HTTP request within
          30 seconds.
- `POST /auth/oauth2/token`
    - Authenticates the client information and authorization code, and generates the access token.
    - HTTP body parameters:
        - `grant_type`: Must be `authorization_code`.
        - `code`: The value of the authorization code.
        - `redirect_uri`: The redirect URI of the client.
        - `client_id`: Client identifier.
    - Response content:
        - `access_token`: The access token to access the Sylvia-IoT HTTP APIs.
        - `refresh_token`: Used to obtain a new token when the access token expires.
        - `token_type`: `bearer`.
        - `expires_in`: Expiration time in seconds.
        - `scope`: Access scope.
- `POST /auth/oauth2/refresh`
    - Obtains a new access token using the refresh token.
    - HTTP body parameters:
        - `grant_type`: Must be `refresh_token`.
        - `refresh_token`: The value of the refresh token.
        - `scope`: (**Optional**) The requested scopes of access.
        - `client_id`: (**Optional**) Client identifier.
    - Response content: Same as the response content of `POST /auth/oauth2/token`.

## Developing Your Own Templates

You can refer to the [**original version**](https://github.com/woofdogtw/sylvia-iot-core/blob/main/sylvia-iot-auth/src/routes/oauth2/template.rs)
of the templates and pay attention to the Jinja2 variables to be preserved within `{{ }}`.

For the account login page, please reserve the following variables:

- `scope_path`: This will determine the endpoint to send the `POST /login` request when the "Login"
  button is clicked.
    - The default for Sylvia-IoT is `SCHEME://SERVER_HOST/auth`, where `SCHEME://SERVER_HOST`
      corresponds to the information from the `GET /auth` endpoint.
- `state`: When `GET /auth` is successful, **sylvia-iot-auth** generates the state content and
  includes it in the template.

For the client authorization page, please reserve the following variables:

- `scope_path`: This will determine the endpoint to send the `POST /authorize` request when the
  "Login" button is clicked.
    - The default for Sylvia-IoT is `SCHEME://SERVER_HOST/auth`, where `SCHEME://SERVER_HOST`
      corresponds to the information from the `POST /login` endpoint.
- Other parameters should be referred to as described in the `GET /auth/oauth2/authorize` endpoint
  section.

You can choose to implement the login or authorization web page content and provide the following
parameters in the [**Configuration File**](../guide/configuration.md):

- `auth.db.templates.login`: The file path to the login page template.
- `auth.db.templates.grant`: The file path to the authorization page template.
