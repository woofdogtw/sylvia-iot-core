use serde::Deserialize;

/// Application configurations.
#[derive(Clone, Deserialize)]
pub struct Config {
    /// **sylvia-iot-auth** API base path with host. For example: `http://localhost:1080/auth`.
    pub auth: String,
    /// **sylvia-router** API base path with host. For example: `http://localhost:1080/router`.
    pub router: String,
    pub wan: Vec<IfaceConfig>,
    pub lan: IfaceConfig,
    pub wlan: Option<IfaceConfig>,
    pub wwan: Option<IfaceConfig>,
}

/// Interface configurations.
#[derive(Clone, Deserialize)]
pub struct IfaceConfig {
    /// NetworkManager interface name.
    pub name: String,
    /// Network interface name in operating system.
    pub ifname: String,
}
