//! This module operates nmcli and dhcpd.
//!
//! **Note**: all functions are blocking, please use async utility to call these functions.
//!
//! For WiFi interfaces, returns [`None`] means that the interface is disabled.

use std::{
    collections::HashMap,
    fs,
    io::{BufRead, BufReader, Cursor, Error as IoError, ErrorKind},
    process::Command,
};

use chrono::{TimeZone, Utc};
use ipnet::Ipv4Net;
use serde::{Deserialize, Serialize};
use shell_escape;
use sylvia_iot_sdk::util::strings;

/// WAN interface configurations.
#[derive(Deserialize, Serialize)]
pub struct WanConf {
    /// Refer to [`ConnType`].
    #[serde(rename = "type")]
    pub conf_type: String,
    /// **Some** if `conf_type` is `ConnType::ETHERNET`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub type4: Option<String>,
    /// **Some** if `type4` is `Some(Type4::STATIC)`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub static4: Option<Static4>,
    /// **Some** if `conf_type` is `ConnType::PPPOE`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pppoe: Option<Pppoe>,
}

#[derive(Deserialize, Serialize)]
pub struct Static4 {
    pub address: String,
    pub gateway: String,
    pub dns: Vec<String>,
}

#[derive(Deserialize, Serialize)]
pub struct Pppoe {
    pub username: String,
    pub password: String,
}

#[derive(Serialize)]
pub struct WanConn4 {
    pub address: String,
    pub gateway: String,
    pub dns: Vec<String>,
}

#[derive(Deserialize, Serialize)]
pub struct LanConf4 {
    pub address: String,
    #[serde(rename = "dhcpStart")]
    pub dhcp_start: String,
    #[serde(rename = "dhcpEnd")]
    pub dhcp_end: String,
    #[serde(rename = "leaseTime")]
    pub lease_time: usize,
}

#[derive(Clone, Default, Serialize)]
pub struct DhcpLease {
    pub ip: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub starts: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ends: Option<String>,
    #[serde(rename = "macAddr", skip_serializing_if = "Option::is_none")]
    pub mac_addr: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client: Option<String>,
}

#[derive(Deserialize, Serialize)]
pub struct WlanConf {
    pub ssid: String,
    pub channel: usize,
    pub password: String,
}

#[derive(Deserialize, Serialize)]
pub struct WwanConf {
    pub ssid: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
}

#[derive(Default, Serialize)]
pub struct WifiApInfo {
    pub ssid: String,
    pub security: Vec<String>,
    pub channel: usize,
    pub signal: isize,
}

pub struct ConnType;
pub struct Type4;

#[derive(Default)]
struct IfaceInfo {
    auto_conn: bool,
    conn_type: String,
    ip4method: String,
    ip4addr: String,
    ip4gw: String,
    ip4dns: Vec<String>,
    pppoe_username: String,
    pppoe_password: String,
    conn_ip4addr: String,
    conn_ip4gw: String,
    conn_ip4dns: Vec<String>,
    wifi_ssid: String,
    wifi_channel: usize,
}

#[derive(Default)]
struct DhcpInfo {
    start: String,
    end: String,
    lease: usize,
}

impl ConnType {
    pub const DISABLE: &'static str = "disable";
    pub const ETHERNET: &'static str = "ethernet";
    pub const PPPOE: &'static str = "pppoe";
}

impl Type4 {
    pub const DHCP: &'static str = "dhcp";
    pub const STATIC: &'static str = "static";
}

const NM_NOT_FOUND_CODE: i32 = 10;
const NM_FIELDS: &'static str = "connection.autoconnect,connection.type,\
    ipv4.method,ipv4.addresses,ipv4.gateway,ipv4.dns,\
    pppoe.username,pppoe.password,\
    IP4.ADDRESS,IP4.GATEWAY,IP4.DNS,\
    802-11-wireless.ssid,802-11-wireless.channel";
const DHCPD_CONF_PATH: &'static str = "/etc/dhcp/dhcpd.conf";
const DHCPD_LEASES_PATH: &'static str = "/var/lib/dhcp/dhcpd.leases";

