use std::error::Error as StdError;

use clap::{
    builder::{BoolValueParser, RangedU64ValueParser},
    Arg, ArgMatches, Command,
};
use reqwest::{header, Client, Method, StatusCode};
use serde::{Deserialize, Serialize};

use sylvia_iot_coremgr_cli::libs::cli::{auth, config, Config as CoremgrCliConfig};
use sylvia_iot_sdk::util::err::ErrResp;

use super::{super::config::Config, API_RETRY};

#[derive(Serialize)]
struct PutWanReq {
    data: WanConf,
}

#[derive(Serialize)]
struct PutLanReq {
    data: PutLanReqData,
}

#[derive(Serialize)]
struct PutLanReqData {
    conf4: LanConf4,
}

#[derive(Serialize)]
struct PutWlanReq {
    data: PutWlanReqData,
}

#[derive(Serialize)]
struct PutWlanReqData {
    enable: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    conf: Option<WlanConf>,
}

#[derive(Serialize)]
struct PutWwanReq {
    data: PutWwanReqData,
}

#[derive(Serialize)]
struct PutWwanReqData {
    enable: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    conf: Option<WwanConf>,
}

#[derive(Deserialize)]
struct GetWanRes {
    data: Vec<GetWanResData>,
}

#[derive(Deserialize, Serialize)]
struct GetWanResData {
    #[serde(rename = "wanId")]
    wan_id: String,
    conf: WanConf,
    conn4: WanConn4,
}

#[derive(Deserialize)]
struct GetLanRes {
    data: GetLanResData,
}

#[derive(Deserialize, Serialize)]
struct GetLanResData {
    conf4: LanConf4,
}

#[derive(Deserialize)]
struct GetLanLeasesRes {
    data: Vec<GetLanLeasesResData>,
}

#[derive(Deserialize, Serialize)]
struct GetLanLeasesResData {
    ip: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    starts: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    ends: Option<String>,
    #[serde(rename = "macAddr", skip_serializing_if = "Option::is_none")]
    mac_addr: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    client: Option<String>,
}

#[derive(Deserialize)]
struct GetWlanRes {
    data: GetWlanResData,
}

#[derive(Deserialize, Serialize)]
struct GetWlanResData {
    enable: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    conf: Option<WlanConf>,
}

#[derive(Deserialize)]
struct GetWwanRes {
    data: GetWwanResData,
}

#[derive(Deserialize, Serialize)]
struct GetWwanResData {
    enable: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    conf: Option<WwanConf>,
    #[serde(skip_serializing_if = "Option::is_none")]
    conn4: Option<WanConn4>,
}

#[derive(Deserialize)]
struct GetWwanListRes {
    data: Vec<WifiApInfo>,
}

#[derive(Deserialize, Serialize)]
struct WanConf {
    #[serde(rename = "type")]
    conf_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    type4: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    static4: Option<Static4>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pppoe: Option<Pppoe>,
}

#[derive(Deserialize, Serialize)]
struct WanConn4 {
    address: String,
    gateway: String,
    dns: Vec<String>,
}

#[derive(Deserialize, Serialize)]
struct LanConf4 {
    address: String,
    #[serde(rename = "dhcpStart")]
    dhcp_start: String,
    #[serde(rename = "dhcpEnd")]
    dhcp_end: String,
    #[serde(rename = "leaseTime")]
    lease_time: u64,
}

#[derive(Deserialize, Serialize)]
struct WlanConf {
    ssid: String,
    channel: u64,
    password: String,
}

#[derive(Deserialize, Serialize)]
struct WwanConf {
    ssid: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    password: Option<String>,
}

#[derive(Deserialize, Serialize)]
struct Static4 {
    address: String,
    gateway: String,
    dns: Vec<String>,
}

#[derive(Deserialize, Serialize)]
struct Pppoe {
    username: String,
    password: String,
}

#[derive(Deserialize, Serialize)]
pub struct WifiApInfo {
    pub ssid: String,
    pub security: Vec<String>,
    pub channel: usize,
    pub signal: isize,
}

