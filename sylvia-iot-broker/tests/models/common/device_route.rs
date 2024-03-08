use chrono::{SubsecRound, TimeDelta, Utc};
use laboratory::expect;
use tokio::runtime::Runtime;

use sylvia_iot_broker::models::device_route::{
    DeviceRoute, DeviceRouteModel, ListOptions, ListQueryCond, QueryCond, SortCond, SortKey,
    UpdateQueryCond, Updates,
};

/// Test `add()`.
pub fn add(runtime: &Runtime, model: &dyn DeviceRouteModel) -> Result<(), String> {
    let now = Utc::now().trunc_subsecs(3);
    let route = DeviceRoute {
        route_id: "route_id_add".to_string(),
        unit_id: "unit_id_add".to_string(),
        unit_code: "unit_code_add".to_string(),
        application_id: "application_id_add".to_string(),
        application_code: "application_code_add".to_string(),
        device_id: "device_id_add".to_string(),
        network_id: "network_id_add".to_string(),
        network_code: "network_code_add".to_string(),
        network_addr: "network_addr_add".to_string(),
        profile: "profile_add".to_string(),
        created_at: now,
        modified_at: now,
    };
    if let Err(e) = runtime.block_on(async { model.add(&route).await }) {
        return Err(format!("model.add() error: {}", e));
    }

    let get_route = match runtime.block_on(async { model.get("route_id_add").await }) {
        Err(e) => return Err(format!("model.get() error: {}", e)),
        Ok(route) => match route {
            None => return Err("should get one".to_string()),
            Some(route) => route,
        },
    };
    expect(get_route).to_equal(route)
}

/// Test `add()` with duplicate key.
pub fn add_dup(runtime: &Runtime, model: &dyn DeviceRouteModel) -> Result<(), String> {
    let now = Utc::now().trunc_subsecs(3);
    let mut route = DeviceRoute {
        route_id: "route_id_add".to_string(),
        unit_id: "unit_id_add".to_string(),
        unit_code: "unit_code_add".to_string(),
        application_id: "application_id_add".to_string(),
        application_code: "application_code_add".to_string(),
        device_id: "device_id_add".to_string(),
        network_id: "network_id_add".to_string(),
        network_code: "network_code_add".to_string(),
        network_addr: "network_addr_add".to_string(),
        profile: "profile_add".to_string(),
        created_at: now,
        modified_at: now,
    };
    if let Err(e) = runtime.block_on(async { model.add(&route).await }) {
        return Err(format!("model.add() error: {}", e));
    }
    route.application_id = "application_id_another".to_string();
    route.device_id = "device_id_another".to_string();
    if let Ok(_) = runtime.block_on(async { model.add(&route).await }) {
        return Err("model.add() duplicate device_id should error".to_string());
    }
    route.route_id = "route_id_another".to_string();
    route.application_id = "application_id_add".to_string();
    route.device_id = "device_id_add".to_string();
    if let Ok(_) = runtime.block_on(async { model.add(&route).await }) {
        return Err("model.add() duplicate application_id-device_id should error".to_string());
    }
    Ok(())
}

/// Test `add_bulk()`.
pub fn add_bulk(runtime: &Runtime, model: &dyn DeviceRouteModel) -> Result<(), String> {
    let mut routes = vec![];
    for i in 0..100 {
        let now = Utc::now().trunc_subsecs(3);
        let route = DeviceRoute {
            route_id: format!("route_id{:#03}", i),
            unit_id: "unit_id_bulk".to_string(),
            unit_code: "unit_code_bulk".to_string(),
            application_id: "application_id_bulk".to_string(),
            application_code: "application_code_bulk".to_string(),
            device_id: format!("device_id{:#03}", i),
            network_id: "network_id_bulk".to_string(),
            network_code: "network_code_bulk".to_string(),
            network_addr: format!("network_addr_bulk{:#03}", i),
            profile: "profile_bulk".to_string(),
            created_at: now,
            modified_at: now,
        };
        routes.push(route);
    }
    if let Err(e) = runtime.block_on(async { model.add_bulk(&routes).await }) {
        return Err(format!("model.add_bulk() error: {}", e));
    }
    match runtime.block_on(async { model.count(&ListQueryCond::default()).await }) {
        Err(e) => return Err(format!("model.count() after add_bulk error: {}", e)),
        Ok(count) => {
            if count as usize != routes.len() {
                return Err(format!(
                    "add_bulk() count wrong: {}/{}",
                    count,
                    routes.len()
                ));
            }
        }
    }

    let now = Utc::now().trunc_subsecs(3);
    routes.push(DeviceRoute {
        route_id: format!("route_id100"),
        unit_id: "unit_id_bulk".to_string(),
        unit_code: "unit_code_bulk".to_string(),
        application_id: "application_id_bulk".to_string(),
        application_code: "application_code_bulk".to_string(),
        device_id: format!("device_id100"),
        network_id: "network_id_bulk".to_string(),
        network_code: "network_code_bulk".to_string(),
        network_addr: format!("network_addr_bulk100"),
        profile: "profile_bulk".to_string(),
        created_at: now,
        modified_at: now,
    });
    if let Err(e) = runtime.block_on(async { model.add_bulk(&routes).await }) {
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
                if !item.network_addr.eq(&format!("network_addr_bulk{:#03}", i)) {
                    return Err(format!("model.add_bulk() content error"));
                }
                i += 1;
            }
        }
    }

    Ok(())
}

/// Test `del()` by specifying a route ID.
pub fn del_by_route_id(runtime: &Runtime, model: &dyn DeviceRouteModel) -> Result<(), String> {
    let now = Utc::now().trunc_subsecs(3);
    let route_id_del = "route_id_del";
    let route_id_not_del = "route_id_not_del";
    let mut route = DeviceRoute {
        route_id: route_id_del.to_string(),
        unit_id: "unit_id_del".to_string(),
        unit_code: "unit_code_del".to_string(),
        application_id: "application_id_del".to_string(),
        application_code: "application_code_del".to_string(),
        device_id: "device_id_del".to_string(),
        network_id: "network_id_del".to_string(),
        network_code: "network_code_del".to_string(),
        network_addr: "network_addr_del".to_string(),
        profile: "profile_del".to_string(),
        created_at: now,
        modified_at: now,
    };
    let cond = QueryCond {
        route_id: Some(route_id_del),
        ..Default::default()
    };
    if let Err(e) = runtime.block_on(async {
        model.add(&route).await?;
        route.route_id = route_id_not_del.to_string();
        route.application_id = "application_id_not_del".to_string();
        route.application_code = "application_code_not_del".to_string();
        model.add(&route).await?;
        model.del(&cond).await
    }) {
        return Err(format!("model.add/del error: {}", e));
    }
    match runtime.block_on(async { model.get(route_id_del).await }) {
        Err(e) => return Err(format!("model.get() error: {}", e)),
        Ok(route) => match route {
            None => (),
            Some(_) => return Err("delete fail".to_string()),
        },
    }
    match runtime.block_on(async { model.get(route_id_not_del).await }) {
        Err(e) => Err(format!("model.get() not delete one error: {}", e)),
        Ok(route) => match route {
            None => Err("delete wrong one".to_string()),
            Some(_) => Ok(()),
        },
    }
}

