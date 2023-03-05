# Schema - Broker

## Unit

    unit: {
        unitId: string,                 // (unique) unit ID
        code: string,                   // (unique) unit code
        createdAt: Date,                // creation time
        modifiedAt: Date,               // modification time
        ownerId: string,                // the owner's user ID
        memberIds: string[],            // members' user ID
        name: string,                   // display name
        info: object                    // other information such as address, telephone number, ...
    }

## Application

    application: {
        appicationId: string,           // (unique) application ID
        code: string,                   // (unit unique) application code for queues
        unitId: string,                 // the associated unit ID
        unitCode: string,               // the associated unit code
        createdAt: Date,                // creation time
        modifiedAt: Date,               // modification time
        hostUri: string,                // host URI for the queues
        name: string,                   // display name
        info: object                    // other information
    }

- Unique keys:
    - unitId,code

## Network

    network: {
        networkId: string,              // (unique) network ID
        code: string,                   // (unit unique) network code for queues
        unitId: string | null,          // null means public network
                                        // string means private network of the specific unit
        unitCode: string | null,        // the associated unit code
        createdAt: Date,                // creation time
        modifiedAt: Date,               // modification time
        hostUri: string,                // host URI for the queues
        name: string,                   // display name
        info: object                    // other information
    }

- Unique keys:
    - unitId,code

## Device

    device: {
        deviceId: string,               // (unique) device ID
        unitId: string,                 // the associated unit ID
        unitCode: string | null,        // null means public network
                                        // string means private network of the specific unit
        networkId: string,              // the associated network ID
        networkCode: string,            // the associated network code
        networkAddr: string,            // the unique address of the associated network
        createdAt: Date,                // creation time
        modifiedAt: Date,               // modification time
        name: string,                   // display name
        info: object                    // other information
    }

- Unique keys:
    - unitCode,networkCode,networkAddr

## Device Route

    deviceRoute: {
        routeId: string,                // the routing rule ID
        unitId: string,                 // the associated unit ID for management
        unitCode: string,               // the associated unit code
        applicationId: string,          // the destination application ID for uplink data
        applicationCode: string,        // the destination application code for uplink data
        deviceId: string,               // the device ID
        networkId: string,              // the network ID of the device
        networkCode: string,            // the network code of the device
        networkAddr: string             // the address of the network
        createdAt: Date,                // creation time
    }

- Unique keys:
    - applicationId,deviceId

## Network Route (for private network only)

    networkRoute: {
        routeId: string,                // the routing rule ID
        unitId: string,                 // the associated unit ID for management
        unitCode: string,               // the associated unit code
        applicationId: string,          // the destination application ID for uplink data
        applicationCode: string,        // the destination application code for uplink data
        networkId: string,              // the network ID
        networkCode: string,            // the network code
        createdAt: Date,                // creation time
    }

- Unique keys:
    - applicationId,networkId

## Downlink Data Buffer (for network or device acknowledgement)

    dldataBuffer: {
        dataId: string,                 // the downlink data ID generated for applications
        unitId: string,                 // the associated unit ID for management
        unitCode: string,               // the associated unit code
        applicationId: string,          // the source application ID for downlink data
        applicationCode: string,        // the source application code for downlink data
        networkId: string,              // the destination network ID
        deviceId: string,               // the destination device ID
        createdAt: Date,                // creation time
        expiredAt: Date                 // expiration time that will not respond downlink result
    }
