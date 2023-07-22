# Code Style

## MVC vs. Microservices

I prefer a bottom-up development approach. Using an architecture like MVC, which designs the
database as a lower-level generic interface, and implementing various functionalities called by the
API upper layer, aligns well with my personal style. This is the reason behind the creation of
`models` and `routes`.

However, when designing the entire Sylvia-IoT platform, I also aimed for modularity and chose a
microservices-based approach (i.e., ABCD), strictly adhering to the principle of hierarchical
dependencies.

Even with a microservices architecture, as described in the previous section
[**Directory Structure**](dir.md), when `main.rs` references the required `routes`, the entire
project can still be compiled into a single executable file and run on a single machine. This
design offers several deployment options, such as:

- Monolith: Running a single all-in-one executable on a single machine.
- Microservices cluster: Running each component independently on different machines, with each
  component setting up its own cluster.
- Monolith cluster: Running the all-in-one on multiple machines to form a clustered architecture.

Sylvia-IoT embodies the combination of both MVC and microservices design &#x1F60A;.

## File Content Arrangement

Each rs file is structured in the following way, with blank lines separating each section:

```rust
use rust_builtin_modules;

use 3rd_party_modules;

use sylvia_iot_modules;

use crate_modules;

pub struct PubStructEnums {}

struct PrvStructEnums {}

pub const PUB_CONSTANTS;

const PRV_CONSTANTS;

pub pub_static_vars;

static prv_static_vars;

impl PubStructEnums {}

pub fn pub_funcs {}

impl PrvStructEnums {}

fn prv_funcs {}
```

The general order is as follows:

- Using modules
- Structures
- Constants
- Variables
- Functions (including structure function implementations)

Within each section, `pub` comes before private.

## Model

The Model layer must provide a unified struct and trait interface.
In the design philosophy of Sylvia-IoT, "plug-and-play" is a concept that is highly valued. Users
should be able to choose appropriate implementations in different scenarios.

### Database Design

When providing CRUD operations, the following order must be followed:

- count
- list
- get
- add
- upsert
- update
- del

Some points to note:

- **count** and **list** should provide consistent parameters so that the API and UI can call count
  and list in a consistent manner.
- Logger should not be used in the `model`. Errors should be returned to the upper layer to print
  the messages.
    - When multiple APIs call the same `model`, errors printed from the model cannot determine who
      made the call.
- When data cannot be retrieved, return `None` or an empty `Vec`, not an `Error`.
- Any database that can fulfill the "complex query" condition should be implementable using the same
  trait interface.
    - SQL, MongoDB meet this requirement.
    - Redis cannot be designed in the form of a database.

### Cache Design

- Any **key-value** store that can fulfill low-complexity read and write should be implementable
  using the same trait interface.
    - Redis, language-specific maps meet this requirement.
    - SQL, MongoDB can also be implemented through querying a single condition. Using SQL or MongoDB
      for cache implementation is allowed when the system does not want to install too many
      different tools.

## Routes (HTTP API)

In this section, the documentation and rules for implementing APIs are provided.

### Verb Order

- POST
- GET /count
- GET /list
- GET
- PUT
- PATCH
- DELETE

### Path

- `/[project]/api/v[version]/[function]`
- `/[project]/api/v[version]/[function]/[op]`
- `/[project]/api/v[version]/[function]/{id}`

There is a potential ambiguity: `[op]` and `{id}`. The former represents a fixed action, while the
latter represents a variable object ID. When designing IDs, it is essential to avoid conflicts with
the names of actions.

> When mounting routes using Actix Web, the fixed `[op]` should be placed before the variable
  `{id}`.

For example, let's consider the Broker's
[**Device API**](https://github.com/woofdogtw/sylvia-iot-core/blob/main/sylvia-iot-broker/doc/api.md#contents):

```
- Device APIs
    - POST /broker/api/v1/device                Create device
    - POST /broker/api/v1/device/bulk           Bulk creating devices
    - POST /broker/api/v1/device/bulk-delete    Bulk deleting devices
    - GET  /broker/api/v1/device/count          Device count
    - GET  /broker/api/v1/device/list           Device list
    - GET  /broker/api/v1/device/{deviceId}     Get device information
```

Here, you can see that the POST method handles creating single devices, bulk creating devices, and
bulk deleting devices. The **bulk**, **bulk-delete**, **count**, **list** are the previously
mentioned `[op]`.
The design of device IDs should avoid conflicts with **count** and **list**.

### Function Naming

The functions in `api.rs` are named as follows:

```
fn [method]_[function]_[op]() {}
```

Continuing with the previous device API example, the functions would be named like this:

```
fn post_device() {}
fn post_device_bulk() {}
fn post_device_bulk_del() {}
fn get_device_count() {}
fn get_device_list() {}
fn get_device() {}
```

### Request and Response Naming

Path variables, queries, and request bodies are defined in `request.rs`, while response bodies are
defined in `response.rs`. The naming convention is as follows (pay attention to capitalization):

```
struct [Id]Path {}
struct [Method][Function]Body {}
struct Get[Function]Query {}
```

For example:

```
struct DeviceIdPath {}      // /device/{deviceId}
struct PostDeviceBody {}
struct GetDeviceListQuery {}
```