/// Test `del()` twice.
pub fn del_twice(runtime: &Runtime, model: &dyn DeviceRouteModel) -> Result<(), String> {
    let now = Utc::now().trunc_subsecs(3);
    let route_id_del = "route_id_del";
    let route = DeviceRoute {
        route_id: route_id_del.to_string(),
        unit_id: "unit_id_del".to_string(),
        unit_code: "unit_code_del".to_string(),
        application_id: "application_id_del".to_string(),
        application_code: "application_code_del".to_string(),
        device_id: "device_id_del".to_string(),
        network_id: "network_id_del".to_string(),
        network_code: "network_code_del".to_string(),
        network_addr: "network_addr_del".to_string(),
        profile: "profile_del".to_string(),
        created_at: now,
        modified_at: now,
    };
    let cond = QueryCond {
        route_id: Some(route_id_del),
        ..Default::default()
    };
    if let Err(e) = runtime.block_on(async {
        model.add(&route).await?;
        model.del(&cond).await?;
        model.del(&cond).await
    }) {
        return Err(format!("model.add/del error: {}", e));
    }
    Ok(())
}

/// Test `del()` by specifying a unit ID.
pub fn del_by_unit_id(runtime: &Runtime, model: &dyn DeviceRouteModel) -> Result<(), String> {
    let now = Utc::now().trunc_subsecs(3);
    let route_id_del1 = "route_id_del1";
    let route_id_del2 = "route_id_del2";
    let route_id_not_del = "route_id_not_del";
    let route_id_not_del2 = "route_id_not_del2";
    let mut route = DeviceRoute {
        route_id: route_id_del1.to_string(),
        unit_id: "unit_id_del".to_string(),
        unit_code: "unit_code_del".to_string(),
        application_id: "application_id_del".to_string(),
        application_code: "application_code_del".to_string(),
        device_id: "device_id_del1".to_string(),
        network_id: "network_id_del".to_string(),
        network_code: "network_code_del".to_string(),
        network_addr: "network_addr_del".to_string(),
        profile: "profile_del".to_string(),
        created_at: now,
        modified_at: now,
    };
    let cond = QueryCond {
        unit_id: Some("unit_id_del"),
        ..Default::default()
    };
    if let Err(e) = runtime.block_on(async {
        model.add(&route).await?;
        route.route_id = route_id_del2.to_string();
        route.device_id = "device_id_del2".to_string();
        model.add(&route).await?;
        route.route_id = route_id_not_del.to_string();
        route.device_id = "device_id_not_del".to_string();
        route.unit_id = "unit_id_not_del".to_string();
        route.unit_code = "unit_code_not_del".to_string();
        model.add(&route).await?;
        route.route_id = route_id_not_del2.to_string();
        route.device_id = "device_id_not_del2".to_string();
        model.add(&route).await?;
        model.del(&cond).await
    }) {
        return Err(format!("model.add/del error: {}", e));
    }
    match runtime.block_on(async { model.get(route_id_del1).await }) {
        Err(e) => return Err(format!("model.get() delete route1 error: {}", e)),
        Ok(route) => match route {
            None => (),
            Some(_) => return Err("delete route1 fail".to_string()),
        },
    }
    match runtime.block_on(async { model.get(route_id_del2).await }) {
        Err(e) => return Err(format!("model.get() delete route2 error: {}", e)),
        Ok(route) => match route {
            None => (),
            Some(_) => return Err("delete route2 fail".to_string()),
        },
    }
    match runtime.block_on(async { model.get(route_id_not_del).await }) {
        Err(e) => return Err(format!("model.get() not delete one error: {}", e)),
        Ok(route) => match route {
            None => return Err("delete wrong one".to_string()),
            Some(_) => (),
        },
    }
    match runtime.block_on(async { model.get(route_id_not_del2).await }) {
        Err(e) => Err(format!("model.get() not delete one2 error: {}", e)),
        Ok(route) => match route {
            None => Err("delete wrong one2".to_string()),
            Some(_) => Ok(()),
        },
    }
}

/// Test `del()` by specifying a pair of unit ID and route ID.
pub fn del_by_unit_route(runtime: &Runtime, model: &dyn DeviceRouteModel) -> Result<(), String> {
    let now = Utc::now().trunc_subsecs(3);
    let route_id_del = "route_id_del";
    let route_id_not_del = "route_id_not_del";
    let mut route = DeviceRoute {
        route_id: route_id_del.to_string(),
        unit_id: "unit_id_del".to_string(),
        unit_code: "unit_code_del".to_string(),
        application_id: "application_id_del".to_string(),
        application_code: "application_code_del".to_string(),
        device_id: "device_id_del".to_string(),
        network_id: "network_id_del".to_string(),
        network_code: "network_code_del".to_string(),
        network_addr: "network_addr_del".to_string(),
        profile: "profile_del".to_string(),
        created_at: now,
        modified_at: now,
    };
    let cond = QueryCond {
        unit_id: Some("unit_id_del"),
        route_id: Some(route_id_del),
        ..Default::default()
    };
    if let Err(e) = runtime.block_on(async {
        model.add(&route).await?;
        route.route_id = route_id_not_del.to_string();
        route.device_id = "device_id_del2".to_string();
        model.add(&route).await?;
        model.del(&cond).await
    }) {
        return Err(format!("model.add/del error: {}", e));
    }
    match runtime.block_on(async { model.get(route_id_del).await }) {
        Err(e) => return Err(format!("model.get() error: {}", e)),
        Ok(route) => match route {
            None => (),
            Some(_) => return Err("delete fail".to_string()),
        },
    }
    match runtime.block_on(async { model.get(route_id_not_del).await }) {
        Err(e) => Err(format!("model.get() not delete one error: {}", e)),
        Ok(route) => match route {
            None => Err("delete wrong one".to_string()),
            Some(_) => Ok(()),
        },
    }
}

/// Test `del()` by specifying an application ID.
pub fn del_by_application_id(
    runtime: &Runtime,
    model: &dyn DeviceRouteModel,
) -> Result<(), String> {
    let now = Utc::now().trunc_subsecs(3);
    let route_id_del1 = "route_id_del1";
    let route_id_del2 = "route_id_del2";
    let route_id_not_del = "route_id_not_del";
    let route_id_not_del2 = "route_id_not_del2";
    let mut route = DeviceRoute {
        route_id: route_id_del1.to_string(),
        unit_id: "unit_id_del".to_string(),
        unit_code: "unit_code_del".to_string(),
        application_id: "application_id_del".to_string(),
        application_code: "application_code_del".to_string(),
        device_id: "device_id_del1".to_string(),
        network_id: "network_id_del".to_string(),
        network_code: "network_code_del".to_string(),
        network_addr: "network_addr_del".to_string(),
        profile: "profile_del".to_string(),
        created_at: now,
        modified_at: now,
    };
    let cond = QueryCond {
        application_id: Some("application_id_del"),
        ..Default::default()
    };
    if let Err(e) = runtime.block_on(async {
        model.add(&route).await?;
        route.route_id = route_id_del2.to_string();
        route.device_id = "device_id_del2".to_string();
        model.add(&route).await?;
        route.route_id = route_id_not_del.to_string();
        route.device_id = "device_id_del".to_string();
        route.application_id = "application_id_not_del".to_string();
        model.add(&route).await?;
        route.route_id = route_id_not_del2.to_string();
        route.device_id = "device_id_del2".to_string();
        model.add(&route).await?;
        model.del(&cond).await
    }) {
        return Err(format!("model.add/del error: {}", e));
    }
    match runtime.block_on(async { model.get(route_id_del1).await }) {
        Err(e) => return Err(format!("model.get() delete route1 error: {}", e)),
        Ok(route) => match route {
            None => (),
            Some(_) => return Err("delete route1 fail".to_string()),
        },
    }
    match runtime.block_on(async { model.get(route_id_del2).await }) {
        Err(e) => return Err(format!("model.get() delete route2 error: {}", e)),
        Ok(route) => match route {
            None => (),
            Some(_) => return Err("delete route2 fail".to_string()),
        },
    }
    match runtime.block_on(async { model.get(route_id_not_del).await }) {
        Err(e) => return Err(format!("model.get() not delete one error: {}", e)),
        Ok(route) => match route {
            None => return Err("delete wrong one".to_string()),
            Some(_) => (),
        },
    }
    match runtime.block_on(async { model.get(route_id_not_del2).await }) {
        Err(e) => Err(format!("model.get() not delete one2 error: {}", e)),
        Ok(route) => match route {
            None => Err("delete wrong one2".to_string()),
            Some(_) => Ok(()),
        },
    }
}

