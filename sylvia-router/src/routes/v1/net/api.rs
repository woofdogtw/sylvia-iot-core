use std::{cmp::Ordering, net::Ipv4Addr};

use axum::{extract::State, http::StatusCode, response::IntoResponse};
use ipnet::Ipv4Net;
use log::error;
use sylvia_iot_sdk::util::{
    err::ErrResp,
    http::{Json, Path, Query},
};
use tokio::task;

use super::{super::super::State as AppState, request, response};
use crate::libs::network::{self, ConnType, LanConf4, Pppoe, Type4, WanConf, WlanConf, WwanConf};

const INVALID_ADDR_MSG: &'static str =
    "cannot be broadcast, link local, loopback, multicast, unspecified address";
const LEASE_TIME_SEC_MAX: usize = 604800;
const LEASE_TIME_SEC_MIN: usize = 60;

/// `GET /{base}/api/v1/net/wan`
pub async fn get_wan(State(state): State<AppState>) -> impl IntoResponse {
    const FN_NAME: &'static str = "get_wan";

    match task::spawn_blocking(move || {
        let mut res = response::GetWan { data: vec![] };
        for iface in state.config.wan.iter() {
            let (conf, conn4) = match network::get_wan_conf(iface.name.as_str()) {
                Err(e) => {
                    error!("[{}] command error: {}", FN_NAME, e);
                    return Err(ErrResp::ErrRsc(Some(format!("command error: {}", e))));
                }
                Ok(info) => match info {
                    None => {
                        error!("[{}] WAN {} not found", FN_NAME, iface.name);
                        continue;
                    }
                    Some(info) => info,
                },
            };

            res.data.push(response::GetWanData {
                wan_id: iface.name.clone(),
                conf,
                conn4,
            });
        }

        Ok(res)
    })
    .await
    {
        Err(e) => {
            error!("[{}] run async error: {}", FN_NAME, e);
            Err(ErrResp::ErrRsc(Some(format!("run async error: {} ", e))))
        }
        Ok(res) => match res {
            Err(e) => Err(e),
            Ok(res) => Ok(Json(res)),
        },
    }
}

/// `PUT /{base}/api/v1/net/wan/{wanId}`
pub async fn put_wan(
    State(state): State<AppState>,
    Path(param): Path<request::WanIdPath>,
    Json(mut body): Json<request::PutWanBody>,
) -> impl IntoResponse {
    const FN_NAME: &'static str = "put_wan";

    if let Err(e) = parse_put_wan(&mut body.data) {
        return Err(ErrResp::ErrParam(Some(e)));
    };

    let mut wan_ifname = "".to_string();
    for wan in state.config.wan.iter() {
        if wan.name.eq(&param.wan_id) {
            wan_ifname = wan.ifname.clone();
            break;
        }
    }
    if wan_ifname.len() == 0 {
        return Err(ErrResp::ErrNotFound(None));
    }

    match task::spawn_blocking(move || {
        match network::set_wan_conf(param.wan_id.as_str(), wan_ifname.as_str(), &body.data) {
            Err(e) => {
                error!("[{}] command error: {}", FN_NAME, e);
                return Err(ErrResp::ErrRsc(Some(format!("command error: {}", e))));
            }
            Ok(info) => match info {
                None => Err(ErrResp::ErrNotFound(None)),
                Some(_) => Ok(()),
            },
        }
    })
    .await
    {
        Err(e) => {
            error!("[{}] run async error: {}", FN_NAME, e);
            return Err(ErrResp::ErrRsc(Some(format!("run async error: {} ", e))));
        }
        Ok(res) => match res {
            Err(e) => return Err(e),
            Ok(_) => (),
        },
    }

    Ok(StatusCode::NO_CONTENT)
}

