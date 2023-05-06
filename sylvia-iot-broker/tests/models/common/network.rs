use chrono::{Duration, SubsecRound, Utc};
use laboratory::expect;
use serde_json::{Map, Value};
use tokio::runtime::Runtime;

use sylvia_iot_broker::models::network::{
    ListOptions, ListQueryCond, Network, NetworkModel, QueryCond, SortCond, SortKey,
    UpdateQueryCond, Updates,
};

/// Test `add()`.
pub fn add(runtime: &Runtime, model: &dyn NetworkModel) -> Result<(), String> {
    let now = Utc::now().trunc_subsecs(3);
    let network = Network {
        network_id: "network_id_add_none".to_string(),
        code: "code_add_none".to_string(),
        unit_id: None,
        unit_code: None,
        created_at: now,
        modified_at: now,
        host_uri: "host_uri_add".to_string(),
        name: "name_add".to_string(),
        info: Map::<String, Value>::new(),
    };
    if let Err(e) = runtime.block_on(async { model.add(&network).await }) {
        return Err(format!("model.add() none error: {}", e));
    }

    let cond = QueryCond {
        network_id: Some(&network.network_id),
        ..Default::default()
    };
    let get_network = match runtime.block_on(async { model.get(&cond).await }) {
        Err(e) => return Err(format!("model.get() none error: {}", e)),
        Ok(network) => match network {
            None => return Err("should get none one".to_string()),
            Some(network) => network,
        },
    };
    expect(get_network).to_equal(network)?;

    let mut info = Map::<String, Value>::new();
    info.insert("boolean".to_string(), Value::Bool(true));
    info.insert("string".to_string(), Value::String("string".to_string()));
    info.insert("number".to_string(), Value::Number(1.into()));
    let info_object_array = vec![Value::String("array".to_string())];
    let mut info_object = Map::<String, Value>::new();
    info_object.insert("array".to_string(), Value::Array(info_object_array));
    info.insert("object".to_string(), Value::Object(info_object));
    let network = Network {
        network_id: "network_id_add_some".to_string(),
        code: "code_add_some".to_string(),
        unit_id: Some("unit_id_add_some".to_string()),
        unit_code: Some("unit_code_add_some".to_string()),
        created_at: now,
        modified_at: now,
        host_uri: "host_uri_add".to_string(),
        name: "name_add".to_string(),
        info: info.clone(),
    };
    if let Err(e) = runtime.block_on(async { model.add(&network).await }) {
        return Err(format!("model.add() some error: {}", e));
    }

    let cond = QueryCond {
        network_id: Some(&network.network_id),
        ..Default::default()
    };
    let get_network = match runtime.block_on(async { model.get(&cond).await }) {
        Err(e) => return Err(format!("model.get() some error: {}", e)),
        Ok(network) => match network {
            None => return Err("should get some one".to_string()),
            Some(network) => network,
        },
    };
    expect(get_network).to_equal(network)
}

/// Test `add()` with duplicate key.
pub fn add_dup(runtime: &Runtime, model: &dyn NetworkModel) -> Result<(), String> {
    let now = Utc::now().trunc_subsecs(3);
    let mut network = Network {
        network_id: "network_id_add".to_string(),
        code: "code_add".to_string(),
        unit_id: None,
        unit_code: None,
        created_at: now,
        modified_at: now,
        host_uri: "host_uri_add".to_string(),
        name: "name_add".to_string(),
        info: Map::<String, Value>::new(),
    };
    if let Err(e) = runtime.block_on(async { model.add(&network).await }) {
        return Err(format!("model.add() error: {}", e));
    }
    network.code = "code_not_exist".to_string();
    if let Ok(_) = runtime.block_on(async { model.add(&network).await }) {
        return Err("model.add() duplicate network_id should error".to_string());
    }
    network.network_id = "network_id_not_exist".to_string();
    network.code = "code_add".to_string();
    if let Ok(_) = runtime.block_on(async { model.add(&network).await }) {
        return Err("model.add() duplicate code should error".to_string());
    }
    network.network_id = "network_id_not_exist_another".to_string();
    network.unit_id = Some("unit_another".to_string());
    network.unit_code = Some("unit_code_another".to_string());
    if let Err(_) = runtime.block_on(async { model.add(&network).await }) {
        return Err("model.add() should not duplicate in another unit".to_string());
    }
    network.network_id = "network_id_not_exist_another2".to_string();
    if let Ok(_) = runtime.block_on(async { model.add(&network).await }) {
        return Err("model.add() duplicate code in another unit should error".to_string());
    }
    network.network_id = "network_id_not_exist_more".to_string();
    network.unit_id = Some("unit_more".to_string());
    network.unit_code = Some("unit_code_more".to_string());
    if let Err(_) = runtime.block_on(async { model.add(&network).await }) {
        return Err("model.add() should not duplicate in more another unit".to_string());
    }
    network.network_id = "network_id_not_exist_more2".to_string();
    if let Ok(_) = runtime.block_on(async { model.add(&network).await }) {
        return Err("model.add() duplicate code in more another unit should error".to_string());
    }
    Ok(())
}

