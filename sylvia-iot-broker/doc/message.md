# Message Format

## Between Broker and Application

### Device and application data

These messages are used between devices and applications with **unicast** and **reliable** queues.

    broker.application.[unit].[code].uldata: {
        dataId: string,                 // unique data ID
        time: string,                   // device time for this data in RFC 3339 format
        pub: string,                    // publish time to queue in RFC 3339 format
        deviceId: string,               // device ID
        networkId: string,              // device's network ID
        networkCode: string,            // device's network code
        networkAddr: string,            // device network address
        isPublic: bool,                 // the network is public or private
        profile: string,                // the device profile of data
        data: string,                   // data in hexadecimal format
        extension: object               // (optional) extensions for application
    }

    broker.application.[unit].[code].dldata: {
        correlationId: string,          // the correlation ID for the data to get response from `dldata-resp` queue
        deviceId: string,               // (required for public network) destination device ID
        networkCode: string,            // (required if no `deviceId`) device network code
        networkAddr: string,            // (required if no `deviceId`) device network address
        data: string,                   // data in hexadecimal format
        extension: object               // (optional) extensions for network
    }

    broker.application.[unit].[code].dldata-resp: {
        correlationId: string,          // the correlation ID for the data
        dataId: string,                 // (present if success) unique data ID
        error: string,                  // (present if fail) error code
        message: string                 // (optional) detail error message
    }

    broker.application.[unit].[code].dldata-result: {
        dataId: string,                 // unique data ID
        status: number,                 // 0 for success, negative for processing, positive for error
        message: string                 // (optional) defail message
    }

## Between Broker and Network

### Device and application data

These messages are used between devices and applications with **unicast** and **reliable** queues.

    broker.network.[unit].[code].uldata: {
        time: string,                   // device time for this data in RFC 3339 format
        networkAddr: string,            // device network address
        data: string,                   // data in hexadecimal format
        extension: object               // (optional) extensions for application
    }

    broker.network.[unit].[code].dldata: {
        dataId: string,                 // unique data ID
        pub: string,                    // publish time to queue in RFC 3339 format
        expiresIn: number,              // data expires in seconds
        networkAddr: string,            // device network address
        data: string,                   // data in hexadecimal format
        extension: object               // (optional) extensions for network
    }

    broker.network.[unit].[code].dldata-result: {
        dataId: string,                 // unique data ID
        status: number,                 // 0 for success, negative for processing, positive for error
        message: string                 // (optional) detail message
    }

### Control messages

These messages are used for notifying network servers that relative devices are modified with **unicast** and **reliable** queues.

    broker.network.[unit].[code].ctrl: {
        operation: string,              // operation
        time: string,                   // timestamp in RFC 3339 format.
        new: object | object[],         // new configuration(s)
        old: object | object[]          // (optional) old configuration(s)
    }

The operations are:

- `add-device`: to add a device.
    - *object* `new`:
        - *string* `networkAddr`: The network address of the specified network.
- `add-device-bulk`: to add devices in bulk.
    - *object* `new`:
        - *string[]* `networkAddrs`: The network addresses of the specified network.
- `add-device-range`: to add devices with address range.
    - *object* `new`:
        - *string* `startAddr`: The start network address of the specified network.
        - *string* `endAddr`: The end network address of the specified network.
- `del-device`: to delete a device.
    - *object* `new`:
        - *string* `networkAddr`: The network address of the specified network.
- `del-device-bulk`: to delete devices in bulk.
    - *object* `new`:
        - *string[]* `networkAddrs`: The network addresses of the specified network.
- `del-device-range`: to delete devices with address range.
    - *object* `new`:
        - *string* `startAddr`: The start network address of the specified network.
        - *string* `endAddr`: The end network address of the specified network.

## Control Channel

The messages are used between `broker`s in the clulster for operating cache with **broadcast** and **best-effort** queues.

    broker.ctrl.[function]: {           // function such as `application`, `network`, ...
        operation: string,              // operation
        new: object | object[],         // new configuration(s)
        old: object | object[]          // (optional) old configuration(s)
    }

### `broker.ctrl.unit` Operations

- `del-unit`: to delete a unit.
    - *object* `new`:
        - *string* `unitId`: unit ID.
        - *string* `unitCode`: unit code.

### `broker.ctrl.application` Operations

- `del-application`: to delete an application.
    - *object* `new`:
        - *string* `unitId`: unit ID.
        - *string* `unitCode`: unit code.
        - *string* `applicationId`: application ID.
        - *string* `applicationCode`: application code.

- `add-manager`: to add an application manager.
    - *object* `new`:
        - *string* `hostUri`: manager host URI.
        - *object* `mgrOptions`:
            - *string* `unitId`: unit ID.
            - *string* `unitCode`: unit code.
            - *string* `id`: application ID.
            - *string* `name`: application code.
            - *number* `prefetch`: (**optional**) AMQP prefetch option.
            - *string* `sharedPrefix`: (**optional**) MQTT shared queue prefix option.
- `del-manager`: to delete an application manager.
    - *string* `new`: the key of application managers.

### `broker.ctrl.network` Operations

- `del-network`: to delete a network.
    - *object* `new`:
        - *string | null* `unitId`: unit ID. **null** for public network.
        - *string | null* `unitCode`: unit code. **null** for public network.
        - *string* `networkId`: network ID.
        - *string* `networkCode`: network code.