pub fn reg_args(cmd: Command) -> Command {
    cmd.about("Network management")
        .subcommand(Command::new("wan.get").about("Get WAN configurations"))
        .subcommand(
            Command::new("wan.set")
                .about("Set WAN configurations")
                .arg(
                    Arg::new("wanid")
                        .short('i')
                        .long("wanid")
                        .help("WAN interface ID")
                        .num_args(1)
                        .required(true),
                )
                .arg(
                    Arg::new("type")
                        .long("type")
                        .help("Connection type")
                        .num_args(1)
                        .required(true)
                        .value_parser(["disable", "ethernet", "pppoe"]),
                )
                .arg(
                    Arg::new("ip4type")
                        .long("ip4type")
                        .help("IPv4 type")
                        .num_args(1)
                        .required_if_eq("type", "ethernet")
                        .value_parser(["dhcp", "static"]),
                )
                .arg(
                    Arg::new("ip4addr")
                        .long("ip4addr")
                        .help("IPv4 address CIDR (ex: 1.2.3.4/24)")
                        .num_args(1)
                        .required_if_eq("ip4type", "static"),
                )
                .arg(
                    Arg::new("ip4gw")
                        .long("ip4gw")
                        .help("IPv4 gateway address")
                        .num_args(1)
                        .required_if_eq("ip4type", "static"),
                )
                .arg(
                    Arg::new("ip4dns")
                        .long("ip4dns")
                        .help("IPv4 DNS server addresses")
                        .num_args(0..)
                        .required_if_eq("ip4type", "static"),
                )
                .arg(
                    Arg::new("username")
                        .long("username")
                        .help("PPPoE user name")
                        .num_args(1)
                        .required_if_eq("type", "pppoe"),
                )
                .arg(
                    Arg::new("password")
                        .long("password")
                        .help("PPPoE password")
                        .num_args(1)
                        .required_if_eq("type", "pppoe"),
                ),
        )
        .subcommand(Command::new("lan.get").about("Get LAN configurations"))
        .subcommand(
            Command::new("lan.set")
                .about("Set LAN configurations")
                .arg(
                    Arg::new("ip4addr")
                        .long("ip4addr")
                        .help("IPv4 address CIDR (ex: 1.2.3.4/24)")
                        .num_args(1)
                        .required(true),
                )
                .arg(
                    Arg::new("ip4start")
                        .long("ip4start")
                        .help("IPv4 DHCP start address")
                        .num_args(1)
                        .required(true),
                )
                .arg(
                    Arg::new("ip4end")
                        .long("ip4end")
                        .help("IPv4 DHCP end address")
                        .num_args(1)
                        .required(true),
                )
                .arg(
                    Arg::new("ip4lease")
                        .long("ip4lease")
                        .help("IPv4 DHCP lease time in seconds(60 to 604800)")
                        .num_args(1)
                        .required(true)
                        .value_parser(RangedU64ValueParser::<u64>::new().range(60..604801)),
                ),
        )
        .subcommand(Command::new("lan.leases").about("Get DHCP leases"))
        .subcommand(Command::new("wlan.get").about("Get wireless LAN configurations"))
        .subcommand(
            Command::new("wlan.set")
                .about("Set wireless LAN configurations")
                .arg(
                    Arg::new("ssid")
                        .long("ssid")
                        .help("SSID")
                        .num_args(1)
                        .required(true),
                )
                .arg(
                    Arg::new("channel")
                        .long("channel")
                        .help("Channel (1 to 11)")
                        .num_args(1)
                        .required(true)
                        .value_parser(RangedU64ValueParser::<u64>::new().range(1..12)),
                )
                .arg(
                    Arg::new("password")
                        .long("password")
                        .help("Password")
                        .num_args(1)
                        .required(true),
                ),
        )
        .subcommand(Command::new("wlan.disable").about("Disable wireless LAN configurations"))
        .subcommand(Command::new("wwan.get").about("Get wireless WAN configurations"))
        .subcommand(
            Command::new("wwan.set")
                .about("Set wireless WAN configurations")
                .arg(
                    Arg::new("ssid")
                        .long("ssid")
                        .help("SSID")
                        .num_args(1)
                        .required(true),
                )
                .arg(
                    Arg::new("password")
                        .long("password")
                        .help("Password")
                        .num_args(1),
                ),
        )
        .subcommand(Command::new("wwan.disable").about("Disable wireless WAN configurations"))
        .subcommand(
            Command::new("wwan.list")
                .about("List available wireless AP for WWAN")
                .arg(
                    Arg::new("rescan")
                        .long("rescan")
                        .help("rescan")
                        .num_args(1)
                        .value_parser(BoolValueParser::new()),
                ),
        )
}

