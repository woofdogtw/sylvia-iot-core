use chrono::{Duration, SubsecRound, Utc};
use laboratory::expect;
use serde_json::{Map, Value};
use tokio::runtime::Runtime;

use sylvia_iot_broker::models::device::{
    Device, DeviceModel, ListOptions, ListQueryCond, QueryCond, SortCond, SortKey, UpdateQueryCond,
    Updates,
};

/// Test `add()`.
pub fn add(runtime: &Runtime, model: &dyn DeviceModel) -> Result<(), String> {
    let now = Utc::now().trunc_subsecs(3);
    let device = Device {
        device_id: "device_id_get_none".to_string(),
        unit_id: "unit_id_get_none".to_string(),
        unit_code: None,
        network_id: "network_id_get_none".to_string(),
        network_code: "network_code_get_none".to_string(),
        network_addr: "network_addr_get_none".to_string(),
        created_at: now,
        modified_at: now,
        profile: "profile_get_none".to_string(),
        name: "name_get".to_string(),
        info: Map::<String, Value>::new(),
    };
    if let Err(e) = runtime.block_on(async { model.add(&device).await }) {
        return Err(format!("model.add() none error: {}", e));
    }

    let cond = QueryCond {
        device_id: Some(&device.device_id),
        ..Default::default()
    };
    let get_device = match runtime.block_on(async { model.get(&cond).await }) {
        Err(e) => return Err(format!("model.get() none error: {}", e)),
        Ok(device) => match device {
            None => return Err("should get none one".to_string()),
            Some(device) => device,
        },
    };
    expect(get_device).to_equal(device)?;

    let mut info = Map::<String, Value>::new();
    info.insert("boolean".to_string(), Value::Bool(true));
    info.insert("string".to_string(), Value::String("string".to_string()));
    info.insert("number".to_string(), Value::Number(1.into()));
    let info_object_array = vec![Value::String("array".to_string())];
    let mut info_object = Map::<String, Value>::new();
    info_object.insert("array".to_string(), Value::Array(info_object_array));
    info.insert("object".to_string(), Value::Object(info_object));
    let device = Device {
        device_id: "device_id_get_some".to_string(),
        unit_id: "unit_id_get_some".to_string(),
        unit_code: Some("unit_code_some".to_string()),
        network_id: "network_id_get_some".to_string(),
        network_code: "network_code_get_some".to_string(),
        network_addr: "network_addr_get_some".to_string(),
        created_at: now,
        modified_at: now,
        profile: "profile_get_some".to_string(),
        name: "name_get".to_string(),
        info: Map::<String, Value>::new(),
    };
    if let Err(e) = runtime.block_on(async { model.add(&device).await }) {
        return Err(format!("model.add() some error: {}", e));
    }

    let cond = QueryCond {
        device_id: Some(&device.device_id),
        ..Default::default()
    };
    let get_device = match runtime.block_on(async { model.get(&cond).await }) {
        Err(e) => return Err(format!("model.get() some error: {}", e)),
        Ok(device) => match device {
            None => return Err("should get some one".to_string()),
            Some(device) => device,
        },
    };
    expect(get_device).to_equal(device)
}

/// Test `add()` with duplicate key.
pub fn add_dup(runtime: &Runtime, model: &dyn DeviceModel) -> Result<(), String> {
    let now = Utc::now().trunc_subsecs(3);
    let mut device = Device {
        device_id: "device_id_add".to_string(),
        unit_id: "unit_id_add".to_string(),
        unit_code: None,
        network_id: "network_id_add".to_string(),
        network_code: "network_code_add".to_string(),
        network_addr: "network_addr_add".to_string(),
        created_at: now,
        modified_at: now,
        profile: "profile_add".to_string(),
        name: "name_add".to_string(),
        info: Map::<String, Value>::new(),
    };
    if let Err(e) = runtime.block_on(async { model.add(&device).await }) {
        return Err(format!("model.add() error: {}", e));
    }
    device.unit_code = Some("unit_code_another".to_string());
    device.network_code = "network_code_another".to_string();
    device.network_addr = "network_addr_another".to_string();
    if let Ok(_) = runtime.block_on(async { model.add(&device).await }) {
        return Err("model.add() duplicate device_id should error".to_string());
    }
    device.device_id = "device_id_another".to_string();
    device.unit_code = None;
    device.network_code = "network_code_add".to_string();
    device.network_addr = "network_addr_add".to_string();
    if let Ok(_) = runtime.block_on(async { model.add(&device).await }) {
        return Err(
            "model.add() duplicate unit_code-network_code-network_addr should error".to_string(),
        );
    }
    device.unit_code = Some("unit_code_another".to_string());
    if let Err(_) = runtime.block_on(async { model.add(&device).await }) {
        return Err("model.add() should not duplicate using another unit_code".to_string());
    }
    device.device_id = "device_id_another2".to_string();
    device.network_code = "network_code_another2".to_string();
    if let Err(_) = runtime.block_on(async { model.add(&device).await }) {
        return Err("model.add() should not duplicate using another network_code".to_string());
    }
    device.device_id = "device_id_another3".to_string();
    device.network_addr = "network_addr_another3".to_string();
    if let Err(_) = runtime.block_on(async { model.add(&device).await }) {
        return Err("model.add() should not duplicate using another network_addr".to_string());
    }
    Ok(())
}

/// Test `add_bulk()`.
pub fn add_bulk(runtime: &Runtime, model: &dyn DeviceModel) -> Result<(), String> {
    let mut devices = vec![];
    for i in 0..100 {
        let now = Utc::now().trunc_subsecs(3);
        let device = Device {
            device_id: format!("device_id{:#03}", i),
            unit_id: "unit_id_bulk".to_string(),
            unit_code: None,
            network_id: "network_id_bulk".to_string(),
            network_code: "network_code_bulk".to_string(),
            network_addr: format!("network_addr_bulk{:#03}", i),
            created_at: now,
            modified_at: now,
            profile: format!("profile_add{:#03}", i),
            name: format!("name_add{:#03}", i),
            info: Map::<String, Value>::new(),
        };
        devices.push(device);
    }
    if let Err(e) = runtime.block_on(async { model.add_bulk(&devices).await }) {
        return Err(format!("model.add_bulk() error: {}", e));
    }
    match runtime.block_on(async { model.count(&ListQueryCond::default()).await }) {
        Err(e) => return Err(format!("model.count() after add_bulk error: {}", e)),
        Ok(count) => {
            if count as usize != devices.len() {
                return Err(format!(
                    "add_bulk() count wrong: {}/{}",
                    count,
                    devices.len()
                ));
            }
        }
    }

    let now = Utc::now().trunc_subsecs(3);
    devices.push(Device {
        device_id: "device_id100".to_string(),
        unit_id: "unit_id_bulk".to_string(),
        unit_code: Some("unit_code_bulk".to_string()),
        network_id: "network_id_bulk".to_string(),
        network_code: "network_code_bulk".to_string(),
        network_addr: "network_addr_bulk100".to_string(),
        created_at: now,
        modified_at: now,
        profile: "profile_add100".to_string(),
        name: "name_add100".to_string(),
        info: Map::<String, Value>::new(),
    });
    if let Err(e) = runtime.block_on(async { model.add_bulk(&devices).await }) {
        return Err(format!("model.add_bulk() with duplicate error: {}", e));
    }
    let cond = ListQueryCond::default();
    let sort = vec![SortCond {
        key: SortKey::NetworkAddr,
        asc: true,
    }];
    let opts = ListOptions {
        cond: &cond,
        offset: None,
        limit: None,
        sort: Some(sort.as_slice()),
        cursor_max: None,
    };
    match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => {
            return Err(format!(
                "model.list() after add_bulk duplicate error: {}",
                e
            ))
        }
        Ok((items, _)) => {
            let mut i = 0;
            for item in items.iter() {
                if !item.profile.eq(&format!("profile_add{:#03}", i))
                    || !item.name.eq(&format!("name_add{:#03}", i))
                {
                    return Err(format!("model.add_bulk() content error"));
                }
                i += 1;
            }
        }
    }

    Ok(())
}

