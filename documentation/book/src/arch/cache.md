# Cache

In the [**Data Flow**](flow.md) section, it is mentioned that the main task of the Broker is "to
match routing rules and forward data".
Typically, routing rules are stored in a database, so the speed of matching becomes a critical
bottleneck. This is especially true when forwarding thousands or even tens of thousands of data at
the same time, putting significant pressure on the database.

As it is well known, one of the best solutions to alleviate pressure on the database is caching, and
Redis is a popular solution for this purpose.

Sylvia-IoT has been designed from the beginning to be as simple as possible and to adopt the minimum
possible variety of technologies (you can run the complete Sylvia-IoT functionality with just SQLite
and MQTT).
Regarding caching, it uses an in-process-memory approach, which means storing data in variables
within the process itself. The matching process does not require network or IPC as it directly
accesses variables within the process.

> Currently, the Broker is implemented using `std::collections::HashMap`.

![Cache](cache.svg)

The diagram above provides an overview of Sylvia-IoT's caching mechanism. To meet the requirements
of a cluster architecture, a broadcast queue is introduced to implement the **Control Channel**.

To ensure data accuracy, updates are first made to the database before updating the cache. Below, we
outline the steps:

1. Users modify routing rules through the HTTP API.
2. Similar to a regular API implementation, the database is directly updated.
3. Before responding to the HTTP request, an update message is sent to the control channel,
   containing necessary update information (optional details like names are excluded).
4. While responding to the HTTP request, the control channel broadcasts the update message to all
   processes in the cluster.
5. Upon receiving the message, each process updates the content of its variables.

> For simplicity, the current implementation mostly involves deleting cache data (the content of
  step 3 is a deletion action) and then filling it with cache-miss.

Let's discuss a few special situations:

- The caching design of the Broker adopts "eventual consistency." After step 3, there might be a
  short period during which the old routing is still in use. However, this period is usually not
  very long (within tens or hundreds of milliseconds, or perhaps even shorter).
- To avoid data inconsistency, when a process detects a reconnection to the control channel's queue,
  it completely clears the cache content. It then reads the data from the database during a
  cache-miss event.

> In the [**Configuration File**](../guide/configuration.md) section, the `mqChannels` contains
  various settings for the control channel corresponding to each API.

Relying on variables within the process as caching allows Sylvia-IoT Broker to achieve efficient
forwarding capabilities &#x1F60A;.