pub async fn run(
    conf: &Config,
    cm_conf: &CoremgrCliConfig,
    args: &ArgMatches,
) -> Result<Option<()>, Box<dyn StdError>> {
    match args.subcommand() {
        Some(("wan.get", _args)) => {
            let data = wan_get(conf, cm_conf).await?;
            println!("{}", serde_json::to_string_pretty(&data)?);
            Ok(Some(()))
        }
        Some(("wan.set", args)) => {
            wan_set(conf, cm_conf, args).await?;
            Ok(Some(()))
        }
        Some(("lan.get", _args)) => {
            let data = lan_get(conf, cm_conf).await?;
            println!("{}", serde_json::to_string_pretty(&data)?);
            Ok(Some(()))
        }
        Some(("lan.set", args)) => {
            lan_set(conf, cm_conf, args).await?;
            Ok(Some(()))
        }
        Some(("lan.leases", _args)) => {
            let data = lan_leases_get(conf, cm_conf).await?;
            println!("{}", serde_json::to_string_pretty(&data)?);
            Ok(Some(()))
        }
        Some(("wlan.get", _args)) => {
            let data = wlan_get(conf, cm_conf).await?;
            println!("{}", serde_json::to_string_pretty(&data)?);
            Ok(Some(()))
        }
        Some(("wlan.set", args)) => {
            wlan_set(conf, cm_conf, args).await?;
            Ok(Some(()))
        }
        Some(("wlan.disable", _args)) => {
            wlan_disable(conf, cm_conf).await?;
            Ok(Some(()))
        }
        Some(("wwan.get", _args)) => {
            let data = wwan_get(conf, cm_conf).await?;
            println!("{}", serde_json::to_string_pretty(&data)?);
            Ok(Some(()))
        }
        Some(("wwan.set", args)) => {
            wwan_set(conf, cm_conf, args).await?;
            Ok(Some(()))
        }
        Some(("wwan.disable", _args)) => {
            wwan_disable(conf, cm_conf).await?;
            Ok(Some(()))
        }
        Some(("wwan.list", args)) => {
            let data = wwan_list_get(conf, cm_conf, args).await?;
            println!("{}", serde_json::to_string_pretty(&data)?);
            Ok(Some(()))
        }
        _ => Ok(None),
    }
}

async fn wan_get(
    config: &Config,
    cm_conf: &CoremgrCliConfig,
) -> Result<Vec<GetWanResData>, ErrResp> {
    let storage = match config::read_storage() {
        Err(e) => {
            let msg = format!("[storage] read storage error: {}", e);
            return Err(ErrResp::ErrRsc(Some(msg)));
        }
        Ok(storage) => storage,
    };

    let client = Client::new();
    let uri = format!("{}/api/v1/net/wan", config.router);
    let mut token = storage.access_token;

    let mut retry = 0;
    loop {
        let req = match client
            .request(Method::GET, uri.as_str())
            .header(header::AUTHORIZATION, format!("Bearer {}", token))
            .build()
        {
            Err(e) => {
                let msg = format!("[API] create request error: {}", e);
                return Err(ErrResp::ErrRsc(Some(msg)));
            }
            Ok(req) => req,
        };
        let resp = match client.execute(req).await {
            Err(e) => {
                let msg = format!("[API] execute request error: {}", e);
                return Err(ErrResp::ErrRsc(Some(msg)));
            }
            Ok(resp) => resp,
        };
        let status_code = resp.status();
        let body = match resp.bytes().await {
            Err(e) => format!("(wrong body: {})", e),
            Ok(bytes) => match String::from_utf8(bytes.to_vec()) {
                Err(e) => format!("(body not UTF-8: {})", e),
                Ok(body) => body,
            },
        };
        if status_code != StatusCode::OK {
            retry += 1;
            if retry <= API_RETRY && status_code == StatusCode::UNAUTHORIZED {
                token = auth::refresh(cm_conf).await?.access_token;
                continue;
            }
            let msg = format!("[API] unexpected status: {}, body: {}", status_code, body);
            return Err(ErrResp::ErrIntMsg(Some(msg)));
        }
        match serde_json::from_str::<GetWanRes>(body.as_str()) {
            Err(e) => {
                let msg = format!("[API] unexpected token error: {}, body: {}", e, body);
                return Err(ErrResp::ErrIntMsg(Some(msg)));
            }
            Ok(res) => return Ok(res.data),
        }
    }
}