/// Test `del()` by specifying a device ID.
pub fn del_by_device_id(runtime: &Runtime, model: &dyn DeviceModel) -> Result<(), String> {
    let now = Utc::now().trunc_subsecs(3);
    let device_id_del = "device_id_del";
    let device_id_not_del = "device_id_not_del";
    let mut device = Device {
        device_id: device_id_del.to_string(),
        unit_id: "unit_id_del".to_string(),
        unit_code: None,
        network_id: "network_id_del".to_string(),
        network_code: "network_code_del".to_string(),
        network_addr: "network_addr_del".to_string(),
        created_at: now,
        modified_at: now,
        profile: "profile_del".to_string(),
        name: "name_del".to_string(),
        info: Map::<String, Value>::new(),
    };
    let mut cond = QueryCond {
        device_id: Some(device_id_del),
        ..Default::default()
    };
    if let Err(e) = runtime.block_on(async {
        model.add(&device).await?;
        device.device_id = device_id_not_del.to_string();
        device.network_addr = "network_addr_not_del".to_string();
        model.add(&device).await?;
        model.del(&cond).await
    }) {
        return Err(format!("model.add/del error: {}", e));
    }
    match runtime.block_on(async { model.get(&cond).await }) {
        Err(e) => return Err(format!("model.get() error: {}", e)),
        Ok(device) => match device {
            None => (),
            Some(_) => return Err("delete fail".to_string()),
        },
    }
    cond.device_id = Some(device_id_not_del);
    match runtime.block_on(async { model.get(&cond).await }) {
        Err(e) => Err(format!("model.get() not delete one error: {}", e)),
        Ok(device) => match device {
            None => Err("delete wrong one".to_string()),
            Some(_) => Ok(()),
        },
    }
}

/// Test `del()` twice.
pub fn del_twice(runtime: &Runtime, model: &dyn DeviceModel) -> Result<(), String> {
    let now = Utc::now().trunc_subsecs(3);
    let device_id_del = "device_id_del";
    let device = Device {
        device_id: device_id_del.to_string(),
        unit_id: "unit_id_del".to_string(),
        unit_code: None,
        network_id: "network_id_del".to_string(),
        network_code: "network_code_del".to_string(),
        network_addr: "network_addr_del".to_string(),
        created_at: now,
        modified_at: now,
        profile: "profile_del".to_string(),
        name: "name_del".to_string(),
        info: Map::<String, Value>::new(),
    };
    let cond = QueryCond {
        device_id: Some(device_id_del),
        ..Default::default()
    };
    if let Err(e) = runtime.block_on(async {
        model.add(&device).await?;
        model.del(&cond).await?;
        model.del(&cond).await
    }) {
        return Err(format!("model.add/del error: {}", e));
    }
    Ok(())
}

/// Test `del()` by specifying a unit ID.
pub fn del_by_unit_id(runtime: &Runtime, model: &dyn DeviceModel) -> Result<(), String> {
    let now = Utc::now().trunc_subsecs(3);
    let device_id_del1 = "device_id_del1";
    let device_id_del2 = "device_id_del2";
    let device_id_not_del = "device_id_not_del";
    let device_id_not_del2 = "device_id_not_del2";
    let mut device = Device {
        device_id: device_id_del1.to_string(),
        unit_id: "unit_id_del".to_string(),
        unit_code: None,
        network_id: "network_id_del".to_string(),
        network_code: "network_code_del".to_string(),
        network_addr: "network_addr_del1".to_string(),
        created_at: now,
        modified_at: now,
        profile: "profile_del".to_string(),
        name: "name_del".to_string(),
        info: Map::<String, Value>::new(),
    };
    let cond = QueryCond {
        unit_id: Some("unit_id_del"),
        ..Default::default()
    };
    if let Err(e) = runtime.block_on(async {
        model.add(&device).await?;
        device.device_id = device_id_del2.to_string();
        device.network_addr = "network_addr_del2".to_string();
        model.add(&device).await?;
        device.device_id = device_id_not_del.to_string();
        device.network_addr = "network_addr_not_del".to_string();
        device.unit_id = "unit_id_not_del".to_string();
        model.add(&device).await?;
        device.device_id = device_id_not_del2.to_string();
        device.network_addr = "network_addr_not_del2".to_string();
        model.add(&device).await?;
        model.del(&cond).await
    }) {
        return Err(format!("model.add/del error: {}", e));
    }
    let mut cond = QueryCond {
        device_id: Some(device_id_del1),
        ..Default::default()
    };
    match runtime.block_on(async { model.get(&cond).await }) {
        Err(e) => return Err(format!("model.get() delete device1 error: {}", e)),
        Ok(device) => match device {
            None => (),
            Some(_) => return Err("delete device1 fail".to_string()),
        },
    }
    cond.device_id = Some(device_id_del2);
    match runtime.block_on(async { model.get(&cond).await }) {
        Err(e) => return Err(format!("model.get() delete device2 error: {}", e)),
        Ok(device) => match device {
            None => (),
            Some(_) => return Err("delete device2 fail".to_string()),
        },
    }
    cond.device_id = Some(device_id_not_del);
    match runtime.block_on(async { model.get(&cond).await }) {
        Err(e) => return Err(format!("model.get() not delete one error: {}", e)),
        Ok(device) => match device {
            None => return Err("delete wrong one".to_string()),
            Some(_) => (),
        },
    }
    cond.device_id = Some(device_id_not_del2);
    match runtime.block_on(async { model.get(&cond).await }) {
        Err(e) => Err(format!("model.get() not delete one2 error: {}", e)),
        Ok(device) => match device {
            None => Err("delete wrong one2".to_string()),
            Some(_) => Ok(()),
        },
    }
}

/// Test `del()` by specifying a pair of unit ID and device ID.
pub fn del_by_unit_device(runtime: &Runtime, model: &dyn DeviceModel) -> Result<(), String> {
    let now = Utc::now().trunc_subsecs(3);
    let device_id_del = "device_id_del";
    let device_id_not_del = "device_id_not_del";
    let mut device = Device {
        device_id: device_id_del.to_string(),
        unit_id: "unit_id_del".to_string(),
        unit_code: None,
        network_id: "network_id_del".to_string(),
        network_code: "network_code_del".to_string(),
        network_addr: "network_addr_del1".to_string(),
        created_at: now,
        modified_at: now,
        profile: "profile_del".to_string(),
        name: "name_del".to_string(),
        info: Map::<String, Value>::new(),
    };
    let mut cond = QueryCond {
        device_id: Some(device_id_del),
        unit_id: Some("unit_id_del"),
        ..Default::default()
    };
    if let Err(e) = runtime.block_on(async {
        model.add(&device).await?;
        device.device_id = device_id_not_del.to_string();
        device.network_addr = "network_addr_del2".to_string();
        model.add(&device).await?;
        model.del(&cond).await
    }) {
        return Err(format!("model.add/del error: {}", e));
    }
    match runtime.block_on(async { model.get(&cond).await }) {
        Err(e) => return Err(format!("model.get() error: {}", e)),
        Ok(device) => match device {
            None => (),
            Some(_) => return Err("delete fail".to_string()),
        },
    }
    cond.device_id = Some(device_id_not_del);
    match runtime.block_on(async { model.get(&cond).await }) {
        Err(e) => Err(format!("model.get() not delete one error: {}", e)),
        Ok(device) => match device {
            None => Err("delete wrong one".to_string()),
            Some(_) => Ok(()),
        },
    }
}

