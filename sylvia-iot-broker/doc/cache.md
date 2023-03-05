# Cache - Broker

## Device

### For uplink data

- Key: `[(network)unit-code].[network-code].[network-addr]`
    - Empty unit code for public network.
- Value (object)
    - none: for data not in database.
    - object
        - *string* `deviceId`

## Device Route

### For uplink data

- Key: `[device-id]`
- Value (array, set, or keys of hash map)
    - none: for data not in database.
    - object
        - `[(application)unit-code].[application-code]: keys of the application managers that matches routing rules.

### For private network downlink data

- Key: `[(application)unit-code].[network-code].[network-addr]`: for private network only.
- Value (object)
    - none: for data not in database.
    - object
        - `[(network)unit-code].[network-code]`: key of the network manager.
        - Network ID of the device.
        - Network address of the device.
        - Device ID.

### For both public/private network downlink data

- Key: `[(application)unit-id].[deviceId]`: for public/private network.
- Value (string)
    - none: for data not in database.
    - object
        - `[(network)unit-code].[network-code]`: key of the network manager.
        - Network ID of the device.
        - Network address of the device.
        - Device ID.

## Network Route

### For uplink data

- Key: `[network-id]`
    - Empty unit code for public network.
- Value (array, set, or keys of hash map)
    - none: for data not in database.
    - object
        - `[(application)unit-code].[application-code]: keys of the application managers that matches routing rules.
