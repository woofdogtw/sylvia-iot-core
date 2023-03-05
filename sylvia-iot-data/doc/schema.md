# Schema - Data

## Application uplink data

    applicationUlData: {
        dataId: string,             // (unique) data ID
        proc: Date,                 // received time when the broker receive this data
        pub: Date,                  // publish time to queue
        unitCode: string | null,    // network's unit code
        networkCode: string,        // network code
        networkAddr: string,        // device network address
        unitId: string,             // routed data's unit ID
        deviceId: string,           // device ID
        time: Date,                 // device time
        data: string,               // data in hexadecimal format
        extension: object           // (optional) extensions
    }

## Application downlink data

    applicationDlData: {
        dataId: string,             // (unique) data ID
        proc: Date,                 // received time when the broker receive this data
        resp: Date,                 // (optional) the last data response time
        status: number,             // 0 for success, negative for processing, positive for error
        unitId: string,             // device's unit ID
        deviceId: string,           // (optional) device ID
        networkCode: string,        // (optional) device network code
        networkAddr: string,        // (optional) device network address
        data: string,               // data in hexadecimal format
        extension: object           // (optional) extensions
    }

## Network uplink data

    networkUlData: {
        dataId: string,             // (unique) data ID
        proc: Date,                 // received time when the broker receive this data
        unitCode: string | null,    // network's unit code
        networkCode: string,        // network code
        networkAddr: string,        // device network address
        unitId: string,             // (present if device exist) private network's unit ID
        deviceId: string,           // (present if device exist) device ID
        time: Date,                 // device time
        data: string,               // data in hexadecimal format
        extension: object           // (optional) extensions
    }

## Network downlink data

    networkDlData: {
        dataId: string,             // (unique) data ID
        proc: Date,                 // received time when the broker receive this data
        pub: Date,                  // publish time to queue
        resp: Date,                 // (optional) the last data response time
        status: number,             // 0 for success, negative for processing, positive for error
        unitId: string,             // device's unit ID
        deviceId: string,           // device ID
        networkCode: string,        // device network code
        networkAddr: string,        // device network address
        data: string,               // data in hexadecimal format
        extension: object           // (optional) extensions
    }

## Coremgr operations data

    coremgrOpData: {
        dataId: string,             // (unique) data ID
        reqTime: Date,              // request time
        resTime: Date,              // response time
        latencyMs: number,          // latency in milliseconds
        status: number,             // response status code
        sourceIp: string,           // client source IP address
        method: string,             // request HTTP method
        path: string,               // request HTTP path
        body: object,               // (optional) request body
        userId: string,             // request user ID
        clientId: string,           // request client ID
        errCode: string,            // (optional) error code
        errMessage: string          // (optional) error message
    }