async fn wan_set(
    config: &Config,
    cm_conf: &CoremgrCliConfig,
    args: &ArgMatches,
) -> Result<(), ErrResp> {
    let storage = match config::read_storage() {
        Err(e) => {
            let msg = format!("[storage] read storage error: {}", e);
            return Err(ErrResp::ErrRsc(Some(msg)));
        }
        Ok(storage) => storage,
    };

    let wan_id = args.get_one::<String>("wanid").unwrap().clone();
    let body = PutWanReq {
        data: WanConf {
            conf_type: args.get_one::<String>("type").unwrap().clone(),
            type4: match args.get_one::<String>("ip4type") {
                None => None,
                Some(t) => Some(t.clone()),
            },
            static4: match args.get_one::<String>("ip4type") {
                None => None,
                Some(t) => match t.as_str() {
                    "static" => Some(Static4 {
                        address: args.get_one::<String>("ip4addr").unwrap().clone(),
                        gateway: args.get_one::<String>("ip4gw").unwrap().clone(),
                        dns: args
                            .get_many::<String>("ip4dns")
                            .unwrap()
                            .map(|x| x.clone())
                            .collect(),
                    }),
                    _ => None,
                },
            },
            pppoe: match args.get_one::<String>("type").unwrap().as_str() {
                "pppoe" => Some(Pppoe {
                    username: args.get_one::<String>("username").unwrap().clone(),
                    password: args.get_one::<String>("password").unwrap().clone(),
                }),
                _ => None,
            },
        },
    };
    let client = Client::new();
    let uri = format!("{}/api/v1/net/wan/{}", config.router, wan_id);
    let mut token = storage.access_token;

    let mut retry = 0;
    loop {
        let req = match client
            .request(Method::PUT, uri.as_str())
            .header(header::AUTHORIZATION, format!("Bearer {}", token))
            .json(&body)
            .build()
        {
            Err(e) => {
                let msg = format!("[API] create request error: {}", e);
                return Err(ErrResp::ErrRsc(Some(msg)));
            }
            Ok(req) => req,
        };
        let resp = match client.execute(req).await {
            Err(e) => {
                let msg = format!("[API] execute request error: {}", e);
                return Err(ErrResp::ErrRsc(Some(msg)));
            }
            Ok(resp) => resp,
        };
        let status_code = resp.status();
        let body = match resp.bytes().await {
            Err(e) => format!("(wrong body: {})", e),
            Ok(bytes) => match String::from_utf8(bytes.to_vec()) {
                Err(e) => format!("(body not UTF-8: {})", e),
                Ok(body) => body,
            },
        };
        if status_code != StatusCode::NO_CONTENT {
            retry += 1;
            if retry <= API_RETRY && status_code == StatusCode::UNAUTHORIZED {
                token = auth::refresh(cm_conf).await?.access_token;
                continue;
            }
            let msg = format!("[API] unexpected status: {}, body: {}", status_code, body);
            return Err(ErrResp::ErrIntMsg(Some(msg)));
        }
        return Ok(());
    }
}

async fn lan_get(config: &Config, cm_conf: &CoremgrCliConfig) -> Result<GetLanResData, ErrResp> {
    let storage = match config::read_storage() {
        Err(e) => {
            let msg = format!("[storage] read storage error: {}", e);
            return Err(ErrResp::ErrRsc(Some(msg)));
        }
        Ok(storage) => storage,
    };

    let client = Client::new();
    let uri = format!("{}/api/v1/net/lan", config.router);
    let mut token = storage.access_token;

    let mut retry = 0;
    loop {
        let req = match client
            .request(Method::GET, uri.as_str())
            .header(header::AUTHORIZATION, format!("Bearer {}", token))
            .build()
        {
            Err(e) => {
                let msg = format!("[API] create request error: {}", e);
                return Err(ErrResp::ErrRsc(Some(msg)));
            }
            Ok(req) => req,
        };
        let resp = match client.execute(req).await {
            Err(e) => {
                let msg = format!("[API] execute request error: {}", e);
                return Err(ErrResp::ErrRsc(Some(msg)));
            }
            Ok(resp) => resp,
        };
        let status_code = resp.status();
        let body = match resp.bytes().await {
            Err(e) => format!("(wrong body: {})", e),
            Ok(bytes) => match String::from_utf8(bytes.to_vec()) {
                Err(e) => format!("(body not UTF-8: {})", e),
                Ok(body) => body,
            },
        };
        if status_code != StatusCode::OK {
            retry += 1;
            if retry <= API_RETRY && status_code == StatusCode::UNAUTHORIZED {
                token = auth::refresh(cm_conf).await?.access_token;
                continue;
            }
            let msg = format!("[API] unexpected status: {}, body: {}", status_code, body);
            return Err(ErrResp::ErrIntMsg(Some(msg)));
        }
        match serde_json::from_str::<GetLanRes>(body.as_str()) {
            Err(e) => {
                let msg = format!("[API] unexpected token error: {}, body: {}", e, body);
                return Err(ErrResp::ErrIntMsg(Some(msg)));
            }
            Ok(res) => return Ok(res.data),
        }
    }
}