/// `GET /{base}/api/v1/net/lan`
pub async fn get_lan(State(state): State<AppState>) -> impl IntoResponse {
    const FN_NAME: &'static str = "get_lan";

    match task::spawn_blocking(move || {
        let iface = &state.config.lan;

        let conf4 = match network::get_lan_conf(iface.name.as_str()) {
            Err(e) => {
                error!("[{}] command error: {}", FN_NAME, e);
                return Err(ErrResp::ErrRsc(Some(format!("command error: {}", e))));
            }
            Ok(info) => match info {
                None => {
                    error!("[{}] LAN {} not found", FN_NAME, iface.name);
                    return Err(ErrResp::ErrRsc(Some(format!(
                        "LAN {} not found",
                        iface.name
                    ))));
                }
                Some(info) => info,
            },
        };

        Ok(response::GetLan {
            data: response::GetLanData { conf4 },
        })
    })
    .await
    {
        Err(e) => {
            error!("[{}] run async error: {}", FN_NAME, e);
            Err(ErrResp::ErrRsc(Some(format!("run async error: {} ", e))))
        }
        Ok(res) => match res {
            Err(e) => Err(e),
            Ok(res) => Ok(Json(res)),
        },
    }
}

/// `PUT /{base}/api/v1/net/lan`
pub async fn put_lan(
    State(state): State<AppState>,
    Json(body): Json<request::PutLanBody>,
) -> impl IntoResponse {
    const FN_NAME: &'static str = "put_lan";

    if let Err(e) = parse_put_lan(&body.data.conf4) {
        return Err(ErrResp::ErrParam(Some(e)));
    };

    match task::spawn_blocking(move || {
        let iface = &state.config.lan;

        match network::set_lan_conf(iface.name.as_str(), &body.data.conf4) {
            Err(e) => {
                error!("[{}] command error: {}", FN_NAME, e);
                return Err(ErrResp::ErrRsc(Some(format!("command error: {}", e))));
            }
            Ok(info) => match info {
                None => Err(ErrResp::ErrNotFound(None)),
                Some(_) => Ok(()),
            },
        }
    })
    .await
    {
        Err(e) => {
            error!("[{}] run async error: {}", FN_NAME, e);
            return Err(ErrResp::ErrRsc(Some(format!("run async error: {} ", e))));
        }
        Ok(res) => match res {
            Err(e) => return Err(e),
            Ok(_) => (),
        },
    }

    Ok(StatusCode::NO_CONTENT)
}

/// `GET /{base}/api/v1/net/lan/leases`
pub async fn get_lan_leases() -> impl IntoResponse {
    const FN_NAME: &'static str = "get_lan_leases";

    match task::spawn_blocking(move || {
        let leases = match network::get_dhcp_leases() {
            Err(e) => {
                error!("[{}] get leases error: {}", FN_NAME, e);
                return Err(ErrResp::ErrRsc(Some(format!("get leases error: {}", e))));
            }
            Ok(info) => info,
        };

        Ok(response::GetLanLeases { data: leases })
    })
    .await
    {
        Err(e) => {
            error!("[{}] run async error: {}", FN_NAME, e);
            Err(ErrResp::ErrRsc(Some(format!("run async error: {} ", e))))
        }
        Ok(res) => match res {
            Err(e) => Err(e),
            Ok(res) => Ok(Json(res)),
        },
    }
}

/// `GET /{base}/api/v1/net/wlan`
pub async fn get_wlan(State(state): State<AppState>) -> impl IntoResponse {
    const FN_NAME: &'static str = "get_wlan";

    let iface = match state.config.wlan.as_ref() {
        None => return Ok(StatusCode::NOT_FOUND.into_response()),
        Some(conf) => conf.clone(),
    };

    match task::spawn_blocking(move || {
        match network::get_wlan_conf(iface.name.as_str(), iface.ifname.as_str()) {
            Err(e) => {
                error!("[{}] command error: {}", FN_NAME, e);
                return Err(ErrResp::ErrRsc(Some(format!("command error: {}", e))));
            }
            Ok(info) => match info {
                None => Ok(response::GetWlan {
                    data: response::GetWlanData {
                        enable: false,
                        conf: None,
                    },
                }),
                Some(info) => Ok(response::GetWlan {
                    data: response::GetWlanData {
                        enable: true,
                        conf: Some(info),
                    },
                }),
            },
        }
    })
    .await
    {
        Err(e) => {
            error!("[{}] run async error: {}", FN_NAME, e);
            Err(ErrResp::ErrRsc(Some(format!("run async error: {} ", e))))
        }
        Ok(res) => match res {
            Err(e) => Err(e),
            Ok(res) => Ok(Json(res).into_response()),
        },
    }
}