/// Get WAN interface configurations.
pub fn get_wan_conf(wan_id: &str) -> Result<Option<(WanConf, WanConn4)>, IoError> {
    let info = match read_iface_info(wan_id)? {
        None => return Ok(None),
        Some(info) => info,
    };

    let conf = WanConf {
        conf_type: match info.auto_conn {
            false => ConnType::DISABLE.to_string(),
            true => match info.conn_type.as_str() {
                "pppoe" => ConnType::PPPOE.to_string(),
                "802-3-ethernet" | "ethernet" => ConnType::ETHERNET.to_string(),
                _ => return Ok(None),
            },
        },
        type4: match info.ip4method.as_str() {
            "auto" => Some(Type4::DHCP.to_string()),
            "manual" => Some(Type4::STATIC.to_string()),
            _ => None,
        },
        static4: match info.ip4method.as_str() {
            "manual" => Some(Static4 {
                address: info.ip4addr.clone(),
                gateway: info.ip4gw.clone(),
                dns: info.ip4dns.clone(),
            }),
            _ => None,
        },
        pppoe: match info.conn_type.as_str() {
            "pppoe" => Some(Pppoe {
                username: info.pppoe_username.clone(),
                password: info.pppoe_password.clone(),
            }),
            _ => None,
        },
    };
    let conn4 = WanConn4 {
        address: info.conn_ip4addr,
        gateway: info.conn_ip4gw,
        dns: info.conn_ip4dns,
    };

    Ok(Some((conf, conn4)))
}

/// Set WAN interface configurations.
///
/// # Panics
///
/// Please make sure that all fields of the relative connection type are `Some`.
pub fn set_wan_conf(wan_id: &str, wan_ifname: &str, conf: &WanConf) -> Result<Option<()>, IoError> {
    // Delete then add.
    let args = &["c", "del", wan_id];
    let output = Command::new("nmcli").args(args).output()?;
    if let Some(code) = output.status.code() {
        if code == NM_NOT_FOUND_CODE {
            return Ok(None);
        } else if code != 0 {
            return Err(IoError::new(ErrorKind::Other, format!("code {}", code)));
        }
    }

    let dns;
    let username;
    let password;

    let args = match conf.conf_type.as_str() {
        ConnType::DISABLE => vec![
            "c",
            "add",
            "type",
            "ethernet",
            "con-name",
            wan_id,
            "ifname",
            wan_ifname,
            "--",
            "+connection.autoconnect",
            "no",
        ],
        ConnType::PPPOE => {
            username = shell_escape::escape(conf.pppoe.as_ref().unwrap().username.as_str().into())
                .to_string();
            password = shell_escape::escape(conf.pppoe.as_ref().unwrap().password.as_str().into())
                .to_string();
            vec![
                "c",
                "add",
                "type",
                "pppoe",
                "con-name",
                wan_id,
                "ifname",
                wan_ifname,
                "username",
                match username.as_str().strip_prefix("'") {
                    None => match username.as_str().strip_suffix("'") {
                        None => username.as_str(),
                        Some(v) => v,
                    },
                    Some(v) => match v.strip_suffix("'") {
                        None => username.as_str(),
                        Some(v) => v,
                    },
                },
                "password",
                match password.as_str().strip_prefix("'") {
                    None => match password.as_str().strip_suffix("'") {
                        None => password.as_str(),
                        Some(v) => v,
                    },
                    Some(v) => match v.strip_suffix("'") {
                        None => password.as_str(),
                        Some(v) => v,
                    },
                },
                "--",
                "+connection.autoconnect",
                "yes",
            ]
        }
        _ => match conf.type4.as_ref().unwrap().as_str() {
            Type4::STATIC => {
                dns = conf.static4.as_ref().unwrap().dns.join(" ");

                vec![
                    "c",
                    "add",
                    "type",
                    "ethernet",
                    "con-name",
                    wan_id,
                    "ifname",
                    wan_ifname,
                    "--",
                    "+connection.autoconnect",
                    "yes",
                    "+ipv4.method",
                    "manual",
                    "+ipv4.addresses",
                    conf.static4.as_ref().unwrap().address.as_str(),
                    "+ipv4.gateway",
                    conf.static4.as_ref().unwrap().gateway.as_str(),
                    "+ipv4.dns",
                    dns.as_str(),
                ]
            }
            _ => vec![
                "c",
                "add",
                "type",
                "ethernet",
                "con-name",
                wan_id,
                "ifname",
                wan_ifname,
                "--",
                "+connection.autoconnect",
                "yes",
                "+ipv4.method",
                "auto",
            ],
        },
    };
    let output = Command::new("nmcli").args(args.as_slice()).output()?;
    if let Some(code) = output.status.code() {
        if code == NM_NOT_FOUND_CODE {
            return Ok(None);
        } else if code != 0 {
            return Err(IoError::new(ErrorKind::Other, format!("code {}", code)));
        }
    }
    Ok(Some(()))
}

