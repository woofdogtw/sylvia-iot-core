use std::collections::HashMap;

use log::error;

use sylvia_iot_corelib::{err::ErrResp, role::Role};

use super::super::State as AppState;
use crate::models::{
    application::{Application, QueryCond as ApplicationQueryCond},
    device::{Device, QueryCond as DeviceQueryCond},
    network::{Network, QueryCond as NetworkQueryCond},
    unit::{QueryCond as UnitQueryCond, Unit},
};

/// To check if the user ID can access the unit. Choose `only_owner` to check if the user is the
/// owner or one of members.
///
/// # Errors
///
/// Returns Ok if the unit is found or not. Otherwise errors will be returned.
pub async fn check_unit(
    fn_name: &str,
    user_id: &str,
    roles: &HashMap<String, bool>,
    unit_id: &str,
    only_owner: bool,
    state: &AppState,
) -> Result<Option<Unit>, ErrResp> {
    let mut cond = UnitQueryCond {
        unit_id: Some(unit_id),
        ..Default::default()
    };
    if !Role::is_role(roles, Role::ADMIN) && !Role::is_role(roles, Role::MANAGER) {
        if only_owner {
            cond.owner_id = Some(user_id);
        } else {
            cond.member_id = Some(user_id);
        }
    }
    match state.model.unit().get(&cond).await {
        Err(e) => {
            error!("[{}] check unit error: {}", fn_name, e);
            return Err(ErrResp::ErrDb(Some(format!("check unit error: {}", e))));
        }
        Ok(unit) => Ok(unit),
    }
}

/// To check if the user ID can access the application. Choose `only_owner` to check if the user is
/// the unit owner or one of unit members.
///
/// # Errors
///
/// Returns Ok if the application is found or not. Otherwise errors will be returned.
pub async fn check_application(
    fn_name: &str,
    application_id: &str,
    user_id: &str,
    only_owner: bool, // to check if this `user_id` is the owner.
    roles: &HashMap<String, bool>,
    state: &AppState,
) -> Result<Option<Application>, ErrResp> {
    let cond = ApplicationQueryCond {
        application_id: Some(application_id),
        ..Default::default()
    };
    let application = match state.model.application().get(&cond).await {
        Err(e) => {
            error!("[{}] get error: {}", fn_name, e);
            return Err(ErrResp::ErrDb(Some(e.to_string())));
        }
        Ok(application) => match application {
            None => return Ok(None),
            Some(application) => application,
        },
    };
    if Role::is_role(roles, Role::ADMIN) || Role::is_role(roles, Role::MANAGER) {
        return Ok(Some(application));
    }
    let unit_id = application.unit_id.as_str();
    match check_unit(fn_name, user_id, &roles, unit_id, only_owner, &state).await? {
        None => Ok(None),
        Some(_) => Ok(Some(application)),
    }
}

/// To check if the user ID can access the network. Choose `only_owner` to check if the user is the
/// unit owner or one of unit members.
///
/// # Errors
///
/// Returns OK if the network is found or not. Otherwise errors will be returned.
pub async fn check_network(
    fn_name: &str,
    network_id: &str,
    user_id: &str,
    only_owner: bool, // to check if this `user_id` is the owner.
    roles: &HashMap<String, bool>,
    state: &AppState,
) -> Result<Option<Network>, ErrResp> {
    let cond = NetworkQueryCond {
        network_id: Some(network_id),
        ..Default::default()
    };
    let network = match state.model.network().get(&cond).await {
        Err(e) => {
            error!("[{}] get error: {}", fn_name, e);
            return Err(ErrResp::ErrDb(Some(e.to_string())));
        }
        Ok(network) => match network {
            None => return Ok(None),
            Some(network) => network,
        },
    };
    if Role::is_role(roles, Role::ADMIN) || Role::is_role(roles, Role::MANAGER) {
        return Ok(Some(network));
    }
    let unit_id = match network.unit_id.as_ref() {
        None => return Ok(None),
        Some(unit_id) => unit_id.as_str(),
    };
    match check_unit(fn_name, user_id, &roles, unit_id, only_owner, &state).await? {
        None => Ok(None),
        Some(_) => Ok(Some(network)),
    }
}

/// To check if the user ID can access the device. Choose `only_owner` to check if the user is the
/// unit owner or one of unit members.
///
/// # Errors
///
/// Returns OK if the device is found or not. Otherwise errors will be returned.
pub async fn check_device(
    fn_name: &str,
    device_id: &str,
    user_id: &str,
    only_owner: bool, // to check if this `user_id` is the owner.
    roles: &HashMap<String, bool>,
    state: &AppState,
) -> Result<Option<Device>, ErrResp> {
    let cond = DeviceQueryCond {
        device_id: Some(device_id),
        ..Default::default()
    };
    let device = match state.model.device().get(&cond).await {
        Err(e) => {
            error!("[{}] get error: {}", fn_name, e);
            return Err(ErrResp::ErrDb(Some(e.to_string())));
        }
        Ok(device) => match device {
            None => return Ok(None),
            Some(device) => device,
        },
    };
    let unit_id = device.unit_id.as_str();
    match check_unit(fn_name, user_id, roles, unit_id, only_owner, state).await? {
        None => Ok(None),
        Some(_) => Ok(Some(device)),
    }
}

/// To generate a key for managing managers. Empty unit for public networks.
pub fn gen_mgr_key(unit: &str, name: &str) -> String {
    format!("{}.{}", unit, name)
}
