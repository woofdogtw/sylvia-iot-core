# 快速開始

本章節描述在 Ubuntu 22.04 環境的快速安裝步驟。

## 安裝工具

```shell
sudo apt -y install curl jq
```

## 安裝 Docker

這裡參考 [**Docker 官方網站**](https://docs.docker.com/engine/install/ubuntu/) 的安裝步驟。

```shell
sudo apt -y install apt-transport-https ca-certificates curl gnupg lsb-release
curl -fsSL https://download.docker.com/linux/ubuntu/gpg | sudo gpg --dearmor -o /usr/share/keyrings/docker-archive-keyring.gpg
echo "deb [arch=amd64 signed-by=/usr/share/keyrings/docker-archive-keyring.gpg] https://download.docker.com/linux/ubuntu $(lsb_release -cs) stable" | sudo tee /etc/apt/sources.list.d/docker.list > /dev/null
sudo apt update
sudo apt -y install docker-ce docker-ce-cli containerd.io docker-compose-plugin
sudo usermod -aG docker $USER
```

> 記得重啟 shell 套用使用者權限。

## 安裝 MongoDB、RabbitMQ、EMQX

啟動服務（版本和資料保存的資料夾可以視情形調整）：

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

> 這裡只是介紹 EMQX 需要使用的 plugin，下面的展示不會使用。您也可以先不啟動 EMQX。

## 下載 Sylvia-IoT

```shell
ARCH=x86_64 # x86_64 or arm64
curl -LO https://github.com/woofdogtw/sylvia-iot-core/releases/latest/download/sylvia-iot-core-$ARCH.tar.xz
curl -LO https://github.com/woofdogtw/sylvia-iot-core/releases/latest/download/sylvia-iot-coremgr-cli-$ARCH.tar.xz
curl -L -o config.json5 https://github.com/woofdogtw/sylvia-iot-core/raw/main/files/config.json5.example
tar xf sylvia-iot-core-$ARCH.tar.xz
tar xf sylvia-iot-coremgr-cli-$ARCH.tar.xz
```

## 修改 config.json5

為了方便展示，這裡對範例的 config.json5 做了一些修改：

- 由於這裡要展示的是 MongoDB，將所有的 `"engine": "sqlite"` 改成 `"engine": "mongodb"`。
  ```
  "db": {
      "engine": "mongodb",
      ...
  },
  ```
- 先不啟用 HTTPS，將憑證檔設定註解掉：
  ```
  //"cacertFile": "/etc/ssl/certs/ca-certificates.crt",
  //"certFile": "/home/user/rust/conf/certs/sylvia-iot.crt",
  //"keyFile": "/home/user/rust/conf/certs/sylvia-iot.key",
  ```
- 建立一個資料夾作為靜態檔案的存放處，此處的例子是 `/home/user/static`。
  ```
  "staticPath": "/home/user/static",
  ```
- 使用預設的登入頁面樣板，將範例的註解掉：
  ```
  "templates": {      // Jinja2 template paths.
      //"login": "/home/user/rust/static/login.j2",
      //"grant": "/home/user/rust/static/grant.j2",
  },
  ```
- 使用 [**rumqttd**](https://github.com/bytebeamio/rumqtt) 而非 EMQX：
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

## 設定初始資料

先進入 MongoDB shell：

```shell
docker exec -it mongodb mongosh
```

在 MongoDB shell 介面建立基本資料：

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

接著按兩次 `Ctrl+C` 離開。

## 開始使用

啟動 Sylvia-IoT core：

```shell
./sylvia-iot-core -f config.json5
```

如果程式沒有結束，表示已經啟動成功了 &#x1F60A;。

另外開一個命令列視窗，使用 CLI 登入：

```shell
./sylvia-iot-coremgr-cli -f config.json5 login -a admin -p admin
```

將會看到如下的畫面（您看到的內容會有些不同）：

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

Access token 會自動被保留在 `$HOME/.sylvia-iot-coremgr-cli.json` 這個檔案中，CLI 會依據裡面的內容來存取 API。

可以使用 `./sylvia-iot-coremgr-cli help` 查詢指令的使用方式。

## 建立資源

為了方便使用 [**mosquitto CLI**](https://mosquitto.org/download/)，這裡我們分別以下的實體：

- 一個單位，單位代碼是 **demo**
- 一個 MQTT 應用，應用代碼是 **test-app-mqtt**
- 一個 MQTT 網路，網路代碼是 **test-net-mqtt**
- 一個裝置，裝置網路位址為 **01000461**
- 一個路由，將該裝置綁定給應用

過程會需要變更連線密碼為 **password**（您看到的內容會有些不同）：

```shell
UNIT_ID=$(./sylvia-iot-coremgr-cli -f config.json5 unit add -c demo -o admin -n 'Demo' | jq -r .unitId)
APP_ID=$(./sylvia-iot-coremgr-cli -f config.json5 application add -c test-app-mqtt -u $UNIT_ID --host 'mqtt://localhost' -n 'TestApp-MQTT' | jq -r .applicationId)
NET_ID=$(./sylvia-iot-coremgr-cli -f config.json5 network add -c test-net-mqtt -u $UNIT_ID --host 'mqtt://localhost' -n 'TestNet-MQTT' | jq -r .networkId)
./sylvia-iot-coremgr-cli -f config.json5 application update -i $APP_ID -p password
./sylvia-iot-coremgr-cli -f config.json5 network update -i $NET_ID -p password
DEV_ID=$(./sylvia-iot-coremgr-cli -f config.json5 device add -u $UNIT_ID --netid $NET_ID -a 01000461 -n 01000461 | jq -r .deviceId)
./sylvia-iot-coremgr-cli -f config.json5 device-route add -d $DEV_ID -a $APP_ID
```

## 上傳裝置資料

可以用以下指令安裝 mosquitto CLI：

```shell
sudo apt -y install mosquitto-clients
```

開啟一個 shell 訂閱應用主題（格式為 `broker.application.[單位代碼].[應用代碼].uldata`）：

```shell
mosquitto_sub -u test-app-mqtt -P password -t broker.application.demo.test-app-mqtt.uldata
```

開啟另一個 shell 模擬網路系統傳送裝置資料（主題格式為 `broker.network.[單位代碼].[網路代碼].uldata`）：

```shell
mosquitto_pub -u test-net-mqtt -P password -t broker.network.demo.test-net-mqtt.uldata -m '{"time":"2023-07-08T06:55:02.000Z","networkAddr":"01000461","data":"74657374"}'
```

這時您應該會在訂閱的 shell 看到如下畫面（內容可能有些不同）：

```
$ mosquitto_sub -u test-app-mqtt -P password -t broker.application.demo.test-app-mqtt.uldata
{"dataId":"1688799672075-iJ4YQeQ5Lyv4","time":"2023-07-08T06:55:02.000Z","pub":"2023-07-08T07:01:12.075Z","deviceId":"1688798563252-aWcZVRML","networkId":"1688798370824-RwAbBDFh","networkCode":"test-net-mqtt","networkAddr":"01000461","isPublic":true,"profile":"","data":"74657374"}
```

如果有看到資料，恭喜您完成基本的 Sylvia-IoT 的使用了（恭喜你解鎖成就 &#x1F606;）！