/// Test `del()` by specifying a network ID.
pub fn del_by_network_id(runtime: &Runtime, model: &dyn DeviceModel) -> Result<(), String> {
    let now = Utc::now().trunc_subsecs(3);
    let device_id_del1 = "device_id_del1";
    let device_id_del2 = "device_id_del2";
    let device_id_not_del = "device_id_not_del";
    let device_id_not_del2 = "device_id_not_del2";
    let mut device = Device {
        device_id: device_id_del1.to_string(),
        unit_id: "unit_id_del".to_string(),
        unit_code: None,
        network_id: "network_id_del".to_string(),
        network_code: "network_code_del".to_string(),
        network_addr: "network_addr_del1".to_string(),
        created_at: now,
        modified_at: now,
        profile: "profile_del".to_string(),
        name: "name_del".to_string(),
        info: Map::<String, Value>::new(),
    };
    let cond = QueryCond {
        network_id: Some("network_id_del"),
        ..Default::default()
    };
    if let Err(e) = runtime.block_on(async {
        model.add(&device).await?;
        device.device_id = device_id_del2.to_string();
        device.network_addr = "network_addr_del2".to_string();
        model.add(&device).await?;
        device.device_id = device_id_not_del.to_string();
        device.network_addr = "network_addr_not_del".to_string();
        device.network_id = "network_id_not_del".to_string();
        model.add(&device).await?;
        device.device_id = device_id_not_del2.to_string();
        device.network_addr = "network_addr_not_del2".to_string();
        model.add(&device).await?;
        model.del(&cond).await
    }) {
        return Err(format!("model.add/del error: {}", e));
    }
    let mut cond = QueryCond {
        device_id: Some(device_id_del1),
        ..Default::default()
    };
    match runtime.block_on(async { model.get(&cond).await }) {
        Err(e) => return Err(format!("model.get() delete device1 error: {}", e)),
        Ok(device) => match device {
            None => (),
            Some(_) => return Err("delete device1 fail".to_string()),
        },
    };
    cond.device_id = Some(device_id_del2);
    match runtime.block_on(async { model.get(&cond).await }) {
        Err(e) => return Err(format!("model.get() delete device2 error: {}", e)),
        Ok(device) => match device {
            None => (),
            Some(_) => return Err("delete device2 fail".to_string()),
        },
    }
    cond.device_id = Some(device_id_not_del);
    match runtime.block_on(async { model.get(&cond).await }) {
        Err(e) => return Err(format!("model.get() not delete one error: {}", e)),
        Ok(device) => match device {
            None => return Err("delete wrong one".to_string()),
            Some(_) => (),
        },
    }
    cond.device_id = Some(device_id_not_del2);
    match runtime.block_on(async { model.get(&cond).await }) {
        Err(e) => Err(format!("model.get() not delete none one error: {}", e)),
        Ok(device) => match device {
            None => Err("delete wrong none one".to_string()),
            Some(_) => Ok(()),
        },
    }
}

/// Test `del()` by specifying network addresses.
pub fn del_by_network_addrs(runtime: &Runtime, model: &dyn DeviceModel) -> Result<(), String> {
    let mut devices = vec![];
    for i in 0..100 {
        let now = Utc::now().trunc_subsecs(3);
        let device = Device {
            device_id: format!("device_id{:#03}", i),
            unit_id: "unit_id_bulk".to_string(),
            unit_code: None,
            network_id: "network_id_bulk".to_string(),
            network_code: "network_code_bulk".to_string(),
            network_addr: format!("network_del{:#03}", i),
            created_at: now,
            modified_at: now,
            profile: "profile_del".to_string(),
            name: format!("name_del{:#03}", i),
            info: Map::<String, Value>::new(),
        };
        devices.push(device);
    }
    if let Err(e) = runtime.block_on(async { model.add_bulk(&devices).await }) {
        return Err(format!("model.add_bulk() error: {}", e));
    }

    let mut addrs = vec![];
    for i in 50..100 {
        addrs.push(format!("network_del{:#03}", i));
    }
    let del_addrs: Vec<&str> = addrs.iter().map(|x| x.as_str()).collect();

    let cond = QueryCond {
        unit_id: Some("unit_id"),
        network_addrs: Some(&del_addrs),
        ..Default::default()
    };
    if let Err(e) = runtime.block_on(async { model.del(&cond).await }) {
        return Err(format!("model.del() wrong unit error: {}", e));
    }
    match runtime.block_on(async { model.count(&ListQueryCond::default()).await }) {
        Err(e) => return Err(format!("model.get() after delete wrong unit error: {}", e)),
        Ok(count) => {
            if count as usize != 100 {
                return Err(format!("del() count wrong: {}/100", count));
            }
        }
    }

    let cond = QueryCond {
        unit_id: Some("unit_id_bulk"),
        network_addrs: Some(&del_addrs),
        ..Default::default()
    };
    if let Err(e) = runtime.block_on(async { model.del(&cond).await }) {
        return Err(format!("model.del() unit error: {}", e));
    }
    let cond = ListQueryCond::default();
    let sort = vec![SortCond {
        key: SortKey::NetworkAddr,
        asc: true,
    }];
    let opts = ListOptions {
        cond: &cond,
        offset: None,
        limit: None,
        sort: Some(sort.as_slice()),
        cursor_max: None,
    };
    match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("model.list() after delete error: {}", e)),
        Ok((items, _)) => {
            if items.len() != 50 {
                return Err(format!("model.del() count error: {}/50", items.len()));
            }
            let mut i = 0;
            for item in items.iter() {
                if !item.name.eq(&format!("name_del{:#03}", i)) {
                    return Err(format!("model.del() content error"));
                }
                i += 1;
            }
        }
    }

    Ok(())
}

