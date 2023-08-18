API - Router
============

## Contents

- [Notes](#notes)
- [Common error codes](#errcode)
- [Service APIs](#service)
    - [`GET /version` Get service version](#get_version)
- [System APIs](#sys)
    - [`GET /router/api/v1/sys/usage` Get resource usage](#get_sys_usage)
    - [`GET /router/api/v1/sys/time` Get system time](#get_sys_time)
- [Network APIs](#net)
    - [`GET /router/api/v1/net/wan` Get WAN configurations](#get_net_wan)
    - [`PUT /router/api/v1/net/wan/{wanId}` Set WAN configurations](#put_net_wan)
    - [`GET /router/api/v1/net/lan` Get LAN configurations](#get_net_lan)
    - [`PUT /router/api/v1/net/lan` Set LAN configurations](#put_net_lan)
    - [`GET /router/api/v1/net/lan/leases` Get DHCP leases](#get_net_lan_leases)
    - [`GET /router/api/v1/net/wlan` Get wireless LAN configurations](#get_net_wlan)
    - [`PUT /router/api/v1/net/wlan` Set wireless LAN configurations](#put_net_wlan)
    - [`GET /router/api/v1/net/wwan` Get wireless WAN configurations](#get_net_wwan)
    - [`PUT /router/api/v1/net/wwan` Set wireless WAN configurations](#put_net_wwan)
    - [`GET /router/api/v1/net/wwan/list` List available wireless AP](#get_net_wwan_list)

## <a name="notes"></a>Notes

All API requests (except `GET /version`) must have a **Authorization** header with a **Bearer** token.

- **Example**

    ```http
    GET /router/api/v1/network/lan HTTP/1.1
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

# <a name="sys"></a>System APIs

## <a name="get_sys_usage"></a>Get resource usage

Get system resource (CPU, memory, disk) usage.

    GET /router/api/v1/sys/usage

#### Response

- **200 OK**: System resource usage information. Parameters are:

    - *object* `data`:
        - *number[]* `cpu`: Usage of each CPU core in percentage (0~100).
        - *object* `mem`: Memory space usage.
            - *number* `total`: Total memory space in bytes.
            - *number* `used`: Used memory space in bytes.
        - *object* `disk`: Disk space usage.
            - *number* `total`: Total disk space in bytes.
            - *number* `used`: Used disk space in bytes.

    - **Example**

        ```json
        {
            "data": {
                "cpu": [ 11, 2, 2, 8, 2, 2, 90, 10 ],
                "mem": {
                    "total": 6300938240,
                    "used": 2344439808
                },
                "disk": {
                    "total": 269490393088,
                    "used": 77246394368
                }
            }
        }
        ```

- **400, 401, 500, 503**: See [Notes](#notes).

## <a name="get_sys_time"></a>Get system time

Get system time information.

    GET /router/api/v1/sys/time

#### Response

- **200 OK**: System time information. Parameters are:

    - *object* `data`:
        - *string* `time`: Current time in RFC 3339 format.

    - **Example**

        ```json
        {
            "data": {
                "time": "2022-11-22T14:45:35.637Z"
            }
        }
        ```

- **400, 401, 500, 503**: See [Notes](#notes).

# <a name="net"></a>Network APIs

## <a name="get_net_wan"></a>Get WAN configurations

Get all WAN interface configurations.

    GET /router/api/v1/net/wan

#### Response

- **200 OK**: All WAN interface configuration information. Parameters are:

    - *object[]* `data`:
        - *string* `wanId`: WAN interface ID.
        - *object* `conf`: Configurations.
            - *string* `type`: Network type. One of **disable**, **ethernet**, **pppoe**.
            - *string* `type4`: (**available when type=ethernet**) IPv4 type. One of **disable**, **static**, **dhcp**.
            - *object* `static4` (**required when v4type=static**)
                - *string* `address`: The IP address CIDR.
                - *string* `gateway`: The gateway address.
                - *string[]* `dns`: DNS server addresses.
            - *object* `pppoe`: (**required when type=pppoe**)
                - *string* `username`: The user name.
                - *string* `password`: The password.
        - *object* `conn4`: The IPv4 connection information when the interface is up. Any empty field means no configurations or it is not connected.
            - *string* `address`: The IP address CIDR.
            - *string* `gateway`: The gateway address.
            - *string[]* `dns`: DNS server addresses.

    - **Example**

        ```json
        {
            "data": [
                {
                    "wanId": "wan1",
                    "conf": {
                        "type": "ethernet",
                        "type4": "static",
                        "static4" {
                            "address": "10.1.0.11/24",
                            "gateway": "10.1.0.254",
                            "dns": [
                                "8.8.8.8",
                                "8.8.4.4"
                            ]
                        }
                    },
                    "conn4": {
                        "address": "10.1.0.11/24",
                        "gateway": "10.1.0.254",
                        "dns": [
                            "8.8.8.8",
                            "8.8.4.4"
                        ]
                    }
                },
                {
                    "wanId": "wan2",
                    "conf": {
                        "type": "pppoe"
                        "pppoe": {
                            "username": "user",
                            "password": "pass"
                        }
                    },
                    "conn4": {
                        "address": "10.1.0.201/24",
                        "gateway": "10.1.0.254",
                        "dns": [
                            "10.1.0.254"
                        ]
                    }
                }
            ]
        }
        ```

- **400, 401, 500, 503**: See [Notes](#notes).

## <a name="put_net_wan"></a>Set WAN configurations

Set one WAN interface configurations.

    PUT /router/api/v1/net/wan/{wanId}

- *string* `wanId`: The specified WAN interface ID.

#### Additional HTTP Headers

    Content-Type: application/json

#### Parameters

- *object* `data`:
    - *string* `type`: Network type. One of **disable**, **ethernet**, **pppoe**.
    - *string* `type4`: (**required when type=ethernet**) IPv4 type. One of **static**, **dhcp**.
    - *object* `static4` (**required when type4=static**)
        - *string* `address`: The IP address CIDR.
        - *string* `gateway`: The gateway address.
        - *string[]* `dns`: DNS server addresses.
    - *object* `pppoe`: (**required when type=pppoe**)
        - *string* `username`: The user name.
        - *string* `password`: The password.

- **Example**

    ```json
    {
        "data": {
            "type": "ethernet",
            "type4": "static",
            "static4" {
                "address": "10.1.0.11/24",
                "gateway": "10.1.0.254",
                "dns": [
                    "8.8.8.8",
                    "8.8.4.4"
                ]
            }
        }
    }
    ```

#### Response

- **204 No Content**
- **400, 401, 500, 503**: See [Notes](#notes).
- **404 Not Found**: The specified WAN interface ID does not exist.

## <a name="get_net_lan"></a>Get LAN configurations

Get the LAN interface configurations.

    GET /router/api/v1/net/lan

#### Response

- **200 OK**: The LAN interface configuration information. Parameters are:

    - *object* `data`:
        - *object* `conf4`: IPv4 configurations.
            - *string* `address`: The IP address CIDR of the router.
            - *string* `dhcpStart`: The start IP address of the DHCP service.
            - *string* `dhcpEnd`: The end IP address of the DHCP service.
            - *number* `leaseTime`: The lease time in seconds. From **60** (one minute) to **604800** (one week).

    - **Example**

        ```json
        {
            "data": {
                "conf4": {
                    "address": "192.168.1.254/24",
                    "dhcpStart": "192.168.1.101",
                    "dhcpEnd": "192.168.1.200",
                    "leaseTime": 86400
                }
            }
        }
        ```

- **400, 401, 500, 503**: See [Notes](#notes).

## <a name="put_net_lan"></a>Set LAN configurations

Set the LAN interface configurations.

    PUT /router/api/v1/net/lan

#### Additional HTTP Headers

    Content-Type: application/json

#### Parameters

- *object* `data`:
    - *object* `conf4`: IPv4 configurations.
        - *string* `address`: The IP address CIDR of the router.
        - *string* `dhcpStart`: The start IP address of the DHCP service.
        - *string* `dhcpEnd`: The end IP address of the DHCP service.
        - *number* `leaseTime`: The lease time in seconds. From **60** (one minute) to **604800** (one week).

- **Example**

    ```json
    {
        "data": {
            "conf4": {
                "address": "192.168.1.254/24",
                "dhcpStart": "192.168.1.101",
                "dhcpEnd": "192.168.1.200",
                "leaseTime": 86400
            }
        }
    }
    ```

#### Response

- **204 No Content**
- **400, 401, 500, 503**: See [Notes](#notes).

## <a name="get_net_lan_leases"></a>Get DHCP leases

Get DHCP leases.

    GET /router/api/v1/net/lan/leases

#### Response

- **200 OK**: The DHCP lease information. Parameters are:

    - *object[]* `data`:
        - *string* `ip`: The IP address.
        - *string* `starts`: (**optional**) The start time of the lease in RFC 3339 format.
        - *string* `ends`: (**optional**) The end time of the lease in RFC 3339 format.
        - *string* `macAddr`: (**optional**) The Ethernet MAC address of the lease.
        - *string* `client`: (**optional**) The client hostname.

    - **Example**

        ```json
        {
            "data": [
                {
                    "ip": "192.168.55.201",
                    "starts": "2018-10-02T05:17:24Z",
                    "ends": "2018-10-02T05:27:24Z",
                    "macAddr": "08:00:27:be:86:94",
                    "client": "someone-winxp"
                },
                {
                    "ip": "192.168.55.202",
                    "starts": "2018-10-02T05:19:06Z",
                    "ends": "2018-10-02T05:29:06Z",
                    "macAddr": "08:00:27:a9:56:84",
                    "client": "anotherone-linux"
                }
            ]
        }
        ```

- **400, 401, 500, 503**: See [Notes](#notes).

## <a name="get_net_wlan"></a>Get wireless LAN configurations

Get the wireless LAN interface configurations.

    GET /router/api/v1/net/wlan

#### Response

- **200 OK**: The wireless LAN configuration information. Parameters are:

    - *object* `data`:
        - *boolean* `enable`: Enable or disable wireless LAN.
        - *object* `conf`: (**present when enable=true**)
            - *string* `ssid`: SSID.
            - *number* `channel`: The WiFi channel. 1~11.
            - *string* `password`: The WPA2 password. 8~63 characters.

    - **Example**

        ```json
        {
            "data": {
                "enable": true,
                "conf": {
                    "ssid": "sylvia-iot",
                    "channel": 1,
                    "password": "p@sSw0rD"
                }
            }
        }
        ```

- **404 Not Found**: The router does not support wireless LAN.
- **400, 401, 500, 503**: See [Notes](#notes).

## <a name="put_net_wlan"></a>Set wireless LAN configurations

Set the wireless LAN interface configurations.

    PUT /router/api/v1/net/wlan

#### Additional HTTP Headers

    Content-Type: application/json

#### Parameters

- *object* `data`:
    - *boolean* `enable`: Enable or disable wireless LAN.
    - *object* `conf`: (**required when enable=true**)
        - *string* `ssid`: SSID.
        - *number* `channel`: The WiFi channel. 1~11.
        - *string* `password`: The WPA2 password. 8~63 characters.

- **Example**

    ```json
    {
        "data": {
            "enable": true,
            "conf": {
                "ssid": "sylvia-iot",
                "channel": 1,
                "password": "p@sSw0rD"
            }
        }
    }
    ```

#### Response

- **204 No Content**
- **404 Not Found**: The router does not support wireless LAN.
- **400, 401, 500, 503**: See [Notes](#notes).

## <a name="get_net_wwan"></a>Get wireless WAN configurations

Get the wireless WAN interface configurations.

    GET /router/api/v1/net/wwan

#### Response

- **200 OK**: The wireless WAN configuration information. Parameters are:

    - *object* `data`:
        - *boolean* `enable`: Enable or disable wireless WAN.
        - *object* `conf`: (**present when enable=true**)
            - *string* `ssid`: SSID.
        - *object* `conn4`: (**present when enable=true**) The IPv4 connection information when the interface is up. Any empty field means no configurations or it is not connected.
            - *string* `address`: The IP address CIDR.
            - *string* `gateway`: The gateway address.
            - *string[]* `dns`: DNS server addresses.

    - **Example**

        ```json
        {
            "data": {
                "enable": true,
                "conf": {
                    "ssid": "sylvia-iot"
                },
                "conn4": {
                    "address": "10.1.0.11/24",
                    "gateway": "10.1.0.254",
                    "dns": [
                        "8.8.8.8",
                        "8.8.4.4"
                    ]
                }
            }
        }
        ```

- **404 Not Found**: The router does not support wireless WAN.
- **400, 401, 500, 503**: See [Notes](#notes).

## <a name="put_net_wwan"></a>Set wireless WAN configurations

Set the wireless WAN interface configurations.

    PUT /router/api/v1/net/wwan

#### Additional HTTP Headers

    Content-Type: application/json

#### Parameters

- *object* `data`:
    - *boolean* `enable`: Enable or disable wireless WAN.
    - *object* `conf`: (**required when enable=true**)
        - *string* `ssid`: SSID.
        - *string* `password`: (**optional**) The password for secure WiFi. At least one character.

- **Example**

    ```json
    {
        "data": {
            "enable": true,
            "conf": {
                "ssid": "sylvia-iot",
                "password": "p@sSw0rD"
            }
        }
    }
    ```

#### Response

- **204 No Content**
- **404 Not Found**: The router does not support wireless WAN.
- **400, 401, 500, 503**: See [Notes](#notes).

## <a name="get_net_wwan_list"></a>List available wireless AP

List available wireless AP.

    GET /router/api/v1/net/wwan/list?rescan={forceRescan}

- *boolean* `rescan`: (**optional**) Force rescan AP list even if the router is connected to one AP. Default is **false**.

#### Response

- **200 OK**: The wireless AP list. Parameters are:

    - *object[]* `data`:
        - *string* `ssid`: SSID.
        - *string[]* `security`: The security list such as **WPA1**, **WPA2**. Empty list means the AP does not support security.
        - *number* `channel`: The WiFi channel. 1~11.
        - *number* `signal`: Signal strength.

    - **Example**

        ```json
        {
            "data": [
                {
                    "ssid": "sylvia-iot",
                    "security": [ "WPA1", "WPA2" ],
                    "channel": 11,
                    "signal": 74
                },
                {
                    "ssid": "Home WiFi",
                    "security": [ "WPA2" ],
                    "channel": 9,
                    "signal": 47
                },
                {
                    "ssid": "Public WiFi",
                    "security": [],
                    "channel": 1,
                    "signal": 16
                }
            ]
        }
        ```

- **404 Not Found**: The router does not support wireless WAN.
- **400, 401, 500, 503**: See [Notes](#notes).