/// `PUT /{base}/api/v1/net/wlan`
pub async fn put_wlan(
    State(state): State<AppState>,
    Json(body): Json<request::PutWlanBody>,
) -> impl IntoResponse {
    const FN_NAME: &'static str = "put_wlan";

    let iface = match state.config.wlan.as_ref() {
        None => return Ok(StatusCode::NOT_FOUND),
        Some(conf) => conf.clone(),
    };

    match task::spawn_blocking(move || {
        let lan_iface = &state.config.lan;

        if !body.data.enable {
            match network::clear_wlan_conf(iface.name.as_str()) {
                Err(e) => {
                    error!("[{}] clear command error: {}", FN_NAME, e);
                    return Err(ErrResp::ErrRsc(Some(format!("clear command error: {}", e))));
                }
                Ok(info) => match info {
                    None => return Err(ErrResp::ErrNotFound(None)),
                    Some(_) => return Ok(()),
                },
            }
        }

        let conf = match body.data.conf.as_ref() {
            None => return Err(ErrResp::ErrParam(Some("missing `conf`".to_string()))),
            Some(conf) => match parse_put_wlan(conf) {
                Err(e) => return Err(ErrResp::ErrParam(Some(e))),
                Ok(()) => conf,
            },
        };
        match network::set_wlan_conf(
            iface.name.as_str(),
            iface.ifname.as_str(),
            lan_iface.name.as_str(),
            conf,
        ) {
            Err(e) => {
                error!("[{}] command error: {}", FN_NAME, e);
                return Err(ErrResp::ErrRsc(Some(format!("command error: {}", e))));
            }
            Ok(info) => match info {
                None => Err(ErrResp::ErrNotFound(None)),
                Some(_) => Ok(()),
            },
        }
    })
    .await
    {
        Err(e) => {
            error!("[{}] run async error: {}", FN_NAME, e);
            return Err(ErrResp::ErrRsc(Some(format!("run async error: {} ", e))).into_response());
        }
        Ok(res) => match res {
            Err(e) => return Err(e.into_response()),
            Ok(_) => (),
        },
    }

    Ok(StatusCode::NO_CONTENT)
}

/// `GET /{base}/api/v1/net/wwan`
pub async fn get_wwan(State(state): State<AppState>) -> impl IntoResponse {
    const FN_NAME: &'static str = "get_wwan";

    let iface = match state.config.wwan.as_ref() {
        None => return Ok(StatusCode::NOT_FOUND.into_response()),
        Some(conf) => conf.clone(),
    };

    match task::spawn_blocking(move || match network::get_wwan_conf(iface.name.as_str()) {
        Err(e) => {
            error!("[{}] command error: {}", FN_NAME, e);
            return Err(ErrResp::ErrRsc(Some(format!("command error: {}", e))));
        }
        Ok(info) => match info {
            None => Ok(response::GetWwan {
                data: response::GetWwanData {
                    enable: false,
                    conf: None,
                    conn4: None,
                },
            }),
            Some((ssid, conn4)) => Ok(response::GetWwan {
                data: response::GetWwanData {
                    enable: true,
                    conf: Some(WwanConf {
                        ssid,
                        password: None,
                    }),
                    conn4: Some(conn4),
                },
            }),
        },
    })
    .await
    {
        Err(e) => {
            error!("[{}] run async error: {}", FN_NAME, e);
            Err(ErrResp::ErrRsc(Some(format!("run async error: {} ", e))))
        }
        Ok(res) => match res {
            Err(e) => Err(e),
            Ok(res) => Ok(Json(res).into_response()),
        },
    }
}