/// Test `update()`.
pub fn update(runtime: &Runtime, model: &dyn DeviceModel) -> Result<(), String> {
    let now = Utc::now().trunc_subsecs(3);
    let device_id_update = "device_id_update";
    let unit_id_update = "unit_id_update";
    let device = Device {
        device_id: device_id_update.to_string(),
        unit_id: unit_id_update.to_string(),
        unit_code: None,
        network_id: "network_id_update".to_string(),
        network_code: "network_code_update".to_string(),
        network_addr: "network_addr_update".to_string(),
        created_at: now,
        modified_at: now,
        profile: "profile_update".to_string(),
        name: "name_update".to_string(),
        info: Map::<String, Value>::new(),
    };
    if let Err(e) = runtime.block_on(async { model.add(&device).await }) {
        return Err(format!("model.add() error: {}", e));
    }

    let get_cond = QueryCond {
        device_id: Some(device_id_update),
        unit_id: Some(unit_id_update),
        ..Default::default()
    };
    let update_cond = UpdateQueryCond {
        device_id: device_id_update,
    };

    // Update only one field.
    let now = now + Duration::milliseconds(1);
    let updates = Updates {
        modified_at: Some(now),
        ..Default::default()
    };
    if let Err(e) = runtime.block_on(async { model.update(&update_cond, &updates).await }) {
        return Err(format!("model.update() one error: {}", e));
    }
    let get_device = match runtime.block_on(async { model.get(&get_cond).await }) {
        Err(e) => return Err(format!("model.get() one error: {}", e)),
        Ok(device) => match device {
            None => return Err(format!("model.get() one should get one")),
            Some(device) => device,
        },
    };
    expect(get_device.device_id.as_str()).to_equal(device.device_id.as_str())?;
    expect(get_device.unit_id.as_str()).to_equal(device.unit_id.as_str())?;
    expect(get_device.unit_code.as_ref()).to_equal(device.unit_code.as_ref())?;
    expect(get_device.network_id.as_str()).to_equal(device.network_id.as_str())?;
    expect(get_device.network_code.as_str()).to_equal(device.network_code.as_str())?;
    expect(get_device.network_addr.as_str()).to_equal(device.network_addr.as_str())?;
    expect(get_device.created_at).to_equal(device.created_at)?;
    expect(get_device.modified_at).to_equal(now)?;
    expect(get_device.profile.as_str()).to_equal(device.profile.as_str())?;
    expect(get_device.name.as_str()).to_equal(device.name.as_str())?;
    expect(get_device.info).to_equal(device.info.clone())?;

    // Update all fields.
    let now = now + Duration::milliseconds(1);
    let mut info = Map::<String, Value>::new();
    info.insert("key".to_string(), Value::String("value".to_string()));
    let updates = Updates {
        modified_at: Some(now),
        profile: Some("profile_update_all"),
        name: Some("name_update_all"),
        info: Some(&info),
    };
    if let Err(e) = runtime.block_on(async { model.update(&update_cond, &updates).await }) {
        return Err(format!("model.update() all error: {}", e));
    }
    let get_device = match runtime.block_on(async { model.get(&get_cond).await }) {
        Err(e) => return Err(format!("model.get() all error: {}", e)),
        Ok(device) => match device {
            None => return Err(format!("model.get() all should get one")),
            Some(device) => device,
        },
    };
    expect(get_device.device_id.as_str()).to_equal(device.device_id.as_str())?;
    expect(get_device.unit_id.as_str()).to_equal(device.unit_id.as_str())?;
    expect(get_device.unit_code.as_ref()).to_equal(device.unit_code.as_ref())?;
    expect(get_device.network_id.as_str()).to_equal(device.network_id.as_str())?;
    expect(get_device.network_code.as_str()).to_equal(device.network_code.as_str())?;
    expect(get_device.network_addr.as_str()).to_equal(device.network_addr.as_str())?;
    expect(get_device.created_at).to_equal(device.created_at)?;
    expect(get_device.modified_at).to_equal(now)?;
    expect(get_device.profile.as_str()).to_equal("profile_update_all")?;
    expect(get_device.name.as_str()).to_equal("name_update_all")?;
    expect(get_device.info).to_equal(info)?;

    // Update all fields back to None.
    let now = now + Duration::milliseconds(1);
    let info = Map::<String, Value>::new();
    let updates = Updates {
        modified_at: Some(now),
        profile: Some(""),
        name: Some(""),
        info: Some(&info),
    };
    if let Err(e) = runtime.block_on(async { model.update(&update_cond, &updates).await }) {
        return Err(format!("model.update() none error: {}", e));
    }
    let get_device = match runtime.block_on(async { model.get(&get_cond).await }) {
        Err(e) => return Err(format!("model.get() none error: {}", e)),
        Ok(device) => match device {
            None => return Err(format!("model.get() none should get one")),
            Some(device) => device,
        },
    };
    expect(get_device.device_id.as_str()).to_equal(device.device_id.as_str())?;
    expect(get_device.unit_id.as_str()).to_equal(device.unit_id.as_str())?;
    expect(get_device.unit_code.as_ref()).to_equal(device.unit_code.as_ref())?;
    expect(get_device.network_id.as_str()).to_equal(device.network_id.as_str())?;
    expect(get_device.network_code.as_str()).to_equal(device.network_code.as_str())?;
    expect(get_device.network_addr.as_str()).to_equal(device.network_addr.as_str())?;
    expect(get_device.created_at).to_equal(device.created_at)?;
    expect(get_device.modified_at).to_equal(now)?;
    expect(get_device.profile.as_str()).to_equal("")?;
    expect(get_device.name.as_str()).to_equal("")?;
    expect(get_device.info).to_equal(info)
}

/// Test `update()` with a non-exist condition.
pub fn update_not_exist(runtime: &Runtime, model: &dyn DeviceModel) -> Result<(), String> {
    let cond = UpdateQueryCond {
        device_id: "device_id_not_exist",
    };
    let updates = Updates {
        modified_at: Some(Utc::now()),
        ..Default::default()
    };
    if let Err(e) = runtime.block_on(async { model.update(&cond, &updates).await }) {
        return Err(format!("model.update() error: {}", e));
    }
    Ok(())
}

/// Test `update()` with invalid update content.
pub fn update_invalid(runtime: &Runtime, model: &dyn DeviceModel) -> Result<(), String> {
    let cond = UpdateQueryCond {
        device_id: "device_id",
    };
    let updates = Updates {
        modified_at: None,
        profile: None,
        name: None,
        info: None,
    };
    if let Err(e) = runtime.block_on(async { model.update(&cond, &updates).await }) {
        return Err(format!("model.update() error: {}", e));
    }
    Ok(())
}

