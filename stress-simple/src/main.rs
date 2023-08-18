use std::{
    env, fs,
    io::{Error as IoError, ErrorKind, Result},
    sync::{Arc, Mutex},
    time::Duration,
};

use async_trait::async_trait;
use chrono::{DateTime, SecondsFormat, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use tokio::time;

use general_mq::{
    connection::GmqConnection,
    queue::{GmqQueue, Message, MessageHandler, Status as QueueStatus},
    AmqpConnection, AmqpConnectionOptions, AmqpQueueOptions, MqttConnection, MqttConnectionOptions,
    MqttQueueOptions, Queue, QueueOptions,
};

#[derive(Deserialize)]
struct Config {
    #[serde(rename = "intervalUs")]
    interval_us: u64,
    count: usize,
    #[serde(rename = "netHost")]
    net_host: String,
    #[serde(rename = "netQueue")]
    net_queue: String,
    #[serde(rename = "appHost")]
    app_host: String,
    #[serde(rename = "appQueue")]
    app_queue: String,
    addr: String,
}

#[derive(Serialize)]
struct NetUlData<'a> {
    time: String,
    #[serde(rename = "networkAddr")]
    network_addr: &'a str,
    data: String,
    extension: Map<String, Value>,
}

#[derive(Deserialize)]
struct AppUlData {
    time: String,
}

struct UlDataHandler {
    start_us: i64,
    count: usize,
    received: Arc<Mutex<usize>>,
    total_latency_ms: Arc<Mutex<u64>>, // use Arc<Mutex<>> for interior mutability.
    last_print_sec: Arc<Mutex<i64>>,
    latency_stats: Arc<Mutex<Vec<u32>>>,
}

#[async_trait]
impl MessageHandler for UlDataHandler {
    async fn on_message(&self, _queue: Arc<dyn GmqQueue>, msg: Box<dyn Message>) {
        let now = Utc::now();
        let _ = msg.ack().await;
        let received;
        {
            let mut mutex = self.received.lock().unwrap();
            *mutex += 1;
            received = *mutex;
        }

        let latency = match serde_json::from_slice::<AppUlData>(msg.payload()) {
            Err(e) => {
                println!("not AppUlData: {}", e);
                0
            }
            Ok(data) => match DateTime::parse_from_rfc3339(data.time.as_str()) {
                Err(_) => {
                    println!("`time` error: {}", data.time.as_str());
                    0
                }
                Ok(time) => now.timestamp_millis() - time.timestamp_millis(),
            },
        };

        let total_latency;
        {
            let mut mutex = self.total_latency_ms.lock().unwrap();
            *mutex += latency as u64;
            total_latency = *mutex;
        }

        {
            let mut mutex = self.latency_stats.lock().unwrap();
            mutex.push(latency as u32);
        }

        let elapse_sec = (now.timestamp_micros() - self.start_us) / 1000000;
        let mut should_print = false;
        {
            let mut mutex = self.last_print_sec.lock().unwrap();
            if *mutex < elapse_sec {
                should_print = true;
                *mutex = elapse_sec;
            }
        }
        if should_print || received >= self.count {
            println!(
                "elapsed {} secs, count: {}, avg latency (ms): {}",
                elapse_sec,
                received,
                total_latency / received as u64
            );
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let conf = init_config()?;
    let net_queue = create_net_queue(&conf).await?;
    let (_app_queue, handler) = create_app_queue(&conf).await?;
    run(&conf, net_queue, handler.start_us).await?;

    for _ in 0..1000 {
        let recv_count = { *handler.received.lock().unwrap() };
        if recv_count >= conf.count {
            println!("complete");
            break;
        }
        time::sleep(Duration::from_millis(10)).await;
    }
    let count;
    {
        let mut mutex = handler.latency_stats.lock().unwrap();
        count = mutex.len();
        if count == conf.count {
            (*mutex).sort();

            let index_50 = count / 2;
            let index_80 = count * 4 / 5;
            let index_90 = count * 9 / 10;
            let index_95 = count * 95 / 100;
            let index_98 = count * 98 / 100;
            let index_99 = count * 99 / 100;
            println!("min: {}", (*mutex)[0]);
            println!("50%: {}", (*mutex)[index_50]);
            println!("80%: {}", (*mutex)[index_80]);
            println!("90%: {}", (*mutex)[index_90]);
            println!("95%: {}", (*mutex)[index_95]);
            println!("98%: {}", (*mutex)[index_98]);
            println!("99%: {}", (*mutex)[index_99]);
            println!("max: {}", (*mutex)[count - 1]);
        }
    }
    let total;
    {
        let mutex = handler.total_latency_ms.lock().unwrap();
        total = *mutex;
    }
    if count == conf.count {
        println!("avg: {}", total / count as u64);
    }
    Ok(())
}

fn init_config() -> Result<Config> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        let e = IoError::new(ErrorKind::InvalidInput, "usage: [prog] [config.json]");
        return Err(e);
    }
    let conf = fs::read_to_string(args[1].as_str())?;
    Ok(serde_json::from_str(conf.as_str())?)
}

