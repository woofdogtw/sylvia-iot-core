# Data Flow

This chapter introduces how Sylvia-IoT handles data flow, including the following scenarios:

- Uplink data: Data sent from devices to applications.
- Downlink data: Data sent from applications to devices.
- Control channel: Messages transmitted from Broker to the network.
- Coremgr operation data: Records of system operation history, including management operations.

## Uplink Data

![Uplink](flow-uplink.svg)

When device data is sent to the corresponding queue through the network service, the data will be
processed and sent to the application as follows:

1. If the data format is correct, it will proceed to the next step; otherwise, it will be discarded.
2. Broker first sends the data directly to the Data module (via the queue) to store the complete
   uplink data content.
3. Scan all device routes and perform the following actions:
    - Send the data to the corresponding application queue.
    - Store the data sent to the application in the Data module.
4. Scan all network routes and perform the following actions:
    - Check if the data has already been sent during the device route stage. If yes, move to the
      next network route action; if not, continue with the following actions:
    - Send the data to the corresponding application queue.
    - Store the data sent to the application in the Data module.

> The purpose of the comparison in Step 4 is to avoid duplicate sending when device routes and
  network routes overlap.

## Downlink Data

![Downlink](flow-downlink.svg)

When the application service sends data to be delivered to a device through the queue, the data is
processed as follows:

1. If the format is correct, proceed to the next step; otherwise, respond with an error message
   through the `resp` queue.
2. Check if the destination device belongs to the specified unit. If it does, proceed to the next
   step; otherwise, respond with an error message through the `resp` queue.
3. Assign a unique identifier (ID) to this data as an independent entry and store it in the Data
   module.
4. Store the ID and the source application of this data in the database to facilitate reporting the
   delivery status back to the application service in the future.
5. Send the data (including the data ID) to the queue of the corresponding network service.
6. If the data is sent to the network service queue, report the data ID back to the application
   service to track the delivery status.

> Compared to uplink data, downlink data is slightly more complex, mainly because
  **reporting the delivery status is required**.

The Broker does not retain the `resp` queue for the network service to report data correctness. This
is because the Broker, being part of the infrastructure, always ensures data correctness.
The network service only needs to focus on delivering the data to the device and reporting the final
result. Even if the data sent by the Broker is invalid, the network service can directly report it
through the `result` queue.

![Downlink-Result](flow-downlink-result.svg)

After processing the data (regardless of success or failure), the network service **MUST** use the
data ID to report back to the Broker in the following order:

1. If the format is correct, proceed to the next step; otherwise, discard the message.
2. Submit a request to the Data module for result updates using the ID.
3. Retrieve the application service information associated with that ID and report the result back
   to the application service that sent this downlink data (ensuring that other applications will
   not receive the result).
4. If step 3 is successful, clear the ID information from the database.

> The use of an additional ID database aims to retain the source application of the downlink data.
  After all, if data is sent by application A, why should application B receive the result
  &#x1F60A;?

## Control Channel

![Ctrl](flow-ctrl.svg)

The Broker or coremgr provides APIs that allow the network service to update device data at any
time. However, relying on periodic API requests for synchronization is inefficient and may impact
routing performance due to frequent requests.
The Broker provides a mechanism that when there are changes in device data, information is provided
to the corresponding network service through `broker.network.[unit-code].[network-code].ctrl`.

Sylvia-IoT allows devices to change their associated networks or addresses. When this operation
occurs, the network service will receive the following messages based on different scenarios:

- Changing from network A to network B:
    - Notify network A that a specific address has been removed.
    - Notify network B that a specific address has been added.
- Changing the address within network A:
    - Notify network A that a specific address has been removed.
    - Notify network A that a specific address has been added.

## Operation Data

![OpData](flow-opdata.svg)

Coremgr has an optional configuration to store all system operation logs (limited to coremgr HTTP
APIs, of course). The current scope includes POST/PUT/PATCH/DELETE, etc.

As shown in the diagram, after each API operation, coremgr records the following data:

- Request time
- Response time
- Processing time
- HTTP status
- Source IP address
- HTTP method
- (Optional) HTTP request body
    - The content of `data.password` is filtered. When the request contains a `password` field, its
      content is cleared. The key is retained to indicate that this request involves password modification.
- User ID
- Client ID
