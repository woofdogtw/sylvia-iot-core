//! Common constants for Sylvia-IoT core modules.

pub struct CacheEngine;
pub struct ContentType;
pub struct DbEngine;
pub struct MqEngine;

impl CacheEngine {
    pub const NONE: &'static str = "none";
    pub const MEMORY: &'static str = "memory";
}

impl ContentType {
    pub const CSV: &'static str = "text/csv";
    pub const JSON: &'static str = "application/json";
}

impl DbEngine {
    pub const MONGODB: &'static str = "mongodb";
    pub const SQLITE: &'static str = "sqlite";
}

impl MqEngine {
    pub const EMQX: &'static str = "emqx";
    pub const RABBITMQ: &'static str = "rabbitmq";
    pub const RUMQTTD: &'static str = "rumqttd";
}