/// Test `count()`.
pub fn count(runtime: &Runtime, model: &dyn DeviceModel) -> Result<(), String> {
    let now = Utc::now().trunc_subsecs(3);
    let mut device = Device {
        device_id: "device_id_count1_1".to_string(),
        unit_id: "unit_id_count".to_string(),
        unit_code: None,
        network_id: "network_id_count".to_string(),
        network_code: "network_code_count".to_string(),
        network_addr: "network_addr_count1_1".to_string(),
        created_at: now,
        modified_at: now,
        profile: "profile_count_1".to_string(),
        name: "name_count_1".to_string(),
        info: Map::<String, Value>::new(),
    };
    if let Err(e) = runtime.block_on(async {
        model.add(&device).await?;
        device.device_id = "device_id_count1_2".to_string();
        device.network_addr = "network_addr_count1_2".to_string();
        device.name = "name_count1_2".to_string();
        model.add(&device).await?;
        device.device_id = "device_id_count2_1".to_string();
        device.network_id = "network_id_count2".to_string();
        device.network_code = "network_code_count2".to_string();
        device.network_addr = "network_addr_count2_1".to_string();
        device.profile = "profile_count_2".to_string();
        device.name = "name_count2_1".to_string();
        model.add(&device).await?;
        device.device_id = "device_id_count3_1".to_string();
        device.unit_id = "unit_id_count3".to_string();
        device.network_id = "network_id_count3".to_string();
        device.network_code = "network_code_count3".to_string();
        device.network_addr = "network_addr_count3_1".to_string();
        device.profile = "profile_count_1".to_string();
        device.name = "name_count_1".to_string();
        model.add(&device).await
    }) {
        return Err(format!("model.add() error: {}", e));
    }

    let cond = ListQueryCond {
        ..Default::default()
    };
    let count = match runtime.block_on(async { model.count(&cond).await }) {
        Err(e) => return Err(format!("count all result error: {}", e)),
        Ok(count) => count,
    };
    expect(count).to_equal(4)?;

    let cond = ListQueryCond {
        device_id: Some("device_id_count1_1"),
        ..Default::default()
    };
    let count = match runtime.block_on(async { model.count(&cond).await }) {
        Err(e) => return Err(format!("count device_id result error: {}", e)),
        Ok(count) => count,
    };
    expect(count).to_equal(1)?;

    let cond = ListQueryCond {
        unit_id: Some("unit_id_count"),
        ..Default::default()
    };
    let count = match runtime.block_on(async { model.count(&cond).await }) {
        Err(e) => return Err(format!("count unit_id result error: {}", e)),
        Ok(count) => count,
    };
    expect(count).to_equal(3)?;

    let cond = ListQueryCond {
        device_id: Some("device_id_count1_1"),
        unit_id: Some("unit_id_count"),
        ..Default::default()
    };
    let count = match runtime.block_on(async { model.count(&cond).await }) {
        Err(e) => return Err(format!("count device-unit result error: {}", e)),
        Ok(count) => count,
    };
    expect(count).to_equal(1)?;

    let cond = ListQueryCond {
        device_id: Some("device_id_count3_1"),
        unit_id: Some("unit_id_count"),
        ..Default::default()
    };
    let count = match runtime.block_on(async { model.count(&cond).await }) {
        Err(e) => return Err(format!("count device3-unit result error: {}", e)),
        Ok(count) => count,
    };
    expect(count).to_equal(0)?;

    let cond = ListQueryCond {
        network_id: Some("network_id_count"),
        ..Default::default()
    };
    let count = match runtime.block_on(async { model.count(&cond).await }) {
        Err(e) => return Err(format!("count network_id result error: {}", e)),
        Ok(count) => count,
    };
    expect(count).to_equal(2)?;

    let cond = ListQueryCond {
        device_id: Some("device_id_count1_1"),
        network_id: Some("network_id_count"),
        ..Default::default()
    };
    let count = match runtime.block_on(async { model.count(&cond).await }) {
        Err(e) => return Err(format!("count device-network result error: {}", e)),
        Ok(count) => count,
    };
    expect(count).to_equal(1)?;

    let cond = ListQueryCond {
        device_id: Some("device_id_count3_1"),
        network_id: Some("network_id_count"),
        ..Default::default()
    };
    let count = match runtime.block_on(async { model.count(&cond).await }) {
        Err(e) => return Err(format!("count device3-network result error: {}", e)),
        Ok(count) => count,
    };
    expect(count).to_equal(0)?;

    let cond = ListQueryCond {
        network_code: Some("network_code_count"),
        ..Default::default()
    };
    let count = match runtime.block_on(async { model.count(&cond).await }) {
        Err(e) => return Err(format!("count code result error: {}", e)),
        Ok(count) => count,
    };
    expect(count).to_equal(2)?;

    let cond = ListQueryCond {
        network_code: Some("network_code_count"),
        network_addr: Some("network_addr_count1_1"),
        ..Default::default()
    };
    let count = match runtime.block_on(async { model.count(&cond).await }) {
        Err(e) => return Err(format!("count code-addr result error: {}", e)),
        Ok(count) => count,
    };
    expect(count).to_equal(1)?;

    let cond = ListQueryCond {
        network_code: Some("network_code_count"),
        network_addr: Some("network_addr_count3_1"),
        ..Default::default()
    };
    let count = match runtime.block_on(async { model.count(&cond).await }) {
        Err(e) => return Err(format!("count code-addr result error: {}", e)),
        Ok(count) => count,
    };
    expect(count).to_equal(0)?;

    let addrs = vec![
        "network_addr_count1_1",
        "network_addr_count1_2",
        "network_addr_count1",
    ];
    let cond = ListQueryCond {
        network_addrs: Some(&addrs),
        ..Default::default()
    };
    let count = match runtime.block_on(async { model.count(&cond).await }) {
        Err(e) => return Err(format!("count addrs result error: {}", e)),
        Ok(count) => count,
    };
    expect(count).to_equal(2)?;

    let cond = ListQueryCond {
        unit_id: Some("_1"),
        network_addrs: Some(&addrs),
        ..Default::default()
    };
    let count = match runtime.block_on(async { model.count(&cond).await }) {
        Err(e) => return Err(format!("count unit-addrs-not-match result error: {}", e)),
        Ok(count) => count,
    };
    expect(count).to_equal(0)?;

    let cond = ListQueryCond {
        profile: Some("profile_count_1"),
        ..Default::default()
    };
    let count = match runtime.block_on(async { model.count(&cond).await }) {
        Err(e) => return Err(format!("count profile result error: {}", e)),
        Ok(count) => count,
    };
    expect(count).to_equal(3)?;

    let cond = ListQueryCond {
        name_contains: Some("count_1"),
        ..Default::default()
    };
    let count = match runtime.block_on(async { model.count(&cond).await }) {
        Err(e) => return Err(format!("count name result error: {}", e)),
        Ok(count) => count,
    };
    expect(count).to_equal(2)?;

    let cond = ListQueryCond {
        name_contains: Some("count_1"),
        unit_id: Some("unit_id_count"),
        ..Default::default()
    };
    let count = match runtime.block_on(async { model.count(&cond).await }) {
        Err(e) => return Err(format!("count name-unit result error: {}", e)),
        Ok(count) => count,
    };
    expect(count).to_equal(1)?;

    let cond = ListQueryCond {
        name_contains: Some("_2"),
        unit_id: Some("unit_id_count3"),
        ..Default::default()
    };
    let count = match runtime.block_on(async { model.count(&cond).await }) {
        Err(e) => return Err(format!("count name-unit-not-match result error: {}", e)),
        Ok(count) => count,
    };
    expect(count).to_equal(0)
}