async fn lan_set(
    config: &Config,
    cm_conf: &CoremgrCliConfig,
    args: &ArgMatches,
) -> Result<(), ErrResp> {
    let storage = match config::read_storage() {
        Err(e) => {
            let msg = format!("[storage] read storage error: {}", e);
            return Err(ErrResp::ErrRsc(Some(msg)));
        }
        Ok(storage) => storage,
    };

    let body = PutLanReq {
        data: PutLanReqData {
            conf4: LanConf4 {
                address: args.get_one::<String>("ip4addr").unwrap().clone(),
                dhcp_start: args.get_one::<String>("ip4start").unwrap().clone(),
                dhcp_end: args.get_one::<String>("ip4end").unwrap().clone(),
                lease_time: *args.get_one::<u64>("ip4lease").unwrap(),
            },
        },
    };
    let client = Client::new();
    let uri = format!("{}/api/v1/net/lan", config.router);
    let mut token = storage.access_token;

    let mut retry = 0;
    loop {
        let req = match client
            .request(Method::PUT, uri.as_str())
            .header(header::AUTHORIZATION, format!("Bearer {}", token))
            .json(&body)
            .build()
        {
            Err(e) => {
                let msg = format!("[API] create request error: {}", e);
                return Err(ErrResp::ErrRsc(Some(msg)));
            }
            Ok(req) => req,
        };
        let resp = match client.execute(req).await {
            Err(e) => {
                let msg = format!("[API] execute request error: {}", e);
                return Err(ErrResp::ErrRsc(Some(msg)));
            }
            Ok(resp) => resp,
        };
        let status_code = resp.status();
        let body = match resp.bytes().await {
            Err(e) => format!("(wrong body: {})", e),
            Ok(bytes) => match String::from_utf8(bytes.to_vec()) {
                Err(e) => format!("(body not UTF-8: {})", e),
                Ok(body) => body,
            },
        };
        if status_code != StatusCode::NO_CONTENT {
            retry += 1;
            if retry <= API_RETRY && status_code == StatusCode::UNAUTHORIZED {
                token = auth::refresh(cm_conf).await?.access_token;
                continue;
            }
            let msg = format!("[API] unexpected status: {}, body: {}", status_code, body);
            return Err(ErrResp::ErrIntMsg(Some(msg)));
        }
        return Ok(());
    }
}

async fn lan_leases_get(
    config: &Config,
    cm_conf: &CoremgrCliConfig,
) -> Result<Vec<GetLanLeasesResData>, ErrResp> {
    let storage = match config::read_storage() {
        Err(e) => {
            let msg = format!("[storage] read storage error: {}", e);
            return Err(ErrResp::ErrRsc(Some(msg)));
        }
        Ok(storage) => storage,
    };

    let client = Client::new();
    let uri = format!("{}/api/v1/net/lan/leases", config.router);
    let mut token = storage.access_token;

    let mut retry = 0;
    loop {
        let req = match client
            .request(Method::GET, uri.as_str())
            .header(header::AUTHORIZATION, format!("Bearer {}", token))
            .build()
        {
            Err(e) => {
                let msg = format!("[API] create request error: {}", e);
                return Err(ErrResp::ErrRsc(Some(msg)));
            }
            Ok(req) => req,
        };
        let resp = match client.execute(req).await {
            Err(e) => {
                let msg = format!("[API] execute request error: {}", e);
                return Err(ErrResp::ErrRsc(Some(msg)));
            }
            Ok(resp) => resp,
        };
        let status_code = resp.status();
        let body = match resp.bytes().await {
            Err(e) => format!("(wrong body: {})", e),
            Ok(bytes) => match String::from_utf8(bytes.to_vec()) {
                Err(e) => format!("(body not UTF-8: {})", e),
                Ok(body) => body,
            },
        };
        if status_code != StatusCode::OK {
            retry += 1;
            if retry <= API_RETRY && status_code == StatusCode::UNAUTHORIZED {
                token = auth::refresh(cm_conf).await?.access_token;
                continue;
            }
            let msg = format!("[API] unexpected status: {}, body: {}", status_code, body);
            return Err(ErrResp::ErrIntMsg(Some(msg)));
        }
        match serde_json::from_str::<GetLanLeasesRes>(body.as_str()) {
            Err(e) => {
                let msg = format!("[API] unexpected token error: {}, body: {}", e, body);
                return Err(ErrResp::ErrIntMsg(Some(msg)));
            }
            Ok(res) => return Ok(res.data),
        }
    }
}