/// Test `del()` by specifying a network ID.
pub fn del_by_network_id(runtime: &Runtime, model: &dyn DeviceRouteModel) -> Result<(), String> {
    let now = Utc::now().trunc_subsecs(3);
    let route_id_del1 = "route_id_del1";
    let route_id_del2 = "route_id_del2";
    let route_id_not_del = "route_id_not_del";
    let route_id_not_del2 = "route_id_not_del2";
    let mut route = DeviceRoute {
        route_id: route_id_del1.to_string(),
        unit_id: "unit_id_del".to_string(),
        unit_code: "unit_code_del".to_string(),
        application_id: "application_id_del".to_string(),
        application_code: "application_code_del".to_string(),
        device_id: "device_id_del1".to_string(),
        network_id: "network_id_del".to_string(),
        network_code: "network_code_del".to_string(),
        network_addr: "network_addr_del".to_string(),
        profile: "profile_del".to_string(),
        created_at: now,
        modified_at: now,
    };
    let cond = QueryCond {
        network_id: Some("network_id_del"),
        ..Default::default()
    };
    if let Err(e) = runtime.block_on(async {
        model.add(&route).await?;
        route.route_id = route_id_del2.to_string();
        route.device_id = "device_id_del2".to_string();
        model.add(&route).await?;
        route.route_id = route_id_not_del.to_string();
        route.device_id = "device_id_not_del".to_string();
        route.network_id = "network_id_not_del".to_string();
        model.add(&route).await?;
        route.route_id = route_id_not_del2.to_string();
        route.device_id = "device_id_not_del2".to_string();
        model.add(&route).await?;
        model.del(&cond).await
    }) {
        return Err(format!("model.add/del error: {}", e));
    }
    match runtime.block_on(async { model.get(route_id_del1).await }) {
        Err(e) => return Err(format!("model.get() delete route1 error: {}", e)),
        Ok(route) => match route {
            None => (),
            Some(_) => return Err("delete route1 fail".to_string()),
        },
    }
    match runtime.block_on(async { model.get(route_id_del2).await }) {
        Err(e) => return Err(format!("model.get() delete route2 error: {}", e)),
        Ok(route) => match route {
            None => (),
            Some(_) => return Err("delete route2 fail".to_string()),
        },
    }
    match runtime.block_on(async { model.get(route_id_not_del).await }) {
        Err(e) => return Err(format!("model.get() not delete one error: {}", e)),
        Ok(route) => match route {
            None => return Err("delete wrong one".to_string()),
            Some(_) => (),
        },
    }
    match runtime.block_on(async { model.get(route_id_not_del2).await }) {
        Err(e) => Err(format!("model.get() not delete one2 error: {}", e)),
        Ok(route) => match route {
            None => Err("delete wrong one2".to_string()),
            Some(_) => Ok(()),
        },
    }
}

/// Test `del()` by specifying a device ID.
pub fn del_by_device_id(runtime: &Runtime, model: &dyn DeviceRouteModel) -> Result<(), String> {
    let now = Utc::now().trunc_subsecs(3);
    let route_id_del1 = "route_id_del1";
    let route_id_del2 = "route_id_del2";
    let route_id_not_del = "route_id_not_del";
    let route_id_not_del2 = "route_id_not_del2";
    let mut route = DeviceRoute {
        route_id: route_id_del1.to_string(),
        unit_id: "unit_id_del".to_string(),
        unit_code: "unit_code_del".to_string(),
        application_id: "application_id_del1".to_string(),
        application_code: "application_code_del".to_string(),
        device_id: "device_id_del".to_string(),
        network_id: "network_id_del".to_string(),
        network_code: "network_code_del".to_string(),
        network_addr: "network_addr_del".to_string(),
        profile: "profile_del".to_string(),
        created_at: now,
        modified_at: now,
    };
    let cond = QueryCond {
        device_id: Some("device_id_del"),
        ..Default::default()
    };
    if let Err(e) = runtime.block_on(async {
        model.add(&route).await?;
        route.route_id = route_id_del2.to_string();
        route.application_id = "application_id_del2".to_string();
        model.add(&route).await?;
        route.route_id = route_id_not_del.to_string();
        route.device_id = "device_id_not_del".to_string();
        route.application_id = "network_id_del".to_string();
        model.add(&route).await?;
        route.route_id = route_id_not_del2.to_string();
        route.application_id = "network_id_del2".to_string();
        model.add(&route).await?;
        model.del(&cond).await
    }) {
        return Err(format!("model.add/del error: {}", e));
    }
    match runtime.block_on(async { model.get(route_id_del1).await }) {
        Err(e) => return Err(format!("model.get() delete route1 error: {}", e)),
        Ok(route) => match route {
            None => (),
            Some(_) => return Err("delete route1 fail".to_string()),
        },
    }
    match runtime.block_on(async { model.get(route_id_del2).await }) {
        Err(e) => return Err(format!("model.get() delete route2 error: {}", e)),
        Ok(route) => match route {
            None => (),
            Some(_) => return Err("delete route2 fail".to_string()),
        },
    }
    match runtime.block_on(async { model.get(route_id_not_del).await }) {
        Err(e) => return Err(format!("model.get() not delete one error: {}", e)),
        Ok(route) => match route {
            None => return Err("delete wrong one".to_string()),
            Some(_) => (),
        },
    }
    match runtime.block_on(async { model.get(route_id_not_del2).await }) {
        Err(e) => Err(format!("model.get() not delete one2 error: {}", e)),
        Ok(route) => match route {
            None => Err("delete wrong one2".to_string()),
            Some(_) => Ok(()),
        },
    }
}

/// Test `del()` by specifying network addresses.
pub fn del_by_network_addrs(runtime: &Runtime, model: &dyn DeviceRouteModel) -> Result<(), String> {
    let mut routes = vec![];
    for i in 0..100 {
        let now = Utc::now().trunc_subsecs(3);
        let route = DeviceRoute {
            route_id: format!("route_id{:#03}", i),
            unit_id: "unit_id_bulk".to_string(),
            unit_code: "unit_code_bulk".to_string(),
            application_id: "application_id_bulk".to_string(),
            application_code: "application_code_bulk".to_string(),
            device_id: format!("device_id{:#03}", i),
            network_id: "network_id_bulk".to_string(),
            network_code: "network_code_bulk".to_string(),
            network_addr: format!("network_del{:#03}", i),
            profile: "profile_del".to_string(),
            created_at: now,
            modified_at: now,
        };
        routes.push(route);
    }
    if let Err(e) = runtime.block_on(async { model.add_bulk(&routes).await }) {
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
        Err(e) => {
            return Err(format!(
                "model.count() after delete wrong unit error: {}",
                e
            ))
        }
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
                if !item.network_addr.eq(&format!("network_del{:#03}", i)) {
                    return Err(format!("model.del() content error"));
                }
                i += 1;
            }
        }
    }

    Ok(())
}