/// Test `list()`.
pub fn list(runtime: &Runtime, model: &dyn DeviceModel) -> Result<(), String> {
    let now = Utc::now().trunc_subsecs(3);
    let mut device = Device {
        device_id: "device_id_list1_1".to_string(),
        unit_id: "unit_id_list".to_string(),
        unit_code: None,
        network_id: "network_id_list".to_string(),
        network_code: "network_code_list".to_string(),
        network_addr: "network_addr_list1_1".to_string(),
        created_at: now,
        modified_at: now,
        profile: "profile_list_1".to_string(),
        name: "name_list_1".to_string(),
        info: Map::<String, Value>::new(),
    };
    if let Err(e) = runtime.block_on(async {
        model.add(&device).await?;
        device.device_id = "device_id_list1_2".to_string();
        device.network_addr = "network_addr_list1_2".to_string();
        device.name = "name_list1_2".to_string();
        model.add(&device).await?;
        device.device_id = "device_id_list2_1".to_string();
        device.network_id = "network_id_list2".to_string();
        device.network_code = "network_code_list2".to_string();
        device.network_addr = "network_addr_list2_1".to_string();
        device.profile = "profile_list_2".to_string();
        device.name = "name_list2_1".to_string();
        model.add(&device).await?;
        device.device_id = "device_id_list3_1".to_string();
        device.unit_id = "unit_id_list3".to_string();
        device.unit_code = Some("unit_code_list3".to_string());
        device.network_id = "network_id_list3".to_string();
        device.network_code = "network_code_list3".to_string();
        device.network_addr = "network_addr_list3_1".to_string();
        device.profile = "profile_list_1".to_string();
        device.name = "name\\\\%%''_list_1".to_string();
        model.add(&device).await
    }) {
        return Err(format!("model.add() error: {}", e));
    }

    let cond = ListQueryCond {
        ..Default::default()
    };
    let mut opts = ListOptions {
        cond: &cond,
        offset: None,
        limit: None,
        sort: None,
        cursor_max: None,
    };
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list all result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(4)?;

    let cond = ListQueryCond {
        device_id: Some("device_id_list1_1"),
        ..Default::default()
    };
    opts.cond = &cond;
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list device_id result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(1)?;

    let cond = ListQueryCond {
        unit_id: Some("unit_id_list"),
        ..Default::default()
    };
    opts.cond = &cond;
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list unit_id result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(3)?;

    let cond = ListQueryCond {
        device_id: Some("device_id_list1_1"),
        unit_id: Some("unit_id_list"),
        ..Default::default()
    };
    opts.cond = &cond;
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list device-unit result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(1)?;

    let cond = ListQueryCond {
        device_id: Some("device_id_list3_1"),
        unit_id: Some("unit_id_list"),
        ..Default::default()
    };
    opts.cond = &cond;
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list device3-unit result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(0)?;

    let cond = ListQueryCond {
        network_id: Some("network_id_list"),
        ..Default::default()
    };
    opts.cond = &cond;
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list network_id result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(2)?;

    let cond = ListQueryCond {
        device_id: Some("device_id_list1_1"),
        network_id: Some("network_id_list"),
        ..Default::default()
    };
    opts.cond = &cond;
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list device-network result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(1)?;

    let cond = ListQueryCond {
        device_id: Some("device_id_list3_1"),
        network_id: Some("network_id_list"),
        ..Default::default()
    };
    opts.cond = &cond;
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list device3-network result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(0)?;

    let cond = ListQueryCond {
        network_code: Some("network_code_list"),
        ..Default::default()
    };
    opts.cond = &cond;
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list code result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(2)?;

    let cond = ListQueryCond {
        network_code: Some("network_code_list"),
        network_addr: Some("network_addr_list1_1"),
        ..Default::default()
    };
    opts.cond = &cond;
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list code-addr result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(1)?;

    let cond = ListQueryCond {
        network_code: Some("network_code_list"),
        network_addr: Some("network_addr_list3_1"),
        ..Default::default()
    };
    opts.cond = &cond;
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list code-addr result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(0)?;

    let addrs = vec![
        "network_addr_list1_1",
        "network_addr_list1_2",
        "network_addr_list1",
    ];
    let cond = ListQueryCond {
        network_addrs: Some(&addrs),
        ..Default::default()
    };
    opts.cond = &cond;
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list addrs result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(2)?;

    let cond = ListQueryCond {
        unit_id: Some("_1"),
        network_addrs: Some(&addrs),
        ..Default::default()
    };
    opts.cond = &cond;
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list unit-addrs-not-match result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(0)?;

    let cond = ListQueryCond {
        profile: Some("profile_list_1"),
        ..Default::default()
    };
    opts.cond = &cond;
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list profile result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(3)?;

    let cond = ListQueryCond {
        name_contains: Some("list_1"),
        ..Default::default()
    };
    opts.cond = &cond;
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list name result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(2)?;

    let cond = ListQueryCond {
        name_contains: Some("list_1"),
        unit_id: Some("unit_id_list"),
        ..Default::default()
    };
    opts.cond = &cond;
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list name-unit result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(1)?;

    let cond = ListQueryCond {
        name_contains: Some("_2"),
        unit_id: Some("unit_id_list3"),
        ..Default::default()
    };
    opts.cond = &cond;
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list name-unit-not-match result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(0)?;

    let cond = ListQueryCond {
        name_contains: Some("lIsT_1"),
        ..Default::default()
    };
    opts.cond = &cond;
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list name-case result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(2)?;

    let cond = ListQueryCond {
        name_contains: Some("\\\\%%''"),
        ..Default::default()
    };
    opts.cond = &cond;
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list name-escape result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(1)
}