async fn wlan_get(config: &Config, cm_conf: &CoremgrCliConfig) -> Result<GetWlanResData, ErrResp> {
    let storage = match config::read_storage() {
        Err(e) => {
            let msg = format!("[storage] read storage error: {}", e);
            return Err(ErrResp::ErrRsc(Some(msg)));
        }
        Ok(storage) => storage,
    };

    let client = Client::new();
    let uri = format!("{}/api/v1/net/wlan", config.router);
    let mut token = storage.access_token;

    let mut retry = 0;
    loop {
        let req = match client
            .request(Method::GET, uri.as_str())
            .header(header::AUTHORIZATION, format!("Bearer {}", token))
            .build()
        {
            Err(e) => {
                let msg = format!("[API] create request error: {}", e);
                return Err(ErrResp::ErrRsc(Some(msg)));
            }
            Ok(req) => req,
        };
        let resp = match client.execute(req).await {
            Err(e) => {
                let msg = format!("[API] execute request error: {}", e);
                return Err(ErrResp::ErrRsc(Some(msg)));
            }
            Ok(resp) => resp,
        };
        let status_code = resp.status();
        let body = match resp.bytes().await {
            Err(e) => format!("(wrong body: {})", e),
            Ok(bytes) => match String::from_utf8(bytes.to_vec()) {
                Err(e) => format!("(body not UTF-8: {})", e),
                Ok(body) => body,
            },
        };
        if status_code != StatusCode::OK {
            retry += 1;
            if retry <= API_RETRY && status_code == StatusCode::UNAUTHORIZED {
                token = auth::refresh(cm_conf).await?.access_token;
                continue;
            }
            let msg = format!("[API] unexpected status: {}, body: {}", status_code, body);
            return Err(ErrResp::ErrIntMsg(Some(msg)));
        }
        match serde_json::from_str::<GetWlanRes>(body.as_str()) {
            Err(e) => {
                let msg = format!("[API] unexpected token error: {}, body: {}", e, body);
                return Err(ErrResp::ErrIntMsg(Some(msg)));
            }
            Ok(res) => return Ok(res.data),
        }
    }
}

async fn wlan_set(
    config: &Config,
    cm_conf: &CoremgrCliConfig,
    args: &ArgMatches,
) -> Result<(), ErrResp> {
    let storage = match config::read_storage() {
        Err(e) => {
            let msg = format!("[storage] read storage error: {}", e);
            return Err(ErrResp::ErrRsc(Some(msg)));
        }
        Ok(storage) => storage,
    };

    let body = PutWlanReq {
        data: PutWlanReqData {
            enable: true,
            conf: Some(WlanConf {
                ssid: args.get_one::<String>("ssid").unwrap().clone(),
                channel: *args.get_one::<u64>("channel").unwrap(),
                password: args.get_one::<String>("password").unwrap().clone(),
            }),
        },
    };
    let client = Client::new();
    let uri = format!("{}/api/v1/net/wlan", config.router);
    let mut token = storage.access_token;

    let mut retry = 0;
    loop {
        let req = match client
            .request(Method::PUT, uri.as_str())
            .header(header::AUTHORIZATION, format!("Bearer {}", token))
            .json(&body)
            .build()
        {
            Err(e) => {
                let msg = format!("[API] create request error: {}", e);
                return Err(ErrResp::ErrRsc(Some(msg)));
            }
            Ok(req) => req,
        };
        let resp = match client.execute(req).await {
            Err(e) => {
                let msg = format!("[API] execute request error: {}", e);
                return Err(ErrResp::ErrRsc(Some(msg)));
            }
            Ok(resp) => resp,
        };
        let status_code = resp.status();
        let body = match resp.bytes().await {
            Err(e) => format!("(wrong body: {})", e),
            Ok(bytes) => match String::from_utf8(bytes.to_vec()) {
                Err(e) => format!("(body not UTF-8: {})", e),
                Ok(body) => body,
            },
        };
        if status_code != StatusCode::NO_CONTENT {
            retry += 1;
            if retry <= API_RETRY && status_code == StatusCode::UNAUTHORIZED {
                token = auth::refresh(cm_conf).await?.access_token;
                continue;
            }
            let msg = format!("[API] unexpected status: {}, body: {}", status_code, body);
            return Err(ErrResp::ErrIntMsg(Some(msg)));
        }
        return Ok(());
    }
}

async fn wlan_disable(config: &Config, cm_conf: &CoremgrCliConfig) -> Result<(), ErrResp> {
    let storage = match config::read_storage() {
        Err(e) => {
            let msg = format!("[storage] read storage error: {}", e);
            return Err(ErrResp::ErrRsc(Some(msg)));
        }
        Ok(storage) => storage,
    };

    let body = PutWlanReq {
        data: PutWlanReqData {
            enable: false,
            conf: None,
        },
    };
    let client = Client::new();
    let uri = format!("{}/api/v1/net/wlan", config.router);
    let mut token = storage.access_token;

    let mut retry = 0;
    loop {
        let req = match client
            .request(Method::PUT, uri.as_str())
            .header(header::AUTHORIZATION, format!("Bearer {}", token))
            .json(&body)
            .build()
        {
            Err(e) => {
                let msg = format!("[API] create request error: {}", e);
                return Err(ErrResp::ErrRsc(Some(msg)));
            }
            Ok(req) => req,
        };
        let resp = match client.execute(req).await {
            Err(e) => {
                let msg = format!("[API] execute request error: {}", e);
                return Err(ErrResp::ErrRsc(Some(msg)));
            }
            Ok(resp) => resp,
        };
        let status_code = resp.status();
        let body = match resp.bytes().await {
            Err(e) => format!("(wrong body: {})", e),
            Ok(bytes) => match String::from_utf8(bytes.to_vec()) {
                Err(e) => format!("(body not UTF-8: {})", e),
                Ok(body) => body,
            },
        };
        if status_code != StatusCode::NO_CONTENT {
            retry += 1;
            if retry <= API_RETRY && status_code == StatusCode::UNAUTHORIZED {
                token = auth::refresh(cm_conf).await?.access_token;
                continue;
            }
            let msg = format!("[API] unexpected status: {}, body: {}", status_code, body);
            return Err(ErrResp::ErrIntMsg(Some(msg)));
        }
        return Ok(());
    }
}