/// Test `update()`.
pub fn update(runtime: &Runtime, model: &dyn DeviceRouteModel) -> Result<(), String> {
    let now = Utc::now().trunc_subsecs(3);
    let route_id_update1 = "route_id_update1";
    let route_id_update2 = "route_id_update2";
    let route_id_not_update1 = "route_id_not_update1";
    let route_id_not_update2 = "route_id_not_update2";
    let mut route = DeviceRoute {
        route_id: route_id_update1.to_string(),
        unit_id: "unit_id_update".to_string(),
        unit_code: "unit_code_update".to_string(),
        application_id: "application_id_update1".to_string(),
        application_code: "application_code_update".to_string(),
        device_id: "device_id_update".to_string(),
        network_id: "network_id_update".to_string(),
        network_code: "network_code_update".to_string(),
        network_addr: "network_addr_update".to_string(),
        profile: "profile_update".to_string(),
        created_at: now,
        modified_at: now,
    };
    if let Err(e) = runtime.block_on(async {
        model.add(&route).await?;
        route.route_id = route_id_update2.to_string();
        route.application_id = "application_id_update2".to_string();
        model.add(&route).await?;
        route.route_id = route_id_not_update1.to_string();
        route.device_id = "device_id_not_update".to_string();
        route.application_id = "network_id_update".to_string();
        model.add(&route).await?;
        route.route_id = route_id_not_update2.to_string();
        route.application_id = "network_id_update2".to_string();
        model.add(&route).await
    }) {
        return Err(format!("model.add error: {}", e));
    }

    // Update.
    let update_cond = UpdateQueryCond {
        device_id: "device_id_update",
    };
    let modified_at = now + TimeDelta::try_milliseconds(1).unwrap();
    let updates = Updates {
        profile: Some(""),
        modified_at: Some(modified_at),
    };
    if let Err(e) = runtime.block_on(async { model.update(&update_cond, &updates).await }) {
        return Err(format!("model.update() one error: {}", e));
    }

    let get_route = match runtime.block_on(async { model.get(route_id_update1).await }) {
        Err(e) => return Err(format!("model.get() update route1 error: {}", e)),
        Ok(route) => match route {
            None => return Err("model.get() should get update1 route".to_string()),
            Some(route) => route,
        },
    };
    expect(get_route.profile.as_str()).to_equal("")?;
    expect(get_route.modified_at).to_equal(modified_at)?;

    let get_route = match runtime.block_on(async { model.get(route_id_update2).await }) {
        Err(e) => return Err(format!("model.get() update route2 error: {}", e)),
        Ok(route) => match route {
            None => return Err("model.get() should get update route2".to_string()),
            Some(route) => route,
        },
    };
    expect(get_route.profile.as_str()).to_equal("")?;
    expect(get_route.modified_at).to_equal(modified_at)?;

    let get_route = match runtime.block_on(async { model.get(route_id_not_update1).await }) {
        Err(e) => return Err(format!("model.get() not update route1 error: {}", e)),
        Ok(route) => match route {
            None => return Err("model.get() should get not update1 route".to_string()),
            Some(route) => route,
        },
    };
    expect(get_route.profile.as_str()).to_equal("profile_update")?;
    expect(get_route.modified_at).to_equal(now)?;

    let get_route = match runtime.block_on(async { model.get(route_id_not_update2).await }) {
        Err(e) => return Err(format!("model.get() not update route2 error: {}", e)),
        Ok(route) => match route {
            None => return Err("model.get() should get not update route2".to_string()),
            Some(route) => route,
        },
    };
    expect(get_route.profile.as_str()).to_equal("profile_update")?;
    expect(get_route.modified_at).to_equal(now)
}