/// Test `del()` by specifying a network ID.
pub fn del_by_network_id(runtime: &Runtime, model: &dyn NetworkModel) -> Result<(), String> {
    let now = Utc::now().trunc_subsecs(3);
    let network_id_del = "network_id_del";
    let network_id_not_del = "network_id_not_del";
    let mut network = Network {
        network_id: network_id_del.to_string(),
        code: "code_del".to_string(),
        unit_id: None,
        unit_code: None,
        created_at: now,
        modified_at: now,
        host_uri: "host_uri_del".to_string(),
        name: "name_del".to_string(),
        info: Map::<String, Value>::new(),
    };
    let mut cond = QueryCond {
        network_id: Some(network_id_del),
        ..Default::default()
    };
    if let Err(e) = runtime.block_on(async {
        model.add(&network).await?;
        network.network_id = network_id_not_del.to_string();
        network.code = "code_not_del".to_string();
        model.add(&network).await?;
        model.del(&cond).await
    }) {
        return Err(format!("model.add/del error: {}", e));
    }
    match runtime.block_on(async { model.get(&cond).await }) {
        Err(e) => return Err(format!("model.get() error: {}", e)),
        Ok(network) => match network {
            None => (),
            Some(_) => return Err("delete fail".to_string()),
        },
    }
    cond.network_id = Some(network_id_not_del);
    match runtime.block_on(async { model.get(&cond).await }) {
        Err(e) => Err(format!("model.get() not delete one error: {}", e)),
        Ok(network) => match network {
            None => Err("delete wrong one".to_string()),
            Some(_) => Ok(()),
        },
    }
}

/// Test `del()` twice.
pub fn del_twice(runtime: &Runtime, model: &dyn NetworkModel) -> Result<(), String> {
    let now = Utc::now().trunc_subsecs(3);
    let network_id_del = "network_id_del";
    let network = Network {
        network_id: network_id_del.to_string(),
        code: "code_del".to_string(),
        unit_id: None,
        unit_code: None,
        created_at: now,
        modified_at: now,
        host_uri: "host_uri_del".to_string(),
        name: "name_del".to_string(),
        info: Map::<String, Value>::new(),
    };
    let cond = QueryCond {
        network_id: Some(network_id_del),
        ..Default::default()
    };
    if let Err(e) = runtime.block_on(async {
        model.add(&network).await?;
        model.del(&cond).await?;
        model.del(&cond).await
    }) {
        return Err(format!("model.add/del error: {}", e));
    }
    Ok(())
}

