API - Auth
==========

## Contents

- [Notes](#notes)
- [Common error codes](#errcode)
- [Roles](#roles)
- [Auth APIs](#auth)
    - [`GET /auth/api/v1/auth/tokeninfo` Get token information](#get_auth_tokeninfo)
    - [`POST /auth/api/v1/auth/logout` Log-out the user](#post_auth_logout)
- [User information APIs](#user)
    - [`GET /auth/api/v1/user` Get user information](#get_user)
    - [`PATCH /auth/api/v1/user` Update user information](#patch_user)
- [User administration APIs](#admin)
    - [`POST /auth/api/v1/user` Create user](#post_admin_user)
    - [`GET /auth/api/v1/user/count` User count](#get_admin_user_count)
    - [`GET /auth/api/v1/user/list` User list](#get_admin_user_list)
    - [`GET /auth/api/v1/user/{userId}` Get user information](#get_admin_user)
    - [`PATCH /auth/api/v1/user/{userId}` Update user information](#patch_admin_user)
    - [`DELETE /auth/api/v1/user/{userId}` Delete user](#delete_admin_user)
- [Client administration APIs](#client)
    - [`POST /auth/api/v1/client` Create client](#post_client)
    - [`GET /auth/api/v1/client/count` Client count](#get_client_count)
    - [`GET /auth/api/v1/client/list` Client list](#get_client_list)
    - [`GET /auth/api/v1/client/{clientId}` Get client information](#get_client)
    - [`PATCH /auth/api/v1/client/{clientId}` Update client information](#patch_client)
    - [`DELETE /auth/api/v1/client/{clientId}` Delete client](#delete_client)
    - [`DELETE /auth/api/v1/client/user/{userId}` Delete user clients](#delete_client_user)

## <a name="notes"></a>Notes

All API requests must have a **Authorization** header with a **Bearer** token.

- **Example**

        GET /auth/api/v1/user HTTP/1.1
        Host: localhost
        Authorization: Bearer 766f29fa8691c81b749c0f316a7af4b7d303e45bf4000fe5829365d37caec2a4

All APIs may respond one of the following status codes:

- **200 OK**: The request is success with body.
- **204 No Content**: The request is success without body.
- **400 Bad Request**: The API request has something wrong.
- **401 Unauthorized**: The access token is invalid or expired.
- **403 Forbidden**: The user does not have the permission to operate APIs.
- **404 Not Found**: The resource (in path) does not exist.
- **500 Internal Server Error**: The server is crash or get an unknown error. You should respond to the system administrators to solve the problem.
- **503 Service Unavailable**: The server has something wrong. Please try again later.

All error responses have the following parameters in JSON format string:

- *string* `code`: The error code.
- *string* `message`: (**optional**) The error message.

- **Example**

        HTTP/1.1 401 Unauthorized
        Access-Control-Allow-Origin: *
        Content-Type: application/json
        Content-Length: 70
        ETag: W/"43-Npr+dy47IJFtraEIw6D8mYLw7Ws"
        Date: Thu, 13 Jan 2022 07:46:09 GMT
        Connection: keep-alive

        {"code":"err_auth","message":"Invalid token: access token is invalid"}

## <a name="errcode"></a>Common error codes

The following are common error codes. The API specific error codes are listed in each API response documents.

- **401** `err_auth`: Authorization code error. Please refresh a new token.
- **503** `err_db`: Database operation error. Please try again later.
- **503** `err_intmsg`: Inter-service communication error. Please try again later.
- **404** `err_not_found`: The request resource (normally in path) not found.
- **400** `err_param`: Request input format error. Please check request parameters.
- **403** `err_perm`: Permission fail. This usually means that API requests with an invalid user role.
- **503** `err_rsc`: Resource allocation error. Please try again later.
- **500** `err_unknown`: Unknown error. This usually means that the server causes bugs or unexpected errors.

## <a name="roles"></a>Roles

This system supports the following roles:

- `admin`: The system administrator.
- `dev`: The 3rd party developer.
- `manager`: The system manager who can manage users' information.
- `service`: The web service.

**Normal user** means users without any roles.

# <a name="auth"></a>Auth APIs

These APIs can be used by all users.

## <a name="get_auth_tokeninfo"></a>Get token information

Get token information.

    GET /auth/api/v1/auth/tokeninfo

#### Response

- **200 OK**: Token information. Parameters are:

    - *object* `data`:
        - *string* `userId`: User ID.
        - *string* `account`: User account.
        - *string* `name`: User display name.
        - *object* `roles`: Roles with `{role}:true/false` format.
        - *string* `clientId`: Associated client ID.
        - *string[]* `scopes`: Allowed scopes.

- **400, 401, 500, 503**: See [Notes](#notes).

## <a name="post_auth_logout"></a>Log-out the user

Log-out all the user's sessions with the access token.

    POST /auth/api/v1/auth/logout

#### Response

- **204, 400, 401, 500, 503**: See [Notes](#notes).

# <a name="user"></a>User information APIs

These APIs can be used by all users.

## <a name="get_user"></a>Get user information

Get user self information.

    GET /auth/api/v1/user

#### Response

- **200 OK**: User information. Parameters are:

    - *object* `data`:
        - *string* `account`: User account.
        - *string* `createdAt`: Creation time in ISO 8601 format.
        - *string* `modifiedAt`: Creation time in ISO 8601 format.
        - *string | null* `verifiedAt`: Verification time in ISO 8601 format.
        - *object* `roles`: (**present for special roles**) Roles.
        - *string* `name`: Display name.
        - *object* `info`: Other information.

    - **Example**

            {
                "data": {
                    "account": "michael-johnson@example.com",
                    "createdAt": "2022-01-01T02:23:47.053Z",
                    "modifiedAt": "2022-01-02T05:17:27.129Z",
                    "validated": "2022-01-01T02:26:52.210Z",
                    "name": "Michael",
                    "info": {
                        "firstName": "Michael",
                        "lastName": "Johnson",
                        "phoneNumber": "0987654321"
                    }
                }
            }

- **400, 401, 500, 503**: See [Notes](#notes).

## <a name="patch_user"></a>Update user information

Update user self information.

    PATCH /auth/api/v1/user

#### Additional HTTP Headers

    Content-Type: application/json

#### Parameters

- *object* `data`:
    - *string* `password`: (**optional**) User password.
    - *string* `name`: (**optional**) The display name.
    - *object* `info`: (**optional**) Other information.

- **Note**: You must give at least one parameter.

- **Example**

        {
            "data": {
                "name": "Michael",
                "info": {
                    "firstName": "Michael",
                    "lastName": "Johnson",
                    "address": "123, abc road, def city",
                    "phoneNumber": "123456"
                }
            }
        }

#### Response

- **204 No Content**
- **400, 401, 500, 503**: See [Notes](#notes).

# <a name="admin"></a>User administration APIs

These APIs can be used by system administrators only, for managers except:

- `GET /auth/api/v1/user/count`
- `GET /auth/api/v1/user/list`
- `GET /auth/api/v1/user/{userId}`
- `PATCH /auth/api/v1/user/{userId}`: (limited)

## <a name="post_admin_user"></a>Create user

Create a user.

    POST /auth/api/v1/user

#### Additional HTTP Headers

    Content-Type: application/json

#### Parameters

- *object* `data`:
    - *string* `account`: User account. Must be **email address** format or the pattern `[A-Za-z0-9]{1}[A-Za-z0-9-_]*`. The account will be transformed to lowercase.
    - *string* `password`: Password.
    - *string* `name`: (**optional**) Display name.
    - *object* `info`: (**optional**) Other information.
- *string* `expiredAt`: (**optional**) Set the expiration time in ISO 8601 format. Without this value means this user is never expired and is verified immediately.

- **Example**

        {
            "data": {
                "account": "michael-johnson@example.com",
                "password": "p@ssw0rD",
                "name": "Michael",
                "info": {
                    "firstName": "Michael",
                    "lastName": "Johnson",
                    "phoneNumber": "0987654321"
                }
            },
            "expiredAt": "2022-01-02T02:23:47.053Z"
        }

#### Response

- **200 OK**: The user ID. Parameters are:

    - *object* `data`:
        - *string* `userId`: The ID of the created user.

    - **Example**

            {
                "data": {
                    "userId": "1641003827053-c2e84RJO"
                }
            }

- **400 Bad Request**: the special error codes are:
    - `err_auth_user_exist`: The account has been used.
- **401, 403, 500, 503**: See [Notes](#notes).

## <a name="get_admin_user_count"></a>User count

Get user list count.

    GET /auth/api/v1/user/count?
        account={specifiedAccount}&
        contains={word}

- *string* `account`: (**optional**) To search the specified account. This is case insensitive and excludes **contains**.
- *string* `contains`: (**optional**) To search accounts which contain the specified word. This is case insensitive.

#### Response

- **200 OK**: User list count. Parameters are:

    - *object* `data`:
        - *number* `count`: User list count.

    - **Example**

            {
                "data": {
                    "count": 2
                }
            }

- **400, 401, 403, 500, 503**: See [Notes](#notes).

## <a name="get_admin_user_list"></a>User list

Get user list.

    GET /auth/api/v1/user/list?
        account={specifiedAccount}&
        contains={word}&
        fields={displayFields}&
        offset={offset}&
        limit={limit}&
        sort={sortKeysAndOrders}&
        format={responseFormat}

- *string* `account`: (**optional**) To search the specified account. This is case insensitive and excludes **contains**.
- *string* `contains`: (**optional**) To search accounts which contain the specified word. This is case insensitive.
- *string* `fields`: (**optional**) To display more data fields with comma separated format. **expired**, **disabled** can be used. For example, `fields=expired,disabled`.
- *number* `offset`: (**optional**) Data offset. Default is **0**.
- *number* `limit`: (**optional**) Number of items to list. **0** to list all data. Default is **100**.
- *string* `sort`: (**optional**) To sort the result. Format is `key:[asc|desc]`. The key can be **account**, **created**, **modified**, **verified**, **name**. Default is **account:asc**.
- *string* `format`: (**optional**) Response body format. Default is array in the **data** field.
    - **array**: The response body is JSON array, not an object.

#### Response

- **200 OK**: An array that contains all users' information. Parameters are:

    - *object[]* `data`:
        - *string* `userId`: User ID.
        - *string* `account`: User account.
        - *string* `createdAt`: Creation time in ISO 8601 format.
        - *string* `modifiedAt`: Modification time in ISO 8601 format.
        - *string | null* `verifiedAt`: Verification time in ISO 8601 format.
        - *string | null* `expiredAt`: (**optional**) Expiration time in ISO 8601 format.
        - *string | null* `disabledAt`: (**optional**) Disabled time in ISO 8601 format.
        - *object* `roles`: Roles.
        - *string* `name`: Display name.
        - *object* `info`: Other information.

    - **Example**

            {
                "data": [
                    {
                        "userId": "1640921188987-x51FTVPD",
                        "account": "admin@example.com",
                        "createdAt": "2021-12-31T03:26:28.987Z",
                        "modifiedAt": "2021-12-31T03:26:28.987Z",
                        "verifiedAt": "2021-12-31T03:26:28.987Z",
                        "expiredAt": null,
                        "disabledAt": null,
                        "roles": {
                            "admin": true
                        },
                        "name": "System administrator",
                        "info": {
                            "phoneNumber": "1234567890"
                        }
                    },
                    {
                        "userId": "1641003827053-c2e84RJO",
                        "account": "michael-johnson@example.com",
                        "created": "2022-01-01T02:23:47.053Z",
                        "modifiedAt": "2022-01-02T05:17:27.129Z",
                        "validated": "2022-01-01T02:26:52.210Z",
                        "expiredAt": null,
                        "disabledAt": null,
                        "roles": {},
                        "name": "Michael",
                        "info": {
                            "firstName": "Michael",
                            "lastName": "Johnson",
                            "phoneNumber": "0987654321"
                        }
                    }
                ]
            }

    - **Example (format=`array`)**

            [
                {
                    "userId": "1640921188987-x51FTVPD",
                    "account": "admin@example.com",
                    "createdAt": "2021-12-31T03:26:28.987Z",
                    "modifiedAt": "2021-12-31T03:26:28.987Z",
                    "verifiedAt": "2021-12-31T03:26:28.987Z",
                    "expiredAt": null,
                    "disabledAt": null,
                    "roles": {
                        "admin": true
                    },
                    "name": "System administrator",
                    "info": {
                        "phoneNumber": "1234567890"
                    }
                }
            ]

- **400, 401, 403, 500, 503**: See [Notes](#notes).

## <a name="get_admin_user"></a>Get user information

Get the specified user information.

    GET /auth/api/v1/user/{userId}

- *string* `userId`: The specified user ID to get user information.

#### Response

- **200 OK**:

    - *object* `data`: An object that contains the user information. See [User administration APIs - User list](#get_admin_user_list).

- **400, 401, 403, 500, 503**: See [Notes](#notes).
- **404 Not Found**: The specified user does not exist.

## <a name="patch_admin_user"></a>Update user information

Update the specified user information.

    PATCH /auth/api/v1/user/{userId}

- *string* `userId`: The specified user ID to update user information.

#### Additional HTTP Headers

    Content-Type: application/json

#### Parameters

- *object* `data`: (**optional**)
    - *string* `verifiedAt`: (**optional for admin**) The validation date time in ISO 8601 format. The **expiredAt** field will be set to **null**.
    - *object* `roles`: (**optional**) Roles. The content must be booleans. Only administrators can set to roles **admin** and **service**.
    - *string* `password`: (**optional for admin**) User password.
    - *string* `name`: (**optional for admin**) The display name.
    - *object* `info`: (**optional for admin**) Other information. You must provide full of fields, or all fields will be replaced with the new value.
- *bool* `disable`: (**optional**) **true** to disable the user and **false** to enable the user. The permissions are:
    - **admin**: (all)
    - **manager**: service, user

- **Note**: You must give at least one parameter.

- **Example**

        {
            "data": {
                "roles": {
                    "developer": true
                },
                "info": {
                    "firstName": "Michael",
                    "lastName": "Johnson",
                    "address": "123, abc road, def city",
                    "phoneNumber": "123456"
                }
            },
            "disable": false
        }

#### Response

- **204 No Content**
- **400, 401, 403, 500, 503**: See [Notes](#notes).
- **404 Not Found**: The specified user does not exist.

## <a name="delete_admin_user"></a>Delete user

Delete a user. One cannot delete himself/herself.

    DELETE /auth/api/v1/user/{userId}

- *string* `userId`: The specified user ID to delete.

#### Response

- **204 No Content**
- **401, 403, 500, 503**: See [Notes](#notes).

# <a name="client"></a>Client administration APIs

These APIs can be used by system administrators and developers only, except:

- `DELETE /auth/api/v1/client/user/{userId}`: for administrators.

## <a name="post_client"></a>Create client

Create a client.

    POST /auth/api/v1/client

#### Additional HTTP Headers

    Content-Type: application/json

#### Parameters

- *object* `data`:
    - *string[]* `redirectUris`: Allowed redirect URIs.
    - *string[]* `scopes`: Allowed scopes in `[a-z0-9]+([\.]{1}[a-z0-9])*` format.
    - *string* `userId`: (**optional for administrators**) Assign to the specified user.
    - *string* `name`: Client name.
    - *string* `image`: (**optional**) The URI of the client icon.
- *boolean* `credentials`: (**optional**) Create the client with secret. Default is **false**.

#### Response

- **200 OK**: The client ID. Parameters are:

    - *object* `data`:
        - *string* `clientId`: The ID of the created client.

    - **Example**

            {
                "data": {
                    "clientId": "1641040728318-zyAnDK9I"
                }
            }

- **400 Bad Request**: the special error codes are:
    - `err_auth_user_not_exist`: The user ID does not exist.
- **401, 403, 500, 503**: See [Notes](#notes).

## <a name="get_client_count"></a>Client count

Get client list count.

    GET /auth/api/v1/client/count?
        user={userId}

- *string* `user`: (**optional for administrators**) The specified user ID.

#### Response

- **200 OK**: Client list count. Parameters are:

    - *object* `data`:
        - *number* `count`: Client list count.

    - **Example**

            {
                "data": {
                    "count": 2
                }
            }

- **400, 401, 403, 500, 503**: See [Notes](#notes).

## <a name="get_client_list"></a>Client list

Get client list.

    GET /auth/api/v1/client/list?
        user={userId}&
        offset={offset}&
        limit={limit}&
        sort={sortKeysAndOrders}&
        format={responseFormat}

- *string* `user`: (**optional for administrators**) The specified user ID.
- *number* `offset`: (**optional**) Data offset. Default is **0**.
- *number* `limit`: (**optional**) Number of items to list. **0** to list all data. Default is **100**.
- *string* `sort`: (**optional**) To sort the result. Format is `key:[asc|desc]`. The key can be **created**, **modified**, **name**. Default is **name:asc**.
- *string* `format`: (**optional**) Response body format. Default is array in the **data** field.
    - **array**: The response body is JSON array, not an object.

- **Note**: Administrators can get all clients and developers can only get their own clients.

#### Response

- **200 OK**: An empty array or an array that contains all clients' information. Parameters are:

    - *object* `data`:
        - *string* `clientId`: Client ID.
        - *string* `createdAt`: Creation time in ISO 8601 format.
        - *string* `modifiedAt`: Modification time in ISO 8601 format.
        - *string | null* `clientSecret`: Client secret. **null** means that this is a public client.
        - *string[]* `redirectUris`: Allowed redirect URIs.
        - *string[]* `scopes`: Allowed scopes.
        - *string* `userId`: (**optional for administrators**) User ID that is associated with the client.
        - *string* `name`: Client name.
        - *string | null* `image`: The URI of the client icon.

    - **Example**

            {
                "data": [
                    {
                        "clientId": "1641040728318-zyAnDK9I",
                        "createdAt": "2022-01-01T12:38:48.318Z",
                        "modifiedAt": "2022-01-01T12:38:48.318Z",
                        "clientSecret": "G3LCtKsJvB3nrkA4CnIH3IUVF+BKMHXHTXbzgNF6REU",
                        "redirectUris": [ "https://localhost/oauth2/desktop" ],
                        "scopes": [ "user.rw", "client.rw" ],
                        "name": "OAuth2 App",
                        "image": "https://localhost/oauth2/app.png"
                    },
                    {
                        "clientId": "1641042262366-1HTBqdPg",
                        "createdAt": "2022-01-01T13:04:22.366Z",
                        "modifiedAt": "2022-01-02T07:52:38.129Z",
                        "redirectUris": [ "https://exmaple.com/oauth2/redirect/uri" ],
                        "scopes": [ "user.rw", "client.rw" ],
                        "name": "OAuth2 Web",
                        "image": "https://example.com/oauth2/web.png"
                    }
                ]
            }

    - **Example (format=`array`)**

            [
                {
                    "clientId": "1641040728318-zyAnDK9I",
                    "createdAt": "2022-01-01T12:38:48.318Z",
                    "modifiedAt": "2022-01-01T12:38:48.318Z",
                    "clientSecret": "G3LCtKsJvB3nrkA4CnIH3IUVF+BKMHXHTXbzgNF6REU",
                    "redirectUris": [ "https://localhost/oauth2/desktop" ],
                    "scopes": [ "user.rw", "client.rw" ],
                    "name": "OAuth2 App",
                    "image": "https://localhost/oauth2/app.png"
                }
            ]

- **400, 401, 403, 500, 503**: See [Notes](#notes).

## <a name="get_client"></a>Get client information

Get the specified client information.

    GET /auth/api/v1/client/{clientId}

- *string* `clientId`: The specified client ID to get client information.

#### Response

- **200 OK**:

    - *object* `data`: An object that contains the client information. See [Client administration APIs - Client list](#get_client_list).

- **400, 401, 403, 500, 503**: See [Notes](#notes).
- **404 Not Found**: The specified client does not exist.

## <a name="patch_client"></a>Update client information

Update the specified client information.

    PATCH /auth/api/v1/client/{clientId}

- *string* `clientId`: The specified client ID to update client information.

#### Additional HTTP Headers

    Content-Type: application/json

#### Parameters

- *object* `data`: (**optional**)
    - *string[]* `redirectUris`: (**optional**) Allowed redirect URIs.
    - *string[]* `scopes`: (**optional**) Allowed scopes in `[a-z0-9]+([\.]{1}[a-z0-9])*` format.
    - *string* `name`: (**optional**) Client name.
    - *string | null* `image`: (**optional**) The URI of the client icon.
- *boolean* `regenSecret`: (**optional**) Re-generate secret of the private client. Default is **false**.

- **Note**: You must give at least one parameter.

- **Example**

        {
            "data": {
                "name": "New client name"
            }
        }

#### Response

- **204 No Content**
- **400, 401, 403, 500, 503**: See [Notes](#notes).
- **404 Not Found**: The specified client does not exist.

## <a name="delete_client"></a>Delete client

Delete a client. One client cannot delete itself.

    DELETE /auth/api/v1/client/{clientId}

- *string* `clientId`: The specified client ID to delete.

#### Response

- **204 No Content**
- **400, 401, 403, 500, 503**: See [Notes](#notes).

## <a name="delete_client_user"></a>Delete user clients

Delete all clients of the specified user.

    DELETE /auth/api/v1/client/user/{userId}

- *string* `userId`: The specified user ID.

#### Response

- **204 No Content**
- **400, 401, 403, 500, 503**: See [Notes](#notes).