/// Test `update()` with a non-exist condition.
pub fn update_not_exist(runtime: &Runtime, model: &dyn DeviceRouteModel) -> Result<(), String> {
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
pub fn update_invalid(runtime: &Runtime, model: &dyn DeviceRouteModel) -> Result<(), String> {
    let cond = UpdateQueryCond {
        device_id: "device_id",
    };
    let updates = Updates {
        modified_at: None,
        profile: None,
    };
    if let Err(e) = runtime.block_on(async { model.update(&cond, &updates).await }) {
        return Err(format!("model.update() error: {}", e));
    }
    Ok(())
}

/// Test `count()`.
pub fn count(runtime: &Runtime, model: &dyn DeviceRouteModel) -> Result<(), String> {
    let now = Utc::now().trunc_subsecs(3);
    let mut route = DeviceRoute {
        route_id: "route_id_count1_1".to_string(),
        unit_id: "unit_id_count".to_string(),
        unit_code: "unit_code_count".to_string(),
        application_id: "application_id_count".to_string(),
        application_code: "application_code_count".to_string(),
        device_id: "device_id_count1".to_string(),
        network_id: "network_id_count".to_string(),
        network_code: "network_code_count".to_string(),
        network_addr: "network_addr_count1".to_string(),
        profile: "profile_count".to_string(),
        created_at: now,
        modified_at: now,
    };
    if let Err(e) = runtime.block_on(async {
        model.add(&route).await?;
        route.route_id = "route_id_count1_2".to_string();
        route.device_id = "device_id_count1_2".to_string();
        route.network_addr = "network_addr_count1_2".to_string();
        model.add(&route).await?;
        route.route_id = "route_id_count1_3".to_string();
        route.device_id = "device_id_count1_3".to_string();
        route.network_id = "network_id_count1_3".to_string();
        route.network_code = "network_code_count1_3".to_string();
        route.network_addr = "network_addr_count1_3".to_string();
        model.add(&route).await?;
        route.route_id = "route_id_count2_1".to_string();
        route.application_id = "application_id_count2".to_string();
        route.application_code = "application_code_count2".to_string();
        route.device_id = "device_id_count1".to_string();
        route.network_addr = "network_addr_count1".to_string();
        model.add(&route).await?;
        route.route_id = "route_id_count3_1".to_string();
        route.unit_id = "unit_id_count3".to_string();
        route.unit_code = "unit_code_count3".to_string();
        route.application_id = "application_id_count3".to_string();
        route.application_code = "application_code_count3".to_string();
        route.device_id = "device_id_count1".to_string();
        route.network_id = "network_id_count3".to_string();
        route.network_code = "network_code_count1".to_string();
        model.add(&route).await
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
    expect(count).to_equal(5)?;

    let cond = ListQueryCond {
        route_id: Some("route_id_count1_1"),
        ..Default::default()
    };
    let count = match runtime.block_on(async { model.count(&cond).await }) {
        Err(e) => return Err(format!("count route_id result error: {}", e)),
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
    expect(count).to_equal(4)?;

    let cond = ListQueryCond {
        unit_code: Some("unit_code_count"),
        ..Default::default()
    };
    let count = match runtime.block_on(async { model.count(&cond).await }) {
        Err(e) => return Err(format!("count unit_code result error: {}", e)),
        Ok(count) => count,
    };
    expect(count).to_equal(4)?;

    let cond = ListQueryCond {
        route_id: Some("route_id_count1_1"),
        unit_id: Some("unit_id_count"),
        ..Default::default()
    };
    let count = match runtime.block_on(async { model.count(&cond).await }) {
        Err(e) => return Err(format!("count route-unit result error: {}", e)),
        Ok(count) => count,
    };
    expect(count).to_equal(1)?;

    let cond = ListQueryCond {
        route_id: Some("route_id_count1_1"),
        unit_code: Some("unit_code_count"),
        ..Default::default()
    };
    let count = match runtime.block_on(async { model.count(&cond).await }) {
        Err(e) => return Err(format!("count route-unit-code result error: {}", e)),
        Ok(count) => count,
    };
    expect(count).to_equal(1)?;

    let cond = ListQueryCond {
        route_id: Some("route_id_count3_1"),
        unit_id: Some("unit_id_count"),
        ..Default::default()
    };
    let count = match runtime.block_on(async { model.count(&cond).await }) {
        Err(e) => return Err(format!("count route3-unit result error: {}", e)),
        Ok(count) => count,
    };
    expect(count).to_equal(0)?;

    let cond = ListQueryCond {
        route_id: Some("route_id_count3_1"),
        unit_code: Some("unit_code_count"),
        ..Default::default()
    };
    let count = match runtime.block_on(async { model.count(&cond).await }) {
        Err(e) => return Err(format!("count route3-unit-code result error: {}", e)),
        Ok(count) => count,
    };
    expect(count).to_equal(0)?;

    let cond = ListQueryCond {
        application_id: Some("application_id_count"),
        ..Default::default()
    };
    let count = match runtime.block_on(async { model.count(&cond).await }) {
        Err(e) => return Err(format!("count application_id result error: {}", e)),
        Ok(count) => count,
    };
    expect(count).to_equal(3)?;

    let cond = ListQueryCond {
        application_code: Some("application_code_count"),
        ..Default::default()
    };
    let count = match runtime.block_on(async { model.count(&cond).await }) {
        Err(e) => return Err(format!("count application_code result error: {}", e)),
        Ok(count) => count,
    };
    expect(count).to_equal(3)?;

    let cond = ListQueryCond {
        route_id: Some("route_id_count1_1"),
        application_id: Some("application_id_count"),
        ..Default::default()
    };
    let count = match runtime.block_on(async { model.count(&cond).await }) {
        Err(e) => return Err(format!("count route-application result error: {}", e)),
        Ok(count) => count,
    };
    expect(count).to_equal(1)?;

    let cond = ListQueryCond {
        route_id: Some("route_id_count1_1"),
        application_code: Some("application_code_count"),
        ..Default::default()
    };
    let count = match runtime.block_on(async { model.count(&cond).await }) {
        Err(e) => return Err(format!("count route-application-code result error: {}", e)),
        Ok(count) => count,
    };
    expect(count).to_equal(1)?;

    let cond = ListQueryCond {
        route_id: Some("route_id_count3_1"),
        application_id: Some("application_id_count"),
        ..Default::default()
    };
    let count = match runtime.block_on(async { model.count(&cond).await }) {
        Err(e) => return Err(format!("count route3-application result error: {}", e)),
        Ok(count) => count,
    };
    expect(count).to_equal(0)?;

    let cond = ListQueryCond {
        route_id: Some("route_id_count3_1"),
        application_code: Some("application_code_count"),
        ..Default::default()
    };
    let count = match runtime.block_on(async { model.count(&cond).await }) {
        Err(e) => return Err(format!("count route3-application-code result error: {}", e)),
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
        network_code: Some("network_code_count"),
        ..Default::default()
    };
    let count = match runtime.block_on(async { model.count(&cond).await }) {
        Err(e) => return Err(format!("count network_code result error: {}", e)),
        Ok(count) => count,
    };
    expect(count).to_equal(2)?;

    let cond = ListQueryCond {
        route_id: Some("route_id_count1_1"),
        network_id: Some("network_id_count"),
        ..Default::default()
    };
    let count = match runtime.block_on(async { model.count(&cond).await }) {
        Err(e) => return Err(format!("count route-network result error: {}", e)),
        Ok(count) => count,
    };
    expect(count).to_equal(1)?;

    let cond = ListQueryCond {
        route_id: Some("route_id_count1_1"),
        network_code: Some("network_code_count"),
        ..Default::default()
    };
    let count = match runtime.block_on(async { model.count(&cond).await }) {
        Err(e) => return Err(format!("count route-network-code result error: {}", e)),
        Ok(count) => count,
    };
    expect(count).to_equal(1)?;

    let cond = ListQueryCond {
        route_id: Some("route_id_count3_1"),
        network_id: Some("network_id_count"),
        ..Default::default()
    };
    let count = match runtime.block_on(async { model.count(&cond).await }) {
        Err(e) => return Err(format!("count route3-network result error: {}", e)),
        Ok(count) => count,
    };
    expect(count).to_equal(0)?;

    let cond = ListQueryCond {
        route_id: Some("route_id_count3_1"),
        network_code: Some("network_code_count"),
        ..Default::default()
    };
    let count = match runtime.block_on(async { model.count(&cond).await }) {
        Err(e) => return Err(format!("count route3-network-code result error: {}", e)),
        Ok(count) => count,
    };
    expect(count).to_equal(0)?;

    let cond = ListQueryCond {
        device_id: Some("device_id_count1"),
        ..Default::default()
    };
    let count = match runtime.block_on(async { model.count(&cond).await }) {
        Err(e) => return Err(format!("count device_id result error: {}", e)),
        Ok(count) => count,
    };
    expect(count).to_equal(3)?;

    let cond = ListQueryCond {
        network_addr: Some("network_addr_count1"),
        ..Default::default()
    };
    let count = match runtime.block_on(async { model.count(&cond).await }) {
        Err(e) => return Err(format!("count network_addr result error: {}", e)),
        Ok(count) => count,
    };
    expect(count).to_equal(3)?;

    let cond = ListQueryCond {
        route_id: Some("route_id_count1_1"),
        device_id: Some("device_id_count1"),
        ..Default::default()
    };
    let count = match runtime.block_on(async { model.count(&cond).await }) {
        Err(e) => return Err(format!("count route-device result error: {}", e)),
        Ok(count) => count,
    };
    expect(count).to_equal(1)?;

    let cond = ListQueryCond {
        route_id: Some("route_id_count1_1"),
        network_addr: Some("network_addr_count1"),
        ..Default::default()
    };
    let count = match runtime.block_on(async { model.count(&cond).await }) {
        Err(e) => return Err(format!("count route-network-addr result error: {}", e)),
        Ok(count) => count,
    };
    expect(count).to_equal(1)?;

    let cond = ListQueryCond {
        route_id: Some("route_id_count3_1"),
        device_id: Some("device_id_count1_3"),
        ..Default::default()
    };
    let count = match runtime.block_on(async { model.count(&cond).await }) {
        Err(e) => return Err(format!("count route3-device result error: {}", e)),
        Ok(count) => count,
    };
    expect(count).to_equal(0)?;

    let cond = ListQueryCond {
        route_id: Some("route_id_count3_1"),
        network_addr: Some("network_addr_count1_3"),
        ..Default::default()
    };
    let count = match runtime.block_on(async { model.count(&cond).await }) {
        Err(e) => return Err(format!("count route3-network-addr result error: {}", e)),
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
    expect(count).to_equal(4)?;

    let cond = ListQueryCond {
        unit_id: Some("_1"),
        network_addrs: Some(&addrs),
        ..Default::default()
    };
    let count = match runtime.block_on(async { model.count(&cond).await }) {
        Err(e) => return Err(format!("count unit-addrs-not-match result error: {}", e)),
        Ok(count) => count,
    };
    expect(count).to_equal(0)
}

/// Test `list()`.
pub fn list(runtime: &Runtime, model: &dyn DeviceRouteModel) -> Result<(), String> {
    let now = Utc::now().trunc_subsecs(3);
    let mut route = DeviceRoute {
        route_id: "route_id_list1_1".to_string(),
        unit_id: "unit_id_list".to_string(),
        unit_code: "unit_code_list".to_string(),
        application_id: "application_id_list".to_string(),
        application_code: "application_code_list".to_string(),
        device_id: "device_id_list1".to_string(),
        network_id: "network_id_list".to_string(),
        network_code: "network_code_list".to_string(),
        network_addr: "network_addr_list1".to_string(),
        profile: "profile".to_string(),
        created_at: now,
        modified_at: now,
    };
    if let Err(e) = runtime.block_on(async {
        model.add(&route).await?;
        route.route_id = "route_id_list1_2".to_string();
        route.device_id = "device_id_list1_2".to_string();
        route.network_addr = "network_addr_list1_2".to_string();
        model.add(&route).await?;
        route.route_id = "route_id_list1_3".to_string();
        route.device_id = "device_id_list1_3".to_string();
        route.network_id = "network_id_list1_3".to_string();
        route.network_code = "network_code_list1_3".to_string();
        route.network_addr = "network_addr_list1_3".to_string();
        model.add(&route).await?;
        route.route_id = "route_id_list2_1".to_string();
        route.application_id = "application_id_list2".to_string();
        route.application_code = "application_code_list2".to_string();
        route.device_id = "device_id_list1".to_string();
        route.network_addr = "network_addr_list1".to_string();
        model.add(&route).await?;
        route.route_id = "route_id_list3_1".to_string();
        route.unit_id = "unit_id_list3".to_string();
        route.unit_code = "unit_code_list3".to_string();
        route.application_id = "application_id_list3".to_string();
        route.application_code = "application_code_list3".to_string();
        route.device_id = "device_id_list1".to_string();
        route.network_id = "network_id_list3".to_string();
        route.network_code = "network_code_list3".to_string();
        route.network_addr = "network_addr_list1".to_string();
        model.add(&route).await
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
    expect(list.len()).to_equal(5)?;

    let cond = ListQueryCond {
        route_id: Some("route_id_list1_1"),
        ..Default::default()
    };
    opts.cond = &cond;
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list route_id result error: {}", e)),
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
    expect(list.len()).to_equal(4)?;

    let cond = ListQueryCond {
        unit_code: Some("unit_code_list"),
        ..Default::default()
    };
    opts.cond = &cond;
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list unit_code result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(4)?;

    let cond = ListQueryCond {
        route_id: Some("route_id_list1_1"),
        unit_id: Some("unit_id_list"),
        ..Default::default()
    };
    opts.cond = &cond;
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list route-unit result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(1)?;

    let cond = ListQueryCond {
        route_id: Some("route_id_list1_1"),
        unit_code: Some("unit_code_list"),
        ..Default::default()
    };
    opts.cond = &cond;
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list route-unit-code result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(1)?;

    let cond = ListQueryCond {
        route_id: Some("route_id_list3_1"),
        unit_id: Some("unit_id_list"),
        ..Default::default()
    };
    opts.cond = &cond;
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list route3-unit result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(0)?;

    let cond = ListQueryCond {
        route_id: Some("route_id_list3_1"),
        unit_code: Some("unit_code_list"),
        ..Default::default()
    };
    opts.cond = &cond;
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list route3-unit-code result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(0)?;

    let cond = ListQueryCond {
        application_id: Some("application_id_list"),
        ..Default::default()
    };
    opts.cond = &cond;
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list application_id result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(3)?;

    let cond = ListQueryCond {
        application_code: Some("application_code_list"),
        ..Default::default()
    };
    opts.cond = &cond;
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list application_code result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(3)?;

    let cond = ListQueryCond {
        route_id: Some("route_id_list1_1"),
        application_id: Some("application_id_list"),
        ..Default::default()
    };
    opts.cond = &cond;
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list route-application result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(1)?;

    let cond = ListQueryCond {
        route_id: Some("route_id_list1_1"),
        application_code: Some("application_code_list"),
        ..Default::default()
    };
    opts.cond = &cond;
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list route-application-code result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(1)?;

    let cond = ListQueryCond {
        route_id: Some("route_id_list3_1"),
        application_id: Some("application_id_list"),
        ..Default::default()
    };
    opts.cond = &cond;
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list route3-application result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(0)?;

    let cond = ListQueryCond {
        route_id: Some("route_id_list3_1"),
        application_code: Some("application_code_list"),
        ..Default::default()
    };
    opts.cond = &cond;
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list route3-application-code result error: {}", e)),
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
        network_code: Some("network_code_list"),
        ..Default::default()
    };
    opts.cond = &cond;
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list network_code result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(2)?;

    let cond = ListQueryCond {
        route_id: Some("route_id_list1_1"),
        network_id: Some("network_id_list"),
        ..Default::default()
    };
    opts.cond = &cond;
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list route-network result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(1)?;

    let cond = ListQueryCond {
        route_id: Some("route_id_list1_1"),
        network_code: Some("network_code_list"),
        ..Default::default()
    };
    opts.cond = &cond;
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list route-network-code result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(1)?;

    let cond = ListQueryCond {
        route_id: Some("route_id_list3_1"),
        network_id: Some("network_id_list"),
        ..Default::default()
    };
    opts.cond = &cond;
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list route3-network result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(0)?;

    let cond = ListQueryCond {
        route_id: Some("route_id_list3_1"),
        network_code: Some("network_code_list"),
        ..Default::default()
    };
    opts.cond = &cond;
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list route3-network-code result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(0)?;

    let cond = ListQueryCond {
        device_id: Some("device_id_list1"),
        ..Default::default()
    };
    opts.cond = &cond;
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list device_id result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(3)?;

    let cond = ListQueryCond {
        network_addr: Some("network_addr_list1"),
        ..Default::default()
    };
    opts.cond = &cond;
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list network_addr result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(3)?;

    let cond = ListQueryCond {
        route_id: Some("route_id_list1_1"),
        device_id: Some("device_id_list1"),
        ..Default::default()
    };
    opts.cond = &cond;
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list route-device result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(1)?;

    let cond = ListQueryCond {
        route_id: Some("route_id_list1_1"),
        network_addr: Some("network_addr_list1"),
        ..Default::default()
    };
    opts.cond = &cond;
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list route-network-addr result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(1)?;

    let cond = ListQueryCond {
        route_id: Some("route_id_list3_1"),
        device_id: Some("device_id_list1_3"),
        ..Default::default()
    };
    opts.cond = &cond;
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list route3-device result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(0)?;

    let cond = ListQueryCond {
        route_id: Some("route_id_list3_1"),
        network_addr: Some("network_addr_list1_3"),
        ..Default::default()
    };
    opts.cond = &cond;
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list route3-network-addr result error: {}", e)),
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
    expect(list.len()).to_equal(4)?;

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
    expect(list.len()).to_equal(0)
}

/// Test `list()` with sorting.
pub fn list_sort(runtime: &Runtime, model: &dyn DeviceRouteModel) -> Result<(), String> {
    let mut now = Utc::now().trunc_subsecs(3);
    let mut route = DeviceRoute {
        route_id: "route_id_list1_1".to_string(),
        unit_id: "unit_id_list".to_string(),
        unit_code: "unit_code_list".to_string(),
        application_id: "application_id_list".to_string(),
        application_code: "application_code_list".to_string(),
        device_id: "device_id_list1".to_string(),
        network_id: "network_id_list".to_string(),
        network_code: "network_code_list".to_string(),
        network_addr: "network_addr_list1_1".to_string(),
        profile: "profile".to_string(),
        created_at: now,
        modified_at: now + TimeDelta::try_seconds(20).unwrap(),
    };
    if let Err(e) = runtime.block_on(async {
        model.add(&route).await?;
        now = now + TimeDelta::try_seconds(1).unwrap();
        route.route_id = "route_id_list1_2".to_string();
        route.device_id = "device_id_list1_2".to_string();
        route.network_addr = "network_addr_list1_2".to_string();
        route.created_at = now;
        route.modified_at = now + TimeDelta::try_seconds(18).unwrap();
        model.add(&route).await?;
        now = now + TimeDelta::try_seconds(1).unwrap();
        route.route_id = "route_id_list1_3".to_string();
        route.device_id = "device_id_list1_3".to_string();
        route.network_id = "network_id_list1_3".to_string();
        route.network_addr = "network_addr_list1_3".to_string();
        route.created_at = now;
        route.modified_at = now + TimeDelta::try_seconds(16).unwrap();
        model.add(&route).await?;
        now = now + TimeDelta::try_seconds(1).unwrap();
        route.route_id = "route_id_list2_1".to_string();
        route.application_id = "application_id_list2".to_string();
        route.application_code = "application_code_list2".to_string();
        route.device_id = "device_id_list1".to_string();
        route.network_id = "network_id_list2".to_string();
        route.network_code = "network_code_list2".to_string();
        route.network_addr = "network_addr_list2_1".to_string();
        route.created_at = now;
        route.modified_at = now + TimeDelta::try_seconds(14).unwrap();
        model.add(&route).await?;
        now = now + TimeDelta::try_seconds(1).unwrap();
        route.route_id = "route_id_list3_1".to_string();
        route.unit_id = "unit_id_list3".to_string();
        route.unit_code = "unit_code_list3".to_string();
        route.application_id = "application_id_list3".to_string();
        route.application_code = "application_code_list1".to_string();
        route.device_id = "device_id_list1".to_string();
        route.network_id = "network_id_list3".to_string();
        route.network_code = "network_code_list3".to_string();
        route.network_addr = "network_addr_list3_1".to_string();
        route.created_at = now;
        route.modified_at = now + TimeDelta::try_seconds(12).unwrap();
        model.add(&route).await
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
    expect(list.len()).to_equal(5)?;
    expect(list[0].route_id.as_str()).to_equal("route_id_list1_1")?;
    expect(list[1].route_id.as_str()).to_equal("route_id_list1_2")?;
    expect(list[2].route_id.as_str()).to_equal("route_id_list1_3")?;
    expect(list[3].route_id.as_str()).to_equal("route_id_list2_1")?;
    expect(list[4].route_id.as_str()).to_equal("route_id_list3_1")?;

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
    expect(list.len()).to_equal(5)?;
    expect(list[0].route_id.as_str()).to_equal("route_id_list3_1")?;
    expect(list[1].route_id.as_str()).to_equal("route_id_list2_1")?;
    expect(list[2].route_id.as_str()).to_equal("route_id_list1_1")?;
    expect(list[3].route_id.as_str()).to_equal("route_id_list1_2")?;
    expect(list[4].route_id.as_str()).to_equal("route_id_list1_3")?;

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
    expect(list.len()).to_equal(5)?;
    expect(list[0].route_id.as_str()).to_equal("route_id_list1_3")?;
    expect(list[1].route_id.as_str()).to_equal("route_id_list1_2")?;
    expect(list[2].route_id.as_str()).to_equal("route_id_list1_1")?;
    expect(list[3].route_id.as_str()).to_equal("route_id_list2_1")?;
    expect(list[4].route_id.as_str()).to_equal("route_id_list3_1")?;

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
    expect(list.len()).to_equal(5)?;
    expect(list[0].route_id.as_str()).to_equal("route_id_list3_1")?;
    expect(list[1].route_id.as_str()).to_equal("route_id_list2_1")?;
    expect(list[2].route_id.as_str()).to_equal("route_id_list1_3")?;
    expect(list[3].route_id.as_str()).to_equal("route_id_list1_2")?;
    expect(list[4].route_id.as_str()).to_equal("route_id_list1_1")?;

    let sort_cond = vec![
        SortCond {
            key: SortKey::ApplicationCode,
            asc: true,
        },
        SortCond {
            key: SortKey::NetworkCode,
            asc: true,
        },
        SortCond {
            key: SortKey::NetworkAddr,
            asc: true,
        },
    ];
    opts.sort = Some(sort_cond.as_slice());
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => {
            return Err(format!(
                "list app-asc-code-asc-addr-asc result error: {}",
                e
            ))
        }
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(5)?;
    expect(list[0].route_id.as_str()).to_equal("route_id_list1_1")?;
    expect(list[1].route_id.as_str()).to_equal("route_id_list1_2")?;
    expect(list[2].route_id.as_str()).to_equal("route_id_list1_3")?;
    expect(list[3].route_id.as_str()).to_equal("route_id_list3_1")?;
    expect(list[4].route_id.as_str()).to_equal("route_id_list2_1")?;

    let sort_cond = vec![
        SortCond {
            key: SortKey::ApplicationCode,
            asc: false,
        },
        SortCond {
            key: SortKey::NetworkCode,
            asc: true,
        },
        SortCond {
            key: SortKey::NetworkAddr,
            asc: true,
        },
    ];
    opts.sort = Some(sort_cond.as_slice());
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => {
            return Err(format!(
                "list app-asc-code-asc-addr-asc result error: {}",
                e
            ))
        }
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(5)?;
    expect(list[0].route_id.as_str()).to_equal("route_id_list2_1")?;
    expect(list[1].route_id.as_str()).to_equal("route_id_list3_1")?;
    expect(list[2].route_id.as_str()).to_equal("route_id_list1_1")?;
    expect(list[3].route_id.as_str()).to_equal("route_id_list1_2")?;
    expect(list[4].route_id.as_str()).to_equal("route_id_list1_3")?;

    let sort_cond = vec![SortCond {
        key: SortKey::CreatedAt,
        asc: true,
    }];
    opts.sort = Some(sort_cond.as_slice());
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list created-asc result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(5)?;
    expect(list[0].route_id.as_str()).to_equal("route_id_list1_1")?;
    expect(list[1].route_id.as_str()).to_equal("route_id_list1_2")?;
    expect(list[2].route_id.as_str()).to_equal("route_id_list1_3")?;
    expect(list[3].route_id.as_str()).to_equal("route_id_list2_1")?;
    expect(list[4].route_id.as_str()).to_equal("route_id_list3_1")?;

    let sort_cond = vec![SortCond {
        key: SortKey::CreatedAt,
        asc: false,
    }];
    opts.sort = Some(sort_cond.as_slice());
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list created-desc result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(5)?;
    expect(list[0].route_id.as_str()).to_equal("route_id_list3_1")?;
    expect(list[1].route_id.as_str()).to_equal("route_id_list2_1")?;
    expect(list[2].route_id.as_str()).to_equal("route_id_list1_3")?;
    expect(list[3].route_id.as_str()).to_equal("route_id_list1_2")?;
    expect(list[4].route_id.as_str()).to_equal("route_id_list1_1")?;

    let sort_cond = vec![SortCond {
        key: SortKey::ModifiedAt,
        asc: true,
    }];
    opts.sort = Some(sort_cond.as_slice());
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list modified-asc result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(5)?;
    expect(list[0].route_id.as_str()).to_equal("route_id_list3_1")?;
    expect(list[1].route_id.as_str()).to_equal("route_id_list2_1")?;
    expect(list[2].route_id.as_str()).to_equal("route_id_list1_3")?;
    expect(list[3].route_id.as_str()).to_equal("route_id_list1_2")?;
    expect(list[4].route_id.as_str()).to_equal("route_id_list1_1")?;

    let sort_cond = vec![SortCond {
        key: SortKey::ModifiedAt,
        asc: false,
    }];
    opts.sort = Some(sort_cond.as_slice());
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list modified-desc result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(5)?;
    expect(list[0].route_id.as_str()).to_equal("route_id_list1_1")?;
    expect(list[1].route_id.as_str()).to_equal("route_id_list1_2")?;
    expect(list[2].route_id.as_str()).to_equal("route_id_list1_3")?;
    expect(list[3].route_id.as_str()).to_equal("route_id_list2_1")?;
    expect(list[4].route_id.as_str()).to_equal("route_id_list3_1")?;

    let sort_cond = vec![];
    opts.sort = Some(sort_cond.as_slice());
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list empty result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(5)
}

/// Test `list()` with offset/limit.
pub fn list_offset_limit(runtime: &Runtime, model: &dyn DeviceRouteModel) -> Result<(), String> {
    let now = Utc::now().trunc_subsecs(3);
    let mut route = DeviceRoute {
        route_id: "route_id_list1_1".to_string(),
        unit_id: "unit_id_list".to_string(),
        unit_code: "unit_code_list".to_string(),
        application_id: "application_id_list".to_string(),
        application_code: "application_code_list".to_string(),
        device_id: "device_id_list1_1".to_string(),
        network_id: "network_id_list".to_string(),
        network_code: "network_code_list".to_string(),
        network_addr: "network_addr_list1_1".to_string(),
        profile: "profile".to_string(),
        created_at: now,
        modified_at: now,
    };
    if let Err(e) = runtime.block_on(async {
        model.add(&route).await?;
        route.route_id = "route_id_list1_2".to_string();
        route.device_id = "device_id_list1_2".to_string();
        route.network_addr = "network_addr_list1_2".to_string();
        model.add(&route).await?;
        route.route_id = "route_id_list2_1".to_string();
        route.device_id = "device_id_list2_1".to_string();
        route.network_addr = "network_addr_list2_1".to_string();
        model.add(&route).await?;
        route.route_id = "route_id_list3_1".to_string();
        route.device_id = "device_id_list3_1".to_string();
        route.network_addr = "network_addr_list3_1".to_string();
        model.add(&route).await
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
    expect(list[0].route_id.as_str()).to_equal("route_id_list1_1")?;
    expect(list[1].route_id.as_str()).to_equal("route_id_list1_2")?;
    expect(list[2].route_id.as_str()).to_equal("route_id_list2_1")?;

    opts.limit = Some(5);
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list limit-5 result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(4)?;
    expect(list[0].route_id.as_str()).to_equal("route_id_list1_1")?;
    expect(list[1].route_id.as_str()).to_equal("route_id_list1_2")?;
    expect(list[2].route_id.as_str()).to_equal("route_id_list2_1")?;
    expect(list[3].route_id.as_str()).to_equal("route_id_list3_1")?;

    opts.limit = None;
    opts.offset = Some(2);
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list offset-2 result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(2)?;
    expect(list[0].route_id.as_str()).to_equal("route_id_list2_1")?;
    expect(list[1].route_id.as_str()).to_equal("route_id_list3_1")?;

    opts.limit = Some(0);
    opts.offset = Some(0);
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list offset-limit0 result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(4)?;
    expect(list[0].route_id.as_str()).to_equal("route_id_list1_1")?;
    expect(list[1].route_id.as_str()).to_equal("route_id_list1_2")?;
    expect(list[2].route_id.as_str()).to_equal("route_id_list2_1")?;
    expect(list[3].route_id.as_str()).to_equal("route_id_list3_1")?;

    opts.limit = Some(3);
    opts.offset = Some(3);
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list offset-limit3 result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(1)?;
    expect(list[0].route_id.as_str()).to_equal("route_id_list3_1")
}

/// Test `list()` with cursors.
pub fn list_cursor(runtime: &Runtime, model: &dyn DeviceRouteModel) -> Result<(), String> {
    let now = Utc::now().trunc_subsecs(3);
    let mut route = DeviceRoute {
        route_id: "route_id_list1_1".to_string(),
        unit_id: "unit_id_list".to_string(),
        unit_code: "unit_code_list".to_string(),
        application_id: "application_id_list".to_string(),
        application_code: "application_code_list".to_string(),
        device_id: "device_id_list1_1".to_string(),
        network_id: "network_id_list".to_string(),
        network_code: "network_code_list".to_string(),
        network_addr: "network_addr_list1_1".to_string(),
        profile: "profile".to_string(),
        created_at: now,
        modified_at: now,
    };
    if let Err(e) = runtime.block_on(async {
        model.add(&route).await?;
        route.route_id = "route_id_list1_2".to_string();
        route.device_id = "device_id_list1_2".to_string();
        route.network_addr = "network_addr_list1_2".to_string();
        model.add(&route).await?;
        route.route_id = "route_id_list2_1".to_string();
        route.device_id = "device_id_list2_1".to_string();
        route.network_addr = "network_addr_list2_1".to_string();
        model.add(&route).await?;
        route.route_id = "route_id_list3_1".to_string();
        route.device_id = "device_id_list3_1".to_string();
        route.network_addr = "network_addr_list3_1".to_string();
        model.add(&route).await
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
    expect(list[0].route_id.as_str()).to_equal("route_id_list1_1")?;
    expect(list[1].route_id.as_str()).to_equal("route_id_list1_2")?;
    expect(list[2].route_id.as_str()).to_equal("route_id_list2_1")?;
    expect(cursor.is_some()).to_equal(true)?;
    expect(cursor.as_ref().unwrap().offset()).to_equal(3)?;
    let (list, cursor) = match runtime.block_on(async { model.list(&opts, cursor).await }) {
        Err(e) => return Err(format!("list cursor-3-2 result error: {}", e)),
        Ok((list, cursor)) => (list, cursor),
    };
    expect(list.len()).to_equal(1)?;
    expect(list[0].route_id.as_str()).to_equal("route_id_list3_1")?;
    expect(cursor.is_none()).to_equal(true)?;

    opts.limit = Some(3);
    opts.cursor_max = Some(2);
    let (list, cursor) = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list cursor-2-1 result error: {}", e)),
        Ok((list, cursor)) => (list, cursor),
    };
    expect(list.len()).to_equal(2)?;
    expect(list[0].route_id.as_str()).to_equal("route_id_list1_1")?;
    expect(list[1].route_id.as_str()).to_equal("route_id_list1_2")?;
    expect(cursor.is_some()).to_equal(true)?;
    expect(cursor.as_ref().unwrap().offset()).to_equal(2)?;
    let (list, cursor) = match runtime.block_on(async { model.list(&opts, cursor).await }) {
        Err(e) => return Err(format!("list cursor-2-2 result error: {}", e)),
        Ok((list, cursor)) => (list, cursor),
    };
    expect(list.len()).to_equal(1)?;
    expect(list[0].route_id.as_str()).to_equal("route_id_list2_1")?;
    expect(cursor.is_none()).to_equal(true)?;

    opts.limit = Some(4);
    opts.cursor_max = Some(2);
    let (list, cursor) = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list cursor-2-2-1 result error: {}", e)),
        Ok((list, cursor)) => (list, cursor),
    };
    expect(list.len()).to_equal(2)?;
    expect(list[0].route_id.as_str()).to_equal("route_id_list1_1")?;
    expect(list[1].route_id.as_str()).to_equal("route_id_list1_2")?;
    expect(cursor.is_some()).to_equal(true)?;
    expect(cursor.as_ref().unwrap().offset()).to_equal(2)?;
    let (list, cursor) = match runtime.block_on(async { model.list(&opts, cursor).await }) {
        Err(e) => return Err(format!("list cursor-2-2-2 result error: {}", e)),
        Ok((list, cursor)) => (list, cursor),
    };
    expect(list.len()).to_equal(2)?;
    expect(list[0].route_id.as_str()).to_equal("route_id_list2_1")?;
    expect(list[1].route_id.as_str()).to_equal("route_id_list3_1")?;
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
    expect(list[0].route_id.as_str()).to_equal("route_id_list2_1")?;
    expect(list[1].route_id.as_str()).to_equal("route_id_list3_1")?;
    expect(cursor.is_none()).to_equal(true)
}