/// Test `list()` with sorting.
pub fn list_sort(runtime: &Runtime, model: &dyn DeviceModel) -> Result<(), String> {
    let mut now = Utc::now().trunc_subsecs(3);
    let mut device = Device {
        device_id: "device_id_list1_1".to_string(),
        unit_id: "unit_id_list".to_string(),
        unit_code: None,
        network_id: "network_id_list".to_string(),
        network_code: "network_code_list1".to_string(),
        network_addr: "network_addr_list1_1".to_string(),
        created_at: now,
        modified_at: now,
        profile: "profile_list1_1".to_string(),
        name: "name_list1_1".to_string(),
        info: Map::<String, Value>::new(),
    };
    if let Err(e) = runtime.block_on(async {
        model.add(&device).await?;
        now = now + Duration::seconds(1);
        device.device_id = "device_id_list1_2".to_string();
        device.network_addr = "network_addr_list1_2".to_string();
        device.created_at = now;
        device.modified_at = now;
        device.profile = "profile_list1_2".to_string();
        device.name = "name_list1_2".to_string();
        model.add(&device).await?;
        now = now + Duration::seconds(1);
        device.device_id = "device_id_list2_1".to_string();
        device.network_id = "network_id_list2".to_string();
        device.network_code = "network_code_list2".to_string();
        device.network_addr = "network_addr_list2_1".to_string();
        device.created_at = now;
        device.modified_at = now;
        device.profile = "profile_list2_1".to_string();
        device.name = "name_list2_1".to_string();
        model.add(&device).await?;
        now = now + Duration::seconds(1);
        device.device_id = "device_id_list3_1".to_string();
        device.unit_id = "unit_id_list3".to_string();
        device.network_id = "network_id_list3".to_string();
        device.network_code = "network_code_list3".to_string();
        device.network_addr = "network_addr_list3_1".to_string();
        device.created_at = now;
        device.modified_at = now;
        device.profile = "profile_list1_3".to_string();
        device.name = "name_list2_1".to_string();
        model.add(&device).await
    }) {
        return Err(format!("model.add() error: {}", e));
    }

    let cond = ListQueryCond {
        ..Default::default()
    };
    let sort_cond = vec![
        SortCond {
            key: SortKey::NetworkCode,
            asc: true,
        },
        SortCond {
            key: SortKey::NetworkAddr,
            asc: true,
        },
    ];
    let mut opts = ListOptions {
        cond: &cond,
        offset: None,
        limit: None,
        sort: Some(sort_cond.as_slice()),
        cursor_max: None,
    };
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list code-asc-addr-asc result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(4)?;
    expect(list[0].device_id.as_str()).to_equal("device_id_list1_1")?;
    expect(list[1].device_id.as_str()).to_equal("device_id_list1_2")?;
    expect(list[2].device_id.as_str()).to_equal("device_id_list2_1")?;
    expect(list[3].device_id.as_str()).to_equal("device_id_list3_1")?;

    let sort_cond = vec![
        SortCond {
            key: SortKey::NetworkCode,
            asc: false,
        },
        SortCond {
            key: SortKey::NetworkAddr,
            asc: true,
        },
    ];
    opts.sort = Some(sort_cond.as_slice());
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list code-desc-addr-asc result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(4)?;
    expect(list[0].device_id.as_str()).to_equal("device_id_list3_1")?;
    expect(list[1].device_id.as_str()).to_equal("device_id_list2_1")?;
    expect(list[2].device_id.as_str()).to_equal("device_id_list1_1")?;
    expect(list[3].device_id.as_str()).to_equal("device_id_list1_2")?;

    let sort_cond = vec![
        SortCond {
            key: SortKey::NetworkCode,
            asc: true,
        },
        SortCond {
            key: SortKey::NetworkAddr,
            asc: false,
        },
    ];
    opts.sort = Some(sort_cond.as_slice());
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list code-asc-addr-desc result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(4)?;
    expect(list[0].device_id.as_str()).to_equal("device_id_list1_2")?;
    expect(list[1].device_id.as_str()).to_equal("device_id_list1_1")?;
    expect(list[2].device_id.as_str()).to_equal("device_id_list2_1")?;
    expect(list[3].device_id.as_str()).to_equal("device_id_list3_1")?;

    let sort_cond = vec![
        SortCond {
            key: SortKey::NetworkCode,
            asc: false,
        },
        SortCond {
            key: SortKey::NetworkAddr,
            asc: false,
        },
    ];
    opts.sort = Some(sort_cond.as_slice());
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list code-desc-addr-desc result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(4)?;
    expect(list[0].device_id.as_str()).to_equal("device_id_list3_1")?;
    expect(list[1].device_id.as_str()).to_equal("device_id_list2_1")?;
    expect(list[2].device_id.as_str()).to_equal("device_id_list1_2")?;
    expect(list[3].device_id.as_str()).to_equal("device_id_list1_1")?;

    let sort_cond = vec![SortCond {
        key: SortKey::Profile,
        asc: true,
    }];
    opts.sort = Some(sort_cond.as_slice());
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list profile-asc result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(4)?;
    expect(list[0].device_id.as_str()).to_equal("device_id_list1_1")?;
    expect(list[1].device_id.as_str()).to_equal("device_id_list1_2")?;
    expect(list[2].device_id.as_str()).to_equal("device_id_list3_1")?;
    expect(list[3].device_id.as_str()).to_equal("device_id_list2_1")?;

    let sort_cond = vec![SortCond {
        key: SortKey::Profile,
        asc: false,
    }];
    opts.sort = Some(sort_cond.as_slice());
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list profile-desc result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(4)?;
    expect(list[0].device_id.as_str()).to_equal("device_id_list2_1")?;
    expect(list[1].device_id.as_str()).to_equal("device_id_list3_1")?;
    expect(list[2].device_id.as_str()).to_equal("device_id_list1_2")?;
    expect(list[3].device_id.as_str()).to_equal("device_id_list1_1")?;

    let sort_cond = vec![SortCond {
        key: SortKey::CreatedAt,
        asc: true,
    }];
    opts.sort = Some(sort_cond.as_slice());
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list created-asc result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(4)?;
    expect(list[0].device_id.as_str()).to_equal("device_id_list1_1")?;
    expect(list[1].device_id.as_str()).to_equal("device_id_list1_2")?;
    expect(list[2].device_id.as_str()).to_equal("device_id_list2_1")?;
    expect(list[3].device_id.as_str()).to_equal("device_id_list3_1")?;

    let sort_cond = vec![SortCond {
        key: SortKey::CreatedAt,
        asc: false,
    }];
    opts.sort = Some(sort_cond.as_slice());
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list created-desc result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(4)?;
    expect(list[0].device_id.as_str()).to_equal("device_id_list3_1")?;
    expect(list[1].device_id.as_str()).to_equal("device_id_list2_1")?;
    expect(list[2].device_id.as_str()).to_equal("device_id_list1_2")?;
    expect(list[3].device_id.as_str()).to_equal("device_id_list1_1")?;

    let sort_cond = vec![SortCond {
        key: SortKey::ModifiedAt,
        asc: true,
    }];
    opts.sort = Some(sort_cond.as_slice());
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list modified-asc result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(4)?;
    expect(list[0].device_id.as_str()).to_equal("device_id_list1_1")?;
    expect(list[1].device_id.as_str()).to_equal("device_id_list1_2")?;
    expect(list[2].device_id.as_str()).to_equal("device_id_list2_1")?;
    expect(list[3].device_id.as_str()).to_equal("device_id_list3_1")?;

    let sort_cond = vec![SortCond {
        key: SortKey::ModifiedAt,
        asc: false,
    }];
    opts.sort = Some(sort_cond.as_slice());
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list modified-desc result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(4)?;
    expect(list[0].device_id.as_str()).to_equal("device_id_list3_1")?;
    expect(list[1].device_id.as_str()).to_equal("device_id_list2_1")?;
    expect(list[2].device_id.as_str()).to_equal("device_id_list1_2")?;
    expect(list[3].device_id.as_str()).to_equal("device_id_list1_1")?;

    let sort_cond = vec![SortCond {
        key: SortKey::Name,
        asc: true,
    }];
    opts.sort = Some(sort_cond.as_slice());
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list name-asc result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(4)?;
    expect(list[0].name.as_str()).to_equal("name_list1_1")?;
    expect(list[1].name.as_str()).to_equal("name_list1_2")?;
    expect(list[2].name.as_str()).to_equal("name_list2_1")?;
    expect(list[3].name.as_str()).to_equal("name_list2_1")?;

    let sort_cond = vec![SortCond {
        key: SortKey::Name,
        asc: false,
    }];
    opts.sort = Some(sort_cond.as_slice());
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list name-desc result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(4)?;
    expect(list[0].name.as_str()).to_equal("name_list2_1")?;
    expect(list[1].name.as_str()).to_equal("name_list2_1")?;
    expect(list[2].name.as_str()).to_equal("name_list1_2")?;
    expect(list[3].name.as_str()).to_equal("name_list1_1")?;

    let sort_cond = vec![
        SortCond {
            key: SortKey::Name,
            asc: true,
        },
        SortCond {
            key: SortKey::CreatedAt,
            asc: true,
        },
    ];
    opts.sort = Some(sort_cond.as_slice());
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list name-created-asc result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(4)?;
    expect(list[0].device_id.as_str()).to_equal("device_id_list1_1")?;
    expect(list[1].device_id.as_str()).to_equal("device_id_list1_2")?;
    expect(list[2].device_id.as_str()).to_equal("device_id_list2_1")?;
    expect(list[3].device_id.as_str()).to_equal("device_id_list3_1")?;

    let sort_cond = vec![
        SortCond {
            key: SortKey::Name,
            asc: true,
        },
        SortCond {
            key: SortKey::CreatedAt,
            asc: false,
        },
    ];
    opts.sort = Some(sort_cond.as_slice());
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list name-created-desc result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(4)?;
    expect(list[0].device_id.as_str()).to_equal("device_id_list1_1")?;
    expect(list[1].device_id.as_str()).to_equal("device_id_list1_2")?;
    expect(list[2].device_id.as_str()).to_equal("device_id_list3_1")?;
    expect(list[3].device_id.as_str()).to_equal("device_id_list2_1")?;

    let sort_cond = vec![];
    opts.sort = Some(sort_cond.as_slice());
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list empty result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(4)
}

