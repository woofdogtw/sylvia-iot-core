API - Broker
============

## Contents

- [Notes](#notes)
- [Common error codes](#errcode)
- [Roles](#roles)
- [Service APIs](#service)
    - [`GET /version` Get service version](#get_version)
- [Unit APIs](#unit)
    - [`POST /broker/api/v1/unit` Create unit](#post_unit)
    - [`GET /broker/api/v1/unit/count` Unit count](#get_unit_count)
    - [`GET /broker/api/v1/unit/list` Unit list](#get_unit_list)
    - [`GET /broker/api/v1/unit/{unitId}` Get unit information](#get_unit)
    - [`PATCH /broker/api/v1/unit/{unitId}` Update unit information](#patch_unit)
    - [`DELETE /broker/api/v1/unit/{unitId}` Delete unit](#delete_unit)
    - [`DELETE /broker/api/v1/unit/user/{userId}` Delete user units](#delete_unit_user)
- [Application APIs](#application)
    - [`POST /broker/api/v1/application` Create application](#post_application)
    - [`GET /broker/api/v1/application/count` Application count](#get_application_count)
    - [`GET /broker/api/v1/application/list` Application list](#get_application_list)
    - [`GET /broker/api/v1/application/{applicationId}` Get application information](#get_application)
    - [`PATCH /broker/api/v1/application/{applicationId}` Update application information](#patch_application)
    - [`DELETE /broker/api/v1/application/{applicationId}` Delete application](#delete_application)
- [Network APIs](#network)
    - [`POST /broker/api/v1/network` Create network](#post_network)
    - [`GET /broker/api/v1/network/count` Network count](#get_network_count)
    - [`GET /broker/api/v1/network/list` Network list](#get_network_list)
    - [`GET /broker/api/v1/network/{networkId}` Get network information](#get_network)
    - [`PATCH /broker/api/v1/network/{networkId}` Update network information](#patch_network)
    - [`DELETE /broker/api/v1/network/{networkId}` Delete network](#delete_network)
- [Device APIs](#device)
    - [`POST /broker/api/v1/device` Create device](#post_device)
    - [`POST /broker/api/v1/device/bulk` Bulk creating devices](#post_device_bulk)
    - [`POST /broker/api/v1/device/bulk-delete` Bulk deleting devices](#post_device_bulk_del)
    - [`POST /broker/api/v1/device/range` Bulk creating devices with address range](#post_device_range)
    - [`POST /broker/api/v1/device/range-delete` Bulk deleting devices with address range](#post_device_range_del)
    - [`GET /broker/api/v1/device/count` Device count](#get_device_count)
    - [`GET /broker/api/v1/device/list` Device list](#get_device_list)
    - [`GET /broker/api/v1/device/{deviceId}` Get device information](#get_device)
    - [`PATCH /broker/api/v1/device/{deviceId}` Update device information](#patch_device)
    - [`DELETE /broker/api/v1/device/{deviceId}` Delete device](#delete_device)
- [Device route APIs](#device_route)
    - [`POST /broker/api/v1/device-route` Create device route](#post_device_route)
    - [`POST /broker/api/v1/device-route/bulk` Bulk creating device routes](#post_device_route_bulk)
    - [`POST /broker/api/v1/device-route/bulk-delete` Bulk deleting device routes](#post_device_route_bulk_del)
    - [`POST /broker/api/v1/device-route/range` Bulk creating device routes with address range](#post_device_route_range)
    - [`POST /broker/api/v1/device-route/range-delete` Bulk deleting device routes with address range](#post_device_route_range_del)
    - [`GET /broker/api/v1/device-route/count` Device route count](#get_device_route_count)
    - [`GET /broker/api/v1/device-route/list` Device route list](#get_device_route_list)
    - [`DELETE /broker/api/v1/device-route/{routeId}` Delete device route](#delete_device_route)
- [Network route APIs](#network_route)
    - [`POST /broker/api/v1/network-route` Create network route](#post_network_route)
    - [`GET /broker/api/v1/network-route/count` Network route count](#get_network_route_count)
    - [`GET /broker/api/v1/network-route/list` Network route list](#get_network_route_list)
    - [`DELETE /broker/api/v1/network-route/{routeId}` Delete network route](#delete_network_route)
- [Downlink data buffer APIs](#dldata_buffer)
    - [`GET /broker/api/v1/dldata-buffer/count` Downlink data buffer count](#get_dldata_buffer_count)
    - [`GET /broker/api/v1/dldata-buffer/list` Downlink data buffer list](#get_dldata_buffer_list)
    - [`DELETE /broker/api/v1/dldata-buffer/{dataId}` Delete downlink data buffer](#delete_dldata_buffer)

## <a name="notes"></a>Notes

All API requests (except `GET /version`) must have a **Authorization** header with a **Bearer** token.

- **Example**

    ```http
    GET /broker/api/v1/unit/list HTTP/1.1
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

All APIs except [Create unit](#post_unit) can be used by unit owners.
- Only `GET` APIs are available for unit members.
- System administrators and managers (role with **admin** and **manager**) can manage all unit resources.
- Unit owners and members can only manage their unit resources.

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
                "name": "sylvia-iot-broker",
                "version": "1.0.0"
            }
        }
        ```

    - **Example** when `q=name`:

        ```
        sylvia-iot-broker
        ```

    - **Example** when `q=version`:

        ```
        1.0.0
        ```

# <a name="unit"></a>Unit APIs

The following API can be only used by system administrators or managers:
- `DELETE /broker/api/v1/unit/user/{userId}`

## <a name="post_unit"></a>Create unit

Create a unit. The user who use this API will be the owner of the unit.

    POST /broker/api/v1/unit

#### Additional HTTP Headers

    Content-Type: application/json

#### Parameters

- *object* `data`:
    - *string* `code`: Unit code. The pattern is `[A-Za-z0-9]{1}[A-Za-z0-9-_]*`. This code will be transformed to lowercase.
    - *string* `ownerId`: (**optional for admin and manager**) The specified user ID for this unit.
    - *string* `name`: (**optional**) Display name.
    - *object* `info`: (**optional**) Other information.

- **Example**

    ```json
    {
        "data": {
            "code": "sylvia",
            "name": "Sylvia unit",
            "info": {
                "address": "Mt. Sylvia"
            }
        }
    }
    ```

#### Response

- **200 OK**: The unit ID. Parameters are:

    - *object* `data`:
        - *string* `unitId`: The ID of the created unit.

    - **Example**

        ```json
        {
            "data": {
                "unitId": "1640923958516-qvdFNpOV"
            }
        }
        ```

- **400 Bad Request**: the special error codes are:
    - `err_broker_owner_not_exist`: The specified owner does not exist.
    - `err_broker_unit_exist`: The unit code has been used.
- **401, 403, 500, 503**: See [Notes](#notes).

## <a name="get_unit_count"></a>Unit count

Get unit list count.

    GET /broker/api/v1/unit/count?
        owner={specifiedOwnerId}&
        member={specifiedMemberId}&
        contains={word}

- *string* `owner`: (**optional for admin and manager**) To search units of the specified owner user ID.
- *string* `member`: (**optional for admin and manager**) To search units of the specified member user ID.
- *string* `contains`: (**optional**) To search codes which contain the specified word. This is case insensitive.

#### Response

- **200 OK**: Unit list count. Parameters are:

    - *object* `data`:
        - *number* `count`: Unit list count.

    - **Example**

        ```json
        {
            "data": {
                "count": 1
            }
        }
        ```

- **400, 401, 403, 500, 503**: See [Notes](#notes).

## <a name="get_unit_list"></a>Unit list

Get unit list.

    GET /broker/api/v1/unit/list?
        owner={specifiedOwnerId}&
        member={specifiedMemberId}&
        contains={word}&
        offset={offset}&
        limit={limit}&
        sort={sortKeysAndOrders}&
        format={responseFormat}

- *string* `owner`: (**optional for admin and manager**) To search units of the specified owner user ID.
- *string* `member`: (**optional for admin and manager**) To search units of the specified member user ID.
- *string* `contains`: (**optional**) To search codes which contain the specified word. This is case insensitive.
- *number* `offset`: (**optional**) Data offset. Default is **0**.
- *number* `limit`: (**optional**) Number of items to list. **0** to list all data. Default is **100**.
- *string* `sort`: (**optional**) To sort the result. Format is `key:[asc|desc]`. The key can be **code**, **created**, **modified**, **name**. Default is **code:asc**.
- *string* `format`: (**optional**) Response body format. Default is array in the **data** field.
    - **array**: The response body is JSON array, not an object.

#### Response

- **200 OK**: An array that contains all units' information. Parameters are:

    - *object[]* `data`:
        - *string* `unitId`: Unit ID.
        - *string* `code`: Unit code.
        - *string* `createdAt`: Creation time in RFC 3339 format.
        - *string* `modifiedAt`: Modification time in RFC 3339 format.
        - *string* `ownerId`: User ID that is the owner of the unit.
        - *string[]* `memberIds`: User IDs who can view the unit's resources (application, network, ...).
        - *string* `name`: Display name.
        - *object* `info`: Other information.

    - **Example**

        ```json
        {
            "data": [
                {
                    "unitId": "1640923958516-qvdFNpOV",
                    "code": "sylvia",
                    "createdAt": "2021-12-31T04:12:38.516Z",
                    "modifiedAt": "2021-12-31T04:12:38.516Z",
                    "ownerId": "1640921188987-x51FTVPD",
                    "memberIds": [
                        "1640921188987-x51FTVPD",
                        "1641003827053-c2e84RJO"
                    ],
                    "name": "Sylvia unit",
                    "info": {
                        "address": "Mt. Sylvia"
                    }
                }
            ]
        }
        ```

    - **Example (format=`array`)**

        ```json
        [
            {
                "unitId": "1640923958516-qvdFNpOV",
                "code": "sylvia",
                "createdAt": "2021-12-31T04:12:38.516Z",
                "modifiedAt": "2021-12-31T04:12:38.516Z",
                "ownerId": "1640921188987-x51FTVPD",
                "memberIds": [
                    "1640921188987-x51FTVPD",
                    "1641003827053-c2e84RJO"
                ],
                "name": "Sylvia unit",
                "info": {
                    "address": "Mt. Sylvia"
                }
            }
        ]
        ```

- **400, 401, 403, 500, 503**: See [Notes](#notes).

## <a name="get_unit"></a>Get unit information

Get the specified unit information.

    GET /broker/api/v1/unit/{unitId}

- *string* `unitId`: The specified unit ID to get unit information.

#### Response

- **200 OK**:

    - *object* `data`: An object that contains the unit information. See [Unit APIs - Unit list](#get_unit_list).

- **400, 401, 403, 500, 503**: See [Notes](#notes).
- **404 Not Found**: The specified unit does not exist.

## <a name="patch_unit"></a>Update unit information

Update the specified unit information.

    PATCH /broker/api/v1/unit/{unitId}

- *string* `unitId`: The specified unit ID to update unit information.

#### Additional HTTP Headers

    Content-Type: application/json

#### Parameters

- *object* `data`:
    - *string* `ownerId`: (**optional for admin and manager**) The user ID of the new owner.
    - *string[]* `memberIds`: (**optional for admin and manager**) The user IDs who can view the unit's resources.
    - *string* `name`: (**optional**) The display name.
    - *object* `info`: (**optional**) Other information. You must provide full of fields, or all fields will be replaced with the new value.

- **Note**: You must give at least one parameter.

- **Example**

    ```json
    {
        "data": {
            "name": "Sylvia depart",
            "info": {
                "contact": "Sylvia depart's contact",
                "address": "Mt. Sylvia"
            }
        }
    }
    ```

#### Response

- **204 No Content**
- **400 Bad Request**: the special error codes are:
    - `err_broker_owner_not_exist`: The specified owner does not exist.
    - `err_broker_member_not_exist`: The specified member(s) does not exist.
- **401, 403, 500, 503**: See [Notes](#notes).
- **404 Not Found**: The specified unit does not exist.

## <a name="delete_unit"></a>Delete unit

Delete a unit and its own resources.

    DELETE /broker/api/v1/unit/{unitId}

- *string* `unitId`: The specified unit ID to delete.

#### Response

- **204 No Content**
- **401, 403, 500, 503**: See [Notes](#notes).
- **404 Not Found**: The specified unit does not exist.

## <a name="delete_unit_user"></a>Delete user units

Delete all units and their resources of the specified owner.

    DELETE /auth/api/v1/unit/user/{userId}

- *string* `userId`: The specified user ID of units' owner.

#### Response

- **204 No Content**
- **400, 401, 403, 500, 503**: See [Notes](#notes).
- **404 Not Found**: The specified user does not exist.

# <a name="application"></a>Application APIs

## <a name="post_application"></a>Create application

Create an application.

    POST /broker/api/v1/application

#### Additional HTTP Headers

    Content-Type: application/json

#### Parameters

- *object* `data`:
    - *string* `code`: Application code for queues. The pattern is `[A-Za-z0-9]{1}[A-Za-z0-9-_]*`. This code will be transformed to lowercase.
    - *string* `unitId`: The associated unit ID.
    - *string* `hostUri`: The application queue URI.
    - *string* `name`: (**optional**) Display name.
    - *object* `info`: (**optional**) Other information.

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
            }
        }
    }
    ```

#### Response

- **200 OK**: The application ID. Parameters are:

    - *object* `data`:
        - *string* `applicationId`: The ID of the created application.

    - **Example**

        ```json
        {
            "data": {
                "applicationId": "1640924063709-rmJIxW0s"
            }
        }
        ```

- **400 Bad Request**: the special error codes are:
    - `err_broker_unit_not_exist`: The unit does not exist.
    - `err_broker_application_exist`: The application code has been used.
- **401, 403, 500, 503**: See [Notes](#notes).

## <a name="get_application_count"></a>Application count

Get application list count.

    GET /broker/api/v1/application/count?
        unit={specifiedUnitId}&
        code={specifiedCode}&
        contains={word}

- *string* `unit`: (**required for normal user**) To search applications of the specified unit ID.
- *string* `code`: (**optional**) To search the specified code. This is case insensitive and excludes **contains**.
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

    GET /broker/api/v1/application/list?
        unit={specifiedUnitId}&
        code={specifiedCode}&
        contains={word}&
        offset={offset}&
        limit={limit}&
        sort={sortKeysAndOrders}&
        format={responseFormat}

- *string* `unit`: (**required for normal user**) To search applications of the specified unit ID.
- *string* `code`: (**optional**) To search the specified code. This is case insensitive and excludes **contains**.
- *string* `contains`: (**optional**) To search codes which contain the specified word. This is case insensitive.
- *number* `offset`: (**optional**) Data offset. Default is **0**.
- *number* `limit`: (**optional**) Number of items to list. **0** to list all data. Default is **100**.
- *string* `sort`: (**optional**) To sort the result. Format is `key:[asc|desc]`. The key can be **code**, **created**, **modified**, **name**. Default is **code:asc**.
- *string* `format`: (**optional**) Response body format. Default is array in the **data** field.
    - **array**: The response body is JSON array, not an object.

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

- **400 Bad Request**: the special error codes are:
    - `err_broker_unit_not_exist`: The unit does not exist.
- **401, 403, 500, 503**: See [Notes](#notes).

## <a name="get_application"></a>Get application information

Get the specified application information.

    GET /broker/api/v1/application/{applicationId}

- *string* `applicationId`: The specified application ID to get application information.

#### Response

- **200 OK**:

    - *object* `data`: An object that contains the application information. See [Application APIs - Application list](#get_application_list).

- **400, 401, 403, 500, 503**: See [Notes](#notes).
- **404 Not Found**: The specified application does not exist.

## <a name="patch_application"></a>Update application information

Update the specified application information.

    PATCH /broker/api/v1/application/{applicationId}

- *string* `applicationId`: The specified application ID to update application information.

#### Additional HTTP Headers

    Content-Type: application/json

#### Parameters

- *object* `data`:
    - *string* `hostUri`: (**optional**) The application queue URI. Changing this value will reconnect to the new message queue.
    - *string* `name`: (**optional**) The display name.
    - *object* `info`: (**optional**) Other information. You must provide full of fields, or all fields will be replaced with the new value.

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
            }
        }
    }
    ```

#### Response

- **204 No Content**
- **400, 401, 403, 500, 503**: See [Notes](#notes).
- **404 Not Found**: The specified application does not exist.

## <a name="delete_application"></a>Delete application

Delete an application and its own resources.

    DELETE /broker/api/v1/application/{applicationId}

- *string* `applicationId`: The specified application ID to delete.

#### Response

- **204 No Content**
- **401, 403, 500, 503**: See [Notes](#notes).

# <a name="network"></a>Network APIs

## <a name="post_network"></a>Create network

Create a network.

    POST /broker/api/v1/network

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
            }
        }
    }
    ```

#### Response

- **200 OK**: The network ID. Parameters are:

    - *object* `data`:
        - *string* `networkId`: The ID of the created network.

    - **Example**

        ```json
        {
            "data": {
                "networkId": "1640924173420-BNg2lwo3"
            }
        }
        ```

- **400 Bad Request**: the special error codes are:
    - `err_broker_unit_not_exist`: The unit does not exist.
    - `err_broker_network_exist`: The network code has been used.
- **401, 403, 500, 503**: See [Notes](#notes).

## <a name="get_network_count"></a>Network count

Get network list count.

    GET /broker/api/v1/network/count?
        unit={specifiedUnitId}&
        code={specifiedCode}&
        contains={word}

- *string* `unit`: (**required for normal user**) To search networks of the specified unit ID.
    - (**for admin or manager only**) Empty string for get public network only.
- *string* `code`: (**optional**) To search the specified code. This is case insensitive and excludes **contains**.
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

    GET /broker/api/v1/network/list?
        unit={specifiedUnitId}&
        code={specifiedCode}&
        contains={word}&
        offset={offset}&
        limit={limit}&
        sort={sortKeysAndOrders}&
        format={responseFormat}

- *string* `unit`: (**required for normal user**) To search networks of the specified unit ID.
    - (**for admin or manager only**) Empty string for get public network only.
- *string* `code`: (**optional**) To search the specified code. This is case insensitive and excludes **contains**.
- *string* `contains`: (**optional**) To search codes which contain the specified word. This is case insensitive.
- *number* `offset`: (**optional**) Data offset. Default is **0**.
- *number* `limit`: (**optional**) Number of items to list. **0** to list all data. Default is **100**.
- *string* `sort`: (**optional**) To sort the result. Format is `key:[asc|desc]`. The key can be **code**, **created**, **modified**, **name**. Default is **code:asc**.
- *string* `format`: (**optional**) Response body format. Default is array in the **data** field.
    - **array**: The response body is JSON array, not an object.

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

- **400 Bad Request**: the special error codes are:
    - `err_broker_unit_not_exist`: The unit does not exist.
- **401, 403, 500, 503**: See [Notes](#notes).

## <a name="get_network"></a>Get network information

Get the specified network information.

    GET /broker/api/v1/network/{networkId}

- *string* `networkId`: The specified network ID to get network information.

#### Response

- **200 OK**:

    - *object* `data`: An object that contains the network information. See [Network APIs - Network list](#get_network_list).

- **400, 401, 403, 500, 503**: See [Notes](#notes).
- **404 Not Found**: The specified network does not exist.

## <a name="patch_network"></a>Update network information

Update the specified network information.

    PATCH /broker/api/v1/network/{networkId}

- *string* `networkId`: The specified network ID to update network information.

#### Additional HTTP Headers

    Content-Type: application/json

#### Parameters

- *object* `data`:
    - *string* `hostUri`: (**optional**) The network queue URI. Changing this value will reconnect to the new message queue.
    - *string* `name`: (**optional**) The display name.
    - *object* `info`: (**optional**) Other information. You must provide full of fields, or all fields will be replaced with the new value.

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

    DELETE /broker/api/v1/network/{networkId}

- *string* `networkId`: The specified network ID to delete.

#### Response

- **204 No Content**
- **401, 403, 500, 503**: See [Notes](#notes).

# <a name="device"></a>Device APIs

## <a name="post_device"></a>Create device

Create a device.

    POST /broker/api/v1/device

#### Additional HTTP Headers

    Content-Type: application/json

#### Parameters

- *object* `data`:
    - *string* `unitId`: The associated unit ID.
    - *string* `networkId`: The associated network ID. The public network can be assigned by **admin** or **manager**.
    - *string* `networkAddr`: The network address of the specified network. This will be transformed to lowercase.
    - *string* `profile`: (**optional**) Device profile that is used for application servers to identify data content. The pattern is empty or `[A-Za-z0-9]{1}[A-Za-z0-9-_]*`.
    - *string* `name`: (**optional**) Display name.
    - *object* `info`: (**optional**) Other information.

- **Example**

    ```json
    {
        "data": {
            "unitId": "1640923958516-qvdFNpOV",
            "networkId": "1640924173420-BNg2lwo3",
            "networkAddr": "800012ae",
            "profile": "tracker",
            "name": "Mt. Sylvia tracker",
            "info": {
                "latitude": "24.38349800818775",
                "longitude": "121.22999674970842"
            }
        }
    }
    ```

#### Response

- **200 OK**: The device ID. Parameters are:

    - *object* `data`:
        - *string* `deviceId`: The ID of the created device.

    - **Example**

        ```json
        {
            "data": {
                "deviceId": "1640924274329-yESwHhKO"
            }
        }
        ```

- **400 Bad Request**: the special error codes are:
    - `err_broker_unit_not_exist`: The unit does not exist.
    - `err_broker_network_not_exist`: The network does not exist.
    - `err_broker_network_addr_exist`: The network address has been used.
- **401, 403, 500, 503**: See [Notes](#notes).

## <a name="post_device_bulk"></a>Bulk creating devices

Create devices in bulk.

    POST /broker/api/v1/device/bulk

Notes:
- Devices created earlier will be skipped.
- **name** will be the network address.
- Maximum 1024 devices.

#### Additional HTTP Headers

    Content-Type: application/json

#### Parameters

- *object* `data`:
    - *string* `unitId`: The associated unit ID.
    - *string* `networkId`: The associated network ID. The public network can be assigned by **admin** or **manager**.
    - *string[]* `networkAddrs`: The network addresses of the specified network. This will be transformed to lowercase.
    - *string* `profile`: (**optional**) Device profile that is used for application servers to identify data content. The pattern is empty or `[A-Za-z0-9]{1}[A-Za-z0-9-_]*`.

- **Example**

    ```json
    {
        "data": {
            "unitId": "1640923958516-qvdFNpOV",
            "networkId": "1640924173420-BNg2lwo3",
            "networkAddrs": [
                "800012ae",
                "8000257f",
                "800022f3"
            ],
            "profile": ""
        }
    }
    ```

#### Response

- **204 No Content**
- **400 Bad Request**: the special error codes are:
    - `err_broker_unit_not_exist`: The unit does not exist.
    - `err_broker_network_not_exist`: The network does not exist.
- **401, 403, 500, 503**: See [Notes](#notes).

## <a name="post_device_bulk_del"></a>Bulk deleting devices

Delete devices in bulk.

    POST /broker/api/v1/device/bulk-delete

Notes:
- Maximum 1024 devices.

#### Additional HTTP Headers

    Content-Type: application/json

#### Parameters

- *object* `data`:
    - *string* `unitId`: The associated unit ID.
    - *string* `networkId`: The associated network ID. The public network can be assigned by **admin** or **manager**.
    - *string[]* `networkAddrs`: The network addresses of the specified network. This will be transformed to lowercase.

- **Example**

    ```json
    {
        "data": {
            "unitId": "1640923958516-qvdFNpOV",
            "networkId": "1640924173420-BNg2lwo3",
            "networkAddrs": [
                "800012ae",
                "8000257f",
                "800022f3"
            ]
        }
    }
    ```

#### Response

- **204 No Content**
- **400 Bad Request**: the special error codes are:
    - `err_broker_unit_not_exist`: The unit does not exist.
    - `err_broker_network_not_exist`: The network does not exist.
- **401, 403, 500, 503**: See [Notes](#notes).

## <a name="post_device_range"></a>Bulk creating devices with address range

Create devices in bulk with address range.

    POST /broker/api/v1/device/range

Notes:
- Devices created earlier will be skipped.
- **name** will be the network address.
- Maximum 1024 devices (0x400).
- `startAddrs` and `endAddrs` must be hexadecimal string with the same (even) length.
- Strings up to 32 bytes (128 bits) are supported.

#### Additional HTTP Headers

    Content-Type: application/json

#### Parameters

- *object* `data`:
    - *string* `unitId`: The associated unit ID.
    - *string* `networkId`: The associated network ID. The public network can be assigned by **admin** or **manager**.
    - *string* `startAddr`: The start network address of the specified network.
    - *string* `endAddr`: The end network address of the specified network.
    - *string* `profile`: (**optional**) The device profile that is used for application servers to identify data content. The pattern is empty or `[A-Za-z0-9]{1}[A-Za-z0-9-_]*`.

- **Example**

    ```json
    {
        "data": {
            "unitId": "1640923958516-qvdFNpOV",
            "networkId": "1640924173420-BNg2lwo3",
            "startAddr": "80001000",
            "endAddr": "800013ff",
            "profile": "tracker"
        }
    }
    ```

#### Response

- **204 No Content**
- **400 Bad Request**: the special error codes are:
    - `err_broker_unit_not_exist`: The unit does not exist.
    - `err_broker_network_not_exist`: The network does not exist.
- **401, 403, 500, 503**: See [Notes](#notes).

## <a name="post_device_range_del"></a>Bulk deleting devices with address range

Delete devices in bulk with address range.

    POST /broker/api/v1/device/range-delete

Notes:
- Maximum 1024 devices (0x400).
- `startAddrs` and `endAddrs` must be hexadecimal string with the same (even) length.
- Strings up to 32 bytes (128 bits) are supported.

#### Additional HTTP Headers

    Content-Type: application/json

#### Parameters

- *object* `data`:
    - *string* `unitId`: The associated unit ID.
    - *string* `networkId`: The associated network ID. The public network can be assigned by **admin** or **manager**.
    - *string* `startAddr`: The start network address of the specified network.
    - *string* `endAddr`: The end network address of the specified network.

- **Example**

    ```json
    {
        "data": {
            "unitId": "1640923958516-qvdFNpOV",
            "networkId": "1640924173420-BNg2lwo3",
            "startAddr": "80001000",
            "endAddr": "800013ff"
        }
    }
    ```

#### Response

- **204 No Content**
- **400 Bad Request**: the special error codes are:
    - `err_broker_unit_not_exist`: The unit does not exist.
    - `err_broker_network_not_exist`: The network does not exist.
- **401, 403, 500, 503**: See [Notes](#notes).

## <a name="get_device_count"></a>Device count

Get device list count.

    GET /broker/api/v1/device/count?
        unit={specifiedUnitId}&
        network={specifiedNetworkId}&
        addr={specifiedNetworkAddr}&
        profile={specifiedProfile}&
        contains={word}

- *string* `unit`: (**required for normal user**) To search devices of the specified unit ID.
- *string* `network`: (**optional**) To search devices of the specified network ID.
- *string* `addr`: (**optional**) To search devices of the specified network address.
- *string* `profile`: (**optional**) To search devices of the specified profile.
- *string* `contains`: (**optional**) To search names which contain the specified word. This is case insensitive.

#### Response

- **200 OK**: Device list count. Parameters are:

    - *object* `data`:
        - *number* `count`: Device list count.

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

## <a name="get_device_list"></a>Device list

Get device list.

    GET /broker/api/v1/device/list?
        unit={specifiedUnitId}&
        network={specifiedNetworkId}&
        addr={specifiedNetworkAddr}&
        profile={specifiedProfile}&
        contains={word}&
        offset={offset}&
        limit={limit}&
        sort={sortKeysAndOrders}&
        format={responseFormat}

- *string* `unit`: (**required for normal user**) To search devices of the specified unit ID.
- *string* `network`: (**optional**) To search devices of the specified network ID.
- *string* `addr`: (**optional**) To search devices of the specified network address.
- *string* `profile`: (**optional**) To search devices of the specified profile.
- *string* `contains`: (**optional**) To search names which contain the specified word. This is case insensitive.
- *number* `offset`: (**optional**) Data offset. Default is **0**.
- *number* `limit`: (**optional**) Number of items to list. **0** to list all data. Default is **100**.
- *string* `sort`: (**optional**) To sort the result. Format is `key:[asc|desc]`. The key can be **network**, **addr**, **created**, **modified**, **profile**, **name**. Default is **network:asc,addr:asc**.
- *string* `format`: (**optional**) Response body format. Default is array in the **data** field.
    - **array**: The response body is JSON array, not an object.

#### Response

- **200 OK**: An array that contains all devices' information. Parameters are:

    - *object[]* `data`:
        - *string* `deviceId`: Device ID.
        - *string* `unitId`: The associated unit ID.
        - *string | null* `unitCode`: The associated unit code for private network or **null** means public network.
        - *string* `networkId`: The associated network ID.
        - *string* `networkCode`: The associated network code.
        - *string* `networkAddr`: The associated network address.
        - *string* `createdAt`: Creation time in RFC 3339 format.
        - *string* `modifiedAt`: Modification time in RFC 3339 format.
        - *string* `profile`: The device profile that is used for application servers to identify data content.
        - *string* `name`: Display name.
        - *object* `info`: Other information.

    - **Example**

        ```json
        {
            "data": [
                {
                    "deviceId": "1640924274329-yESwHhKO",
                    "unitId": "1640923958516-qvdFNpOV",
                    "unitCode": "sylvia",
                    "networkId": "1640924173420-BNg2lwo3",
                    "networkCode": "lora",
                    "networkAddr": "800012ae",
                    "createdAt": "2021-12-31T04:17:54.329Z",
                    "modifiedAt": "2021-12-31T04:17:54.329Z",
                    "profile": "tracker",
                    "name": "Mt. Sylvia tracker",
                    "info": {
                        "latitude": "24.38349800818775",
                        "longitude": "121.22999674970842"
                    }
                },
                {
                    "unitId": "1640924263290-Pyc1T5fT",
                    "unitCode": null,
                    "networkId": "1640924213217-oDwwyNK3",
                    "networkCode": "zigbee",
                    "networkAddr": "13a2",
                    "createdAt": "2021-12-31T04:17:43.290Z",
                    "modifiedAt": "2021-12-31T04:17:43.290Z",
                    "profile": "",
                    "name": "Mt. Sylvia meter",
                    "info": {
                        "latitude": "24.38349800818775",
                        "longitude": "121.22999674970842"
                    }
                }
            ]
        }
        ```

    - **Example (format=`array`)**

        ```json
        [
            {
                "deviceId": "1640924274329-yESwHhKO",
                "unitId": "1640923958516-qvdFNpOV",
                "unitCode": "sylvia",
                "networkId": "1640924173420-BNg2lwo3",
                "networkCode": "lora",
                "networkAddr": "800012ae",
                "createdAt": "2021-12-31T04:17:54.329Z",
                "modifiedAt": "2021-12-31T04:17:54.329Z",
                "profile": "tracker",
                "name": "Mt. Sylvia tracker",
                "info": {
                    "latitude": "24.38349800818775",
                    "longitude": "121.22999674970842"
                }
            },
            {
                "deviceId": "1640924263290-Pyc1T5fT",
                "unitId": "1640923958516-qvdFNpOV",
                "unitCode": null,
                "networkId": "1640924213217-oDwwyNK3",
                "networkCode": "zigbee",
                "networkAddr": "13a2",
                "createdAt": "2021-12-31T04:17:43.290Z",
                "modifiedAt": "2021-12-31T04:17:43.290Z",
                "profile": "",
                "name": "Mt. Sylvia meter",
                "info": {
                    "latitude": "24.38349800818775",
                    "longitude": "121.22999674970842"
                }
            }
        ]
        ```

- **400 Bad Request**: the special error codes are:
    - `err_broker_unit_not_exist`: The unit does not exist.
- **401, 403, 500, 503**: See [Notes](#notes).

## <a name="get_device"></a>Get device information

Get the specified device information.

    GET /broker/api/v1/device/{deviceId}

- *string* `deviceId`: The specified device ID to get device information.

#### Response

- **200 OK**:

    - *object* `data`: An object that contains the device information. See [Device APIs - Device list](#get_device_list).

- **400, 401, 403, 500, 503**: See [Notes](#notes).
- **404 Not Found**: The specified device does not exist.

## <a name="patch_device"></a>Update device information

Update the specified device information.

    PATCH /broker/api/v1/device/{deviceId}

- *string* `deviceId`: The specified device ID to update device information.

#### Additional HTTP Headers

    Content-Type: application/json

#### Parameters

- *object* `data`:
    - *string* `networkId`: (**optional**) The associated network ID. The public network can be assigned by **admin** or **manager**.
    - *string* `networkAddr`: (**optional**) The network address of the specified network. This will be transformed to lowercase.
    - *string* `profile`: (**optional**) The device profile. The pattern is empty or `[A-Za-z0-9]{1}[A-Za-z0-9-_]*`.
    - *string* `name`: (**optional**) The display name.
    - *object* `info`: (**optional**) Other information. You must provide full of fields, or all fields will be replaced with the new value.

- **Note**: You must give at least one parameter.

- **Example**

    ```json
    {
        "data": {
            "networkId": "1640924213217-oDwwyNK3",
            "networkAddr": "13a2",
            "profile": "e-meter",
            "name": "Mt. Sylvia e-meter",
            "info": {
                "latitude": "24.38349800818775",
                "longitude": "121.22999674970842",
                "changelog": "upgrade meter Dec 31st"
            }
        }
    }
    ```

#### Response

- **204 No Content**
- **400 Bad Request**: the special error codes are:
    - `err_broker_network_not_exist`: The network does not exist.
    - `err_broker_network_addr_exist`: The network address has been used.
- **401, 403, 500, 503**: See [Notes](#notes).
- **404 Not Found**: The specified device does not exist.

## <a name="delete_device"></a>Delete device

Delete a device and its own resources.

    DELETE /broker/api/v1/device/{deviceId}

- *string* `deviceId`: The specified device ID to delete.

#### Response

- **204 No Content**
- **401, 403, 500, 503**: See [Notes](#notes).

# <a name="device_route"></a>Device route APIs

## <a name="post_device_route"></a>Create device route

Create an device route.

    POST /broker/api/v1/device-route

#### Additional HTTP Headers

    Content-Type: application/json

#### Parameters

- *object* `data`:
    - *string* `deviceId`: The device ID.
    - *string* `applicationId`: The target application ID.

- **Example**

    ```json
    {
        "data": {
            "deviceId": "1640924274329-yESwHhKO",
            "applicationId": "1640924063709-rmJIxW0s"
        }
    }
    ```

#### Response

- **200 OK**: The device ID. Parameters are:

    - *object* `data`:
        - *string* `routeId`: The route ID of the created device route.

    - **Example**

        ```json
        {
            "data": {
                "routeId": "1640924308457-D455ZtkDW033"
            }
        }
        ```

- **400 Bad Request**: the special error codes are:
    - `err_broker_application_not_exist`: The application does not exist.
    - `err_broker_device_not_exist`: The device does not exist.
    - `err_broker_route_exist`: The device route has been created.
    - `err_broker_unit_not_match`: The unit of the device and application is not the same.
- **401, 403, 500, 503**: See [Notes](#notes).

## <a name="post_device_route_bulk"></a>Bulk creating device routes

Create device routes in bulk.

    POST /broker/api/v1/device-route/bulk

Notes:
- Devices routes created earlier will be skipped.
- Maximum 1024 devices.

#### Additional HTTP Headers

    Content-Type: application/json

#### Parameters

- *object* `data`:
    - *string* `applicationId`: The target application ID.
    - *string* `networkId`: The associated network ID. The public network can be assigned by **admin** or **manager**.
    - *string[]* `networkAddrs`: The network addresses of the specified network. This will be transformed to lowercase.

- **Example**

    ```json
    {
        "data": {
            "applicationId": "1640924063709-rmJIxW0s",
            "networkId": "1640924173420-BNg2lwo3",
            "networkAddrs": [
                "800012ae",
                "8000257f",
                "800022f3"
            ]
        }
    }
    ```

#### Response

- **204 No Content**
- **400 Bad Request**: the special error codes are:
    - `err_broker_application_not_exist`: The application does not exist.
    - `err_broker_network_not_exist`: The network does not exist.
    - `err_broker_device_not_exist`: The device does not exist.
- **401, 403, 500, 503**: See [Notes](#notes).

## <a name="post_device_route_bulk_del"></a>Bulk deleting device routes

Delete device routes in bulk.

    POST /broker/api/v1/device-route/bulk-delete

Notes:
- Maximum 1024 devices.

#### Additional HTTP Headers

    Content-Type: application/json

#### Parameters

- *object* `data`:
    - *string* `applicationId`: The target application ID.
    - *string* `networkId`: The associated network ID. The public network can be assigned by **admin** or **manager**.
    - *string[]* `networkAddrs`: The network addresses of the specified network. This will be transformed to lowercase.

- **Example**

    ```json
    {
        "data": {
            "applicationId": "1640924063709-rmJIxW0s",
            "networkId": "1640924173420-BNg2lwo3",
            "networkAddrs": [
                "800012ae",
                "8000257f",
                "800022f3"
            ]
        }
    }
    ```

#### Response

- **204 No Content**
- **400 Bad Request**: the special error codes are:
    - `err_broker_application_not_exist`: The application does not exist.
    - `err_broker_network_not_exist`: The network does not exist.
- **401, 403, 500, 503**: See [Notes](#notes).

## <a name="post_device_route_range"></a>Bulk creating device routes with address range

Create device routes in bulk with address range.

    POST /broker/api/v1/device-route/range

Notes:
- Devices routes created earlier will be skipped.
- Maximum 1024 devices (0x400).
- `startAddrs` and `endAddrs` must be hexadecimal string with the same (even) length.
- Strings up to 32 bytes (128 bits) are supported.

#### Additional HTTP Headers

    Content-Type: application/json

#### Parameters

- *object* `data`:
    - *string* `applicationId`: The target application ID.
    - *string* `networkId`: The associated network ID. The public network can be assigned by **admin** or **manager**.
    - *string* `startAddr`: The start network address of the specified network.
    - *string* `endAddr`: The end network address of the specified network.

- **Example**

    ```json
    {
        "data": {
            "applicationId": "1640924063709-rmJIxW0s",
            "networkId": "1640924173420-BNg2lwo3",
            "startAddr": "80001000",
            "endAddr": "800013ff"
        }
    }
    ```

#### Response

- **204 No Content**
- **400 Bad Request**: the special error codes are:
    - `err_broker_application_not_exist`: The application does not exist.
    - `err_broker_network_not_exist`: The network does not exist.
    - `err_broker_device_not_exist`: The device does not exist.
- **401, 403, 500, 503**: See [Notes](#notes).

## <a name="post_device_route_range_del"></a>Bulk deleting device routes with address range

Delete device routes in bulk with address range.

    POST /broker/api/v1/device-route/range-delete

Notes:
- Maximum 1024 devices (0x400).
- `startAddrs` and `endAddrs` must be hexadecimal string with the same (even) length.
- Strings up to 32 bytes (128 bits) are supported.

#### Additional HTTP Headers

    Content-Type: application/json

#### Parameters

- *object* `data`:
    - *string* `applicationId`: The target application ID.
    - *string* `networkId`: The associated network ID. The public network can be assigned by **admin** or **manager**.
    - *string* `startAddr`: The start network address of the specified network.
    - *string* `endAddr`: The end network address of the specified network.

- **Example**

    ```json
    {
        "data": {
            "applicationId": "1640924063709-rmJIxW0s",
            "networkId": "1640924173420-BNg2lwo3",
            "startAddr": "80001000",
            "endAddr": "800013ff"
        }
    }
    ```

#### Response

- **204 No Content**
- **400 Bad Request**: the special error codes are:
    - `err_broker_application_not_exist`: The application does not exist.
    - `err_broker_network_not_exist`: The network does not exist.
- **401, 403, 500, 503**: See [Notes](#notes).

## <a name="get_device_route_count"></a>Device route count

Get device route list count.

    GET /broker/api/v1/device-route/count?
        unit={specifiedUnitId}&
        application={specifiedApplicationId}&
        network={specifiedNetworkId}&
        device={specifiedDeviceId}

- *string* `unit`: (**required for normal user**) To search device routes of the specified unit ID.
- *string* `application`: (**optional**) To search device routes of the specified application ID.
- *string* `network`: (**optional**) To search device routes of the specified network ID.
- *string* `device`: (**optional**) To search device routes of the specified device ID.

#### Response

- **200 OK**: Device route list count. Parameters are:

    - *object* `data`:
        - *number* `count`: Device route list count.

    - **Example**

        ```json
        {
            "data": {
                "count": 1
            }
        }
        ```

- **400 Bad Request**: the special error codes are:
    - `err_broker_unit_not_exist`: The unit does not exist.
- **401, 403, 500, 503**: See [Notes](#notes).

## <a name="get_device_route_list"></a>Device route list

Get device route list.

    GET /broker/api/v1/device-route/list?
        unit={specifiedUnitId}&
        application={specifiedApplicationId}&
        network={specifiedNetworkId}&
        device={specifiedDeviceId}&
        offset={offset}&
        limit={limit}&
        sort={sortKeysAndOrders}&
        format={responseFormat}

- *string* `unit`: (**required for normal user**) To search device routes of the specified unit ID.
- *string* `application`: (**optional**) To search device routes of the specified application ID.
- *string* `network`: (**optional**) To search device routes of the specified network ID.
- *string* `device`: (**optional**) To search device routes of the specified device ID.
- *number* `offset`: (**optional**) Data offset. Default is **0**.
- *number* `limit`: (**optional**) Number of items to list. **0** to list all data. Default is **100**.
- *string* `sort`: (**optional**) To sort the result. Format is `key:[asc|desc]`. The key can be **application**, **network**, **addr**, **created**, **modified**. Default is **network:asc,addr:asc,created:desc**.
- *string* `format`: (**optional**) Response body format. Default is array in the **data** field.
    - **array**: The response body is JSON array, not an object.

#### Response

- **200 OK**: An array that contains all device routes' information. Parameters are:

    - *object[]* `data`:
        - *string* `routeId`: Route ID.
        - *string* `unitId`: The associated unit ID.
        - *string* `applicationId`: The target application ID.
        - *string* `applicationCode`: The target application code.
        - *string* `deviceId`: The device ID.
        - *string* `networkId`: The network ID of the device.
        - *string* `networkCode`: The network code of the device.
        - *string* `networkAddr`: The network address of the device.
        - *string* `profile`: The device profile.
        - *string* `createdAt`: Creation time in RFC 3339 format.
        - *string* `modifiedAt`: Modification time in RFC 3339 format.

    - **Example**

        ```json
        {
            "data": [
                {
                    "routeId": "1640924308457-D455ZtkDW033",
                    "unitId": "1640923958516-qvdFNpOV",
                    "applicationId": "1640924063709-rmJIxW0s",
                    "applicationCode": "tracker",
                    "deviceId": "1640924274329-yESwHhKO",
                    "networkId": "1640924173420-BNg2lwo3",
                    "networkCode": "lora",
                    "networkAddr": "800012ae",
                    "profile": "tracker",
                    "createdAt": "2021-12-31T04:18:28.457Z",
                    "modifiedAt": "2021-12-31T04:18:28.457Z"
                }
            ]
        }
        ```

    - **Example (format=`array`)**

        ```json
        [
            {
                "routeId": "1640924308457-D455ZtkDW033",
                "unitId": "1640923958516-qvdFNpOV",
                "applicationId": "1640924063709-rmJIxW0s",
                "applicationCode": "tracker",
                "deviceId": "1640924274329-yESwHhKO",
                "networkId": "1640924173420-BNg2lwo3",
                "networkCode": "lora",
                "networkAddr": "800012ae",
                "profile": "tracker",
                "createdAt": "2021-12-31T04:18:28.457Z",
                "modifiedAt": "2021-12-31T04:18:28.457Z"
            }
        ]
        ```

- **400 Bad Request**: the special error codes are:
    - `err_broker_unit_not_exist`: The unit does not exist.
- **401, 403, 500, 503**: See [Notes](#notes).

## <a name="delete_device_route"></a>Delete device route

Delete a device route.

    DELETE /broker/api/v1/device-route/{routeId}

- *string* `routeId`: The specified route ID to delete.

#### Response

- **204 No Content**
- **401, 403, 500, 503**: See [Notes](#notes).

# <a name="network_route"></a>Network route APIs

## <a name="post_network_route"></a>Create network route

Create an network route.

    POST /broker/api/v1/network-route

#### Additional HTTP Headers

    Content-Type: application/json

#### Parameters

- *object* `data`:
    - *string* `networkId`: The network ID.
    - *string* `applicationId`: The target application ID.

- **Example**

    ```json
    {
        "data": {
            "networkId": "1640924173420-BNg2lwo3",
            "applicationId": "1640924063709-rmJIxW0s"
        }
    }
    ```

#### Response

- **200 OK**: The network ID. Parameters are:

    - *object* `data`:
        - *string* `routeId`: The route ID of the created network route.

    - **Example**

        ```json
        {
            "data": {
                "routeId": "1640924311420-po5HWJAyIZPY"
            }
        }
        ```

- **400 Bad Request**: the special error codes are:
    - `err_broker_application_not_exist`: The application does not exist.
    - `err_broker_network_not_exist`: The network does not exist.
    - `err_broker_route_exist`: The network route has been created.
- **401, 403, 500, 503**: See [Notes](#notes).

## <a name="get_network_route_count"></a>Network route count

Get network route list count.

    GET /broker/api/v1/network-route/count?
        unit={specifiedUnitId}&
        application={specifiedApplicationId}&
        network={specifiedNetworkId}

- *string* `unit`: (**required for normal user**) To search network routes of the specified unit ID.
- *string* `application`: (**optional**) To search network routes of the specified application ID.
- *string* `network`: (**optional**) To search network routes of the specified network ID.

#### Response

- **200 OK**: Network route list count. Parameters are:

    - *object* `data`:
        - *number* `count`: Network route list count.

    - **Example**

        ```json
        {
            "data": {
                "count": 1
            }
        }
        ```

- **400 Bad Request**: the special error codes are:
    - `err_broker_unit_not_exist`: The unit does not exist.
- **401, 403, 500, 503**: See [Notes](#notes).

## <a name="get_network_route_list"></a>Network route list

Get network route list.

    GET /broker/api/v1/network-route/list?
        unit={specifiedUnitId}&
        application={specifiedApplicationId}&
        network={specifiedNetworkId}&
        offset={offset}&
        limit={limit}&
        sort={sortKeysAndOrders}&
        format={responseFormat}

- *string* `unit`: (**required for normal user**) To search network routes of the specified unit ID.
- *string* `application`: (**optional**) To search network routes of the specified application ID.
- *string* `network`: (**optional**) To search network routes of the specified network ID.
- *number* `offset`: (**optional**) Data offset. Default is **0**.
- *number* `limit`: (**optional**) Number of items to list. **0** to list all data. Default is **100**.
- *string* `sort`: (**optional**) To sort the result. Format is `key:[asc|desc]`. The key can be **application**, **network**, **created**. Default is **network:asc,created:false**.
- *string* `format`: (**optional**) Response body format. Default is array in the **data** field.
    - **array**: The response body is JSON array, not an object.

#### Response

- **200 OK**: An array that contains all network routes' information. Parameters are:

    - *object[]* `data`:
        - *string* `routeId`: Route ID.
        - *string* `unitId`: The associated unit ID.
        - *string* `applicationId`: The target application ID.
        - *string* `applicationCode`: The target application code.
        - *string* `networkId`: The network ID.
        - *string* `networkCode`: The code of the network.
        - *string* `createdAt`: Creation time in RFC 3339 format.

    - **Example**

        ```json
        {
            "data": [
                {
                    "routeId": "1640924311420-po5HWJAyIZPY",
                    "unitId": "1640923958516-qvdFNpOV",
                    "applicationId": "1640924063709-rmJIxW0s",
                    "applicationCode": "tracker",
                    "networkId": "1640924173420-BNg2lwo3",
                    "networkCode": "lora",
                    "createdAt": "2021-12-31T04:18:31.420Z"
                }
            ]
        }
        ```

    - **Example (format=`array`)**

        ```json
        [
            {
                "routeId": "1640924311420-po5HWJAyIZPY",
                "unitId": "1640923958516-qvdFNpOV",
                "applicationId": "1640924063709-rmJIxW0s",
                "applicationCode": "tracker",
                "networkId": "1640924173420-BNg2lwo3",
                "networkCode": "lora",
                "createdAt": "2021-12-31T04:18:31.420Z"
            }
        ]
        ```

- **400 Bad Request**: the special error codes are:
    - `err_broker_unit_not_exist`: The unit does not exist.
- **401, 403, 500, 503**: See [Notes](#notes).

## <a name="delete_network_route"></a>Delete network route

Delete a network route.

    DELETE /broker/api/v1/network-route/{routeId}

- *string* `routeId`: The specified route ID to delete.

#### Response

- **204 No Content**
- **401, 403, 500, 503**: See [Notes](#notes).

# <a name="dldata_buffer"></a>Downlink data buffer APIs

## <a name="get_dldata_buffer_count"></a>Downlink data buffer count

Get downlink data buffer list count.

    GET /broker/api/v1/dldata-buffer/count?
        unit={specifiedUnitId}&
        application={specifiedApplicationId}&
        network={specifiedNetworkId}&
        device={specifiedDeviceId}

- *string* `unit`: (**required for normal user**) To search downlink data buffers of the specified unit ID.
- *string* `application`: (**optional**) To search downlink data buffers of the specified source application ID.
- *string* `network`: (**optional**) To search downlink data buffers of the specified destination network ID.
- *string* `device`: (**optional**) To search downlink data buffers of the specified destination device ID.

#### Response

- **200 OK**: Downlink data buffer list count. Parameters are:

    - *object* `data`:
        - *number* `count`: Downlink data buffer list count.

    - **Example**

        ```json
        {
            "data": {
                "count": 1
            }
        }
        ```

- **400 Bad Request**: the special error codes are:
    - `err_broker_unit_not_exist`: The unit does not exist.
- **401, 403, 500, 503**: See [Notes](#notes).

## <a name="get_dldata_buffer_list"></a>Downlink data buffer list

Get downlink data buffer list.

    GET /broker/api/v1/dldata-buffer/list?
        unit={specifiedUnitId}&
        application={specifiedApplicationId}&
        network={specifiedNetworkId}&
        device={specifiedDeviceId}&
        offset={offset}&
        limit={limit}&
        sort={sortKeysAndOrders}&
        format={responseFormat}

- *string* `unit`: (**required for normal user**) To search downlink data buffers of the specified unit ID.
- *string* `application`: (**optional**) To search downlink data buffers of the specified source application ID.
- *string* `network`: (**optional**) To search downlink data buffers of the specified destination network ID.
- *string* `device`: (**optional**) To search downlink data buffers of the specified destination device ID.
- *number* `offset`: (**optional**) Data offset. Default is **0**.
- *number* `limit`: (**optional**) Number of items to list. **0** to list all data. Default is **100**.
- *string* `sort`: (**optional**) To sort the result. Format is `key:[asc|desc]`. The key can be **application**, **created**, **expired**. Default is **application:asc,created:false**.
- *string* `format`: (**optional**) Response body format. Default is array in the **data** field.
    - **array**: The response body is JSON array, not an object.

#### Response

- **200 OK**: An array that contains all downlink data buffers' information. Parameters are:

    - *object[]* `data`:
        - *string* `dataId`: Data ID.
        - *string* `unitId`: The associated unit ID.
        - *string* `applicationId`: The source application ID.
        - *string* `applicationCode`: The source application code.
        - *string* `deviceId`: The destination device ID.
        - *string* `networkId`: The network ID of the device.
        - *string* `networkAddr`: The network address of the device.
        - *string* `createdAt`: Creation time in RFC 3339 format.
        - *string* `expiredAt`: Expiration time in RFC 3339 format.

    - **Example**

        ```json
        {
            "data": [
                {
                    "dataId": "1640924391768-DyyneusWeveV",
                    "unitId": "1640923958516-qvdFNpOV",
                    "applicationId": "1640924063709-rmJIxW0s",
                    "applicationCode": "tracker",
                    "deviceId": "1640924274329-yESwHhKO",
                    "networkId": "1640924173420-BNg2lwo3",
                    "networkAddr": "800012ae",
                    "createdAt": "2021-12-31T04:19:51.768Z",
                    "expiredAt": "2021-12-31T04:29:51.768Z"
                }
            ]
        }
        ```

    - **Example (format=`array`)**

        ```json
        [
            {
                "dataId": "1640924391768-DyyneusWeveV",
                "unitId": "1640923958516-qvdFNpOV",
                "applicationId": "1640924063709-rmJIxW0s",
                "applicationCode": "tracker",
                "deviceId": "1640924274329-yESwHhKO",
                "networkId": "1640924173420-BNg2lwo3",
                "networkAddr": "800012ae",
                "createdAt": "2021-12-31T04:19:51.768Z",
                "expiredAt": "2021-12-31T04:29:51.768Z"
            }
        ]
        ```

- **400 Bad Request**: the special error codes are:
    - `err_broker_unit_not_exist`: The unit does not exist.
- **401, 403, 500, 503**: See [Notes](#notes).

## <a name="delete_dldata_buffer"></a>Delete downlink data buffer

Delete a downlink data buffer.

    DELETE /broker/api/v1/dldata-buffer/{dataId}

- *string* `dataId`: The specified data ID to delete.

#### Response

- **204 No Content**
- **401, 403, 500, 503**: See [Notes](#notes).
