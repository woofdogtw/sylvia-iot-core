API - Coremgr (Core Manager)
============================

## Contents

- [Notes](#notes)
- [Common error codes](#errcode)
- [Roles](#roles)
- [API wrapping](#wrap)
- [List API formats](#format)
- [Service APIs](#service)
    - [`GET /version` Get service version](#get_version)
- [Application APIs](#application)
    - [`POST /coremgr/api/v1/application` Create application](#post_application)
    - [`GET /coremgr/api/v1/application/count` Application count](#get_application_count)
    - [`GET /coremgr/api/v1/application/list` Application list](#get_application_list)
    - [`GET /coremgr/api/v1/application/{applicationId}` Get application information](#get_application)
    - [`PATCH /coremgr/api/v1/application/{applicationId}` Update application information](#patch_application)
    - [`DELETE /coremgr/api/v1/application/{applicationId}` Delete application](#delete_application)
    - [`GET /coremgr/api/v1/application/{applicationId}/stats` Get application statistics](#get_application_stats)
    - [`POST /coremgr/api/v1/application/{applicationId}/dldata` Send downlink data](#post_application_dldata)
- [Network APIs](#network)
    - [`POST /coremgr/api/v1/network` Create network](#post_network)
    - [`GET /coremgr/api/v1/network/count` Network count](#get_network_count)
    - [`GET /coremgr/api/v1/network/list` Network list](#get_network_list)
    - [`GET /coremgr/api/v1/network/{networkId}` Get network information](#get_network)
    - [`PATCH /coremgr/api/v1/network/{networkId}` Update network information](#patch_network)
    - [`DELETE /coremgr/api/v1/network/{networkId}` Delete network](#delete_network)
    - [`GET /coremgr/api/v1/network/{networkId}/stats` Get network statistics](#get_network_stats)
    - [`POST /coremgr/api/v1/network/{networkId}/uldata` Simulate device uplink data](#post_network_uldata)

## <a name="notes"></a>Notes

All API requests (except `GET /version`) must have a **Authorization** header with a **Bearer** token.

- **Example**

    ```http
    GET /coremgr/api/v1/user HTTP/1.1
    Host: localhost
    Authorization: Bearer 766f29fa8691c81b749c0f316a7af4b7d303e45bf4000fe5829365d37caec2a4
    ```

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

    ```http
    HTTP/1.1 401 Unauthorized
    Access-Control-Allow-Origin: *
    Content-Type: application/json
    Content-Length: 70
    ETag: W/"43-Npr+dy47IJFtraEIw6D8mYLw7Ws"
    Date: Thu, 13 Jan 2022 07:46:09 GMT
    Connection: keep-alive

    {"code":"err_auth","message":"Invalid token: access token is invalid"}
    ```

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
- `manager`: The system manager who acts as admin to view/edit all units' data.
- `service`: The web service.

**Normal user** means users without any roles.

## <a name="wrap"></a>API wrapping

The coremgr module wraps most of `sylvia-iot-auth` and `sylvia-iot-broker` APIs by replacing `/auth` and `/broker` with `/coremgr`. The exceptions are **application** and **network** APIs because the coremgr not only manages them by requesting APIs but also controls underlying message brokers (RabbitMQ/EMQX/...) for **authentication** and **access control**.

In this document, we only describe APIs that have different body from auth/broker modules.

## <a name="format"></a>List API formats

In addition to JSON object and array, all list APIs support CSV format with a header line and UTF-8 BOM.

The special data types will be transformed:

| JSON type | CSV field type |
| --------- | -------------- |
| `null`    | empty string   |
| `object`  | JSON string    |
| `array`   | JSON string    |

# <a name="service"></a>Service APIs

## <a name="get_version"></a>Get service version

Get service name and version information.

    GET /version?
        q={query}

- *string* `q`: (**optional**) To query the specific information **in plain text**. You can use:
    - `name`: To query the service name.
    - `version`: To query current version.

#### Response

- **200 OK**: Version information. Parameters are:

    - *object* `data`:
        - *string* `name`: The service name.
        - *string* `version`: Current version.

    - **Example**

        ```json
        {
            "data": {
                "name": "sylvia-iot-coremgr",
                "version": "1.0.0"
            }
        }
        ```

    - **Example** when `q=name`:

        ```
        sylvia-iot-coremgr
        ```

    - **Example** when `q=version`:

        ```
        1.0.0
        ```

# <a name="application"></a>Application APIs

## <a name="post_application"></a>Create application

Create an application and get the default password.

    POST /coremgr/api/v1/application

#### Additional HTTP Headers

    Content-Type: application/json

#### Parameters

- *object* `data`:
    - *string* `code`: Application code for queues. The pattern is `[A-Za-z0-9]{1}[A-Za-z0-9-_]*`. This code will be transformed to lowercase.
        - This code will be the **user name** of the queue connection.
    - *string* `unitId`: The associated unit ID.
    - *string* `hostUri`: The application queue URI.
        - The **authority** part of URI will be ignored.
    - *string* `name`: (**optional**) Display name.
    - *object* `info`: (**optional**) Other information.
    - *number* `ttl`: (**optional for AMQP**) Message TTL in milliseconds. **0** this is no limit.
    - *number* `length`: (**optional for AMQP**) Maximum queue length. **0** this is no limit.

- **Note**:
    - You must assign both `ttl` and `length`. Giving only one value will cause another one to be unlimited.

- **Example**

    ```json
    {
        "data": {
            "code": "tracker",
            "unitId": "1640923958516-qvdFNpOV",
            "hostUri": "amqp://localhost/sylvia",
            "name": "Position tracker application",
            "info": {
                "description": "This is the position tracker application with maps"
            },
            "ttl": 3600000,
            "length": 10000
        }
    }
    ```

#### Response

- **200 OK**: The application ID. Parameters are:

    - *object* `data`:
        - *string* `applicationId`: The ID of the created application.
        - *string* `password`: The default password.
            - This value will be returned only once. Use the [`PATCH`](#patch_application) API to change the password.

    - **Example**

        ```json
        {
            "data": {
                "applicationId": "1640924063709-rmJIxW0s",
                "password": "Ihimdqjy"
            }
        }
        ```

- **400 Bad Request**: the special error codes are:
    - `err_broker_unit_not_exist`: The unit does not exist.
    - `err_broker_application_exist`: The application code has been used.
- **401, 403, 500, 503**: See [Notes](#notes).

## <a name="get_application_count"></a>Application count

Get application list count.

    GET /coremgr/api/v1/application/count?
        unit={specifiedUnitId}&
        contains={word}

- *string* `unit`: (**required for normal user**) To search applications of the specified unit ID.
- *string* `contains`: (**optional**) To search codes which contain the specified word. This is case insensitive.

#### Response

- **200 OK**: Application list count. Parameters are:

    - *object* `data`:
        - *number* `count`: Application list count.

    - **Example**

        ```json
        {
            "data": {
                "count": 2
            }
        }
        ```

- **400 Bad Request**: the special error codes are:
    - `err_broker_unit_not_exist`: The unit does not exist.
- **401, 403, 500, 503**: See [Notes](#notes).

## <a name="get_application_list"></a>Application list

Get application list.

    GET /coremgr/api/v1/application/list?
        unit={specifiedUnitId}&
        contains={word}&
        offset={offset}&
        limit={limit}&
        sort={sortKeysAndOrders}&
        format={responseFormat}

- *string* `unit`: (**required for normal user**) To search applications of the specified unit ID.
- *string* `contains`: (**optional**) To search codes which contain the specified word. This is case insensitive.
- *number* `offset`: (**optional**) Data offset. Default is **0**.
- *number* `limit`: (**optional**) Number of items to list. **0** to list all data. Default is **100**.
- *string* `sort`: (**optional**) To sort the result. Format is `key:[asc|desc]`. The key can be **code**, **created**, **modified**, **name**. Default is **code:asc**.
- *string* `format`: (**optional**) Response body format. Default is array in the **data** field.
    - **array**: The response body is JSON array, not an object.
    - **csv**: The response body is CSV.

#### Response

- **200 OK**: An array that contains all applications' information. Parameters are:

    - *object[]* `data`:
        - *string* `applicationId`: Application ID.
        - *string* `code`: Application code for queues.
        - *string* `unitId`: The associated unit ID.
        - *string* `unitCode`: The associated unit code.
        - *string* `createdAt`: Creation time in RFC 3339 format.
        - *string* `modifiedAt`: Modification time in RFC 3339 format.
        - *string* `hostUri`: The application queue URI.
        - *string* `name`: Display name.
        - *object* `info`: Other information.

    - **Example**

        ```json
        {
            "data": [
                {
                    "applicationId": "1640924152438-eVJe8UhY",
                    "code": "meter",
                    "unitId": "1640923958516-qvdFNpOV",
                    "unitCode": "sylvia",
                    "createdAt": "2021-12-31T04:15:52.438Z",
                    "modifiedAt": "2021-12-31T04:15:52.438Z",
                    "hostUri": "amqp://localhost/sylvia",
                    "name": "Meter application",
                    "info": {
                        "description": "This is the meter program"
                    }
                },
                {
                    "applicationId": "1640924063709-rmJIxW0s",
                    "code": "tracker",
                    "unitId": "1640923958516-qvdFNpOV",
                    "unitCode": "sylvia",
                    "createdAt": "2021-12-31T04:14:23.709Z",
                    "modifiedAt": "2021-12-31T04:14:23.709Z",
                    "hostUri": "amqp://localhost/sylvia",
                    "name": "Position tracker application",
                    "info": {
                        "description": "This is the position tracker application with maps"
                    }
                }
            ]
        }
        ```

    - **Example (format=`array`)**

        ```json
        [
            {
                "applicationId": "1640924152438-eVJe8UhY",
                "code": "meter",
                "unitId": "1640923958516-qvdFNpOV",
                "unitCode": "sylvia",
                "createdAt": "2021-12-31T04:15:52.438Z",
                "modifiedAt": "2021-12-31T04:15:52.438Z",
                "hostUri": "amqp://localhost/sylvia",
                "name": "Meter application",
                "info": {
                    "description": "This is the meter program"
                }
            },
            {
                "applicationId": "1640924063709-rmJIxW0s",
                "code": "tracker",
                "unitId": "1640923958516-qvdFNpOV",
                "unitCode": "sylvia",
                "createdAt": "2021-12-31T04:14:23.709Z",
                "modifiedAt": "2021-12-31T04:14:23.709Z",
                "hostUri": "amqp://localhost/sylvia",
                "name": "Position tracker application",
                "info": {
                    "description": "This is the position tracker application with maps"
                }
            }
        ]
        ```

    - **Example (format=`csv`)**

        ```
        applicationId,code,unitId,unitCode,createdAt,modifiedAt,hostUri,name,info
        1640924152438-eVJe8UhY,meter,1640923958516-qvdFNpOV,sylvia,2021-12-31T04:15:52.438Z,2021-12-31T04:15:52.438Z,amqp://localhost/sylvia,Meter application,{"description":"This is the meter program"}
        ```

- **400 Bad Request**: the special error codes are:
    - `err_broker_unit_not_exist`: The unit does not exist.
- **401, 403, 500, 503**: See [Notes](#notes).

## <a name="get_application"></a>Get application information

Get the specified application information.

    GET /coremgr/api/v1/application/{applicationId}

- *string* `applicationId`: The specified application ID to get application information.

#### Response

- **200 OK**:

    - *object* `data`: An object that contains the application information. See [Application APIs - Application list](#get_application_list). Additional fields are:
        - *number* `ttl`: (**present for AMQP**) Message TTL in milliseconds. **0** this is no limit.
        - *number* `length`: (**present for AMQP**) Maximum queue length. **0** this is no limit.

- **400, 401, 403, 500, 503**: See [Notes](#notes).
- **404 Not Found**: The specified application does not exist.

## <a name="patch_application"></a>Update application information

Update the specified application information.

    PATCH /coremgr/api/v1/application/{applicationId}

- *string* `applicationId`: The specified application ID to update application information.

#### Additional HTTP Headers

    Content-Type: application/json

#### Parameters

- *object* `data`:
    - *string* `hostUri`: (**optional**) The application queue URI. Changing this value will reconnect to the new message queue.
    - *string* `name`: (**optional**) The display name.
    - *object* `info`: (**optional**) Other information. You must provide full of fields, or all fields will be replaced with the new value.
    - *number* `ttl`: (**optional for AMQP**) Message TTL in milliseconds. **0** is no limit.
    - *number* `length`: (**optional for AMQP**) Maximum queue length. **0** is no limit.
    - *string* `password`: (**optional. required when changing `hostUri`**) New password for connecting to the queues.

- **Note**: You must give at least one parameter.

- **Example**

    ```json
    {
        "data": {
            "hostUri": "amqp://192.168.1.1/sylvia",
            "name": "The tracker v2 application",
            "info": {
                "description": "The version 2 tracker application",
                "changelog": "add altitude"
            },
            "password": "new password"
        }
    }
    ```

#### Response

- **204 No Content**
- **400, 401, 403, 500, 503**: See [Notes](#notes).
- **404 Not Found**: The specified application does not exist.

## <a name="delete_application"></a>Delete application

Delete an application and its own resources.

    DELETE /coremgr/api/v1/application/{applicationId}

- *string* `applicationId`: The specified application ID to delete.

#### Response

- **204 No Content**
- **401, 403, 500, 503**: See [Notes](#notes).

## <a name="get_application_stats"></a>Get application statistics

Get the specified application statistics.

    GET /coremgr/api/v1/application/{applicationId}/stats

- *string* `applicationId`: The specified application ID to get application statistics.

#### Response

- **200 OK**:

    - *object* `data`: An object that contains the application statistics.
        - *object* `uldata`: The statistics of the `uldata` queue.
            - *number* `consumers`: Number of consumers.
            - *number* `messages`: Number of queuing messages.
            - *number* `publishRate`: Data rate (msg/s) from the sylvia-iot-broker to the queue.
            - *number* `deliverRate`: Data rate (msg/s) from the queue to the consumer(s).
        - *object* `dldataResp`: The statistics of the `dldata-resp` queue.
            - *number* `consumers`: Number of consumers.
            - *number* `messages`: Number of queuing messages.
            - *number* `publishRate`: Data rate (msg/s) from the sylvia-iot-broker to the queue.
            - *number* `deliverRate`: Data rate (msg/s) from the queue to the consumer(s).
        - *object* `dldataResult`: The statistics of the `dldata-result` queue.
            - *number* `consumers`: Number of consumers.
            - *number* `messages`: Number of queuing messages.
            - *number* `publishRate`: Data rate (msg/s) from the sylvia-iot-broker to the queue.
            - *number* `deliverRate`: Data rate (msg/s) from the queue to the consumer(s).

- **400, 401, 403, 500, 503**: See [Notes](#notes).
- **404 Not Found**: The specified application does not exist.

## <a name="post_application_dldata"></a>Send downlink data

Send downlink data to a device.

    POST /coremgr/api/v1/application/{applicationId}/dldata

- *string* `applicationId`: The specified application ID to send application downlink data to a device.

#### Parameters

- *object* `data`: An object that contains the downlink data information.
    - *string* `deviceId`: The target device ID.
    - *string* `payload`: The data payload in **hexadecimal** string format.

- **Example**

    ```json
    {
        "data": {
            "deviceId": "1640924274329-yESwHhKO",
            "data": "74657374"
        }
    }
    ```

- **204 No content**
- **400 Bad Request**: the special error codes are:
    - `err_broker_device_not_exist`: The specified device does not exist.
- **401, 403, 500, 503**: See [Notes](#notes).
- **404 Not Found**: The specified application does not exist.

# <a name="network"></a>Network APIs

## <a name="post_network"></a>Create network

Create a network.

    POST /coremgr/api/v1/network

#### Additional HTTP Headers

    Content-Type: application/json

#### Parameters

- *object* `data`:
    - *string* `code`: Network code for queues. The pattern is `[A-Za-z0-9]{1}[A-Za-z0-9-_]*`. This code will be transformed to lowercase.
    - *string | null* `unitId`: The associated unit ID for private network.
        - **null**: (**for admin or manager only**) the public network.
    - *string* `hostUri`: The network queue URI.
    - *string* `name`: (**optional**) Display name.
    - *object* `info`: (**optional**) Other information.
    - *number* `ttl`: (**optional for AMQP**) Message TTL in milliseconds. **0** is no limit.
    - *number* `length`: (**optional for AMQP**) Maximum queue length. **0** is no limit.

- **Note**:
    - You must assign both `ttl` and `length`. Giving only one value will cause another one to be unlimited.

- **Example**

    ```json
    {
        "data": {
            "code": "lora",
            "unitId": "1640923958516-qvdFNpOV",
            "hostUri": "amqp://localhost/sylvia",
            "name": "Sylvia unit's private LoRa network",
            "info": {
                "description": "This is the private LoRa network"
            },
            "ttl": 3600000,
            "length": 10000
        }
    }
    ```

#### Response

- **200 OK**: The network ID. Parameters are:

    - *object* `data`:
        - *string* `networkId`: The ID of the created network.
        - *string* `password`: The default password.
            - This value will be returned only once. Use the [`PATCH`](#patch_network) API to change the password.

    - **Example**

        ```json
        {
            "data": {
                "networkId": "1640924173420-BNg2lwo3",
                "password": "E4AaTD5H"
            }
        }
        ```

- **400 Bad Request**: the special error codes are:
    - `err_broker_unit_not_exist`: The unit does not exist.
    - `err_broker_network_exist`: The network code has been used.
- **401, 403, 500, 503**: See [Notes](#notes).

## <a name="get_network_count"></a>Network count

Get network list count.

    GET /coremgr/api/v1/network/count?
        unit={specifiedUnitId}&
        contains={word}

- *string* `unit`: (**required for normal user**) To search networks of the specified unit ID.
    - (**for admin or manager only**) Empty string for get public network only.
- *string* `contains`: (**optional**) To search codes which contain the specified word. This is case insensitive.

#### Response

- **200 OK**: Network list count. Parameters are:

    - *object* `data`:
        - *number* `count`: Network list count.

    - **Example**

        ```json
        {
            "data": {
                "count": 2
            }
        }
        ```

- **400, 401, 403, 500, 503**: See [Notes](#notes).

## <a name="get_network_list"></a>Network list

Get network list.

    GET /coremgr/api/v1/network/list?
        unit={specifiedUnitId}&
        contains={word}&
        offset={offset}&
        limit={limit}&
        sort={sortKeysAndOrders}&
        format={responseFormat}

- *string* `unit`: (**required for normal user**) To search networks of the specified unit ID.
    - (**for admin or manager only**) Empty string for get public network only.
- *string* `contains`: (**optional**) To search codes which contain the specified word. This is case insensitive.
- *number* `offset`: (**optional**) Data offset. Default is **0**.
- *number* `limit`: (**optional**) Number of items to list. **0** to list all data. Default is **100**.
- *string* `sort`: (**optional**) To sort the result. Format is `key:[asc|desc]`. The key can be **code**, **created**, **modified**, **name**. Default is **code:asc**.
- *string* `format`: (**optional**) Response body format. Default is array in the **data** field.
    - **array**: The response body is JSON array, not an object.
    - **csv**: The response body is CSV.

#### Response

- **200 OK**: An array that contains all networks' information. Parameters are:

    - *object[]* `data`:
        - *string* `networkId`: Network ID.
        - *string* `code`: Network code for queues.
        - *string | null* `unitId`: The associated unit ID for private network or **null** means public network.
        - *string | null* `unitCode`: The associated unit code.
        - *string* `createdAt`: Creation time in RFC 3339 format.
        - *string* `modifiedAt`: Modification time in RFC 3339 format.
        - *string* `hostUri`: The network queue URI.
        - *string* `name`: Display name.
        - *object* `info`: Other information.

    - **Example**

        ```json
        {
            "data": [
                {
                    "networkId": "1640924173420-BNg2lwo3",
                    "code": "lora",
                    "unitId": "1640923958516-qvdFNpOV",
                    "unitCode": "sylvia",
                    "createdAt": "2021-12-31T04:16:13.420Z",
                    "modifiedAt": "2021-12-31T04:16:13.420Z",
                    "hostUri": "amqp://localhost/sylvia",
                    "name": "Sylvia unit's private LoRa network",
                    "info": {
                        "description": "This is the private LoRa network"
                    }
                },
                {
                    "networkId": "1640924213217-oDwwyNK3",
                    "code": "zigbee",
                    "unitId": null,
                    "unitCode": null,
                    "createdAt": "2021-12-31T04:16:53.217Z",
                    "modifiedAt": "2021-12-31T04:16:53.217Z",
                    "hostUri": "amqp://localhost/sylvia",
                    "name": "Public Zigbee network",
                    "info": {
                        "description": "This is the public Zigbee network"
                    }
                }
            ]
        }
        ```

    - **Example (format=`array`)**

        ```json
        [
            {
                "networkId": "1640924173420-BNg2lwo3",
                "code": "lora",
                "unitId": "1640923958516-qvdFNpOV",
                "unitCode": "sylvia",
                "createdAt": "2021-12-31T04:16:13.420Z",
                "modifiedAt": "2021-12-31T04:16:13.420Z",
                "hostUri": "amqp://localhost/sylvia",
                "name": "Sylvia unit's private LoRa network",
                "info": {
                    "description": "This is the private LoRa network"
                }
            },
            {
                "networkId": "1640924213217-oDwwyNK3",
                "code": "zigbee",
                "unitId": null,
                "unitCode": null,
                "createdAt": "2021-12-31T04:16:53.217Z",
                "modifiedAt": "2021-12-31T04:16:53.217Z",
                "hostUri": "amqp://localhost/sylvia",
                "name": "Public Zigbee network",
                "info": {
                    "description": "This is the public Zigbee network"
                }
            }
        ]
        ```

    - **Example (format=`csv`)**

        ```
        networkId,code,unitId,unitCode,createdAt,modifiedAt,hostUri,name,info
        1640924173420-BNg2lwo3,lora,1640923958516-qvdFNpOV,sylvia,2021-12-31T04:16:13.420Z,2021-12-31T04:16:13.420Z,amqp://localhost/sylvia,Sylvia unit's private LoRa network,{"description":"This is the private LoRa network"}
        ```

- **400 Bad Request**: the special error codes are:
    - `err_broker_unit_not_exist`: The unit does not exist.
- **401, 403, 500, 503**: See [Notes](#notes).

## <a name="get_network"></a>Get network information

Get the specified network information.

    GET /coremgr/api/v1/network/{networkId}

- *string* `networkId`: The specified network ID to get network information.

#### Response

- **200 OK**:

    - *object* `data`: An object that contains the network information. See [Network APIs - Network list](#get_network_list). Additional fields are:
        - *number* `ttl`: (**present for AMQP**) Message TTL in milliseconds. **0** is no limit.
        - *number* `length`: (**present for AMQP**) Maximum queue length. **0** is no limit.

- **400, 401, 403, 500, 503**: See [Notes](#notes).
- **404 Not Found**: The specified network does not exist.

## <a name="patch_network"></a>Update network information

Update the specified network information.

    PATCH /coremgr/api/v1/network/{networkId}

- *string* `networkId`: The specified network ID to update network information.

#### Additional HTTP Headers

    Content-Type: application/json

#### Parameters

- *object* `data`:
    - *string* `hostUri`: (**optional**) The network queue URI. Changing this value will reconnect to the new message queue.
    - *string* `name`: (**optional**) The display name.
    - *object* `info`: (**optional**) Other information. You must provide full of fields, or all fields will be replaced with the new value.
    - *number* `ttl`: (**optional for AMQP**) Message TTL in milliseconds. Without this is no limit.
    - *number* `length`: (**optional for AMQP**) Maximum queue length. Without this is no limit.
    - *string* `password`: (**optional. required when changing `hostUri`**) New password for connecting to the queues.

- **Note**: You must give at least one parameter.

- **Example**

    ```json
    {
        "data": {
            "hostUri": "amqp://192.168.1.1/sylvia",
            "name": "The OpenLoRa network server",
            "info": {
                "changelog": "upgrade LoRa network server"
            }
        }
    }
    ```

#### Response

- **204 No Content**
- **400, 401, 403, 500, 503**: See [Notes](#notes).
- **404 Not Found**: The specified network does not exist.

## <a name="delete_network"></a>Delete network

Delete a network and its own resources.

    DELETE /coremgr/api/v1/network/{networkId}

- *string* `networkId`: The specified network ID to delete.

#### Response

- **204 No Content**
- **401, 403, 500, 503**: See [Notes](#notes).

## <a name="get_network_stats"></a>Get network statistics

Get the specified network statistics.

    GET /coremgr/api/v1/network/{networkId}/stats

- *string* `networkId`: The specified network ID to get network statistics.

#### Response

- **200 OK**:

    - *object* `data`: An object that contains the network statistics.
        - *object* `dldata`: The statistics of the `dldata` queue.
            - *number* `consumers`: Number of consumers.
            - *number* `messages`: Number of queuing messages.
            - *number* `publishRate`: Data rate (msg/s) from the sylvia-iot-broker to the queue.
            - *number* `deliverRate`: Data rate (msg/s) from the queue to the consumer(s).

- **400, 401, 403, 500, 503**: See [Notes](#notes).
- **404 Not Found**: The specified network does not exist.

## <a name="post_network_uldata"></a>Simulate device uplink data

Simulate uplink data from a device.

    POST /coremgr/api/v1/network/{networkId}/uldata

- *string* `networkId`: The specified network ID to send network uplink data from a device.

#### Parameters

- *object* `data`: An object that contains the downlink data information.
    - *string* `deviceId`: The source device ID.
    - *string* `payload`: The data payload in **hexadecimal** string format.

- **Example**

    ```json
    {
        "data": {
            "deviceId": "1640924274329-yESwHhKO",
            "data": "74657374"
        }
    }
    ```

- **204 No content**
- **400 Bad Request**: the special error codes are:
    - `err_broker_device_not_exist`: The specified device does not exist.
- **401, 403, 500, 503**: See [Notes](#notes).
- **404 Not Found**: The specified network does not exist.