/// Test `del()` by specifying a unit ID.
pub fn del_by_unit_id(runtime: &Runtime, model: &dyn NetworkModel) -> Result<(), String> {
    let now = Utc::now().trunc_subsecs(3);
    let network_id_del1 = "network_id_del1";
    let network_id_del2 = "network_id_del2";
    let network_id_not_del = "network_id_not_del";
    let network_id_not_del2 = "network_id_not_del2";
    let mut network = Network {
        network_id: network_id_del1.to_string(),
        code: "code_del".to_string(),
        unit_id: Some("unit_id_del".to_string()),
        unit_code: Some("unit_code_del".to_string()),
        created_at: now,
        modified_at: now,
        host_uri: "host_uri_del".to_string(),
        name: "name_del".to_string(),
        info: Map::<String, Value>::new(),
    };
    let cond = QueryCond {
        unit_id: Some(Some("unit_id_del")),
        ..Default::default()
    };
    if let Err(e) = runtime.block_on(async {
        model.add(&network).await?;
        network.network_id = network_id_del2.to_string();
        network.code = "code_del2".to_string();
        model.add(&network).await?;
        network.network_id = network_id_not_del.to_string();
        network.code = "code_not_del".to_string();
        network.unit_id = Some("unit_id_not_del".to_string());
        network.unit_code = Some("unit_code_not_del".to_string());
        model.add(&network).await?;
        network.unit_id = None;
        network.unit_code = None;
        network.network_id = network_id_not_del2.to_string();
        model.add(&network).await?;
        model.del(&cond).await
    }) {
        return Err(format!("model.add/del error: {}", e));
    }
    let mut cond = QueryCond {
        network_id: Some(network_id_del1),
        ..Default::default()
    };
    match runtime.block_on(async { model.get(&cond).await }) {
        Err(e) => return Err(format!("model.get() delete network1 error: {}", e)),
        Ok(network) => match network {
            None => (),
            Some(_) => return Err("delete network1 fail".to_string()),
        },
    }
    cond.network_id = Some(network_id_del2);
    match runtime.block_on(async { model.get(&cond).await }) {
        Err(e) => return Err(format!("model.get() delete network2 error: {}", e)),
        Ok(network) => match network {
            None => (),
            Some(_) => return Err("delete network2 fail".to_string()),
        },
    }
    cond.network_id = Some(network_id_not_del);
    match runtime.block_on(async { model.get(&cond).await }) {
        Err(e) => return Err(format!("model.get() not delete one error: {}", e)),
        Ok(network) => match network {
            None => return Err("delete wrong one".to_string()),
            Some(_) => (),
        },
    }
    cond.network_id = Some(network_id_not_del2);
    match runtime.block_on(async { model.get(&cond).await }) {
        Err(e) => return Err(format!("model.get() not delete none one error: {}", e)),
        Ok(network) => match network {
            None => return Err("delete wrong none one".to_string()),
            Some(_) => (),
        },
    }

    let cond = QueryCond {
        unit_id: Some(None),
        ..Default::default()
    };
    if let Err(e) = runtime.block_on(async { model.del(&cond).await }) {
        return Err(format!("model.del error: {}", e));
    }
    let mut cond = QueryCond {
        network_id: Some(network_id_not_del),
        ..Default::default()
    };
    match runtime.block_on(async { model.get(&cond).await }) {
        Err(e) => return Err(format!("model.get() not delete unit-none one error: {}", e)),
        Ok(network) => match network {
            None => return Err("delete wrong one".to_string()),
            Some(_) => (),
        },
    }
    cond.network_id = Some(network_id_not_del2);
    match runtime.block_on(async { model.get(&cond).await }) {
        Err(e) => Err(format!("model.get() delete unit-none one error: {}", e)),
        Ok(network) => match network {
            None => Ok(()),
            Some(_) => Err("delete unit-none fail".to_string()),
        },
    }
}

/// Test `del()` by specifying a pair of unit ID and network ID.
pub fn del_by_unit_network(runtime: &Runtime, model: &dyn NetworkModel) -> Result<(), String> {
    let now = Utc::now().trunc_subsecs(3);
    let network_id_del = "network_id_del";
    let network_id_not_del = "network_id_not_del";
    let mut network = Network {
        network_id: network_id_del.to_string(),
        code: "code_del".to_string(),
        unit_id: Some("unit_id_del".to_string()),
        unit_code: Some("unit_code_del".to_string()),
        created_at: now,
        modified_at: now,
        host_uri: "host_uri_del".to_string(),
        name: "name_del".to_string(),
        info: Map::<String, Value>::new(),
    };
    let mut cond = QueryCond {
        network_id: Some(network_id_del),
        unit_id: Some(Some("unit_id_del")),
        ..Default::default()
    };
    if let Err(e) = runtime.block_on(async {
        model.add(&network).await?;
        network.network_id = network_id_not_del.to_string();
        network.code = "code_not_del".to_string();
        model.add(&network).await?;
        model.del(&cond).await
    }) {
        return Err(format!("model.add/del error: {}", e));
    }
    match runtime.block_on(async { model.get(&cond).await }) {
        Err(e) => return Err(format!("model.get() error: {}", e)),
        Ok(network) => match network {
            None => (),
            Some(_) => return Err("delete fail".to_string()),
        },
    }
    cond.network_id = Some(network_id_not_del);
    match runtime.block_on(async { model.get(&cond).await }) {
        Err(e) => Err(format!("model.get() not delete one error: {}", e)),
        Ok(network) => match network {
            None => Err("delete wrong one".to_string()),
            Some(_) => Ok(()),
        },
    }
}

