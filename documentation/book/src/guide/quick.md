# Quick Start

This chapter describes the quick installation steps in the Ubuntu 22.04 environment.

## Install Tools

```shell
sudo apt -y install curl jq
```

## Install Docker

Refer to the installation steps on the [**Docker official website**](https://docs.docker.com/engine/install/ubuntu/).

```shell
sudo apt -y install apt-transport-https ca-certificates curl gnupg lsb-release
curl -fsSL https://download.docker.com/linux/ubuntu/gpg | sudo gpg --dearmor -o /usr/share/keyrings/docker-archive-keyring.gpg
echo "deb [arch=amd64 signed-by=/usr/share/keyrings/docker-archive-keyring.gpg] https://download.docker.com/linux/ubuntu $(lsb_release -cs) stable" | sudo tee /etc/apt/sources.list.d/docker.list > /dev/null
sudo apt update
sudo apt -y install docker-ce docker-ce-cli containerd.io docker-compose-plugin
sudo usermod -aG docker $USER
```

> Remember to restart the shell to apply user permissions.

## Install MongoDB、RabbitMQ、EMQX

Start the services (versions and data storage folders can be adjusted as needed):

```shell
export MONGODB_VER=8.2.2
export RABBITMQ_VER=4.2.0
export EMQX_VER=6.0.1

export MONGODB_DIR=$HOME/db/mongodb
export RABBITMQ_DIR=$HOME/db/rabbitmq
export EMQX_DIR=$HOME/db/emqx

mkdir -p $MONGODB_DIR
docker run --rm --name mongodb -d \
  -p 27017:27017 \
  -v $MONGODB_DIR:/data/db \
  mongo:$MONGODB_VER

mkdir -p $RABBITMQ_DIR
docker run --rm --name rabbitmq -d \
  -e RABBITMQ_NODENAME="rabbit@localhost" \
  -p 5671:5671 -p 5672:5672 -p 15672:15672 \
  -v $RABBITMQ_DIR:/var/lib/rabbitmq \
  rabbitmq:$RABBITMQ_VER-management-alpine

mkdir -p $EMQX_DIR
docker run --rm --name emqx -d \
  -e EMQX_LOADED_PLUGINS="emqx_dashboard|emqx_management|emqx_auth_mnesia" \
  -e EMQX_LOADED_MODULES="emqx_mod_acl_internal,emqx_mod_presence,emqx_mod_topic_metrics" \
  -p 1883:1883 -p 8883:8883 -p 18083:18083 \
  -v $EMQX_DIR:/opt/emqx/data \
  emqx/emqx:$EMQX_VER
```

> The following information only introduces the plugins required by EMQX, which will not be used in
  the following demonstrations. You can also choose not to start EMQX at this stage.

## Download Sylvia-IoT

```shell
ARCH=x86_64 # x86_64 or arm64
curl -LO https://github.com/woofdogtw/sylvia-iot-core/releases/latest/download/sylvia-iot-core-$ARCH.tar.xz
curl -LO https://github.com/woofdogtw/sylvia-iot-core/releases/latest/download/sylvia-iot-coremgr-cli-$ARCH.tar.xz
curl -L -o config.json5 https://github.com/woofdogtw/sylvia-iot-core/raw/main/files/config.json5.example
tar xf sylvia-iot-core-$ARCH.tar.xz
tar xf sylvia-iot-coremgr-cli-$ARCH.tar.xz
```

## Modify config.json5

For demonstration purposes, we make some modifications to the example config.json5:

- Since we are showcasing MongoDB here, we change all `"engine": "sqlite"` to `"engine": "mongodb"`.
  ```
  "db": {
      "engine": "mongodb",
      ...
  },
  ```
- We don't enable HTTPS for now, so the certificate file settings are commented out:
  ```
  //"cacertFile": "/etc/ssl/certs/ca-certificates.crt",
  //"certFile": "/home/user/rust/conf/certs/sylvia-iot.crt",
  //"keyFile": "/home/user/rust/conf/certs/sylvia-iot.key",
  ```
- We create a folder to store static files, and in this example, it's `/home/user/static`.
  ```
  "staticPath": "/home/user/static",
  ```
- We use the default login page template and comment out the example template:
  ```
  "templates": {      // Jinja2 template paths.
      //"login": "/home/user/rust/static/login.j2",
      //"grant": "/home/user/rust/static/grant.j2",
  },
  ```
- We use [**rumqttd**](https://github.com/bytebeamio/rumqtt) instead of EMQX:
  ```
  "coremgr": {
    ...
    "mq": {
      "engine": {
        "amqp": "rabbitmq",
        "mqtt": "rumqttd",
      },
      ...
    },
    ...
  },
  ```

## Set Up Initial Data

First, let's enter the MongoDB shell:

```shell
docker exec -it mongodb mongosh
```

In the MongoDB shell interface, we create the basic data:

```
use test1

db.user.insertOne({
  userId: 'admin',
  account: 'admin',
  createdAt: new Date(),
  modifiedAt: new Date(),
  verifiedAt: new Date(),
  expiredAt: null,
  disabledAt: null,
  roles: {"admin":true,"dev":false},
  password: '27258772d876ffcef7ca2c75d6f4e6bcd81c203bd3e93c0791c736e5a2df4afa',
  salt: 'YsBsou2O',
  name: 'Admin',
  info: {}
})

db.client.insertOne({
  clientId: 'public',
  createdAt: new Date(),
  modifiedAt: new Date(),
  clientSecret: null,
  redirectUris: ['http://localhost:1080/auth/oauth2/redirect'],
  scopes: [],
  userId: 'dev',
  name: 'Public',
  imageUrl: null
})
```

Then, press `Ctrl+C` twice to exit.

## Getting Started

Start Sylvia-IoT core:

```shell
./sylvia-iot-core -f config.json5
```

If the program doesn't terminate, it means the startup was successful &#x1F60A;.

Open another command-line window and log in using the CLI:

```shell
./sylvia-iot-coremgr-cli -f config.json5 login -a admin -p admin
```

You will see the following screen (the content you see may be slightly different):

```
$ ./sylvia-iot-coremgr-cli -f config.json5 login -a admin -p admin
{
  "access_token": "ef9cf7cfc645f9092b9af62666d903c5a8e4579ff6941b479c1d9c9b63b0b634",
  "refresh_token": "265983a08af706fbe2912ff2edb1750311d1b689e4dab3a83c4b494c4cf2d033",
  "token_type": "bearer",
  "expires_in": 3599
}
OK (146 ms)
```

The access token will be automatically saved in the file `$HOME/.sylvia-iot-coremgr-cli.json`. The
CLI will use the content of this file to access the APIs.

You can use `./sylvia-iot-coremgr-cli help` to inquire about the usage of commands.

## Create Resources

For the convenience of using [**mosquitto CLI**](https://mosquitto.org/download/), we create the
following entities:

- A unit with the code **demo**
- An MQTT application with the code **test-app-mqtt**
- An MQTT network with the code **test-net-mqtt**
- A device with the network address **01000461**
- A route to bind the device to the application

During this process, you will need to change the connection password to **password** (the content
you see may be slightly different):

```shell
UNIT_ID=$(./sylvia-iot-coremgr-cli -f config.json5 unit add -c demo -o admin -n 'Demo' | jq -r .unitId)
APP_ID=$(./sylvia-iot-coremgr-cli -f config.json5 application add -c test-app-mqtt -u $UNIT_ID --host 'mqtt://localhost' -n 'TestApp-MQTT' | jq -r .applicationId)
NET_ID=$(./sylvia-iot-coremgr-cli -f config.json5 network add -c test-net-mqtt -u $UNIT_ID --host 'mqtt://localhost' -n 'TestNet-MQTT' | jq -r .networkId)
./sylvia-iot-coremgr-cli -f config.json5 application update -i $APP_ID -p password
./sylvia-iot-coremgr-cli -f config.json5 network update -i $NET_ID -p password
DEV_ID=$(./sylvia-iot-coremgr-cli -f config.json5 device add -u $UNIT_ID --netid $NET_ID -a 01000461 -n 01000461 | jq -r .deviceId)
./sylvia-iot-coremgr-cli -f config.json5 device-route add -d $DEV_ID -a $APP_ID
```

## Upload Device Data

You can install mosquitto CLI with the following command:

```shell
sudo apt -y install mosquitto-clients
```

Open a shell to subscribe to the application topic (format:
`broker.application.[unit-code].[app-code].uldata`):

```shell
mosquitto_sub -u test-app-mqtt -P password -t broker.application.demo.test-app-mqtt.uldata
```

Open another shell to simulate the network system sending device data (topic format:
`broker.network.[unit-code].[net-code].uldata`):

```shell
mosquitto_pub -u test-net-mqtt -P password -t broker.network.demo.test-net-mqtt.uldata -m '{"time":"2023-07-08T06:55:02.000Z","networkAddr":"01000461","data":"74657374"}'
```

At this point, you should see the following screen in the subscribed shell (the content may be
slightly different):

```
$ mosquitto_sub -u test-app-mqtt -P password -t broker.application.demo.test-app-mqtt.uldata
{"dataId":"1688799672075-iJ4YQeQ5Lyv4","time":"2023-07-08T06:55:02.000Z","pub":"2023-07-08T07:01:12.075Z","deviceId":"1688798563252-aWcZVRML","networkId":"1688798370824-RwAbBDFh","networkCode":"test-net-mqtt","networkAddr":"01000461","isPublic":true,"profile":"","data":"74657374"}
```

If you see the data, congratulations! You have completed the basic use of Sylvia-IoT! (Congratulations! Achievement unlocked!
&#x1F606;)
