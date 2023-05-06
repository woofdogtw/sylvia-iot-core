use chrono::{Duration, SubsecRound, Utc};
use laboratory::expect;
use tokio::runtime::Runtime;

use sylvia_iot_broker::models::network_route::{
    ListOptions, ListQueryCond, NetworkRoute, NetworkRouteModel, QueryCond, SortCond, SortKey,
};

/// Test `add()`.
pub fn add(runtime: &Runtime, model: &dyn NetworkRouteModel) -> Result<(), String> {
    let now = Utc::now().trunc_subsecs(3);
    let route = NetworkRoute {
        route_id: "route_id_add".to_string(),
        unit_id: "unit_id_add".to_string(),
        unit_code: "unit_code_add".to_string(),
        application_id: "application_id_add".to_string(),
        application_code: "application_code_add".to_string(),
        network_id: "network_id_add".to_string(),
        network_code: "network_code_add".to_string(),
        created_at: now,
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
pub fn add_dup(runtime: &Runtime, model: &dyn NetworkRouteModel) -> Result<(), String> {
    let now = Utc::now().trunc_subsecs(3);
    let mut route = NetworkRoute {
        route_id: "route_id_add".to_string(),
        unit_id: "unit_id_add".to_string(),
        unit_code: "unit_code_add".to_string(),
        application_id: "application_id_add".to_string(),
        application_code: "application_code_add".to_string(),
        network_id: "network_id_add".to_string(),
        network_code: "network_code_add".to_string(),
        created_at: now,
    };
    if let Err(e) = runtime.block_on(async { model.add(&route).await }) {
        return Err(format!("model.add() error: {}", e));
    }
    route.application_id = "application_id_another".to_string();
    route.network_id = "network_id_another".to_string();
    if let Ok(_) = runtime.block_on(async { model.add(&route).await }) {
        return Err("model.add() duplicate network_id should error".to_string());
    }
    route.route_id = "route_id_another".to_string();
    route.application_id = "application_id_add".to_string();
    route.network_id = "network_id_add".to_string();
    if let Ok(_) = runtime.block_on(async { model.add(&route).await }) {
        return Err("model.add() duplicate application_id-network_id should error".to_string());
    }
    Ok(())
}

/// Test `del()` by specifying a route ID.
pub fn del_by_route_id(runtime: &Runtime, model: &dyn NetworkRouteModel) -> Result<(), String> {
    let now = Utc::now().trunc_subsecs(3);
    let route_id_del = "route_id_del";
    let route_id_not_del = "route_id_not_del";
    let mut route = NetworkRoute {
        route_id: route_id_del.to_string(),
        unit_id: "unit_id_del".to_string(),
        unit_code: "unit_code_del".to_string(),
        application_id: "application_id_del".to_string(),
        application_code: "application_code_del".to_string(),
        network_id: "network_id_del".to_string(),
        network_code: "network_code_del".to_string(),
        created_at: now,
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
pub fn del_twice(runtime: &Runtime, model: &dyn NetworkRouteModel) -> Result<(), String> {
    let now = Utc::now().trunc_subsecs(3);
    let route_id_del = "route_id_del";
    let route = NetworkRoute {
        route_id: route_id_del.to_string(),
        unit_id: "unit_id_del".to_string(),
        unit_code: "unit_code_del".to_string(),
        application_id: "application_id_del".to_string(),
        application_code: "application_code_del".to_string(),
        network_id: "network_id_del".to_string(),
        network_code: "network_code_del".to_string(),
        created_at: now,
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
pub fn del_by_unit_id(runtime: &Runtime, model: &dyn NetworkRouteModel) -> Result<(), String> {
    let now = Utc::now().trunc_subsecs(3);
    let route_id_del1 = "route_id_del1";
    let route_id_del2 = "route_id_del2";
    let route_id_not_del = "route_id_not_del";
    let route_id_not_del2 = "route_id_not_del2";
    let mut route = NetworkRoute {
        route_id: route_id_del1.to_string(),
        unit_id: "unit_id_del".to_string(),
        unit_code: "unit_code_del".to_string(),
        application_id: "application_id_del".to_string(),
        application_code: "application_code_del".to_string(),
        network_id: "network_id_del1".to_string(),
        network_code: "network_code_del".to_string(),
        created_at: now,
    };
    let cond = QueryCond {
        unit_id: Some("unit_id_del"),
        ..Default::default()
    };
    if let Err(e) = runtime.block_on(async {
        model.add(&route).await?;
        route.route_id = route_id_del2.to_string();
        route.network_id = "network_id_del2".to_string();
        model.add(&route).await?;
        route.route_id = route_id_not_del.to_string();
        route.network_id = "network_id_not_del".to_string();
        route.unit_id = "unit_id_not_del".to_string();
        route.unit_code = "unit_code_not_del".to_string();
        model.add(&route).await?;
        route.route_id = route_id_not_del2.to_string();
        route.network_id = "network_id_not_del2".to_string();
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
pub fn del_by_unit_route(runtime: &Runtime, model: &dyn NetworkRouteModel) -> Result<(), String> {
    let now = Utc::now().trunc_subsecs(3);
    let route_id_del = "route_id_del";
    let route_id_not_del = "route_id_not_del";
    let mut route = NetworkRoute {
        route_id: route_id_del.to_string(),
        unit_id: "unit_id_del".to_string(),
        unit_code: "unit_code_del".to_string(),
        application_id: "application_id_del".to_string(),
        application_code: "application_code_del".to_string(),
        network_id: "network_id_del".to_string(),
        network_code: "network_code_del".to_string(),
        created_at: now,
    };
    let cond = QueryCond {
        unit_id: Some("unit_id_del"),
        route_id: Some(route_id_del),
        ..Default::default()
    };
    if let Err(e) = runtime.block_on(async {
        model.add(&route).await?;
        route.route_id = route_id_not_del.to_string();
        route.network_id = "network_id_del2".to_string();
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
    model: &dyn NetworkRouteModel,
) -> Result<(), String> {
    let now = Utc::now().trunc_subsecs(3);
    let route_id_del1 = "route_id_del1";
    let route_id_del2 = "route_id_del2";
    let route_id_not_del = "route_id_not_del";
    let route_id_not_del2 = "route_id_not_del2";
    let mut route = NetworkRoute {
        route_id: route_id_del1.to_string(),
        unit_id: "unit_id_del".to_string(),
        unit_code: "unit_code_del".to_string(),
        application_id: "application_id_del".to_string(),
        application_code: "application_code_del".to_string(),
        network_id: "network_id_del1".to_string(),
        network_code: "network_code_del".to_string(),
        created_at: now,
    };
    let cond = QueryCond {
        application_id: Some("application_id_del"),
        ..Default::default()
    };
    if let Err(e) = runtime.block_on(async {
        model.add(&route).await?;
        route.route_id = route_id_del2.to_string();
        route.network_id = "network_id_del2".to_string();
        model.add(&route).await?;
        route.route_id = route_id_not_del.to_string();
        route.network_id = "network_id_del".to_string();
        route.application_id = "application_id_not_del".to_string();
        model.add(&route).await?;
        route.route_id = route_id_not_del2.to_string();
        route.network_id = "network_id_del2".to_string();
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
pub fn del_by_network_id(runtime: &Runtime, model: &dyn NetworkRouteModel) -> Result<(), String> {
    let now = Utc::now().trunc_subsecs(3);
    let route_id_del1 = "route_id_del1";
    let route_id_del2 = "route_id_del2";
    let route_id_not_del = "route_id_not_del";
    let route_id_not_del2 = "route_id_not_del2";
    let mut route = NetworkRoute {
        route_id: route_id_del1.to_string(),
        unit_id: "unit_id_del".to_string(),
        unit_code: "unit_code_del".to_string(),
        application_id: "application_id_del".to_string(),
        application_code: "application_code_del".to_string(),
        network_id: "network_id_del".to_string(),
        network_code: "network_code_del".to_string(),
        created_at: now,
    };
    let cond = QueryCond {
        network_id: Some("network_id_del"),
        ..Default::default()
    };
    if let Err(e) = runtime.block_on(async {
        model.add(&route).await?;
        route.route_id = route_id_del2.to_string();
        route.application_id = "application_id_del2".to_string();
        model.add(&route).await?;
        route.route_id = route_id_not_del.to_string();
        route.network_id = "network_id_not_del".to_string();
        model.add(&route).await?;
        route.route_id = route_id_not_del2.to_string();
        route.network_id = "network_id_not_del2".to_string();
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

/// Test `count()`.
pub fn count(runtime: &Runtime, model: &dyn NetworkRouteModel) -> Result<(), String> {
    let now = Utc::now().trunc_subsecs(3);
    let mut route = NetworkRoute {
        route_id: "route_id_count1_1".to_string(),
        unit_id: "unit_id_count".to_string(),
        unit_code: "unit_code_count".to_string(),
        application_id: "application_id_count".to_string(),
        application_code: "application_code_count".to_string(),
        network_id: "network_id_count1".to_string(),
        network_code: "network_code_count1".to_string(),
        created_at: now,
    };
    if let Err(e) = runtime.block_on(async {
        model.add(&route).await?;
        route.route_id = "route_id_count1_2".to_string();
        route.network_id = "network_id_count1_2".to_string();
        route.network_code = "network_code_count1_2".to_string();
        model.add(&route).await?;
        route.route_id = "route_id_count1_3".to_string();
        route.network_id = "network_id_count1_3".to_string();
        route.network_code = "network_code_count1_3".to_string();
        model.add(&route).await?;
        route.route_id = "route_id_count2_1".to_string();
        route.application_id = "application_id_count2".to_string();
        route.application_code = "application_code_count2".to_string();
        route.network_id = "network_id_count1".to_string();
        route.network_code = "network_code_count1".to_string();
        model.add(&route).await?;
        route.route_id = "route_id_count3_1".to_string();
        route.unit_id = "unit_id_count3".to_string();
        route.unit_code = "unit_code_count3".to_string();
        route.application_id = "application_id_count3".to_string();
        route.application_code = "application_code_count3".to_string();
        route.network_id = "network_id_count1".to_string();
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
        network_id: Some("network_id_count1"),
        ..Default::default()
    };
    let count = match runtime.block_on(async { model.count(&cond).await }) {
        Err(e) => return Err(format!("count network_id result error: {}", e)),
        Ok(count) => count,
    };
    expect(count).to_equal(3)?;

    let cond = ListQueryCond {
        network_code: Some("network_code_count1"),
        ..Default::default()
    };
    let count = match runtime.block_on(async { model.count(&cond).await }) {
        Err(e) => return Err(format!("count network_code result error: {}", e)),
        Ok(count) => count,
    };
    expect(count).to_equal(3)?;

    let cond = ListQueryCond {
        route_id: Some("route_id_count1_1"),
        network_id: Some("network_id_count1"),
        ..Default::default()
    };
    let count = match runtime.block_on(async { model.count(&cond).await }) {
        Err(e) => return Err(format!("count route-network result error: {}", e)),
        Ok(count) => count,
    };
    expect(count).to_equal(1)?;

    let cond = ListQueryCond {
        route_id: Some("route_id_count1_1"),
        network_code: Some("network_code_count1"),
        ..Default::default()
    };
    let count = match runtime.block_on(async { model.count(&cond).await }) {
        Err(e) => return Err(format!("count route-network-code result error: {}", e)),
        Ok(count) => count,
    };
    expect(count).to_equal(1)?;

    let cond = ListQueryCond {
        route_id: Some("route_id_count3_1"),
        network_id: Some("network_id_count1_3"),
        ..Default::default()
    };
    let count = match runtime.block_on(async { model.count(&cond).await }) {
        Err(e) => return Err(format!("count route3-network result error: {}", e)),
        Ok(count) => count,
    };
    expect(count).to_equal(0)?;

    let cond = ListQueryCond {
        route_id: Some("route_id_count3_1"),
        network_code: Some("network_code_count1_3"),
        ..Default::default()
    };
    let count = match runtime.block_on(async { model.count(&cond).await }) {
        Err(e) => return Err(format!("count route3-network-code result error: {}", e)),
        Ok(count) => count,
    };
    expect(count).to_equal(0)
}

/// Test `list()`.
pub fn list(runtime: &Runtime, model: &dyn NetworkRouteModel) -> Result<(), String> {
    let now = Utc::now().trunc_subsecs(3);
    let mut route = NetworkRoute {
        route_id: "route_id_list1_1".to_string(),
        unit_id: "unit_id_list".to_string(),
        unit_code: "unit_code_list".to_string(),
        application_id: "application_id_list".to_string(),
        application_code: "application_code_list".to_string(),
        network_id: "network_id_list1".to_string(),
        network_code: "network_code_list1".to_string(),
        created_at: now,
    };
    if let Err(e) = runtime.block_on(async {
        model.add(&route).await?;
        route.route_id = "route_id_list1_2".to_string();
        route.network_id = "network_id_list1_2".to_string();
        route.network_code = "network_code_list1_2".to_string();
        model.add(&route).await?;
        route.route_id = "route_id_list1_3".to_string();
        route.network_id = "network_id_list1_3".to_string();
        route.network_code = "network_code_list1_3".to_string();
        model.add(&route).await?;
        route.route_id = "route_id_list2_1".to_string();
        route.application_id = "application_id_list2".to_string();
        route.application_code = "application_code_list2".to_string();
        route.network_id = "network_id_list1".to_string();
        route.network_code = "network_code_list1".to_string();
        model.add(&route).await?;
        route.route_id = "route_id_list3_1".to_string();
        route.unit_id = "unit_id_list3".to_string();
        route.unit_code = "unit_code_list3".to_string();
        route.application_id = "application_id_list3".to_string();
        route.application_code = "application_code_list3".to_string();
        route.network_id = "network_id_list1".to_string();
        route.network_code = "network_code_list1".to_string();
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
        network_id: Some("network_id_list1"),
        ..Default::default()
    };
    opts.cond = &cond;
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list network_id result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(3)?;

    let cond = ListQueryCond {
        network_code: Some("network_code_list1"),
        ..Default::default()
    };
    opts.cond = &cond;
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list network_code result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(3)?;

    let cond = ListQueryCond {
        route_id: Some("route_id_list1_1"),
        network_id: Some("network_id_list1"),
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
        network_code: Some("network_code_list1"),
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
        network_id: Some("network_id_list1_3"),
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
        network_code: Some("network_code_list1_3"),
        ..Default::default()
    };
    opts.cond = &cond;
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list route3-network-code result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(0)
}

/// Test `list()` with sorting.
pub fn list_sort(runtime: &Runtime, model: &dyn NetworkRouteModel) -> Result<(), String> {
    let mut now = Utc::now().trunc_subsecs(3);
    let mut route = NetworkRoute {
        route_id: "route_id_list1_1".to_string(),
        unit_id: "unit_id_list".to_string(),
        unit_code: "unit_code_list".to_string(),
        application_id: "application_id_list".to_string(),
        application_code: "application_code_list".to_string(),
        network_id: "network_id_list1".to_string(),
        network_code: "network_code_list".to_string(),
        created_at: now,
    };
    if let Err(e) = runtime.block_on(async {
        model.add(&route).await?;
        now = now + Duration::seconds(1);
        route.route_id = "route_id_list1_2".to_string();
        route.network_id = "network_id_list1_2".to_string();
        route.network_code = "network_code_list1_2".to_string();
        route.created_at = now;
        model.add(&route).await?;
        now = now + Duration::seconds(1);
        route.route_id = "route_id_list2_1".to_string();
        route.application_id = "application_id_list2".to_string();
        route.application_code = "application_code_list2".to_string();
        route.network_id = "network_id_list2".to_string();
        route.network_code = "network_code_list2".to_string();
        route.created_at = now;
        model.add(&route).await?;
        now = now + Duration::seconds(1);
        route.route_id = "route_id_list3_1".to_string();
        route.unit_id = "unit_id_list3".to_string();
        route.unit_code = "unit_code_list3".to_string();
        route.application_id = "application_id_list3".to_string();
        route.application_code = "application_code_list3".to_string();
        route.network_id = "network_id_list3".to_string();
        route.network_code = "network_code_list3".to_string();
        route.created_at = now;
        model.add(&route).await
    }) {
        return Err(format!("model.add() error: {}", e));
    }

    let cond = ListQueryCond {
        ..Default::default()
    };
    let sort_cond = vec![
        SortCond {
            key: SortKey::ApplicationCode,
            asc: true,
        },
        SortCond {
            key: SortKey::NetworkCode,
            asc: true,
        },
    ];
    let opts = ListOptions {
        cond: &cond,
        offset: None,
        limit: None,
        sort: Some(sort_cond.as_slice()),
        cursor_max: None,
    };
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => {
            return Err(format!(
                "list application-asc-network-asc result error: {}",
                e
            ))
        }
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(4)?;
    expect(list[0].route_id.as_str()).to_equal("route_id_list1_1")?;
    expect(list[1].route_id.as_str()).to_equal("route_id_list1_2")?;
    expect(list[2].route_id.as_str()).to_equal("route_id_list2_1")?;
    expect(list[3].route_id.as_str()).to_equal("route_id_list3_1")?;

    let cond = ListQueryCond {
        ..Default::default()
    };
    let sort_cond = vec![
        SortCond {
            key: SortKey::ApplicationCode,
            asc: false,
        },
        SortCond {
            key: SortKey::NetworkCode,
            asc: true,
        },
    ];
    let opts = ListOptions {
        cond: &cond,
        offset: None,
        limit: None,
        sort: Some(sort_cond.as_slice()),
        cursor_max: None,
    };
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => {
            return Err(format!(
                "list application-desc-network-asc result error: {}",
                e
            ))
        }
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(4)?;
    expect(list[0].route_id.as_str()).to_equal("route_id_list3_1")?;
    expect(list[1].route_id.as_str()).to_equal("route_id_list2_1")?;
    expect(list[2].route_id.as_str()).to_equal("route_id_list1_1")?;
    expect(list[3].route_id.as_str()).to_equal("route_id_list1_2")?;

    let cond = ListQueryCond {
        ..Default::default()
    };
    let sort_cond = vec![
        SortCond {
            key: SortKey::ApplicationCode,
            asc: true,
        },
        SortCond {
            key: SortKey::NetworkCode,
            asc: false,
        },
    ];
    let opts = ListOptions {
        cond: &cond,
        offset: None,
        limit: None,
        sort: Some(sort_cond.as_slice()),
        cursor_max: None,
    };
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => {
            return Err(format!(
                "list application-asc-network-desc result error: {}",
                e
            ))
        }
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(4)?;
    expect(list[0].route_id.as_str()).to_equal("route_id_list1_2")?;
    expect(list[1].route_id.as_str()).to_equal("route_id_list1_1")?;
    expect(list[2].route_id.as_str()).to_equal("route_id_list2_1")?;
    expect(list[3].route_id.as_str()).to_equal("route_id_list3_1")?;

    let cond = ListQueryCond {
        ..Default::default()
    };
    let sort_cond = vec![
        SortCond {
            key: SortKey::ApplicationCode,
            asc: false,
        },
        SortCond {
            key: SortKey::NetworkCode,
            asc: false,
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
        Err(e) => {
            return Err(format!(
                "list application-desc-network-desc result error: {}",
                e
            ))
        }
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(4)?;
    expect(list[0].route_id.as_str()).to_equal("route_id_list3_1")?;
    expect(list[1].route_id.as_str()).to_equal("route_id_list2_1")?;
    expect(list[2].route_id.as_str()).to_equal("route_id_list1_2")?;
    expect(list[3].route_id.as_str()).to_equal("route_id_list1_1")?;

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
    expect(list[0].route_id.as_str()).to_equal("route_id_list1_1")?;
    expect(list[1].route_id.as_str()).to_equal("route_id_list1_2")?;
    expect(list[2].route_id.as_str()).to_equal("route_id_list2_1")?;
    expect(list[3].route_id.as_str()).to_equal("route_id_list3_1")?;

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
    expect(list[0].route_id.as_str()).to_equal("route_id_list3_1")?;
    expect(list[1].route_id.as_str()).to_equal("route_id_list2_1")?;
    expect(list[2].route_id.as_str()).to_equal("route_id_list1_2")?;
    expect(list[3].route_id.as_str()).to_equal("route_id_list1_1")?;

    let sort_cond = vec![];
    opts.sort = Some(sort_cond.as_slice());
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list empty result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(4)
}

/// Test `list()` with offset/limit.
pub fn list_offset_limit(runtime: &Runtime, model: &dyn NetworkRouteModel) -> Result<(), String> {
    let now = Utc::now().trunc_subsecs(3);
    let mut route = NetworkRoute {
        route_id: "route_id_list1_1".to_string(),
        unit_id: "unit_id_list".to_string(),
        unit_code: "unit_code_list".to_string(),
        application_id: "application_id_list".to_string(),
        application_code: "application_code_list".to_string(),
        network_id: "network_id_list".to_string(),
        network_code: "network_code_list".to_string(),
        created_at: now,
    };
    if let Err(e) = runtime.block_on(async {
        model.add(&route).await?;
        route.route_id = "route_id_list1_2".to_string();
        route.network_id = "network_id_list1_2".to_string();
        route.network_code = "network_code_list1_2".to_string();
        model.add(&route).await?;
        route.route_id = "route_id_list2_1".to_string();
        route.network_id = "network_id_list2_1".to_string();
        route.network_code = "network_code_list2_1".to_string();
        model.add(&route).await?;
        route.route_id = "route_id_list3_1".to_string();
        route.network_id = "network_id_list3_1".to_string();
        route.network_code = "network_code_list3_1".to_string();
        model.add(&route).await
    }) {
        return Err(format!("model.add() error: {}", e));
    }

    let cond = ListQueryCond {
        ..Default::default()
    };
    let sort_cond = vec![SortCond {
        key: SortKey::NetworkCode,
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
pub fn list_cursor(runtime: &Runtime, model: &dyn NetworkRouteModel) -> Result<(), String> {
    let now = Utc::now().trunc_subsecs(3);
    let mut route = NetworkRoute {
        route_id: "route_id_list1_1".to_string(),
        unit_id: "unit_id_list".to_string(),
        unit_code: "unit_code_list".to_string(),
        application_id: "application_id_list".to_string(),
        application_code: "application_code_list".to_string(),
        network_id: "network_id_list".to_string(),
        network_code: "network_code_list".to_string(),
        created_at: now,
    };
    if let Err(e) = runtime.block_on(async {
        model.add(&route).await?;
        route.route_id = "route_id_list1_2".to_string();
        route.network_id = "network_id_list1_2".to_string();
        route.network_code = "network_code_list1_2".to_string();
        model.add(&route).await?;
        route.route_id = "route_id_list2_1".to_string();
        route.network_id = "network_id_list2_1".to_string();
        route.network_code = "network_code_list2_1".to_string();
        model.add(&route).await?;
        route.route_id = "route_id_list3_1".to_string();
        route.network_id = "network_id_list3_1".to_string();
        route.network_code = "network_code_list3_1".to_string();
        model.add(&route).await
    }) {
        return Err(format!("model.add() error: {}", e));
    }

    let cond = ListQueryCond {
        ..Default::default()
    };
    let sort_cond = vec![SortCond {
        key: SortKey::NetworkCode,
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