/// Get LAN interface configurations.
pub fn get_lan_conf(lan_id: &str) -> Result<Option<LanConf4>, IoError> {
    let info = match read_iface_info(lan_id)? {
        None => return Ok(None),
        Some(info) => info,
    };
    let dhcp_info = read_dhcp_info()?;

    Ok(Some(LanConf4 {
        address: info.ip4addr.clone(),
        dhcp_start: dhcp_info.start,
        dhcp_end: dhcp_info.end,
        lease_time: dhcp_info.lease,
    }))
}

/// Set LAN interface configurations.
///
/// # Panics
///
/// Please make sure that the `address` is a correct CIDR.
pub fn set_lan_conf(lan_id: &str, conf: &LanConf4) -> Result<Option<()>, IoError> {
    // Use nmcli to modify IPv4 address.
    let args = &["c", "mod", lan_id, "ipv4.addresses", conf.address.as_str()];
    let output = Command::new("nmcli").args(args).output()?;
    if let Some(code) = output.status.code() {
        if code == NM_NOT_FOUND_CODE {
            return Ok(None);
        } else if code != 0 {
            return Err(IoError::new(ErrorKind::Other, format!("mod code {}", code)));
        }
    }
    let args = &["c", "down", lan_id];
    let output = Command::new("nmcli").args(args).output()?;
    if let Some(code) = output.status.code() {
        if code == NM_NOT_FOUND_CODE {
            return Ok(None);
        } else if code != 0 {
            return Err(IoError::new(
                ErrorKind::Other,
                format!("down code {}", code),
            ));
        }
    }
    let args = &["c", "up", lan_id];
    let output = Command::new("nmcli").args(args).output()?;
    if let Some(code) = output.status.code() {
        if code == NM_NOT_FOUND_CODE {
            return Ok(None);
        } else if code != 0 {
            return Err(IoError::new(ErrorKind::Other, format!("up code {}", code)));
        }
    }

    // Reconfigure dhcpd and restart the service.
    let addr_net = conf.address.parse::<Ipv4Net>().unwrap();
    let dhcp_conf_str = format!(
        "default-lease-time {};\nmax-lease-time {};\nlog-facility local7;\n\
        subnet {} netmask {} {{\n  range {} {};\n  option routers {};\n  option domain-name-servers {};\n\
        }}\n",
        conf.lease_time, conf.lease_time, addr_net.network(), addr_net.netmask(),
        conf.dhcp_start, conf.dhcp_end, addr_net.addr(), addr_net.addr()
    );
    fs::write(DHCPD_CONF_PATH, dhcp_conf_str.as_bytes())?;

    let args = &["restart", "isc-dhcp-server.service"];
    let output = Command::new("systemctl").args(args).output()?;
    if let Some(code) = output.status.code() {
        if code != 0 {
            return Err(IoError::new(
                ErrorKind::Other,
                format!("restart code {}", code),
            ));
        }
    }

    Ok(Some(()))
}

