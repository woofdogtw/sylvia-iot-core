# Configuration

This chapter describes the configuration format and usage of Sylvia-IoT.

Sylvia-IoT supports four sources of configuration, prioritized as follows (from highest to lowest):

- JSON5 configuration file
- Command-line parameters
- Environment variables
- Internal default values (may not exist; if required but not provided, an error message will be
  displayed)

You can refer to
[**the sample JSON5 file**](https://github.com/woofdogtw/sylvia-iot-core/blob/main/files/config.json5.example)
for a complete list of configuration options. This chapter will provide corresponding explanations.
The following conventions apply to the configuration:

- The nested structure in JSON5 is represented using `.` (dot).
- For command-line parameters that encounter nested JSON5, the dot notation is also used.
- For command-line parameters corresponding to camelCase JSON5 properties, they will be written in
  all lowercase or with `-` followed by lowercase. For example:
    - JSON5 property `server.httpPort` corresponds to `--server.httpport`.
    - JSON5 property `broker.mqChannels` corresponds to `--broker.mq-channels`.
- Environment variables are written in all uppercase.
- For environment variables that encounter nested JSON5, `_` (underscore) is used.
- For environment variables corresponding to camelCase JSON5 properties, they will be written in all
  uppercase or with `_` (underscore) separating words. For example:
    - JSON5 property `server.httpPort` corresponds to `SERVER_HTTP_PORT`.
    - JSON5 property `broker.mqChannels` corresponds to `BROKER_MQCHANNELS`.

Here are the complete table explanations:

> If marked with **Refer to example**, it means that the sample JSON5 is provided or you can use
  the CLI **help** command to view the supported options.

## Common Settings

| JSON5             | CLI Parameters    | Environment Variables | Default   | Description |
| -                 | -                 | -                     | -         | - |
| log.level         | log.level         | LOG_LEVEL             | info      | Log level. Refer to example |
| log.style         | log.style         | LOG_STYLE             | json      | Log style. Refer to example |
| server.httpPort   | log.httpport      | SERVER_HTTP_PORT      | 1080      | HTTP listening port |
| server.httpsPort  | log.httpsport     | SERVER_HTTPS_PORT     | 1443      | HTTPS listening port |
| server.cacertFile | log.cacertfile    | SERVER_CACERT_FILE    |           | HTTPS root certificate file location |
| server.certFile   | log.certfile      | SERVER_CERT_FILE      |           | HTTPS certificate file location |
| server.keyFile    | log.keyfile       | SERVER_KEY_FILE       |           | HTTPS private key file location |
| server.staticPath | log.static        | SERVER_STATIC_PATH    |           | Static files directory location |

### Detailed Explanation

- Root certificate is not currently used.
- Both certificate and private key must be used simultaneously to enable HTTPS service.

## API Scopes

All APIs require access through registered clients and access tokens. Each token is associated with
a specific client, and only authorized clients can access the APIs.

When a particular API is configured with `apiScopes` settings, the access token must include the
relevant scopes enabled by the client during registration and authorized by the user to access that
API.

Both command-line parameters and environment variables should be provided as JSON strings. For
example:
```
--auth.api-scopes='{"auth.tokeninfo.get":[]}'
```

You can define custom scope names and apply them to various API scopes. You can refer to the example
provided in the **Authentication Service** section for more details.

## Authentication Service (auth)

| JSON5                     | CLI Parameters            | Environment Variables     | Default                   | Description |
| -                         | -                         | -                         | -                         | - |
| auth.db.engine            | auth.db.engine            | AUTH_DB_ENGINE            | sqlite                    | Database type |
| auth.db.mongodb.url       | auth.db.mongodb.url       | AUTH_DB_MONGODB_URL       | mongodb://localhost:27017 | MongoDB connection URL |
| auth.db.mongodb.database  | auth.db.mongodb.database  | AUTH_DB_MONGODB_DATABASE  | auth                      | MongoDB database name |
| auth.db.mongodb.poolSize  | auth.db.mongodb.poolsize  | AUTH_DB_MONGODB_POOLSIZE  |                           | Maximum number of MongoDB connections |
| auth.db.sqlite.path       | auth.db.sqlite.path       | AUTH_DB_SQLITE_PATH       | auth.db                   | SQLite file location |
| auth.db.templates.login   | auth.db.templates         | AUTH_TEMPLATES            |                           | Login page template file location |
| auth.db.templates.grant   | auth.db.templates         | AUTH_TEMPLATES            |                           | Authorization page template file location |
| auth.db.apiScopes         | auth.api-scopes           | AUTH_API_SCOPES           |                           | API scope settings |

### Detailed Explanation

- Templates:
    - These are the web pages required for the OAuth2 authorization code grant flow.
      **sylvia-iot-auth** provides default pages, but Sylvia-IoT allows you to customize web pages
      to match your own style.
    - The templates use the Jinja2 format (dependent on the [**tera**](https://tera.netlify.app/)
      package).
    - Both command-line parameters and environment variables should use JSON strings. For example:
        ```
        --auth.templates='{"login":"xxx"}'
        ```
    - For more details, please refer to [**OAuth2 Authentication**](../dev/oauth2.md).
- API scopes
    - The **auth** module provides the following scopes that can be configured for corresponding
      APIs to limit the scope of access for clients:
        - `auth.tokeninfo.get`: Authorize clients to read token data.
            - `GET /api/v1/auth/tokeninfo`
        - `auth.logout.post`: Authorize clients to log out tokens.
            - `POST /auth/api/v1/auth/logout`
        - `user.get`: Authorize clients to access current user's profile data.
            - `GET /api/v1/user`
        - `user.path`: Authorize clients to modify current user's profile data.
            - `PATCH /api/v1/user`
        - `user.get.admin`: Authorize clients to access data of all system users.
            - `GET /api/v1/user/count`
            - `GET /api/v1/user/list`
            - `GET /api/v1/user/{userId}`
        - `user.post.admin`: Authorize clients to create new users in the system.
            - `POST /api/v1/user`
        - `user.patch.admin`: Authorize clients to modify data of any system user.
            - `PATCH /api/v1/user/{userId}`
        - `user.delete.admin`: Authorize clients to delete data of any system user.
            - `DELETE /api/v1/user/{userId}`
        - `client.get`: Authorize clients to access data of all system clients.
            - `GET /api/v1/client/count`
            - `GET /api/v1/client/list`
            - `GET /api/v1/client/{clientId}`
        - `client.post`: Authorize clients to create new clients in the system.
            - `POST /api/v1/client`
        - `client.patch`: Authorize clients to modify data of any system client.
            - `PATCH /api/v1/client/{clientId}`
        - `client.delete`: Authorize clients to delete data of any system client.
            - `DELETE /api/v1/client/{clientId}`
        - `client.delete.user`: Authorize clients to delete all clients of any system user.
            - `DELETE /api/v1/client/user/{userId}`
    - For example, in your service, you define the following scopes:
        - `api.admin`: Only authorize the removal of all clients of a user.
        - `api.rw`: Allow read and write access to all APIs except
          `DELETE /api/v1/client/user/{userId}`.
        - `api.readonly`： Only allow access to GET APIs.
        - Allow access to token data and log out actions for all clients.
        ```
        "auth": {
            ...
            "apiScopes": {
                "auth.tokeninfo.get": [],
                "auth.logout.post": [],
                "user.get": ["api.rw", "api.readonly"],
                "user.patch": ["api.rw"],
                "user.post.admin": ["api.rw"],
                "user.get.admin": ["api.rw", "api.readonly"],
                "user.patch.admin": ["api.rw"],
                "user.delete.admin": ["api.rw"],
                "client.post": ["api.rw"],
                "client.get": ["api.rw", "api.readonly"],
                "client.patch": ["api.rw"],
                "client.delete": ["api.rw"],
                "client.delete.user": ["api.admin"],
            },
            ...
        }
        ```
        - In this example, registered clients can freely select these three scopes. Subsequently,
          users will be informed of this information on the authorization page and decide whether to
          grant authorization to the client.

## Message Broker Service (broker)

| JSON5                                     | CLI Parameters                            | Environment Variables                     | Default                       | Description |
| -                                         | -                                         | -                                         | -                             | - |
| broker.auth                               | broker.auth                               | BROKER_AUTH                               | http://localhost:1080/auth    | Authentication service URL |
| broker.db.engine                          | broker.db.engine                          | BROKER_DB_ENGINE                          | sqlite                        | Database type |
| broker.db.mongodb.url                     | broker.db.mongodb.url                     | BROKER_DB_MONGODB_URL                     | mongodb://localhost:27017     | MongoDB connection URL |
| broker.db.mongodb.database                | broker.db.mongodb.database                | BROKER_DB_MONGODB_DATABASE                | auth                          | MongoDB database name |
| broker.db.mongodb.poolSize                | broker.db.mongodb.poolsize                | BROKER_DB_MONGODB_POOLSIZE                |                               | Maximum number of MongoDB connections |
| broker.db.sqlite.path                     | broker.db.sqlite.path                     | BROKER_DB_SQLITE_PATH                     | auth.db                       | SQLite file location |
| broker.cache.engine                       | broker.cache.engine                       | BROKER_CACHE_ENGINE                       | none                          | Cache type |
| broker.cache.memory.device                | broker.cache.memory.device                | BROKER_CACHE_MEMORY_DEVICE                | 1,000,000                     | Memory cache size for devices |
| broker.cache.memory.deviceRoute           | broker.cache.memory.device-route          | BROKER_CACHE_MEMORY_DEVICE_ROUTE          | 1,000,000                     | Memory cache size for device routes |
| broker.cache.memory.networkRoute          | broker.cache.memory.network-route         | BROKER_CACHE_MEMORY_NETWORK_ROUTE         | 1,000,000                     | Memory cache size for network routes |
| broker.mq.prefetch                        | broker.mq.prefetch                        | BROKER_MQ_PREFETCH                        | 100                           | Maximum number of AMQP consumers |
| broker.mq.persistent                      | broker.mq.persistent                      | BROKER_MQ_PERSISTENT                      | false                         | Persistent message delivery for AMQP producers |
| broker.mq.sharedPrefix                    | broker.mq.sharedprefix                    | BROKER_MQ_SHAREDPREFIX                    | $share/sylvia-iot-broker/     | MQTT shared subscription prefix |
| broker.mqChannels.unit.url                | broker.mq-channels.unit.url               | BROKER_MQCHANNELS_UNIT_URL                | amqp://localhost              | Unit control message host |
| broker.mqChannels.unit.prefetch           | broker.mq-channels.unit.prefetch          | BROKER_MQCHANNELS_UNIT_PREFETCH           | 100                           | Maximum number of AMQP consumers for unit control messages |
| broker.mqChannels.application.url         | broker.mq-channels.application.url        | BROKER_MQCHANNELS_APPLICATION_URL         | amqp://localhost              | Application control message host |
| broker.mqChannels.application.prefetch    | broker.mq-channels.application.prefetch   | BROKER_MQCHANNELS_APPLICATION_PREFETCH    | 100                           | Maximum number of AMQP consumers for application control messages |
| broker.mqChannels.network.url             | broker.mq-channels.network.url            | BROKER_MQCHANNELS_NETWORK_URL             | amqp://localhost              | Network control message host |
| broker.mqChannels.network.prefetch        | broker.mq-channels.network.prefetch       | BROKER_MQCHANNELS_NETWORK_PREFETCH        | 100                           | Maximum number of AMQP consumers for network control messages |
| broker.mqChannels.device.url              | broker.mq-channels.device.url             | BROKER_MQCHANNELS_DEVICE_URL              | amqp://localhost              | Device control message host |
| broker.mqChannels.device.prefetch         | broker.mq-channels.device.prefetch        | BROKER_MQCHANNELS_DEVICE_PREFETCH         | 100                           | Maximum number of AMQP consumers for device control messages |
| broker.mqChannels.deviceRoute.url         | broker.mq-channels.device-route.url       | BROKER_MQCHANNELS_DEVICE_ROUTE_URL        | amqp://localhost              | Device route control message host |
| broker.mqChannels.deviceRoute.prefetch    | broker.mq-channels.device-route.prefetch  | BROKER_MQCHANNELS_DEVICE_ROUTE_PREFETCH   | 100                           | Maximum number of AMQP consumers for device route control messages |
| broker.mqChannels.networkRoute.url        | broker.mq-channels.network-route.url      | BROKER_MQCHANNELS_NETWORK_ROUTE_URL       | amqp://localhost              | Network route control message host |
| broker.mqChannels.networkRoute.prefetch   | broker.mq-channels.network-route.prefetch | BROKER_MQCHANNELS_NETWORK_ROUTE_PREFETCH  | 100                           | Maximum number of AMQP consumers for network route control messages |
| broker.mqChannels.data.url                | broker.mq-channels.data.url               | BROKER_MQCHANNELS_DATA_URL                |                               | Data message host |
| broker.mqChannels.data.persistent         | broker.mq-channels.data.persistent        | BROKER_MQCHANNELS_DATA_PERSISTENT         | false                         | Persistent delivery for data messages |
| broker.db.apiScopes                       | broker.api-scopes                         | BROKER_API_SCOPES                         |                               | API scope settings |

### Detailed Explanation

- The purpose of specifying the Authentication Service URL (`broker.auth`) is to verify the
  legitimacy of API calls, including user accounts and clients.

- MQ channels:
    - As the Sylvia-IoT Message Broker Service is a critical module that determines performance,
      many configurations are stored in memory. These configurations need to be propagated to
      various instances of the cluster through **Control Channel Messages** via message queues when
      API changes are made.
        - For relevant details, please refer to the [**Cache**](../arch/cache.md) chapter.
    - **data** represents the **Data Channel Message**, which records all data into the
      **sylvia-iot-data** module.
        - If no parameters are specified (or JSON5 is set to **null**), no data will be stored.
        - For relevant details, please refer to the [**Data Flow**](../arch/flow.md) chapter.

- API scopes: Please refer to the explanation in the **Authentication Service** section.

## Core Manager Service (coremgr)

| JSON5                                 | CLI Parameters                        | Environment Variables                 | Default                       | Description |
| -                                     | -                                     | -                                     | -                             | - |
| coremgr.auth                          | coremgr.auth                          | COREMGR_AUTH                          | http://localhost:1080/auth    | Authentication service URL |
| coremgr.broker                        | coremgr.broker                        | COREMGR_BROKER                        | http://localhost:2080/broker  | Message broker service URL |
| coremgr.mq.engine.amqp                | coremgr.mq.engine.amqp                | COREMGR_MQ_ENGINE_AMQP                | rabbitmq                      | AMQP type |
| coremgr.mq.engine.mqtt                | coremgr.mq.engine.mqtt                | COREMGR_MQ_ENGINE_MQTT                | emqx                          | MQTT type |
| coremgr.mq.rabbitmq.username          | coremgr.mq.rabbitmq.username          | COREMGR_MQ_RABBITMQ_USERNAME          | guest                         | RabbitMQ administrator account |
| coremgr.mq.rabbitmq.password          | coremgr.mq.rabbitmq.password          | COREMGR_MQ_RABBITMQ_PASSWORD          | guest                         | RabbitMQ administrator password |
| coremgr.mq.rabbitmq.ttl               | coremgr.mq.rabbitmq.ttl               | COREMGR_MQ_RABBITMQ_TTL               |                               | RabbitMQ default message TTL (seconds) |
| coremgr.mq.rabbitmq.length            | coremgr.mq.rabbitmq.length            | COREMGR_MQ_RABBITMQ_LENGTH            |                               | RabbitMQ default maximum number of messages in queues |
| coremgr.mq.rabbitmq.hosts             | coremgr.mq.rabbitmq.hosts             | COREMGR_MQ_RABBITMQ_HOSTS             |                               | (Reserved) |
| coremgr.mq.emqx.apiKey                | coremgr.mq.emqx.apikey                | COREMGR_MQ_EMQX_APIKEY                |                               | EMQX management API key |
| coremgr.mq.emqx.apiSecret             | coremgr.mq.emqx.apisecret             | COREMGR_MQ_EMQX_APISECRET             |                               | EMQX management API secret |
| coremgr.mq.emqx.hosts                 | coremgr.mq.emqx.hosts                 | COREMGR_MQ_EMQX_HOSTS                 |                               | (Reserved) |
| coremgr.mq.rumqttd.mqttPort           | coremgr.mq.rumqttd.mqtt-port          | COREMGR_MQ_RUMQTTD_MQTT_PORT          | 1883                          | rumqttd MQTT port |
| coremgr.mq.rumqttd.mqttsPort          | coremgr.mq.rumqttd.mqtts-port         | COREMGR_MQ_RUMQTTD_MQTTS_PORT         | 8883                          | rumqttd MQTTS port |
| coremgr.mq.rumqttd.consolePort        | coremgr.mq.rumqttd.console-port       | COREMGR_MQ_RUMQTTD_CONSOLE_PORT       | 18083                         | rumqttd management API port |
| coremgr.mqChannels.data.url           | coremgr.mq-channels.data.url          | COREMGR_MQCHANNELS_DATA_URL           |                               | Data message host |
| coremgr.mqChannels.data.persistent    | coremgr.mq-channels.data.persistent   | COREMGR_MQCHANNELS_DATA_PERSISTENT    | false                         | Persistent delivery for data messages |

### Detailed Explanation

- MQ channels:
    - **data** represents the **Data Channel Message**.
        - Currently, coremgr supports recording HTTP request content for all API requests except
          GET. Enabling the data channel will record the API usage history.
        - If no parameters are specified (or JSON5 is set to **null**), no data will be stored.

## Core Manager Command-Line Interface (coremgr-cli)

| JSON5                     | CLI Parameters            | Environment Variables     | Default                       | Description |
| -                         | -                         | -                         | -                             | - |
| coremgrCli.auth           | coremgr-cli.auth          | COREMGRCLI_AUTH           | http://localhost:1080/auth    | Authentication service URL |
| coremgrCli.coremgr        | coremgr-cli.coremgr       | COREMGRCLI_COREMGR        | http://localhost:3080/coremgr | Core manager service URL |
| coremgrCli.data           | coremgr-cli.data          | COREMGRCLI_DATA           | http://localhost:4080/data    | Data service URL |
| coremgrCli.clientId       | coremgr-cli.client-id     | COREMGRCLI_CLIENT_ID      |                               | CLI client ID |
| coremgrCli.redirectUri    | coremgr-cli.redirect-uri  | COREMGRCLI_REDIRECT_URI   |                               | CLI client redirect URI |

## Data Service (data)

| JSON5                                 | CLI Parameters                        | Environment Variables                 | Default                       | Description |
| -                                     | -                                     | -                                     | -                             | - |
| data.auth                             | data.auth                             | DATA_AUTH                             | http://localhost:1080/auth    | Authentication service URL |
| data.broker                           | data.broker                           | DATA_BROKER                           | http://localhost:2080/broker  | Message broker service URL |
| data.db.engine                        | data.db.engine                        | DATA_DB_ENGINE                        | sqlite                        | Database type |
| data.db.mongodb.url                   | data.db.mongodb.url                   | DATA_DB_MONGODB_URL                   | mongodb://localhost:27017     | MongoDB connection URL |
| data.db.mongodb.database              | data.db.mongodb.database              | DATA_DB_MONGODB_DATABASE              | data                          | MongoDB database name |
| data.db.mongodb.poolSize              | data.db.mongodb.poolsize              | DATA_DB_MONGODB_POOLSIZE              |                               | Maximum number of MongoDB connections |
| data.db.sqlite.path                   | data.db.sqlite.path                   | DATA_DB_SQLITE_PATH                   | data.db                       | SQLite file location |
| data.mqChannels.broker.url            | data.mq-channels.broker.url           | DATA_MQCHANNELS_BROKER_URL            | amqp://localhost              | Data message host |
| data.mqChannels.broker.prefetch       | data.mq-channels.broker.prefetch      | DATA_MQCHANNELS_BROKER_PREFETCH       | 100                           | Maximum number of AMQP consumers for data messages |
| data.mqChannels.broker.sharedPrefix   | data.mq-channels.broker.sharedprefix  | DATA_MQCHANNELS_BROKER_SHAREDPREFIX   | $share/sylvia-iot-data/       | MQTT shared subscription prefix |
| data.mqChannels.coremgr.url           | data.mq-channels.coremgr.url          | DATA_MQCHANNELS_COREMGR_URL           | amqp://localhost              | Data message host |
| data.mqChannels.coremgr.prefetch      | data.mq-channels.coremgr.prefetch     | DATA_MQCHANNELS_COREMGR_PREFETCH      | 100                           | Maximum number of AMQP consumers for data messages |
| data.mqChannels.coremgr.sharedPrefix  | data.mq-channels.coremgr.sharedprefix | DATA_MQCHANNELS_COREMGR_SHAREDPREFIX  | $share/sylvia-iot-data/       | MQTT shared subscription prefix |