/// `PUT /{base}/api/v1/net/wwan`
pub async fn put_wwan(
    State(state): State<AppState>,
    Json(body): Json<request::PutWwanBody>,
) -> impl IntoResponse {
    const FN_NAME: &'static str = "put_wwan";

    let iface = match state.config.wwan.as_ref() {
        None => return Ok(StatusCode::NOT_FOUND),
        Some(conf) => conf.clone(),
    };

    match task::spawn_blocking(move || {
        if !body.data.enable {
            match network::clear_wlan_conf(iface.name.as_str()) {
                Err(e) => {
                    error!("[{}] clear command error: {}", FN_NAME, e);
                    return Err(ErrResp::ErrRsc(Some(format!("clear command error: {}", e))));
                }
                Ok(info) => match info {
                    None => return Err(ErrResp::ErrNotFound(None)),
                    Some(_) => return Ok(()),
                },
            }
        }

        let conf = match body.data.conf.as_ref() {
            None => return Err(ErrResp::ErrParam(Some("missing `conf`".to_string()))),
            Some(conf) => match parse_put_wwan(conf) {
                Err(e) => return Err(ErrResp::ErrParam(Some(e))),
                Ok(()) => conf,
            },
        };
        match network::set_wwan_conf(iface.name.as_str(), iface.ifname.as_str(), conf) {
            Err(e) => {
                error!("[{}] command error: {}", FN_NAME, e);
                return Err(ErrResp::ErrRsc(Some(format!("command error: {}", e))));
            }
            Ok(info) => match info {
                None => Err(ErrResp::ErrNotFound(None)),
                Some(_) => Ok(()),
            },
        }
    })
    .await
    {
        Err(e) => {
            error!("[{}] run async error: {}", FN_NAME, e);
            return Err(ErrResp::ErrRsc(Some(format!("run async error: {} ", e))));
        }
        Ok(res) => match res {
            Err(e) => return Err(e),
            Ok(_) => (),
        },
    }

    Ok(StatusCode::NO_CONTENT)
}

/// `GET /{base}/api/v1/net/wwan/list`
pub async fn get_wwan_list(
    State(state): State<AppState>,
    Query(query): Query<request::GetWwanListQuery>,
) -> impl IntoResponse {
    const FN_NAME: &'static str = "get_wwan_list";

    let iface = match state.config.wwan.as_ref() {
        None => return Ok(StatusCode::NOT_FOUND.into_response()),
        Some(conf) => conf.clone(),
    };
    let rescan = match query.rescan.as_ref() {
        None => false,
        Some(v) => *v,
    };

    match task::spawn_blocking(move || {
        match network::get_wifi_ap_list(iface.ifname.as_str(), rescan) {
            Err(e) => {
                error!("[{}] command error: {}", FN_NAME, e);
                return Err(ErrResp::ErrRsc(Some(format!("command error: {}", e))));
            }
            Ok(info) => Ok(response::GetWwanList { data: info }),
        }
    })
    .await
    {
        Err(e) => {
            error!("[{}] run async error: {}", FN_NAME, e);
            Err(ErrResp::ErrRsc(Some(format!("run async error: {} ", e))))
        }
        Ok(res) => match res {
            Err(e) => Err(e),
            Ok(res) => Ok(Json(res).into_response()),
        },
    }
}

fn parse_put_wan(data: &mut WanConf) -> Result<(), String> {
    match data.conf_type.as_str() {
        ConnType::DISABLE | ConnType::ETHERNET | ConnType::PPPOE => (),
        _ => return Err(format!("invalid `type`: {}", data.conf_type)),
    }
    if let Some(type4) = data.type4.as_ref() {
        match type4.as_str() {
            Type4::DHCP | Type4::STATIC => (),
            _ => return Err(format!("invalid `type4`: {}", type4)),
        }
    }
    match data.static4.as_ref() {
        None => {
            if data.conf_type.eq(ConnType::ETHERNET) {
                if let Some(type4) = data.type4.as_ref() {
                    if type4.eq(Type4::STATIC) {
                        return Err("missing `static4`".to_string());
                    }
                }
            }
        }
        Some(static4) => {
            let ipnet = match static4.address.parse::<Ipv4Net>() {
                Err(_) => return Err(format!("invalid `static4.address`: {}", static4.address)),
                Ok(ipnet) => ipnet,
            };
            if invalid_addr(&ipnet.addr()) {
                return Err(format!("`static4.address` {}", INVALID_ADDR_MSG));
            }
            let gateway = match static4.gateway.parse::<Ipv4Addr>() {
                Err(_) => return Err(format!("invalid `static4.gateway`: {}", static4.gateway)),
                Ok(addr) => addr,
            };
            if invalid_addr(&gateway) {
                return Err(format!("`static4.gateway` {}", INVALID_ADDR_MSG));
            }
            if !ipnet.contains(&gateway) {
                return Err(format!(
                    "`static4.address` {} does not contains `static4.gateway` {}",
                    static4.address, static4.gateway
                ));
            }
            for addr in static4.dns.iter() {
                if addr.parse::<Ipv4Addr>().is_err() {
                    return Err(format!("invalid `static4.dns`: {}", addr));
                }
            }
        }
    }
    match data.pppoe.as_ref() {
        None => {
            if data.conf_type.eq(ConnType::PPPOE) {
                return Err("missing `pppoe`".to_string());
            }
        }
        Some(pppoe) => {
            if pppoe.username.len() == 0 {
                return Err(format!("invalid `pppoe.username`: {}", pppoe.username));
            }
            if pppoe.password.len() == 0 {
                return Err(format!("invalid `pppoe.password`: {}", pppoe.password));
            }
            data.pppoe = Some(Pppoe {
                username: pppoe.username.to_lowercase(),
                password: pppoe.password.clone(),
            })
        }
    }

    Ok(())
}