/// Get DHCP leases information by parsing dhcpd.leases.
pub fn get_dhcp_leases() -> Result<Vec<DhcpLease>, IoError> {
    let leases_str = fs::read_to_string(DHCPD_LEASES_PATH)?;

    let mut leases_map = HashMap::<String, DhcpLease>::new();
    let mut new_lease = DhcpLease::default();
    let mut lines = BufReader::new(Cursor::new(leases_str)).lines();
    while let Some(Ok(line)) = lines.next() {
        if line.starts_with("lease ") && line.ends_with(" {") {
            let content = line.strip_prefix("lease ").unwrap();
            let content = content.strip_suffix(" {").unwrap();
            new_lease.ip = content.to_string();
        } else if line.eq("}") {
            if new_lease.ip.len() > 0 {
                leases_map.insert(new_lease.ip.clone(), new_lease.clone());
            }
            new_lease = DhcpLease::default();
        } else if line.starts_with("  starts ") && line.ends_with(";") {
            let content = line.strip_prefix("  starts ").unwrap();
            let content = content.strip_suffix(";").unwrap();
            let mut splits = content.split(" ");
            match splits.next() {
                None => continue,
                Some(_) => (),
            }
            let mut dt_str = match splits.next() {
                None => continue,
                Some(v) => v.to_string(),
            };
            dt_str.push_str(" ");
            match splits.next() {
                None => continue,
                Some(v) => dt_str.push_str(v),
            }
            if let Ok(dt) = Utc.datetime_from_str(dt_str.as_str(), "%Y/%m/%d %H:%M:%S") {
                new_lease.starts = Some(strings::time_str(&dt));
            }
        } else if line.starts_with("  ends ") && line.ends_with(";") {
            let content = line.strip_prefix("  ends ").unwrap();
            let content = content.strip_suffix(";").unwrap();
            let mut splits = content.split(" ");
            match splits.next() {
                None => continue,
                Some(_) => (),
            }
            let mut dt_str = match splits.next() {
                None => continue,
                Some(v) => v.to_string(),
            };
            dt_str.push_str(" ");
            match splits.next() {
                None => continue,
                Some(v) => dt_str.push_str(v),
            }
            if let Ok(dt) = Utc.datetime_from_str(dt_str.as_str(), "%Y/%m/%d %H:%M:%S") {
                new_lease.ends = Some(strings::time_str(&dt));
            }
        } else if line.starts_with("  hardware ethernet ") && line.ends_with(";") {
            let content = line.strip_prefix("  hardware ethernet ").unwrap();
            let content = content.strip_suffix(";").unwrap();
            new_lease.mac_addr = Some(content.to_lowercase());
        } else if line.starts_with("  client-hostname \"") && line.ends_with("\";") {
            let content = line.strip_prefix("  client-hostname \"").unwrap();
            let content = content.strip_suffix("\";").unwrap();
            new_lease.client = Some(content.to_string());
        }
    }

    let mut leases = vec![];
    for (_k, v) in leases_map.iter() {
        leases.push(v.clone());
    }

    Ok(leases)
}

/// Get wireless LAN interface configurations.
pub fn get_wlan_conf(wlan_id: &str, wlan_ifname: &str) -> Result<Option<WlanConf>, IoError> {
    let info = match read_iface_info(wlan_id)? {
        None => return Ok(None),
        Some(info) => info,
    };
    let password = match read_wifi_ap_password(wlan_ifname)? {
        None => return Ok(None),
        Some(password) => password,
    };

    Ok(Some(WlanConf {
        ssid: info.wifi_ssid,
        channel: info.wifi_channel,
        password,
    }))
}