- `add-manager`: to add a network manager.
    - *object* `new`:
        - *string* `hostUri`: manager host URI.
        - *object* `mgrOptions`:
            - *string* `unitId`: unit ID.
            - *string* `unitCode`: unit code.
            - *string* `id`: network ID. Empty for public network.
            - *string* `name`: network code. Empty for public network.
            - *number* `prefetch`: (**optional**) AMQP prefetch option.
            - *string* `sharedPrefix`: (**optional**) MQTT shared queue prefix option.
- `del-manager`: to delete a network manager.
    - *string* `new`: the key of application managers.

### `broker.ctrl.device` Operations

- `del-device`: to delete a device.
    - *object* `new`:
        - *string* `unitId`: unit ID.
        - *string | null* `unitCode`: unit code. **null** for public network.
        - *string* `networkId`: network ID.
        - *string* `networkCode`: network code.
        - *string* `networkAddr`: network address.
        - *string* `deviceId`: device ID.

### `broker.ctrl.device-route` Operations

- `del-route`: to delete a device route.
    - *object* `new`:
        - *string* `routeId`: route ID.
        - *string* `unitId`: unit ID.
        - *string* `unitCode`: unit code.
        - *string* `applicationId`: application ID.
        - *string* `applicationCode`: application code.
        - *string* `deviceId`: device ID.
        - *string* `networkId`: network ID.
        - *string* `networkCode`: network code.
        - *string* `networkAddr`: network address.

### `broker.ctrl.network-route` Operations

- `del-route`: to delete a network route.
    - *object* `new`:
        - *string* `routeId`: route ID.
        - *string* `unitId`: unit ID.
        - *string* `unitCode`: unit code.
        - *string* `applicationId`: application ID.
        - *string* `applicationCode`: application code.
        - *string* `networkId`: network ID.
        - *string* `networkCode`: network code.

## Data Channel

These messages are used from `broker` to `data` module(s) with **unicast** and **reliable** queues.

    broker.data: {
        kind: string,   // network-uldata, network-dldata, ...
        data: object    // data content
    }

### `application-uldata` Kind

- `application-uldata`: uplink data route to the application.
    - *object* `data`:
        - *string* `dataId`: unique data ID.
        - *string* `proc`: received time in RFC 3339 format when the broker receive this data.
        - *string* `pub`: publish time in RFC 3339 format to the queue.
        - *string | null* `unitCode`: network's unit code.
        - *string* `networkCode`: network code.
        - *string* `networkAddr`: device network address.
        - *string* `unitId`: routed data's unit ID.
        - *string* `deviceId`: device ID.
        - *string* `time`: data time in RFC 3339 format from the device.
        - *string* `profile`: device profile.
        - *string* `data`: data in hexadecimal format.
        - *object* `extension`: (**optional**) extensions.

### `application-dldata` Kind

- `application-dldata`: uplink data route to the application.
    - *object* `data`:
        - *string* `dataId`: unique data ID.
        - *string* `proc`: received time in RFC 3339 format when the broker receive this data.
        - *number* `status`: **0** for success, negative for processing, positive for error.
        - *string* `unitId`: device's unit ID.
        - *string* `deviceId`: (**optional**) device ID.
        - *string* `networkCode`: (**optional**) device network code.
        - *string* `networkAddr`: (**optional**) device network address.
        - *string* `profile`: device profile.
        - *string* `data`: data in hexadecimal format.
        - *object* `extension`: (**optional**) extensions.

### `application-dldata-result` Kind

- `application-dldata-result`: uplink data route to the application.
    - *object* `data`:
        - *string* `dataId`: unique data ID.
        - *string* `resp`: received time in RFC 3339 format when the broker receive this result.
        - *number* `status`: **0** for success, negative for processing, positive for error.

### `network-uldata` Kind

- `network-uldata`: uplink data from network.
    - *object* `data`:
        - *string* `dataId`: unique data ID.
        - *string* `proc`: received time in RFC 3339 format when the broker receive this data.
        - *string | null* `unitCode`: network's unit code.
        - *string* `networkCode`: network code.
        - *string* `networkAddr`: device network address.
        - *string* `unitId`: (**present for private network**) network's unit ID.
        - *string* `deviceId`: (**present if the device exist**) device ID.
        - *string* `time`: data time in RFC 3339 format from the device.
        - *string* `profile`: (**present if the device exist**) device profile.
        - *string* `data`: data in hexadecimal format.
        - *object* `extension`: (**optional**) extensions.

### `network-dldata` Kind

- `network-dldata`: downlink data from applications to the network.
    - *object* `data`:
        - *string* `dataId`: unique data ID.
        - *string* `proc`: received time in RFC 3339 format when the broker receive this data.
        - *string* `pub`: publish time in RFC 3339 format to the queue.
        - *number* `status`: **0** for success, negative for processing, positive for error.
        - *string* `unitId`: device's unit ID.
        - *string* `deviceId`: device ID.
        - *string* `networkCode`: device network code.
        - *string* `networkAddr`: device network address.
        - *string* `profile`: device profile.
        - *string* `data`: data in hexadecimal format.
        - *object* `extension`: (**optional**) extensions.

### `network-dldata-result` Kind

- `network-dldata-result`: downlink data result from the network.
    - *object* `data`:
        - *string* `dataId`: unique data ID.
        - *string* `resp`: received time in RFC 3339 format when the broker receive this result.
        - *number* `status`: **0** for success, negative for processing, positive for error.