async fn wwan_get(config: &Config, cm_conf: &CoremgrCliConfig) -> Result<GetWwanResData, ErrResp> {
    let storage = match config::read_storage() {
        Err(e) => {
            let msg = format!("[storage] read storage error: {}", e);
            return Err(ErrResp::ErrRsc(Some(msg)));
        }
        Ok(storage) => storage,
    };

    let client = Client::new();
    let uri = format!("{}/api/v1/net/wwan", config.router);
    let mut token = storage.access_token;

    let mut retry = 0;
    loop {
        let req = match client
            .request(Method::GET, uri.as_str())
            .header(header::AUTHORIZATION, format!("Bearer {}", token))
            .build()
        {
            Err(e) => {
                let msg = format!("[API] create request error: {}", e);
                return Err(ErrResp::ErrRsc(Some(msg)));
            }
            Ok(req) => req,
        };
        let resp = match client.execute(req).await {
            Err(e) => {
                let msg = format!("[API] execute request error: {}", e);
                return Err(ErrResp::ErrRsc(Some(msg)));
            }
            Ok(resp) => resp,
        };
        let status_code = resp.status();
        let body = match resp.bytes().await {
            Err(e) => format!("(wrong body: {})", e),
            Ok(bytes) => match String::from_utf8(bytes.to_vec()) {
                Err(e) => format!("(body not UTF-8: {})", e),
                Ok(body) => body,
            },
        };
        if status_code != StatusCode::OK {
            retry += 1;
            if retry <= API_RETRY && status_code == StatusCode::UNAUTHORIZED {
                token = auth::refresh(cm_conf).await?.access_token;
                continue;
            }
            let msg = format!("[API] unexpected status: {}, body: {}", status_code, body);
            return Err(ErrResp::ErrIntMsg(Some(msg)));
        }
        match serde_json::from_str::<GetWwanRes>(body.as_str()) {
            Err(e) => {
                let msg = format!("[API] unexpected token error: {}, body: {}", e, body);
                return Err(ErrResp::ErrIntMsg(Some(msg)));
            }
            Ok(res) => return Ok(res.data),
        }
    }
}

async fn wwan_set(
    config: &Config,
    cm_conf: &CoremgrCliConfig,
    args: &ArgMatches,
) -> Result<(), ErrResp> {
    let storage = match config::read_storage() {
        Err(e) => {
            let msg = format!("[storage] read storage error: {}", e);
            return Err(ErrResp::ErrRsc(Some(msg)));
        }
        Ok(storage) => storage,
    };

    let body = PutWwanReq {
        data: PutWwanReqData {
            enable: true,
            conf: Some(WwanConf {
                ssid: args.get_one::<String>("ssid").unwrap().clone(),
                password: match args.get_one::<String>("password").take() {
                    None => None,
                    Some(v) => Some(v.clone()),
                },
            }),
        },
    };
    let client = Client::new();
    let uri = format!("{}/api/v1/net/wwan", config.router);
    let mut token = storage.access_token;

    let mut retry = 0;
    loop {
        let req = match client
            .request(Method::PUT, uri.as_str())
            .header(header::AUTHORIZATION, format!("Bearer {}", token))
            .json(&body)
            .build()
        {
            Err(e) => {
                let msg = format!("[API] create request error: {}", e);
                return Err(ErrResp::ErrRsc(Some(msg)));
            }
            Ok(req) => req,
        };
        let resp = match client.execute(req).await {
            Err(e) => {
                let msg = format!("[API] execute request error: {}", e);
                return Err(ErrResp::ErrRsc(Some(msg)));
            }
            Ok(resp) => resp,
        };
        let status_code = resp.status();
        let body = match resp.bytes().await {
            Err(e) => format!("(wrong body: {})", e),
            Ok(bytes) => match String::from_utf8(bytes.to_vec()) {
                Err(e) => format!("(body not UTF-8: {})", e),
                Ok(body) => body,
            },
        };
        if status_code != StatusCode::NO_CONTENT {
            retry += 1;
            if retry <= API_RETRY && status_code == StatusCode::UNAUTHORIZED {
                token = auth::refresh(cm_conf).await?.access_token;
                continue;
            }
            let msg = format!("[API] unexpected status: {}, body: {}", status_code, body);
            return Err(ErrResp::ErrIntMsg(Some(msg)));
        }
        return Ok(());
    }
}

