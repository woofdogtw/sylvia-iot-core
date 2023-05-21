use std::{env, sync::Arc, time::Duration};

use async_trait::async_trait;

use general_mq::{
    connection::{
        Event as ConnEvent, EventHandler as ConnHandler, GmqConnection, Status as ConnStatus,
    },
    queue::{
        Event as QueueEvent, EventHandler as QueueHandler, GmqQueue, Message, Status as QueueStatus,
    },
    AmqpConnection, AmqpConnectionOptions, AmqpQueue, AmqpQueueOptions, MqttConnection,
    MqttConnectionOptions, MqttQueue, MqttQueueOptions,
};

struct TestConnHandler;

struct TestQueueHandler {
    pub name: String,
}

const TEST_BROADCAST: bool = true;
const TEST_RELIABLE: bool = true;

#[async_trait]
impl ConnHandler for TestConnHandler {
    async fn on_event(&self, handler_id: String, _conn: Arc<dyn GmqConnection>, ev: ConnEvent) {
        let event = match ev {
            ConnEvent::Status(status) => match status {
                ConnStatus::Closing => "status: closing".to_string(),
                ConnStatus::Closed => "status: closed".to_string(),
                ConnStatus::Connecting => "status: connecting".to_string(),
                ConnStatus::Connected => "status: connected".to_string(),
                ConnStatus::Disconnected => "status: disconnected".to_string(),
            },
            ConnEvent::Error(e) => format!("error: {:?}", e),
        };
        println!(
            "handler_id: {}, ev: {}",
            handler_id.as_str(),
            event.as_str()
        );
    }
}

#[async_trait]
impl QueueHandler for TestQueueHandler {
    async fn on_event(&self, queue: Arc<dyn GmqQueue>, ev: QueueEvent) {
        let event = match ev {
            QueueEvent::Status(status) => match status {
                QueueStatus::Closing => "status: closing".to_string(),
                QueueStatus::Closed => "status: closed".to_string(),
                QueueStatus::Connecting => "status: connecting".to_string(),
                QueueStatus::Connected => "status: connected".to_string(),
                QueueStatus::Disconnected => "status: disconnected".to_string(),
            },
            QueueEvent::Error(e) => format!("error: {:?}", e),
        };
        println!(
            "name: {}, queue: {}, ev: {}",
            self.name.as_str(),
            queue.name(),
            event.as_str()
        );
    }

    async fn on_message(&self, _queue: Arc<dyn GmqQueue>, msg: Box<dyn Message>) {
        match String::from_utf8(msg.payload().to_vec()) {
            Err(e) => {
                println!(
                    "name {} received bin {:?} with parse error: {}",
                    self.name.as_str(),
                    msg.payload(),
                    e
                );
                match msg.ack().await {
                    Err(e) => println!(
                        "name {} ack {:?} error: {}",
                        self.name.as_str(),
                        msg.payload(),
                        e
                    ),
                    Ok(()) => {
                        println!("name {} ack {:?} ok", self.name.as_str(), msg.payload())
                    }
                }
            }
            Ok(payload) => {
                println!("name {} received {}", self.name.as_str(), payload.as_str());
                match msg.ack().await {
                    Err(e) => println!(
                        "name {} ack {} error: {}",
                        self.name.as_str(),
                        payload.as_str(),
                        e
                    ),
                    Ok(()) => println!("name {} ack {} ok", self.name.as_str(), payload.as_str()),
                }
            }
        };
    }
}

#[tokio::main]
async fn main() {
    let run_mqtt = env::var("RUN_MQTT").is_ok();
    if run_mqtt {
        println!("Run MQTT");
        test_mqtt().await;
    } else {
        println!("Run AMQP");
        test_amqp().await;
    }
}