/// Test `list()` with offset/limit.
pub fn list_offset_limit(runtime: &Runtime, model: &dyn DeviceModel) -> Result<(), String> {
    let now = Utc::now().trunc_subsecs(3);
    let mut device = Device {
        device_id: "device_id_list1_1".to_string(),
        unit_id: "unit_id_list".to_string(),
        unit_code: None,
        network_id: "network_id_list".to_string(),
        network_code: "network_code_list1".to_string(),
        network_addr: "network_addr_list1_1".to_string(),
        created_at: now,
        modified_at: now,
        profile: "profile_list_1".to_string(),
        name: "name_list_1".to_string(),
        info: Map::<String, Value>::new(),
    };
    if let Err(e) = runtime.block_on(async {
        model.add(&device).await?;
        device.device_id = "device_id_list1_2".to_string();
        device.network_addr = "network_addr_list1_2".to_string();
        model.add(&device).await?;
        device.device_id = "device_id_list2_1".to_string();
        device.network_addr = "network_addr_list2_1".to_string();
        model.add(&device).await?;
        device.device_id = "device_id_list3_1".to_string();
        device.network_addr = "network_addr_list3_1".to_string();
        model.add(&device).await
    }) {
        return Err(format!("model.add() error: {}", e));
    }

    let cond = ListQueryCond {
        ..Default::default()
    };
    let sort_cond = vec![SortCond {
        key: SortKey::NetworkAddr,
        asc: true,
    }];
    let mut opts = ListOptions {
        cond: &cond,
        offset: None,
        limit: Some(3),
        sort: Some(sort_cond.as_slice()),
        cursor_max: None,
    };
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list limit-3 result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(3)?;
    expect(list[0].device_id.as_str()).to_equal("device_id_list1_1")?;
    expect(list[1].device_id.as_str()).to_equal("device_id_list1_2")?;
    expect(list[2].device_id.as_str()).to_equal("device_id_list2_1")?;

    opts.limit = Some(5);
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list limit-5 result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(4)?;
    expect(list[0].device_id.as_str()).to_equal("device_id_list1_1")?;
    expect(list[1].device_id.as_str()).to_equal("device_id_list1_2")?;
    expect(list[2].device_id.as_str()).to_equal("device_id_list2_1")?;
    expect(list[3].device_id.as_str()).to_equal("device_id_list3_1")?;

    opts.limit = None;
    opts.offset = Some(2);
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list offset-2 result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(2)?;
    expect(list[0].device_id.as_str()).to_equal("device_id_list2_1")?;
    expect(list[1].device_id.as_str()).to_equal("device_id_list3_1")?;

    opts.limit = Some(0);
    opts.offset = Some(0);
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list offset-limit0 result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(4)?;
    expect(list[0].device_id.as_str()).to_equal("device_id_list1_1")?;
    expect(list[1].device_id.as_str()).to_equal("device_id_list1_2")?;
    expect(list[2].device_id.as_str()).to_equal("device_id_list2_1")?;
    expect(list[3].device_id.as_str()).to_equal("device_id_list3_1")?;

    opts.limit = Some(3);
    opts.offset = Some(3);
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list offset-limit3 result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(1)?;
    expect(list[0].device_id.as_str()).to_equal("device_id_list3_1")
}

/// Test `list()` with cursors.
pub fn list_cursor(runtime: &Runtime, model: &dyn DeviceModel) -> Result<(), String> {
    let now = Utc::now().trunc_subsecs(3);
    let mut device = Device {
        device_id: "device_id_list1_1".to_string(),
        unit_id: "unit_id_list".to_string(),
        unit_code: None,
        network_id: "network_id_list".to_string(),
        network_code: "network_code_list1".to_string(),
        network_addr: "network_addr_list1_1".to_string(),
        created_at: now,
        modified_at: now,
        profile: "profile_list".to_string(),
        name: "name_list".to_string(),
        info: Map::<String, Value>::new(),
    };
    if let Err(e) = runtime.block_on(async {
        model.add(&device).await?;
        device.device_id = "device_id_list1_2".to_string();
        device.network_addr = "network_addr_list1_2".to_string();
        model.add(&device).await?;
        device.device_id = "device_id_list2_1".to_string();
        device.network_addr = "network_addr_list2_1".to_string();
        model.add(&device).await?;
        device.device_id = "device_id_list3_1".to_string();
        device.network_addr = "network_addr_list3_1".to_string();
        model.add(&device).await
    }) {
        return Err(format!("model.add() error: {}", e));
    }

    let cond = ListQueryCond {
        ..Default::default()
    };
    let sort_cond = vec![SortCond {
        key: SortKey::NetworkAddr,
        asc: true,
    }];
    let mut opts = ListOptions {
        cond: &cond,
        offset: None,
        limit: None,
        sort: Some(sort_cond.as_slice()),
        cursor_max: Some(3),
    };
    let (list, cursor) = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list cursor-3-1 result error: {}", e)),
        Ok((list, cursor)) => (list, cursor),
    };
    expect(list.len()).to_equal(3)?;
    expect(list[0].device_id.as_str()).to_equal("device_id_list1_1")?;
    expect(list[1].device_id.as_str()).to_equal("device_id_list1_2")?;
    expect(list[2].device_id.as_str()).to_equal("device_id_list2_1")?;
    expect(cursor.is_some()).to_equal(true)?;
    expect(cursor.as_ref().unwrap().offset()).to_equal(3)?;
    let (list, cursor) = match runtime.block_on(async { model.list(&opts, cursor).await }) {
        Err(e) => return Err(format!("list cursor-3-2 result error: {}", e)),
        Ok((list, cursor)) => (list, cursor),
    };
    expect(list.len()).to_equal(1)?;
    expect(list[0].device_id.as_str()).to_equal("device_id_list3_1")?;
    expect(cursor.is_none()).to_equal(true)?;

    opts.limit = Some(3);
    opts.cursor_max = Some(2);
    let (list, cursor) = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list cursor-2-1 result error: {}", e)),
        Ok((list, cursor)) => (list, cursor),
    };
    expect(list.len()).to_equal(2)?;
    expect(list[0].device_id.as_str()).to_equal("device_id_list1_1")?;
    expect(list[1].device_id.as_str()).to_equal("device_id_list1_2")?;
    expect(cursor.is_some()).to_equal(true)?;
    expect(cursor.as_ref().unwrap().offset()).to_equal(2)?;
    let (list, cursor) = match runtime.block_on(async { model.list(&opts, cursor).await }) {
        Err(e) => return Err(format!("list cursor-2-2 result error: {}", e)),
        Ok((list, cursor)) => (list, cursor),
    };
    expect(list.len()).to_equal(1)?;
    expect(list[0].device_id.as_str()).to_equal("device_id_list2_1")?;
    expect(cursor.is_none()).to_equal(true)?;

    opts.limit = Some(4);
    opts.cursor_max = Some(2);
    let (list, cursor) = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list cursor-2-2-1 result error: {}", e)),
        Ok((list, cursor)) => (list, cursor),
    };
    expect(list.len()).to_equal(2)?;
    expect(list[0].device_id.as_str()).to_equal("device_id_list1_1")?;
    expect(list[1].device_id.as_str()).to_equal("device_id_list1_2")?;
    expect(cursor.is_some()).to_equal(true)?;
    expect(cursor.as_ref().unwrap().offset()).to_equal(2)?;
    let (list, cursor) = match runtime.block_on(async { model.list(&opts, cursor).await }) {
        Err(e) => return Err(format!("list cursor-2-2-2 result error: {}", e)),
        Ok((list, cursor)) => (list, cursor),
    };
    expect(list.len()).to_equal(2)?;
    expect(list[0].device_id.as_str()).to_equal("device_id_list2_1")?;
    expect(list[1].device_id.as_str()).to_equal("device_id_list3_1")?;
    expect(cursor.is_some()).to_equal(true)?;
    expect(cursor.as_ref().unwrap().offset()).to_equal(4)?;
    let (list, cursor) = match runtime.block_on(async { model.list(&opts, cursor).await }) {
        Err(e) => return Err(format!("list cursor-2-2-3 result error: {}", e)),
        Ok((list, cursor)) => (list, cursor),
    };
    expect(list.len()).to_equal(0)?;
    expect(cursor.is_none()).to_equal(true)?;

    opts.offset = Some(2);
    opts.limit = Some(3);
    opts.cursor_max = Some(5);
    let (list, cursor) = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list cursor-5 result error: {}", e)),
        Ok((list, cursor)) => (list, cursor),
    };
    expect(list.len()).to_equal(2)?;
    expect(list[0].device_id.as_str()).to_equal("device_id_list2_1")?;
    expect(list[1].device_id.as_str()).to_equal("device_id_list3_1")?;
    expect(cursor.is_none()).to_equal(true)
}