fn parse_put_lan(data: &LanConf4) -> Result<(), String> {
    let ipnet = match data.address.parse::<Ipv4Net>() {
        Err(_) => return Err(format!("invalid `address`: {}", data.address)),
        Ok(ipnet) => ipnet,
    };
    if invalid_addr(&ipnet.addr()) {
        return Err(format!("`address` {}", INVALID_ADDR_MSG));
    }
    let start = match data.dhcp_start.parse::<Ipv4Addr>() {
        Err(_) => return Err(format!("invalid `dhcpStart`: {}", data.dhcp_start)),
        Ok(addr) => addr,
    };
    if invalid_addr(&start) {
        return Err(format!("`dhcpStart` {}", INVALID_ADDR_MSG));
    }
    if !ipnet.contains(&start) {
        return Err(format!(
            "`address` {} does not contains `dhcpStart` {}",
            data.address, data.dhcp_start
        ));
    }
    let end = match data.dhcp_end.parse::<Ipv4Addr>() {
        Err(_) => return Err(format!("invalid `dhcpEnd`: {}", data.dhcp_end)),
        Ok(addr) => addr,
    };
    if invalid_addr(&end) {
        return Err(format!("`dhcpEnd` {}", INVALID_ADDR_MSG));
    }
    if !ipnet.contains(&end) {
        return Err(format!(
            "`address` {} does not contains `dhcpEnd` {}",
            data.address, data.dhcp_end
        ));
    }
    if end.cmp(&start) == Ordering::Less {
        return Err("`dhcpEnd` cannot smaller than `dhcpStart`".to_string());
    }
    if ipnet.addr().cmp(&start) != Ordering::Less && ipnet.addr().cmp(&end) != Ordering::Greater {
        return Err(
            "the address of `address` cannot between `dhcpStart` and `dhcpEnd`".to_string(),
        );
    }
    if data.lease_time < LEASE_TIME_SEC_MIN || data.lease_time > LEASE_TIME_SEC_MAX {
        return Err(format!(
            "`leaseTime` must between {} and {}",
            LEASE_TIME_SEC_MIN, LEASE_TIME_SEC_MAX
        ));
    }

    Ok(())
}

fn parse_put_wlan(data: &WlanConf) -> Result<(), String> {
    if data.ssid.len() == 0 {
        return Err("`ssid` must be non-empty string".to_string());
    } else if data.channel < 1 || data.channel > 11 {
        return Err("`channel` must between 1 and 11".to_string());
    } else if data.password.len() == 0 {
        return Err("`password` must be non-empty string".to_string());
    }

    Ok(())
}

fn parse_put_wwan(data: &WwanConf) -> Result<(), String> {
    if data.ssid.len() == 0 {
        return Err("`ssid` must be non-empty string".to_string());
    }
    if let Some(password) = data.password.as_ref() {
        if password.len() == 0 {
            return Err("`password` must be non-empty string".to_string());
        }
    }

    Ok(())
}

fn invalid_addr(addr: &Ipv4Addr) -> bool {
    addr.is_broadcast()
        || addr.is_link_local()
        || addr.is_loopback()
        || addr.is_multicast()
        || addr.is_unspecified()
}