/// Enable wireless LAN interface configurations.
pub fn set_wlan_conf(
    wlan_id: &str,
    wlan_ifname: &str,
    brlan_id: &str,
    conf: &WlanConf,
) -> Result<Option<()>, IoError> {
    let args = &["c", "down", wlan_id];
    let output = Command::new("nmcli").args(args).output()?;
    if let Some(code) = output.status.code() {
        if code != 0 && code != NM_NOT_FOUND_CODE {
            return Err(IoError::new(
                ErrorKind::Other,
                format!("down code {}", code),
            ));
        }
    }
    let args = &["c", "del", wlan_id];
    let output = Command::new("nmcli").args(args).output()?;
    if let Some(code) = output.status.code() {
        if code != 0 && code != NM_NOT_FOUND_CODE {
            return Err(IoError::new(ErrorKind::Other, format!("del code {}", code)));
        }
    }
    let args = &[
        "c",
        "add",
        "type",
        "wifi",
        "slave-type",
        "bridge",
        "master",
        brlan_id,
        "con-name",
        wlan_id,
        "ifname",
        wlan_ifname,
        "autoconnect",
        "yes",
        "ssid",
        conf.ssid.as_str(),
    ];
    let output = Command::new("nmcli").args(args).output()?;
    if let Some(code) = output.status.code() {
        if code == NM_NOT_FOUND_CODE {
            return Ok(None);
        } else if code != 0 {
            return Err(IoError::new(ErrorKind::Other, format!("add code {}", code)));
        }
    }
    let channel = format!("{}", conf.channel);
    let args = &[
        "c",
        "mod",
        wlan_id,
        "802-11-wireless.mode",
        "ap",
        "802-11-wireless.band",
        "bg",
        "802-11-wireless.channel",
        channel.as_str(),
    ];
    let output = Command::new("nmcli").args(args).output()?;
    if let Some(code) = output.status.code() {
        if code == NM_NOT_FOUND_CODE {
            return Ok(None);
        } else if code != 0 {
            return Err(IoError::new(
                ErrorKind::Other,
                format!("mod mode code {}", code),
            ));
        }
    }
    let args = &["c", "mod", wlan_id, "wifi-sec.key-mgmt", "wpa-psk"];
    let output = Command::new("nmcli").args(args).output()?;
    if let Some(code) = output.status.code() {
        if code == NM_NOT_FOUND_CODE {
            return Ok(None);
        } else if code != 0 {
            return Err(IoError::new(
                ErrorKind::Other,
                format!("mod key code {}", code),
            ));
        }
    }
    let args = &["c", "mod", wlan_id, "wifi-sec.psk", conf.password.as_str()];
    let output = Command::new("nmcli").args(args).output()?;
    if let Some(code) = output.status.code() {
        if code == NM_NOT_FOUND_CODE {
            return Ok(None);
        } else if code != 0 {
            return Err(IoError::new(
                ErrorKind::Other,
                format!("mod psk code {}", code),
            ));
        }
    }
    let args = &["c", "up", wlan_id];
    let output = Command::new("nmcli").args(args).output()?;
    if let Some(code) = output.status.code() {
        if code == NM_NOT_FOUND_CODE {
            return Ok(None);
        } else if code != 0 {
            return Err(IoError::new(ErrorKind::Other, format!("up code {}", code)));
        }
    }

    Ok(Some(()))
}

/// Get wireless WAN interface configurations.
pub fn get_wwan_conf(wwan_id: &str) -> Result<Option<(String, WanConn4)>, IoError> {
    let info = match read_iface_info(wwan_id)? {
        None => return Ok(None),
        Some(info) => info,
    };

    let conn4 = WanConn4 {
        address: info.conn_ip4addr,
        gateway: info.conn_ip4gw,
        dns: info.conn_ip4dns,
    };
    Ok(Some((info.wifi_ssid, conn4)))
}

/// Enable wireless WAN interface configurations.
pub fn set_wwan_conf(
    wwan_id: &str,
    wwan_ifname: &str,
    conf: &WwanConf,
) -> Result<Option<()>, IoError> {
    let args = match conf.password.as_ref() {
        None => vec![
            "d",
            "wifi",
            "connect",
            conf.ssid.as_str(),
            "name",
            wwan_id,
            "ifname",
            wwan_ifname,
        ],
        Some(v) => vec![
            "d",
            "wifi",
            "connect",
            conf.ssid.as_str(),
            "name",
            wwan_id,
            "ifname",
            wwan_ifname,
            "password",
            v.as_str(),
        ],
    };
    let output = Command::new("nmcli").args(args.as_slice()).output()?;
    if let Some(code) = output.status.code() {
        if code == NM_NOT_FOUND_CODE {
            return Ok(None);
        } else if code != 0 {
            return Err(IoError::new(ErrorKind::Other, format!("code {}", code)));
        }
    }

    Ok(Some(()))
}