/// Test `update()`.
pub fn update(runtime: &Runtime, model: &dyn NetworkModel) -> Result<(), String> {
    let now = Utc::now().trunc_subsecs(3);
    let network_id_update = "network_id_update";
    let unit_id_update = "unit_id_update";
    let network = Network {
        network_id: network_id_update.to_string(),
        code: "code_update".to_string(),
        unit_id: Some(unit_id_update.to_string()),
        unit_code: Some("unit_code_update".to_string()),
        created_at: now,
        modified_at: now,
        host_uri: "host_uri_update".to_string(),
        name: "name_update".to_string(),
        info: Map::<String, Value>::new(),
    };
    if let Err(e) = runtime.block_on(async { model.add(&network).await }) {
        return Err(format!("model.add() error: {}", e));
    }

    let get_cond = QueryCond {
        network_id: Some(network_id_update),
        unit_id: Some(Some(unit_id_update)),
        ..Default::default()
    };
    let update_cond = UpdateQueryCond {
        network_id: network_id_update,
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
    let get_network = match runtime.block_on(async { model.get(&get_cond).await }) {
        Err(e) => return Err(format!("model.get() one error: {}", e)),
        Ok(network) => match network {
            None => return Err(format!("model.get() one should get one")),
            Some(network) => network,
        },
    };
    expect(get_network.network_id.as_str()).to_equal(network.network_id.as_str())?;
    expect(get_network.code.as_str()).to_equal(network.code.as_str())?;
    expect(get_network.unit_id.as_ref()).to_equal(network.unit_id.as_ref())?;
    expect(get_network.unit_code.as_ref()).to_equal(network.unit_code.as_ref())?;
    expect(get_network.created_at).to_equal(network.created_at)?;
    expect(get_network.modified_at).to_equal(now)?;
    expect(get_network.host_uri.as_str()).to_equal(network.host_uri.as_str())?;
    expect(get_network.name.as_str()).to_equal(network.name.as_str())?;
    expect(get_network.info).to_equal(network.info.clone())?;

    // Update all fields.
    let now = now + Duration::milliseconds(1);
    let mut info = Map::<String, Value>::new();
    info.insert("key".to_string(), Value::String("value".to_string()));
    let updates = Updates {
        modified_at: Some(now),
        host_uri: Some("host_uri_update_all"),
        name: Some("name_update_all"),
        info: Some(&info),
    };
    if let Err(e) = runtime.block_on(async { model.update(&update_cond, &updates).await }) {
        return Err(format!("model.update() all error: {}", e));
    }
    let get_network = match runtime.block_on(async { model.get(&get_cond).await }) {
        Err(e) => return Err(format!("model.get() all error: {}", e)),
        Ok(network) => match network {
            None => return Err(format!("model.get() all should get one")),
            Some(network) => network,
        },
    };
    expect(get_network.network_id.as_str()).to_equal(network.network_id.as_str())?;
    expect(get_network.code.as_str()).to_equal(network.code.as_str())?;
    expect(get_network.unit_id.as_ref()).to_equal(network.unit_id.as_ref())?;
    expect(get_network.unit_code.as_ref()).to_equal(network.unit_code.as_ref())?;
    expect(get_network.created_at).to_equal(network.created_at)?;
    expect(get_network.modified_at).to_equal(now)?;
    expect(get_network.host_uri.as_str()).to_equal("host_uri_update_all")?;
    expect(get_network.name.as_str()).to_equal("name_update_all")?;
    expect(get_network.info).to_equal(info)?;

    // Update all fields back to None.
    let now = now + Duration::milliseconds(1);
    let info = Map::<String, Value>::new();
    let updates = Updates {
        modified_at: Some(now),
        host_uri: Some("host_uri_update"),
        name: Some(""),
        info: Some(&info),
    };
    if let Err(e) = runtime.block_on(async { model.update(&update_cond, &updates).await }) {
        return Err(format!("model.update() none error: {}", e));
    }
    let get_network = match runtime.block_on(async { model.get(&get_cond).await }) {
        Err(e) => return Err(format!("model.get() none error: {}", e)),
        Ok(network) => match network {
            None => return Err(format!("model.get() none should get one")),
            Some(network) => network,
        },
    };
    expect(get_network.network_id.as_str()).to_equal(network.network_id.as_str())?;
    expect(get_network.code.as_str()).to_equal(network.code.as_str())?;
    expect(get_network.unit_id.as_ref()).to_equal(network.unit_id.as_ref())?;
    expect(get_network.unit_code.as_ref()).to_equal(network.unit_code.as_ref())?;
    expect(get_network.created_at).to_equal(network.created_at)?;
    expect(get_network.modified_at).to_equal(now)?;
    expect(get_network.host_uri.as_str()).to_equal(network.host_uri.as_str())?;
    expect(get_network.name.as_str()).to_equal("")?;
    expect(get_network.info).to_equal(info)
}

/// Test `update()` with a non-exist condition.
pub fn update_not_exist(runtime: &Runtime, model: &dyn NetworkModel) -> Result<(), String> {
    let cond = UpdateQueryCond {
        network_id: "network_id_not_exist",
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
pub fn update_invalid(runtime: &Runtime, model: &dyn NetworkModel) -> Result<(), String> {
    let cond = UpdateQueryCond {
        network_id: "network_id",
    };
    let updates = Updates {
        modified_at: None,
        host_uri: None,
        name: None,
        info: None,
    };
    if let Err(e) = runtime.block_on(async { model.update(&cond, &updates).await }) {
        return Err(format!("model.update() error: {}", e));
    }
    Ok(())
}

/// Test `count()`.
pub fn count(runtime: &Runtime, model: &dyn NetworkModel) -> Result<(), String> {
    let now = Utc::now().trunc_subsecs(3);
    let mut network = Network {
        network_id: "network_id_count1_1".to_string(),
        code: "code_count1_1".to_string(),
        unit_id: Some("unit_id_count".to_string()),
        unit_code: Some("unit_code_count".to_string()),
        created_at: now,
        modified_at: now,
        host_uri: "host_uri_count_1".to_string(),
        name: "name_count_1".to_string(),
        info: Map::<String, Value>::new(),
    };
    if let Err(e) = runtime.block_on(async {
        model.add(&network).await?;
        network.network_id = "network_id_count1_2".to_string();
        network.code = "code_count1_2".to_string();
        network.name = "name_count1_2".to_string();
        model.add(&network).await?;
        network.network_id = "network_id_count2_1".to_string();
        network.code = "code_count2_1".to_string();
        network.name = "name_count2_1".to_string();
        model.add(&network).await?;
        network.network_id = "network_id_count3_1".to_string();
        network.code = "code_count3_1".to_string();
        network.unit_id = None;
        network.unit_code = None;
        network.name = "name_count_1".to_string();
        model.add(&network).await
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
        network_id: Some("network_id_count1_1"),
        ..Default::default()
    };
    let count = match runtime.block_on(async { model.count(&cond).await }) {
        Err(e) => return Err(format!("count network_id result error: {}", e)),
        Ok(count) => count,
    };
    expect(count).to_equal(1)?;

    let cond = ListQueryCond {
        unit_id: Some(Some("unit_id_count")),
        ..Default::default()
    };
    let count = match runtime.block_on(async { model.count(&cond).await }) {
        Err(e) => return Err(format!("count unit_id result error: {}", e)),
        Ok(count) => count,
    };
    expect(count).to_equal(3)?;

    let cond = ListQueryCond {
        network_id: Some("network_id_count1_1"),
        unit_id: Some(Some("unit_id_count")),
        ..Default::default()
    };
    let count = match runtime.block_on(async { model.count(&cond).await }) {
        Err(e) => return Err(format!("count network-unit result error: {}", e)),
        Ok(count) => count,
    };
    expect(count).to_equal(1)?;

    let cond = ListQueryCond {
        network_id: Some("network_id_count3_1"),
        unit_id: Some(Some("unit_id_count")),
        ..Default::default()
    };
    let count = match runtime.block_on(async { model.count(&cond).await }) {
        Err(e) => return Err(format!("count network3-unit result error: {}", e)),
        Ok(count) => count,
    };
    expect(count).to_equal(0)?;

    let cond = ListQueryCond {
        code_contains: Some("_1"),
        ..Default::default()
    };
    let count = match runtime.block_on(async { model.count(&cond).await }) {
        Err(e) => return Err(format!("count code result error: {}", e)),
        Ok(count) => count,
    };
    expect(count).to_equal(3)?;

    let cond = ListQueryCond {
        code_contains: Some("_1"),
        unit_id: Some(Some("unit_id_count")),
        ..Default::default()
    };
    let count = match runtime.block_on(async { model.count(&cond).await }) {
        Err(e) => return Err(format!("count code-unit result error: {}", e)),
        Ok(count) => count,
    };
    expect(count).to_equal(2)?;

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
        unit_id: Some(Some("unit_id_count")),
        ..Default::default()
    };
    let count = match runtime.block_on(async { model.count(&cond).await }) {
        Err(e) => return Err(format!("count name-unit result error: {}", e)),
        Ok(count) => count,
    };
    expect(count).to_equal(1)?;

    let cond = ListQueryCond {
        name_contains: Some("_2"),
        unit_id: Some(None),
        ..Default::default()
    };
    let count = match runtime.block_on(async { model.count(&cond).await }) {
        Err(e) => return Err(format!("count name-unit-none result error: {}", e)),
        Ok(count) => count,
    };
    expect(count).to_equal(0)
}

/// Test `list()`.
pub fn list(runtime: &Runtime, model: &dyn NetworkModel) -> Result<(), String> {
    let now = Utc::now().trunc_subsecs(3);
    let mut network = Network {
        network_id: "network_id_list1_1".to_string(),
        code: "code_list1_1".to_string(),
        unit_id: Some("unit_id_list".to_string()),
        unit_code: Some("unit_code_list".to_string()),
        created_at: now,
        modified_at: now,
        host_uri: "host_uri_list_1".to_string(),
        name: "name_list_1".to_string(),
        info: Map::<String, Value>::new(),
    };
    if let Err(e) = runtime.block_on(async {
        model.add(&network).await?;
        network.network_id = "network_id_list1_2".to_string();
        network.code = "code_list1_2".to_string();
        network.name = "name_list1_2".to_string();
        model.add(&network).await?;
        network.network_id = "network_id_list2_1".to_string();
        network.code = "code_list2_1".to_string();
        network.name = "name_list2_1".to_string();
        model.add(&network).await?;
        network.network_id = "network_id_list3_1".to_string();
        network.code = "code_list3_1".to_string();
        network.unit_id = None;
        network.unit_code = None;
        network.name = "name\\\\%%''_list_1".to_string();
        model.add(&network).await
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
        network_id: Some("network_id_list1_1"),
        ..Default::default()
    };
    opts.cond = &cond;
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list network_id result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(1)?;

    let cond = ListQueryCond {
        unit_id: Some(Some("unit_id_list")),
        ..Default::default()
    };
    opts.cond = &cond;
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list unit_id result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(3)?;

    let cond = ListQueryCond {
        network_id: Some("network_id_list1_1"),
        unit_id: Some(Some("unit_id_list")),
        ..Default::default()
    };
    opts.cond = &cond;
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list network-unit result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(1)?;

    let cond = ListQueryCond {
        network_id: Some("network_id_list3_1"),
        unit_id: Some(Some("unit_id_list")),
        ..Default::default()
    };
    opts.cond = &cond;
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list network3-unit result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(0)?;

    let cond = ListQueryCond {
        code_contains: Some("_1"),
        ..Default::default()
    };
    opts.cond = &cond;
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list code result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(3)?;

    let cond = ListQueryCond {
        code_contains: Some("_1"),
        unit_id: Some(Some("unit_id_list")),
        ..Default::default()
    };
    opts.cond = &cond;
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list code-unit result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(2)?;

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
        unit_id: Some(Some("unit_id_list")),
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
        unit_id: Some(None),
        ..Default::default()
    };
    opts.cond = &cond;
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list name-unit3 result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(0)?;

    let cond = ListQueryCond {
        code_contains: Some("lIsT1"),
        ..Default::default()
    };
    opts.cond = &cond;
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list name-case result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(2)?;

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
pub fn list_sort(runtime: &Runtime, model: &dyn NetworkModel) -> Result<(), String> {
    let mut now = Utc::now().trunc_subsecs(3);
    let mut network = Network {
        network_id: "network_id_list1_1".to_string(),
        code: "code_list1_1".to_string(),
        unit_id: Some("unit_id_list".to_string()),
        unit_code: Some("unit_code_list".to_string()),
        created_at: now,
        modified_at: now,
        host_uri: "host_uri_list1_1".to_string(),
        name: "name_list1_1".to_string(),
        info: Map::<String, Value>::new(),
    };
    if let Err(e) = runtime.block_on(async {
        model.add(&network).await?;
        now = now + Duration::seconds(1);
        network.network_id = "network_id_list1_2".to_string();
        network.code = "code_list1_2".to_string();
        network.created_at = now;
        network.modified_at = now;
        network.name = "name_list1_2".to_string();
        model.add(&network).await?;
        now = now + Duration::seconds(1);
        network.network_id = "network_id_list2_1".to_string();
        network.code = "code_list2_1".to_string();
        network.created_at = now;
        network.modified_at = now;
        network.name = "name_list2_1".to_string();
        model.add(&network).await?;
        now = now + Duration::seconds(1);
        network.network_id = "network_id_list3_1".to_string();
        network.code = "code_list3_1".to_string();
        network.created_at = now;
        network.modified_at = now;
        network.name = "name_list2_1".to_string();
        model.add(&network).await
    }) {
        return Err(format!("model.add() error: {}", e));
    }

    let cond = ListQueryCond {
        ..Default::default()
    };
    let sort_cond = vec![SortCond {
        key: SortKey::Code,
        asc: true,
    }];
    let mut opts = ListOptions {
        cond: &cond,
        offset: None,
        limit: None,
        sort: Some(sort_cond.as_slice()),
        cursor_max: None,
    };
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list code-asc result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(4)?;
    expect(list[0].code.as_str()).to_equal("code_list1_1")?;
    expect(list[1].code.as_str()).to_equal("code_list1_2")?;
    expect(list[2].code.as_str()).to_equal("code_list2_1")?;
    expect(list[3].code.as_str()).to_equal("code_list3_1")?;

    let sort_cond = vec![SortCond {
        key: SortKey::Code,
        asc: false,
    }];
    opts.sort = Some(sort_cond.as_slice());
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list code-desc result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(4)?;
    expect(list[0].code.as_str()).to_equal("code_list3_1")?;
    expect(list[1].code.as_str()).to_equal("code_list2_1")?;
    expect(list[2].code.as_str()).to_equal("code_list1_2")?;
    expect(list[3].code.as_str()).to_equal("code_list1_1")?;

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
    expect(list[0].code.as_str()).to_equal("code_list1_1")?;
    expect(list[1].code.as_str()).to_equal("code_list1_2")?;
    expect(list[2].code.as_str()).to_equal("code_list2_1")?;
    expect(list[3].code.as_str()).to_equal("code_list3_1")?;

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
    expect(list[0].code.as_str()).to_equal("code_list3_1")?;
    expect(list[1].code.as_str()).to_equal("code_list2_1")?;
    expect(list[2].code.as_str()).to_equal("code_list1_2")?;
    expect(list[3].code.as_str()).to_equal("code_list1_1")?;

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
    expect(list[0].code.as_str()).to_equal("code_list1_1")?;
    expect(list[1].code.as_str()).to_equal("code_list1_2")?;
    expect(list[2].code.as_str()).to_equal("code_list2_1")?;
    expect(list[3].code.as_str()).to_equal("code_list3_1")?;

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
    expect(list[0].code.as_str()).to_equal("code_list3_1")?;
    expect(list[1].code.as_str()).to_equal("code_list2_1")?;
    expect(list[2].code.as_str()).to_equal("code_list1_2")?;
    expect(list[3].code.as_str()).to_equal("code_list1_1")?;

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
    expect(list[0].code.as_str()).to_equal("code_list1_1")?;
    expect(list[1].code.as_str()).to_equal("code_list1_2")?;
    expect(list[2].code.as_str()).to_equal("code_list2_1")?;
    expect(list[3].code.as_str()).to_equal("code_list3_1")?;

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
    expect(list[0].code.as_str()).to_equal("code_list1_1")?;
    expect(list[1].code.as_str()).to_equal("code_list1_2")?;
    expect(list[2].code.as_str()).to_equal("code_list3_1")?;
    expect(list[3].code.as_str()).to_equal("code_list2_1")?;

    let sort_cond = vec![];
    opts.sort = Some(sort_cond.as_slice());
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list empty result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(4)
}

/// Test `list()` with offset/limit.
pub fn list_offset_limit(runtime: &Runtime, model: &dyn NetworkModel) -> Result<(), String> {
    let now = Utc::now().trunc_subsecs(3);
    let mut network = Network {
        network_id: "network_id_list1_1".to_string(),
        code: "code_list1_1".to_string(),
        unit_id: Some("unit_id_list".to_string()),
        unit_code: Some("unit_code_list".to_string()),
        created_at: now,
        modified_at: now,
        host_uri: "host_uri_list_1".to_string(),
        name: "name_list_1".to_string(),
        info: Map::<String, Value>::new(),
    };
    if let Err(e) = runtime.block_on(async {
        model.add(&network).await?;
        network.network_id = "network_id_list1_2".to_string();
        network.code = "code_list1_2".to_string();
        model.add(&network).await?;
        network.network_id = "network_id_list2_1".to_string();
        network.code = "code_list2_1".to_string();
        model.add(&network).await?;
        network.network_id = "network_id_list3_1".to_string();
        network.code = "code_list3_1".to_string();
        model.add(&network).await
    }) {
        return Err(format!("model.add() error: {}", e));
    }

    let cond = ListQueryCond {
        ..Default::default()
    };
    let sort_cond = vec![SortCond {
        key: SortKey::Code,
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
    expect(list[0].code.as_str()).to_equal("code_list1_1")?;
    expect(list[1].code.as_str()).to_equal("code_list1_2")?;
    expect(list[2].code.as_str()).to_equal("code_list2_1")?;

    opts.limit = Some(5);
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list limit-5 result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(4)?;
    expect(list[0].code.as_str()).to_equal("code_list1_1")?;
    expect(list[1].code.as_str()).to_equal("code_list1_2")?;
    expect(list[2].code.as_str()).to_equal("code_list2_1")?;
    expect(list[3].code.as_str()).to_equal("code_list3_1")?;

    opts.limit = None;
    opts.offset = Some(2);
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list offset-2 result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(2)?;
    expect(list[0].code.as_str()).to_equal("code_list2_1")?;
    expect(list[1].code.as_str()).to_equal("code_list3_1")?;

    opts.limit = Some(0);
    opts.offset = Some(0);
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list offset-limit0 result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(4)?;
    expect(list[0].code.as_str()).to_equal("code_list1_1")?;
    expect(list[1].code.as_str()).to_equal("code_list1_2")?;
    expect(list[2].code.as_str()).to_equal("code_list2_1")?;
    expect(list[3].code.as_str()).to_equal("code_list3_1")?;

    opts.limit = Some(3);
    opts.offset = Some(3);
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list offset-limit3 result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(1)?;
    expect(list[0].code.as_str()).to_equal("code_list3_1")
}

/// Test `list()` with cursors.
pub fn list_cursor(runtime: &Runtime, model: &dyn NetworkModel) -> Result<(), String> {
    let now = Utc::now().trunc_subsecs(3);
    let mut network = Network {
        network_id: "network_id_list1_1".to_string(),
        code: "code_list1_1".to_string(),
        unit_id: Some("unit_id_list".to_string()),
        unit_code: Some("unit_code_list".to_string()),
        created_at: now,
        modified_at: now,
        host_uri: "host_uri_list".to_string(),
        name: "name_list".to_string(),
        info: Map::<String, Value>::new(),
    };
    if let Err(e) = runtime.block_on(async {
        model.add(&network).await?;
        network.network_id = "network_id_list1_2".to_string();
        network.code = "code_list1_2".to_string();
        model.add(&network).await?;
        network.network_id = "network_id_list2_1".to_string();
        network.code = "code_list2_1".to_string();
        model.add(&network).await?;
        network.network_id = "network_id_list3_1".to_string();
        network.code = "code_list3_1".to_string();
        model.add(&network).await
    }) {
        return Err(format!("model.add() error: {}", e));
    }

    let cond = ListQueryCond {
        ..Default::default()
    };
    let sort_cond = vec![SortCond {
        key: SortKey::Code,
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
    expect(list[0].code.as_str()).to_equal("code_list1_1")?;
    expect(list[1].code.as_str()).to_equal("code_list1_2")?;
    expect(list[2].code.as_str()).to_equal("code_list2_1")?;
    expect(cursor.is_some()).to_equal(true)?;
    expect(cursor.as_ref().unwrap().offset()).to_equal(3)?;
    let (list, cursor) = match runtime.block_on(async { model.list(&opts, cursor).await }) {
        Err(e) => return Err(format!("list cursor-3-2 result error: {}", e)),
        Ok((list, cursor)) => (list, cursor),
    };
    expect(list.len()).to_equal(1)?;
    expect(list[0].code.as_str()).to_equal("code_list3_1")?;
    expect(cursor.is_none()).to_equal(true)?;

    opts.limit = Some(3);
    opts.cursor_max = Some(2);
    let (list, cursor) = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list cursor-2-1 result error: {}", e)),
        Ok((list, cursor)) => (list, cursor),
    };
    expect(list.len()).to_equal(2)?;
    expect(list[0].code.as_str()).to_equal("code_list1_1")?;
    expect(list[1].code.as_str()).to_equal("code_list1_2")?;
    expect(cursor.is_some()).to_equal(true)?;
    expect(cursor.as_ref().unwrap().offset()).to_equal(2)?;
    let (list, cursor) = match runtime.block_on(async { model.list(&opts, cursor).await }) {
        Err(e) => return Err(format!("list cursor-2-2 result error: {}", e)),
        Ok((list, cursor)) => (list, cursor),
    };
    expect(list.len()).to_equal(1)?;
    expect(list[0].code.as_str()).to_equal("code_list2_1")?;
    expect(cursor.is_none()).to_equal(true)?;

    opts.limit = Some(4);
    opts.cursor_max = Some(2);
    let (list, cursor) = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list cursor-2-2-1 result error: {}", e)),
        Ok((list, cursor)) => (list, cursor),
    };
    expect(list.len()).to_equal(2)?;
    expect(list[0].code.as_str()).to_equal("code_list1_1")?;
    expect(list[1].code.as_str()).to_equal("code_list1_2")?;
    expect(cursor.is_some()).to_equal(true)?;
    expect(cursor.as_ref().unwrap().offset()).to_equal(2)?;
    let (list, cursor) = match runtime.block_on(async { model.list(&opts, cursor).await }) {
        Err(e) => return Err(format!("list cursor-2-2-2 result error: {}", e)),
        Ok((list, cursor)) => (list, cursor),
    };
    expect(list.len()).to_equal(2)?;
    expect(list[0].code.as_str()).to_equal("code_list2_1")?;
    expect(list[1].code.as_str()).to_equal("code_list3_1")?;
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
    expect(list[0].code.as_str()).to_equal("code_list2_1")?;
    expect(list[1].code.as_str()).to_equal("code_list3_1")?;
    expect(cursor.is_none()).to_equal(true)
}