async fn test_amqp() {
    let opts = AmqpConnectionOptions::default();
    let mut conn = match AmqpConnection::new(opts) {
        Err(e) => {
            println!("new AmqpConnection error: {}", e);
            return;
        }
        Ok(conn) => conn,
    };
    conn.add_handler(Arc::new(TestConnHandler {}));
    conn.add_handler(Arc::new(TestConnHandler {}));

    let opts = AmqpQueueOptions {
        name: "test".to_string(),
        is_recv: false,
        reliable: TEST_RELIABLE,
        broadcast: TEST_BROADCAST,
        reconnect_millis: 1000,
        prefetch: 10,
        ..Default::default()
    };
    let mut send_queue = match AmqpQueue::new(opts, &conn) {
        Err(e) => {
            println!("new AmqpQueue error: {}", e);
            return;
        }
        Ok(queue) => queue,
    };
    send_queue.set_handler(Arc::new(TestQueueHandler {
        name: "send".to_string(),
    }));
    if let Err(e) = send_queue.connect() {
        println!("connect send queue error: {}", e);
        return;
    }

    let opts = AmqpQueueOptions {
        name: "test".to_string(),
        is_recv: true,
        reliable: TEST_RELIABLE,
        broadcast: TEST_BROADCAST,
        reconnect_millis: 1000,
        prefetch: 10,
        ..Default::default()
    };
    let mut recv_queue1 = match AmqpQueue::new(opts.clone(), &conn) {
        Err(e) => {
            println!("new AmqpQueue error: {}", e);
            return;
        }
        Ok(queue) => queue,
    };
    recv_queue1.set_handler(Arc::new(TestQueueHandler {
        name: "recv1".to_string(),
    }));
    if let Err(e) = recv_queue1.connect() {
        println!("connect recv1 queue error: {}", e);
        return;
    }
    let mut recv_queue2 = match AmqpQueue::new(opts, &conn) {
        Err(e) => {
            println!("new AmqpQueue error: {}", e);
            return;
        }
        Ok(queue) => queue,
    };
    recv_queue2.set_handler(Arc::new(TestQueueHandler {
        name: "recv2".to_string(),
    }));
    if let Err(e) = recv_queue2.connect() {
        println!("connect recv2 queue error: {}", e);
        return;
    }

    loop {
        if let Err(e) = conn.connect() {
            println!("connect error: {}", e);
            return;
        }
        let mut count = 10;
        while count > 0 {
            tokio::time::sleep(Duration::from_secs(2)).await;
            let str = format!("count {}", count);
            match send_queue.send_msg(str.as_bytes().to_vec()).await {
                Err(e) => println!("send {} error: {}", str, e),
                Ok(()) => println!("send {} ok", str),
            }
            count = count - 1;
        }
        if let Err(e) = conn.close().await {
            println!("close error: {}", e);
            return;
        }
        tokio::time::sleep(Duration::from_secs(5)).await;
    }
}

async fn test_mqtt() {
    let opts = MqttConnectionOptions::default();
    let mut conn = match MqttConnection::new(opts) {
        Err(e) => {
            println!("new MqttConnection error: {}", e);
            return;
        }
        Ok(conn) => conn,
    };
    conn.add_handler(Arc::new(TestConnHandler {}));
    conn.add_handler(Arc::new(TestConnHandler {}));
    let opts = MqttConnectionOptions::default();
    let mut conn2 = match MqttConnection::new(opts) {
        Err(e) => {
            println!("new MqttConnection error: {}", e);
            return;
        }
        Ok(conn) => conn,
    };
    conn2.add_handler(Arc::new(TestConnHandler {}));

    let opts = MqttQueueOptions {
        name: "test".to_string(),
        is_recv: false,
        reliable: TEST_RELIABLE,
        broadcast: TEST_BROADCAST,
        reconnect_millis: 1000,
        shared_prefix: Some("$share/general-mq/".to_string()),
        ..Default::default()
    };
    let mut send_queue = match MqttQueue::new(opts, &conn) {
        Err(e) => {
            println!("new MqttQueue error: {}", e);
            return;
        }
        Ok(queue) => queue,
    };
    send_queue.set_handler(Arc::new(TestQueueHandler {
        name: "send".to_string(),
    }));
    if let Err(e) = send_queue.connect() {
        println!("connect send queue error: {}", e);
        return;
    }

    let opts = MqttQueueOptions {
        name: "test".to_string(),
        is_recv: true,
        reliable: TEST_RELIABLE,
        broadcast: TEST_BROADCAST,
        reconnect_millis: 1000,
        shared_prefix: Some("$share/general-mq/".to_string()),
        ..Default::default()
    };
    let mut recv_queue1 = match MqttQueue::new(opts.clone(), &conn) {
        Err(e) => {
            println!("new MqttQueue error: {}", e);
            return;
        }
        Ok(queue) => queue,
    };
    recv_queue1.set_handler(Arc::new(TestQueueHandler {
        name: "recv1".to_string(),
    }));
    if let Err(e) = recv_queue1.connect() {
        println!("connect recv1 queue error: {}", e);
        return;
    }
    let mut recv_queue2 = match MqttQueue::new(opts, &conn2) {
        Err(e) => {
            println!("new MqttQueue error: {}", e);
            return;
        }
        Ok(queue) => queue,
    };
    recv_queue2.set_handler(Arc::new(TestQueueHandler {
        name: "recv2".to_string(),
    }));
    if let Err(e) = recv_queue2.connect() {
        println!("connect recv2 queue error: {}", e);
        return;
    }

    loop {
        if let Err(e) = conn.connect() {
            println!("connect error: {}", e);
            return;
        }
        if let Err(e) = conn2.connect() {
            println!("connect 2 error: {}", e);
            return;
        }
        let mut count = 10;
        while count > 0 {
            tokio::time::sleep(Duration::from_secs(2)).await;
            let str = format!("count {}", count);
            match send_queue.send_msg(str.as_bytes().to_vec()).await {
                Err(e) => println!("send {} error: {}", str, e),
                Ok(()) => println!("send {} ok", str),
            }
            count = count - 1;
        }
        if let Err(e) = conn.close().await {
            println!("close error: {}", e);
            return;
        }
        if let Err(e) = conn2.close().await {
            println!("close 2 error: {}", e);
            return;
        }
        tokio::time::sleep(Duration::from_secs(5)).await;
    }
}