/// Disable wireless LAN or WAN interface configurations.
pub fn clear_wlan_conf(wlan_wwan_id: &str) -> Result<Option<()>, IoError> {
    let args = &["c", "down", wlan_wwan_id];
    let output = Command::new("nmcli").args(args).output()?;
    if let Some(code) = output.status.code() {
        if code != 0 && code != NM_NOT_FOUND_CODE {
            return Err(IoError::new(
                ErrorKind::Other,
                format!("down code {}", code),
            ));
        }
    }
    let args = &["c", "del", wlan_wwan_id];
    let output = Command::new("nmcli").args(args).output()?;
    if let Some(code) = output.status.code() {
        if code == NM_NOT_FOUND_CODE {
            return Ok(None);
        } else if code != 0 {
            return Err(IoError::new(ErrorKind::Other, format!("del code {}", code)));
        }
    }

    Ok(Some(()))
}

/// Get WiFi AP list.
pub fn get_wifi_ap_list(ifname: &str, rescan: bool) -> Result<Vec<WifiApInfo>, IoError> {
    if rescan {
        let args = &["d", "wifi", "rescan", "ifname", ifname];
        let output = Command::new("nmcli").args(args).output()?;
        if let Some(code) = output.status.code() {
            if code != 0 {
                return Err(IoError::new(
                    ErrorKind::Other,
                    format!("rescan code {}", code),
                ));
            }
        }
    }

    let output = Command::new("nmcli")
        .args(&[
            "-t",
            "-f",
            "SSID,SECURITY,CHAN,SIGNAL",
            "d",
            "wifi",
            "list",
            "ifname",
            ifname,
        ])
        .output()?;
    if let Some(code) = output.status.code() {
        if code != 0 {
            return Err(IoError::new(
                ErrorKind::Other,
                format!("list code {}", code),
            ));
        }
    }
    let mut lines = BufReader::new(Cursor::new(output.stdout)).lines();

    let mut list = vec![];
    while let Some(Ok(line)) = lines.next() {
        let mut info = WifiApInfo::default();
        let mut splits = line.split(":");
        info.ssid = match splits.next() {
            None => continue,
            Some(v) => v.to_string(),
        };
        info.security = match splits.next() {
            None => continue,
            Some(v) => v.split(" ").map(|x| x.to_string()).collect(),
        };
        info.channel = match splits.next() {
            None => continue,
            Some(v) => match v.parse::<usize>() {
                Err(_) => continue,
                Ok(v) => v,
            },
        };
        info.signal = match splits.next() {
            None => continue,
            Some(v) => match v.parse::<isize>() {
                Err(_) => continue,
                Ok(v) => v,
            },
        };
        list.push(info);
    }
    return Ok(list);
}