async fn create_net_queue(config: &Config) -> Result<Queue> {
    let mut queue = if config.net_host.starts_with("amqp") {
        let opts = AmqpConnectionOptions {
            uri: config.net_host.clone(),
            ..Default::default()
        };
        let mut conn = match AmqpConnection::new(opts) {
            Err(e) => return Err(IoError::new(ErrorKind::ConnectionRefused, e)),
            Ok(conn) => conn,
        };
        if let Err(e) = conn.connect() {
            return Err(IoError::new(ErrorKind::ConnectionRefused, e.to_string()));
        }
        let opts = QueueOptions::Amqp(
            AmqpQueueOptions {
                name: config.net_queue.clone(),
                is_recv: false,
                reliable: true,
                broadcast: false,
                ..Default::default()
            },
            &conn,
        );
        match Queue::new(opts) {
            Err(e) => return Err(IoError::new(ErrorKind::ConnectionRefused, e)),
            Ok(queue) => queue,
        }
    } else if config.net_host.starts_with("mqtt") {
        let opts = MqttConnectionOptions {
            uri: config.net_host.clone(),
            ..Default::default()
        };
        let mut conn = match MqttConnection::new(opts) {
            Err(e) => return Err(IoError::new(ErrorKind::ConnectionRefused, e)),
            Ok(conn) => conn,
        };
        if let Err(e) = conn.connect() {
            return Err(IoError::new(ErrorKind::ConnectionRefused, e.to_string()));
        }
        let opts = QueueOptions::Mqtt(
            MqttQueueOptions {
                name: config.net_queue.clone(),
                is_recv: false,
                reliable: true,
                broadcast: false,
                ..Default::default()
            },
            &conn,
        );
        match Queue::new(opts) {
            Err(e) => return Err(IoError::new(ErrorKind::ConnectionRefused, e)),
            Ok(queue) => queue,
        }
    } else {
        return Err(IoError::new(ErrorKind::InvalidInput, "invalid scheme"));
    };

    if let Err(e) = queue.connect() {
        return Err(IoError::new(ErrorKind::ConnectionRefused, e.to_string()));
    }

    for _ in 0..200 {
        if queue.status() == QueueStatus::Connected {
            break;
        }
        time::sleep(Duration::from_millis(10)).await;
    }
    if queue.status() != QueueStatus::Connected {
        return Err(IoError::new(ErrorKind::ConnectionRefused, "not connected"));
    }
    Ok(queue)
}

async fn create_app_queue(config: &Config) -> Result<(Queue, Arc<UlDataHandler>)> {
    let mut queue = if config.app_host.starts_with("amqp") {
        let opts = AmqpConnectionOptions {
            uri: config.app_host.clone(),
            ..Default::default()
        };
        let mut conn = match AmqpConnection::new(opts) {
            Err(e) => return Err(IoError::new(ErrorKind::ConnectionRefused, e)),
            Ok(conn) => conn,
        };
        if let Err(e) = conn.connect() {
            return Err(IoError::new(ErrorKind::ConnectionRefused, e.to_string()));
        }
        let opts = QueueOptions::Amqp(
            AmqpQueueOptions {
                name: config.app_queue.clone(),
                is_recv: true,
                reliable: true,
                broadcast: false,
                prefetch: 100,
                ..Default::default()
            },
            &conn,
        );
        match Queue::new(opts) {
            Err(e) => return Err(IoError::new(ErrorKind::ConnectionRefused, e)),
            Ok(queue) => queue,
        }
    } else if config.app_host.starts_with("mqtt") {
        let opts = MqttConnectionOptions {
            uri: config.app_host.clone(),
            ..Default::default()
        };
        let mut conn = match MqttConnection::new(opts) {
            Err(e) => return Err(IoError::new(ErrorKind::ConnectionRefused, e)),
            Ok(conn) => conn,
        };
        if let Err(e) = conn.connect() {
            return Err(IoError::new(ErrorKind::ConnectionRefused, e.to_string()));
        }
        let opts = QueueOptions::Mqtt(
            MqttQueueOptions {
                name: config.app_queue.clone(),
                is_recv: true,
                reliable: true,
                broadcast: false,
                ..Default::default()
            },
            &conn,
        );
        match Queue::new(opts) {
            Err(e) => return Err(IoError::new(ErrorKind::ConnectionRefused, e)),
            Ok(queue) => queue,
        }
    } else {
        return Err(IoError::new(ErrorKind::InvalidInput, "invalid scheme"));
    };

    let handler = Arc::new(UlDataHandler {
        start_us: Utc::now().timestamp_micros(),
        count: config.count,
        received: Arc::new(Mutex::new(0)),
        total_latency_ms: Arc::new(Mutex::new(0)),
        last_print_sec: Arc::new(Mutex::new(0)),
        latency_stats: Arc::new(Mutex::new(Vec::with_capacity(config.count as usize))),
    });
    queue.set_msg_handler(handler.clone());
    if let Err(e) = queue.connect() {
        return Err(IoError::new(ErrorKind::ConnectionRefused, e.to_string()));
    }

    for _ in 0..200 {
        if queue.status() == QueueStatus::Connected {
            break;
        }
        time::sleep(Duration::from_millis(10)).await;
    }
    if queue.status() != QueueStatus::Connected {
        return Err(IoError::new(ErrorKind::ConnectionRefused, "not connected"));
    }
    Ok((queue, handler))
}

async fn run(config: &Config, queue: Queue, start_us: i64) -> Result<()> {
    let dbg_exceed = std::env::var("DEBUG_EXCEED").is_ok();
    for i in 0..config.count {
        let payload = serde_json::to_vec(&NetUlData {
            time: Utc::now().to_rfc3339_opts(SecondsFormat::Millis, true),
            network_addr: config.addr.as_str(),
            data: format!("{:016x}", i & 0xffff_ffff_ffff_ffff),
            extension: Map::new(),
        })?;
        if let Err(e) = queue.send_msg(payload).await {
            return Err(IoError::new(ErrorKind::ConnectionRefused, e.to_string()));
        }
        let diff = (Utc::now().timestamp_micros() - start_us) as u64;
        let next_time = config.interval_us * (i as u64 + 1);
        if diff > next_time {
            if dbg_exceed {
                println!("exceed interval");
            }
        } else {
            time::sleep(Duration::from_micros(next_time - diff)).await;
        }
    }
    Ok(())
}