async fn wwan_disable(config: &Config, cm_conf: &CoremgrCliConfig) -> Result<(), ErrResp> {
    let storage = match config::read_storage() {
        Err(e) => {
            let msg = format!("[storage] read storage error: {}", e);
            return Err(ErrResp::ErrRsc(Some(msg)));
        }
        Ok(storage) => storage,
    };

    let body = PutWwanReq {
        data: PutWwanReqData {
            enable: false,
            conf: None,
        },
    };
    let client = Client::new();
    let uri = format!("{}/api/v1/net/wwan", config.router);
    let mut token = storage.access_token;

    let mut retry = 0;
    loop {
        let req = match client
            .request(Method::PUT, uri.as_str())
            .header(header::AUTHORIZATION, format!("Bearer {}", token))
            .json(&body)
            .build()
        {
            Err(e) => {
                let msg = format!("[API] create request error: {}", e);
                return Err(ErrResp::ErrRsc(Some(msg)));
            }
            Ok(req) => req,
        };
        let resp = match client.execute(req).await {
            Err(e) => {
                let msg = format!("[API] execute request error: {}", e);
                return Err(ErrResp::ErrRsc(Some(msg)));
            }
            Ok(resp) => resp,
        };
        let status_code = resp.status();
        let body = match resp.bytes().await {
            Err(e) => format!("(wrong body: {})", e),
            Ok(bytes) => match String::from_utf8(bytes.to_vec()) {
                Err(e) => format!("(body not UTF-8: {})", e),
                Ok(body) => body,
            },
        };
        if status_code != StatusCode::NO_CONTENT {
            retry += 1;
            if retry <= API_RETRY && status_code == StatusCode::UNAUTHORIZED {
                token = auth::refresh(cm_conf).await?.access_token;
                continue;
            }
            let msg = format!("[API] unexpected status: {}, body: {}", status_code, body);
            return Err(ErrResp::ErrIntMsg(Some(msg)));
        }
        return Ok(());
    }
}

async fn wwan_list_get(
    config: &Config,
    cm_conf: &CoremgrCliConfig,
    args: &ArgMatches,
) -> Result<Vec<WifiApInfo>, ErrResp> {
    let storage = match config::read_storage() {
        Err(e) => {
            let msg = format!("[storage] read storage error: {}", e);
            return Err(ErrResp::ErrRsc(Some(msg)));
        }
        Ok(storage) => storage,
    };

    let client = Client::new();
    let uri = match args.get_one::<bool>("rescan") {
        None => format!("{}/api/v1/net/wwan/list", config.router),
        Some(rescan) => format!("{}/api/v1/net/wwan/list?rescan={}", config.router, rescan),
    };
    let mut token = storage.access_token;

    let mut retry = 0;
    loop {
        let req = match client
            .request(Method::GET, uri.as_str())
            .header(header::AUTHORIZATION, format!("Bearer {}", token))
            .build()
        {
            Err(e) => {
                let msg = format!("[API] create request error: {}", e);
                return Err(ErrResp::ErrRsc(Some(msg)));
            }
            Ok(req) => req,
        };
        let resp = match client.execute(req).await {
            Err(e) => {
                let msg = format!("[API] execute request error: {}", e);
                return Err(ErrResp::ErrRsc(Some(msg)));
            }
            Ok(resp) => resp,
        };
        let status_code = resp.status();
        let body = match resp.bytes().await {
            Err(e) => format!("(wrong body: {})", e),
            Ok(bytes) => match String::from_utf8(bytes.to_vec()) {
                Err(e) => format!("(body not UTF-8: {})", e),
                Ok(body) => body,
            },
        };
        if status_code != StatusCode::OK {
            retry += 1;
            if retry <= API_RETRY && status_code == StatusCode::UNAUTHORIZED {
                token = auth::refresh(cm_conf).await?.access_token;
                continue;
            }
            let msg = format!("[API] unexpected status: {}, body: {}", status_code, body);
            return Err(ErrResp::ErrIntMsg(Some(msg)));
        }
        match serde_json::from_str::<GetWwanListRes>(body.as_str()) {
            Err(e) => {
                let msg = format!("[API] unexpected token error: {}, body: {}", e, body);
                return Err(ErrResp::ErrIntMsg(Some(msg)));
            }
            Ok(res) => return Ok(res.data),
        }
    }
}