/// Read interface information by nmcli.
fn read_iface_info(nm_ifname: &str) -> Result<Option<IfaceInfo>, IoError> {
    let output = Command::new("nmcli")
        .args(&["-s", "-t", "-f", NM_FIELDS, "c", "show", nm_ifname])
        .output()?;
    if let Some(code) = output.status.code() {
        if code == NM_NOT_FOUND_CODE {
            return Ok(None);
        } else if code != 0 {
            return Err(IoError::new(ErrorKind::Other, format!("code {}", code)));
        }
    }
    let mut lines = BufReader::new(Cursor::new(output.stdout)).lines();

    let mut info = IfaceInfo::default();
    while let Some(Ok(line)) = lines.next() {
        let mut splits = line.split(":");
        let key = match splits.next() {
            None => continue,
            Some(v) => {
                if v.starts_with("IP4.ADDRESS[") {
                    "IP4.ADDRESS"
                } else if v.starts_with("IP4.DNS[") {
                    "IP4.DNS"
                } else {
                    v
                }
            }
        };
        let value = match splits.next() {
            None => continue,
            Some(v) => v,
        };

        match key {
            "connection.autoconnect" => info.auto_conn = value.eq("yes"),
            "connection.type" => info.conn_type = value.to_lowercase(),
            "ipv4.addresses" => info.ip4addr = value.to_lowercase(),
            "ipv4.dns" => {
                let mut dns = value.split(",");
                while let Some(v) = dns.next() {
                    info.ip4dns.push(v.to_lowercase());
                }
            }
            "ipv4.gateway" => info.ip4gw = value.to_lowercase(),
            "ipv4.method" => info.ip4method = value.to_lowercase(),
            "pppoe.username" => info.pppoe_username = value.to_lowercase(),
            "pppoe.password" => info.pppoe_password = value.to_string(),
            "IP4.ADDRESS" => info.conn_ip4addr = value.to_lowercase(),
            "IP4.DNS" => {
                let mut dns = value.split(",");
                while let Some(v) = dns.next() {
                    info.conn_ip4dns.push(v.to_lowercase());
                }
            }
            "IP4.GATEWAY" => info.conn_ip4gw = value.to_lowercase(),
            "802-11-wireless.ssid" => info.wifi_ssid = value.to_string(),
            "802-11-wireless.channel" => {
                info.wifi_channel = match value.parse::<usize>() {
                    Err(_) => 0,
                    Ok(v) => v,
                }
            }
            _ => continue,
        }
    }

    Ok(Some(info))
}

/// Read DHCP information by parsing dhcpd.conf.
fn read_dhcp_info() -> Result<DhcpInfo, IoError> {
    let conf_str = fs::read_to_string(DHCPD_CONF_PATH)?;
    let mut lines = BufReader::new(Cursor::new(conf_str)).lines();

    let mut info = DhcpInfo::default();
    while let Some(Ok(line)) = lines.next() {
        if line.starts_with("default-lease-time ") {
            let mut line = line.as_str().strip_prefix("default-lease-time ").unwrap();
            if line.ends_with(";") {
                line = line.strip_suffix(";").unwrap();
            }
            info.lease = match line.parse::<usize>() {
                Err(_) => {
                    return Err(IoError::new(
                        ErrorKind::InvalidData,
                        format!("invalid default-lease-time: {}", line),
                    ));
                }
                Ok(v) => v,
            }
        } else if line.starts_with("  range ") {
            let mut line = line.as_str().strip_prefix("  range ").unwrap();
            if line.ends_with(";") {
                line = line.strip_suffix(";").unwrap();
            }
            let mut splits = line.split(" ");
            info.start = match splits.next() {
                None => {
                    return Err(IoError::new(
                        ErrorKind::InvalidData,
                        format!("invalid range: {}", line),
                    ));
                }
                Some(v) => v.to_string(),
            };
            info.end = match splits.next() {
                None => {
                    return Err(IoError::new(
                        ErrorKind::InvalidData,
                        format!("invalid range: {}", line),
                    ));
                }
                Some(v) => v.to_string(),
            };
        }
    }
    if info.lease == 0 {
        return Err(IoError::new(
            ErrorKind::InvalidData,
            "missing default-lease-time",
        ));
    }
    if info.start.len() == 0 || info.end.len() == 0 {
        return Err(IoError::new(ErrorKind::InvalidData, "missing range"));
    }

    Ok(info)
}

/// Read WiFi AP password.
fn read_wifi_ap_password(ifname: &str) -> Result<Option<String>, IoError> {
    let output = Command::new("nmcli")
        .env("LANG", "en")
        .args(&["d", "wifi", "show-password", "ifname", ifname])
        .output()?;
    if let Some(code) = output.status.code() {
        if code != 0 {
            return Err(IoError::new(ErrorKind::Other, format!("code {}", code)));
        }
    }
    let mut lines = BufReader::new(Cursor::new(output.stdout)).lines();

    while let Some(Ok(line)) = lines.next() {
        let mut splits = line.split(": ");
        let key = match splits.next() {
            None => continue,
            Some(v) => v,
        };
        let value = match splits.next() {
            None => continue,
            Some(v) => v,
        };

        match key {
            "Password" => return Ok(Some(value.to_string())),
            _ => continue,
        }
    }
    return Ok(Some("".to_string()));
}
