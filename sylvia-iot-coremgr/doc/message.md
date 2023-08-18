# Message Format

## Data Channel

These messages are used from `coremgr` to `data` module(s) with **unicast** and **reliable** queues.

    coremgr.data: {
        kind: string,   // operation
        data: object    // data content
    }

### `operation` Kind

- `operation`: HTTP API operation.
    - *string* `dataId`: unique data ID.
    - *string* `reqTime`: request time in RFC 3339 format (from `resTime` and `latencyMs`).
    - *string* `resTime`: response time in RFC 3339 format.
    - *number* `latencyMs`: latency in milliseconds.
    - *number* `status`: response status code.
    - *string* `sourceIp`: client source IP address.
    - *string* `method`: request HTTP method.
    - *object* `body`: (optional) request body.
    - *string* `userId`: request user ID.
    - *string* `clientId`: request request client ID.
    - *string* `errCode`: (optional) error code.
    - *string* `errMessage`: (optional) error message.
