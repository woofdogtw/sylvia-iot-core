API - Data
==========

## Contents

- [Notes](#notes)
- [Common error codes](#errcode)
- [Roles](#roles)
- [Service APIs](#service)
    - [`GET /version` Get service version](#get_version)
- [Application data APIs](#application)
    - [`GET /data/api/v1/application-uldata/count` Application uplink data count](#get_application_uldata_count)
    - [`GET /data/api/v1/application-uldata/list` Application uplink data list](#get_application_uldata_list)
    - [`GET /data/api/v1/application-dldata/count` Application downlink data count](#get_application_dldata_count)
    - [`GET /data/api/v1/application-dldata/list` Application downlink data list](#get_application_dldata_list)
- [Network data APIs](#network)
    - [`GET /data/api/v1/network-uldata/count` Network uplink data count](#get_network_uldata_count)
    - [`GET /data/api/v1/network-uldata/list` Network uplink data list](#get_network_uldata_list)
    - [`GET /data/api/v1/network-dldata/count` Network downlink data count](#get_network_dldata_count)
    - [`GET /data/api/v1/network-dldata/list` Network downlink data list](#get_network_dldata_list)
- [Coremgr data APIs](#coremgr)
    - [`GET /data/api/v1/coremgr-opdata/count` Coremgr operation data count](#get_coremgr_opdata_count)
    - [`GET /data/api/v1/coremgr-opdata/list` Coremgr operation data list](#get_coremgr_opdata_list)

## <a name="notes"></a>Notes

All API requests (except `GET /version`) must have a **Authorization** header with a **Bearer** token.

- **Example**

    ```http
    GET /data/api/v1/network/uldata HTTP/1.1
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
- `manager`: The system manager who acts as admin to view all data.
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
                "name": "sylvia-iot-auth",
                "version": "1.0.0"
            }
        }
        ```

    - **Example** when `q=name`:

        ```
        sylvia-iot-auth
        ```

    - **Example** when `q=version`:

        ```
        1.0.0
        ```

# <a name="application"></a>Application data APIs

## <a name="get_application_uldata_count"></a>Application uplink data count

Get application uplink data list count.

    GET /data/api/v1/application-uldata/count?
        unit={specifiedUnitId}&
        device={deviceId}&
        network={networkCode}&
        addr={networkAddr}&
        profile={deviceProfile}&
        tfield={timeFieldFilter}&
        tstart={startTimeMs}&
        tend={endTimeMs}

- *string* `unit`: (**required for normal user**) To search data of the specified unit ID.
- *string* `device`: (**optional**) To search data of the specified device ID.
- *string* `network`: (**optional**) To search data of the specified device network code.
- *string* `addr`: (**optional**) To search data of the specified device network address.
- *string* `profile`: (**optional**) To search data of the specified device/data profile.
- *string* `tfield`: (**required for tstart and tend**) Time field to filter data. **proc**, **pub**, **time** are available.
- *number* `tstart`: (**optional**) The start time in milliseconds to filter data.
- *number* `tend`: (**optional**) The end time in milliseconds to filter data.

#### Response

- **200 OK**: Application uplink data list count. Parameters are:

    - *object* `data`:
        - *number* `count`: Application uplink data list count.

    - **Example**

        ```json
        {
            "data": {
                "count": 2
            }
        }
        ```

- **400 Bad Request**: the special error codes are:
    - `err_data_unit_not_exist`: The unit does not exist.
- **401, 403, 500, 503**: See [Notes](#notes).

## <a name="get_application_uldata_list"></a>Application uplink data list

Get application uplink data list.

    GET /data/api/v1/application-uldata/list?
        unit={specifiedUnitId}&
        device={deviceId}&
        network={networkCode}&
        addr={networkAddr}&
        profile={deviceProfile}&
        tfield={timeFieldFilter}&
        tstart={startTimeMs}&
        tend={endTimeMs}&
        offset={offset}&
        limit={limit}&
        sort={sortKeysAndOrders}&
        format={responseFormat}

- *string* `unit`: (**required for normal user**) To search data of the specified unit ID.
- *string* `device`: (**optional**) To search data of the specified device ID.
- *string* `network`: (**optional**) To search data of the specified device network code.
- *string* `addr`: (**optional**) To search data of the specified device network address.
- *string* `profile`: (**optional**) To search data of the specified device/data profile.
- *string* `tfield`: (**required for tstart and tend**) Time field to filter data. **proc**, **pub**, **time** are available.
- *number* `tstart`: (**optional**) The start time in milliseconds to filter data.
- *number* `tend`: (**optional**) The end time in milliseconds to filter data.
- *number* `offset`: (**optional**) Data offset. Default is **0**.
- *number* `limit`: (**optional**) Number of items to list. **0** to list all data. Default is **100**.
- *string* `sort`: (**optional**) To sort the result. Format is `key:[asc|desc]`. The key can be **proc**, **pub**, **time**, **network**, **addr**. Default is **proc:desc**.
- *string* `format`: (**optional**) Response body format. Default is array in the **data** field.
    - **array**: The response body is JSON array, not an object.
    - **csv**: The response body is CSV.

#### Response

- **200 OK**:  An array that contains all application uplink data information. Parameters are:

    - *object[]* `data`:
        - *string* `dataId`: Data ID.
        - *string* `proc`: Received time in ISO 8601 format when the broker receive this data.
        - *string* `pub`: Publish time in ISO 8601 format to queue.
        - *string | null* `unitCode`: Network's unit code.
        - *string* `networkCode`: Network code.
        - *string* `networkAddr`: Device network address.
        - *string* `unitId`: Device's unit ID.
        - *string* `deviceId`: Device ID.
        - *string* `time`: Device time in ISO 8601 format.
        - *string* `profile`: Device/data profile.
        - *string* `data`: Data in hexadecimal format.
        - *object* `extension`: (**optional**) Extensions from the network to application(s).

    - **Example**

        ```json
        {
            "data": [
                {
                    "dataId": "1665486661848-DTAjZX6JsTSH",
                    "proc": "2022-10-11T11:11:01.848Z",
                    "pub": "2022-10-11T11:11:02.043Z",
                    "unitCode": "sylvia",
                    "networkCode": "lora",
                    "networkAddr": "800012ae",
                    "unitId": "1640923958516-qvdFNpOV",
                    "deviceId": "1640924274329-yESwHhKO",
                    "time": "2022-10-11T11:10:48.768Z",
                    "profile": "tracker",
                    "data": "74657374",
                    "extension": {
                        "latitude": "24.38349800818775",
                        "longitude": "121.22999674970842"
                    }
                }
            ]
        }
        ```

- **400 Bad Request**: the special error codes are:
    - `err_data_unit_not_exist`: The unit does not exist.
- **401, 403, 500, 503**: See [Notes](#notes).

## <a name="get_application_dldata_count"></a>Application downlink data count

Get application downlink data list count.

    GET /data/api/v1/application-dldata/count?
        unit={specifiedUnitId}&
        device={deviceId}&
        network={networkCode}&
        addr={networkAddr}&
        profile={deviceProfile}&
        tfield={timeFieldFilter}&
        tstart={startTimeMs}&
        tend={endTimeMs}

- *string* `unit`: (**required for normal user**) To search data of the specified unit ID.
- *string* `device`: (**optional**) To search data of the specified device ID.
- *string* `network`: (**optional**) To search data of the specified device network code.
- *string* `addr`: (**optional**) To search data of the specified device network address.
- *string* `profile`: (**optional**) To search data of the specified device/data profile.
- *string* `tfield`: (**required for tstart and tend**) Time field to filter data. **proc**, **resp** are available.
- *number* `tstart`: (**optional**) The start time in milliseconds to filter data.
- *number* `tend`: (**optional**) The end time in milliseconds to filter data.

#### Response

- **200 OK**: Application downlink data list count. Parameters are:

    - *object* `data`:
        - *number* `count`: Application downlink data list count.

    - **Example**

        ```json
        {
            "data": {
                "count": 2
            }
        }
        ```

- **400 Bad Request**: the special error codes are:
    - `err_data_unit_not_exist`: The unit does not exist.
- **401, 403, 500, 503**: See [Notes](#notes).

## <a name="get_application_dldata_list"></a>Application downlink data list

Get application downlink data list.

    GET /data/api/v1/application-dldata/list?
        unit={specifiedUnitId}&
        device={deviceId}&
        network={networkCode}&
        addr={networkAddr}&
        profile={deviceProfile}&
        tfield={timeFieldFilter}&
        tstart={startTimeMs}&
        tend={endTimeMs}&
        offset={offset}&
        limit={limit}&
        sort={sortKeysAndOrders}&
        format={responseFormat}

- *string* `unit`: (**required for normal user**) To search data of the specified unit ID.
- *string* `device`: (**optional**) To search data of the specified device ID.
- *string* `network`: (**optional**) To search data of the specified device network code.
- *string* `addr`: (**optional**) To search data of the specified device network address.
- *string* `profile`: (**optional**) To search data of the specified device/data profile.
- *string* `tfield`: (**required for tstart and tend**) Time field to filter data. **proc**, **resp** are available.
- *number* `tstart`: (**optional**) The start time in milliseconds to filter data.
- *number* `tend`: (**optional**) The end time in milliseconds to filter data.
- *number* `offset`: (**optional**) Data offset. Default is **0**.
- *number* `limit`: (**optional**) Number of items to list. **0** to list all data. Default is **100**.
- *string* `sort`: (**optional**) To sort the result. Format is `key:[asc|desc]`. The key can be **proc**, **resp**, **network**, **addr**. Default is **proc:desc**.
- *string* `format`: (**optional**) Response body format. Default is array in the **data** field.
    - **array**: The response body is JSON array, not an object.
    - **csv**: The response body is CSV.

#### Response

- **200 OK**:  An array that contains all application downlink data information. Parameters are:

    - *object[]* `data`:
        - *string* `dataId`: Data ID.
        - *string* `proc`: Received time in ISO 8601 format when the broker receive this data.
        - *string* `resp`: (**optional**) The last response time in ISO 8601 format.
        - *number* `status`: 0 for success, negative for processing, positive for error.
        - *string* `unitId`: Device's unit ID.
        - *string* `deviceId`: (**optional**) Device ID from the source data of the application.
        - *string* `networkCode`: (**optional**) Device network code from the source data of the application.
        - *string* `networkAddr`: (**optional**) Device network address from the source data of the application.
        - *string* `profile`: Device/data profile.
        - *string* `data`: Data in hexadecimal format.
        - *object* `extension`: (**optional**) Extensions from the application to the network.

    - **Example**

        ```json
        {
            "data": [
                {
                    "dataId": "1665486663768-jK6EnD3pcGDC",
                    "proc": "2022-10-11T11:11:03.768Z",
                    "resp": "2022-10-11T11:11:24.154Z",
                    "status": 1,
                    "unitId": "1640923958516-qvdFNpOV",
                    "deviceId": "1640924274329-yESwHhKO",
                    "networkCode": "lora",
                    "networkAddr": "800012ae",
                    "profile": "tracker",
                    "data": "74657374",
                    "extension": {
                        "schedule": "2022-10-12T00:00:00Z"
                    }
                }
            ]
        }
        ```

- **400 Bad Request**: the special error codes are:
    - `err_data_unit_not_exist`: The unit does not exist.
- **401, 403, 500, 503**: See [Notes](#notes).

# <a name="network"></a>Network data APIs

## <a name="get_network_uldata_count"></a>Network uplink data count

Get network uplink data list count.

    GET /data/api/v1/network-uldata/count?
        unit={specifiedUnitId}&
        device={deviceId}&
        network={networkCode}&
        addr={networkAddr}&
        profile={deviceProfile}&
        tfield={timeFieldFilter}&
        tstart={startTimeMs}&
        tend={endTimeMs}

- *string* `unit`: (**required for normal user**) To search data of the specified unit ID.
- *string* `device`: (**optional**) To search data of the specified device ID.
- *string* `network`: (**optional**) To search data of the specified device network code.
- *string* `addr`: (**optional**) To search data of the specified device network address.
- *string* `profile`: (**optional**) To search data of the specified device/data profile.
- *string* `tfield`: (**required for tstart and tend**) Time field to filter data. **proc**, **time** are available.
- *number* `tstart`: (**optional**) The start time in milliseconds to filter data.
- *number* `tend`: (**optional**) The end time in milliseconds to filter data.

#### Response

- **200 OK**: Network uplink data list count. Parameters are:

    - *object* `data`:
        - *number* `count`: Network uplink data list count.

    - **Example**

        ```json
        {
            "data": {
                "count": 2
            }
        }
        ```

- **400 Bad Request**: the special error codes are:
    - `err_data_unit_not_exist`: The unit does not exist.
- **401, 403, 500, 503**: See [Notes](#notes).

## <a name="get_network_uldata_list"></a>Network uplink data list

Get network uplink data list.

    GET /data/api/v1/network-uldata/list?
        unit={specifiedUnitId}&
        device={deviceId}&
        network={networkCode}&
        addr={networkAddr}&
        profile={deviceProfile}&
        tfield={timeFieldFilter}&
        tstart={startTimeMs}&
        tend={endTimeMs}&
        offset={offset}&
        limit={limit}&
        sort={sortKeysAndOrders}&
        format={responseFormat}

- *string* `unit`: (**required for normal user**) To search data of the specified unit ID.
- *string* `device`: (**optional**) To search data of the specified device ID.
- *string* `network`: (**optional**) To search data of the specified device network code.
- *string* `addr`: (**optional**) To search data of the specified device network address.
- *string* `profile`: (**optional**) To search data of the specified device/data profile.
- *string* `tfield`: (**required for tstart and tend**) Time field to filter data. **proc**, **time** are available.
- *number* `tstart`: (**optional**) The start time in milliseconds to filter data.
- *number* `tend`: (**optional**) The end time in milliseconds to filter data.
- *number* `offset`: (**optional**) Data offset. Default is **0**.
- *number* `limit`: (**optional**) Number of items to list. **0** to list all data. Default is **100**.
- *string* `sort`: (**optional**) To sort the result. Format is `key:[asc|desc]`. The key can be **proc**, **time**, **network**, **addr**. Default is **proc:desc**.
- *string* `format`: (**optional**) Response body format. Default is array in the **data** field.
    - **array**: The response body is JSON array, not an object.
    - **csv**: The response body is CSV.

#### Response

- **200 OK**:  An array that contains all network uplink data information. Parameters are:

    - *object[]* `data`:
        - *string* `dataId`: Data ID.
        - *string* `proc`: Received time in ISO 8601 format when the broker receive this data.
        - *string | null* `unitCode`: Network's unit code.
        - *string* `networkCode`: Network code.
        - *string* `networkAddr`: Device network address.
        - *string* `unitId`: (**optional**) Network's unit ID.
        - *string* `deviceId`: (**optional**) Device ID.
        - *string* `time`: Device time in ISO 8601 format.
        - *string* `profile`: Device/data profile.
        - *string* `data`: Data in hexadecimal format.
        - *object* `extension`: (**optional**) Extensions from the network to network(s).

    - **Example**

        ```json
        {
            "data": [
                {
                    "dataId": "1665486661848-93r1JITdLaSw",
                    "proc": "2022-10-11T11:11:01.848Z",
                    "unitCode": "sylvia",
                    "networkCode": "lora",
                    "networkAddr": "800012ae",
                    "unitId": "1640923958516-qvdFNpOV",
                    "deviceId": "1640924274329-yESwHhKO",
                    "time": "2022-10-11T11:10:48.768Z",
                    "profile": "tracker",
                    "data": "74657374",
                    "extension": {
                        "latitude": "24.38349800818775",
                        "longitude": "121.22999674970842"
                    }
                }
            ]
        }
        ```

- **400 Bad Request**: the special error codes are:
    - `err_data_unit_not_exist`: The unit does not exist.
- **401, 403, 500, 503**: See [Notes](#notes).

## <a name="get_network_dldata_count"></a>Network downlink data count

Get network downlink data list count.

    GET /data/api/v1/network-dldata/count?
        unit={specifiedUnitId}&
        device={deviceId}&
        network={networkCode}&
        addr={networkAddr}&
        profile={deviceProfile}&
        tfield={timeFieldFilter}&
        tstart={startTimeMs}&
        tend={endTimeMs}

- *string* `unit`: (**required for normal user**) To search data of the specified unit ID.
- *string* `device`: (**optional**) To search data of the specified device ID.
- *string* `network`: (**optional**) To search data of the specified device network code.
- *string* `addr`: (**optional**) To search data of the specified device network address.
- *string* `profile`: (**optional**) To search data of the specified device/data profile.
- *string* `tfield`: (**required for tstart and tend**) Time field to filter data. **proc**, **pub**, **resp** are available.
- *number* `tstart`: (**optional**) The start time in milliseconds to filter data.
- *number* `tend`: (**optional**) The end time in milliseconds to filter data.

#### Response

- **200 OK**: Network downlink data list count. Parameters are:

    - *object* `data`:
        - *number* `count`: Network downlink data list count.

    - **Example**

        ```json
        {
            "data": {
                "count": 2
            }
        }
        ```

- **400 Bad Request**: the special error codes are:
    - `err_data_unit_not_exist`: The unit does not exist.
- **401, 403, 500, 503**: See [Notes](#notes).

## <a name="get_network_dldata_list"></a>Network downlink data list

Get network downlink data list.

    GET /data/api/v1/network-dldata/list?
        unit={specifiedUnitId}&
        device={deviceId}&
        network={networkCode}&
        addr={networkAddr}&
        profile={deviceProfile}&
        tfield={timeFieldFilter}&
        tstart={startTimeMs}&
        tend={endTimeMs}&
        offset={offset}&
        limit={limit}&
        sort={sortKeysAndOrders}&
        format={responseFormat}

- *string* `unit`: (**required for normal user**) To search data of the specified unit ID.
- *string* `device`: (**optional**) To search data of the specified device ID.
- *string* `network`: (**optional**) To search data of the specified device network code.
- *string* `addr`: (**optional**) To search data of the specified device network address.
- *string* `profile`: (**optional**) To search data of the specified device/data profile.
- *string* `tfield`: (**required for tstart and tend**) Time field to filter data. **proc**, **pub**, **resp** are available.
- *number* `tstart`: (**optional**) The start time in milliseconds to filter data.
- *number* `tend`: (**optional**) The end time in milliseconds to filter data.
- *number* `offset`: (**optional**) Data offset. Default is **0**.
- *number* `limit`: (**optional**) Number of items to list. **0** to list all data. Default is **100**.
- *string* `sort`: (**optional**) To sort the result. Format is `key:[asc|desc]`. The key can be **proc**, **pub**, **resp**, **network**, **addr**. Default is **proc:desc**.
- *string* `format`: (**optional**) Response body format. Default is array in the **data** field.
    - **array**: The response body is JSON array, not an object.
    - **csv**: The response body is CSV.

#### Response

- **200 OK**:  An array that contains all network downlink data information. Parameters are:

    - *object[]* `data`:
        - *string* `dataId`: Data ID.
        - *string* `proc`: Received time in ISO 8601 format when the broker receive this data.
        - *string* `pub`: Publish time in ISO 8601 format to queue.
        - *string* `resp`: (**optional**) The last response time in ISO 8601 format.
        - *number* `status`: 0 for success, negative for processing, positive for error.
        - *string* `unitId`: Device's unit ID.
        - *string* `deviceId`: Device ID.
        - *string* `networkCode`: Device network code.
        - *string* `networkAddr`: Device network address.
        - *string* `profile`: Device/data profile.
        - *string* `data`: Data in hexadecimal format.
        - *object* `extension`: (**optional**) Extensions from the network to the network.

    - **Example**

        ```json
        {
            "data": [
                {
                    "dataId": "1665486663768-JFlZbm7Od0Je",
                    "proc": "2022-10-11T11:11:03.768Z",
                    "pub": "2022-10-11T11:11:03.898Z",
                    "resp": "2022-10-11T11:11:24.154Z",
                    "status": 1,
                    "unitId": "1640923958516-qvdFNpOV",
                    "deviceId": "1640924274329-yESwHhKO",
                    "networkCode": "lora",
                    "networkAddr": "800012ae",
                    "profile": "tracker",
                    "data": "74657374",
                    "extension": {
                        "schedule": "2022-10-12T00:00:00Z"
                    }
                }
            ]
        }
        ```

- **400 Bad Request**: the special error codes are:
    - `err_data_unit_not_exist`: The unit does not exist.
- **401, 403, 500, 503**: See [Notes](#notes).

# <a name="coremgr"></a>Coremgr operation data APIs

## <a name="get_coremgr_opdata_count"></a>Coremgr operation data count

Get coremgr operation data list count.

    GET /data/api/v1/coremgr-opdata/count?
        user={specifiedUserId}&
        tfield={timeFieldFilter}&
        tstart={startTimeMs}&
        tend={endTimeMs}

- *string* `user`: (**optional for admin and manager**) To search data of the specified user ID.
- *string* `tfield`: (**required for tstart and tend**) Time field to filter data. **proc**, **pub**, **resp** are available.
- *number* `tstart`: (**optional**) The start time in milliseconds to filter data.
- *number* `tend`: (**optional**) The end time in milliseconds to filter data.

#### Response

- **200 OK**: Coremgr operation data list count. Parameters are:

    - *object* `data`:
        - *number* `count`: Coremgr operation data list count.

    - **Example**

        ```json
        {
            "data": {
                "count": 2
            }
        }
        ```

- **400 Bad Request**: the special error codes are:
    - `err_data_user_not_exist`: The user does not exist.
- **401, 403, 500, 503**: See [Notes](#notes).

## <a name="get_coremgr_opdata_list"></a>Coremgr operation data list

Get coremgr operation data list.

    GET /data/api/v1/coremgr-opdata/list?
        user={specifiedUserId}&
        tfield={timeFieldFilter}&
        tstart={startTimeMs}&
        tend={endTimeMs}&
        offset={offset}&
        limit={limit}&
        sort={sortKeysAndOrders}&
        format={responseFormat}

- *string* `user`: (**optional for admin and manager**) To search data of the specified user ID.
- *string* `tfield`: (**required for tstart and tend**) Time field to filter data. **req**, **res** are available.
- *number* `tstart`: (**optional**) The start time in milliseconds to filter data.
- *number* `tend`: (**optional**) The end time in milliseconds to filter data.
- *number* `offset`: (**optional**) Data offset. Default is **0**.
- *number* `limit`: (**optional**) Number of items to list. **0** to list all data. Default is **100**.
- *string* `sort`: (**optional**) To sort the result. Format is `key:[asc|desc]`. The key can be **req**, **res**, **latency**. Default is **req:desc**.
- *string* `format`: (**optional**) Response body format. Default is array in the **data** field.
    - **array**: The response body is JSON array, not an object.
    - **csv**: The response body is CSV.

#### Response

- **200 OK**:  An array that contains all Coremgr operation data information. Parameters are:

    - *object[]* `data`:
        - *string* `dataId`: Data ID.
        - *string* `reqTime`: Request time in ISO 8601 format.
        - *string* `resTime`: Response time in ISO 8601 format.
        - *number* `latencyMs`: Latency in milliseconds.
        - *number* `status`: Response status code.
        - *string* `sourceIp`: Client source IP address
        - *string* `method`: Request HTTP method. Now we record **DELETE**, **PATCH**, **POST**, **PUT** operations.
        - *string* `path`: Request HTTP path.
        - *object* `body`: (**optional**) Request body. `data.password` contents are removed before recoding.
        - *string* `userId`: Request user ID.
        - *string* `clientId`: Request client ID.
        - *string* `errCode`: (**optional for error status**) Error code.
        - *string* `errMessage`: (**optional for error status**) Error message.

    - **Example**

        ```json
        {
            "data": [
                {
                    "dataId": "1665493184163-uplOitvHEc2u",
                    "reqTime": "2022-10-11T12:59:44.163Z",
                    "resTime": "2022-10-11T12:59:44.402Z",
                    "latencyMs": 239,
                    "status": 400,
                    "sourceIp": "192.168.1.1",
                    "method": "POST",
                    "path": "/coremgr/api/v1/user",
                    "body": {"account": ""},
                    "userId": "1641003827053-c2e84RJO",
                    "clientId": "1641040728318-zyAnDK9I",
                    "errCode": "err_param",
                    "errMessage": "empty `account`"
                }
            ]
        }
        ```

- **400 Bad Request**: the special error codes are:
    - `err_data_user_not_exist`: The user does not exist.
- **401, 403, 500, 503**: See [Notes](#notes).
