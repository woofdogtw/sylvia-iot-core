use std::collections::HashMap;

use chrono::{Duration, SubsecRound, Utc};
use laboratory::{expect, SpecContext};
use mongodb::bson::{doc, DateTime, Document};
use serde::{Deserialize, Serialize};

use sylvia_iot_broker::models::{
    dldata_buffer::{DlDataBuffer, ListOptions, ListQueryCond, QueryCond, SortCond, SortKey},
    Model,
};

use super::{TestState, STATE};

/// MongoDB schema.
#[derive(Debug, Deserialize, Serialize)]
struct Schema {
    #[serde(rename = "dataId")]
    data_id: String,
    #[serde(rename = "unitId")]
    unit_id: String,
    #[serde(rename = "unitCode")]
    unit_code: String,
    #[serde(rename = "applicationId")]
    application_id: String,
    #[serde(rename = "applicationCode")]
    application_code: String,
    #[serde(rename = "networkId")]
    network_id: String,
    #[serde(rename = "networkAddr")]
    network_addr: String,
    #[serde(rename = "deviceId")]
    device_id: String,
    #[serde(rename = "createdAt")]
    created_at: DateTime,
    #[serde(rename = "expiredAt")]
    expired_at: DateTime,
}

const COL_NAME: &'static str = "dldataBuffer";

pub fn after_each_fn(state: &mut HashMap<&'static str, TestState>) -> () {
    let state = state.get_mut(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let conn = state.mongodb.as_ref().unwrap().get_connection();
    let _ = runtime.block_on(async {
        conn.collection::<Schema>(COL_NAME)
            .delete_many(Document::new(), None)
            .await
    });
}

/// Test table initialization.
pub fn init(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.mongodb.as_ref().unwrap().dldata_buffer();

    let result = runtime.block_on(async { model.init().await });
    expect(result.is_ok()).to_equal(true)
}

/// Test `get()`.
pub fn get(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let conn = state.mongodb.as_ref().unwrap().get_connection();
    let model = state.mongodb.as_ref().unwrap().dldata_buffer();

    let now = Utc::now().trunc_subsecs(3);
    let item = Schema {
        data_id: "data_id_get".to_string(),
        unit_id: "unit_id_get".to_string(),
        unit_code: "unit_code_get".to_string(),
        application_id: "application_id_get".to_string(),
        application_code: "application_code_get".to_string(),
        network_id: "network_id_get".to_string(),
        network_addr: "network_addr_get".to_string(),
        device_id: "device_id_get".to_string(),
        created_at: now.into(),
        expired_at: now.into(),
    };
    if let Err(e) = runtime.block_on(async {
        conn.collection::<Schema>(COL_NAME)
            .insert_one(item, None)
            .await
    }) {
        return Err(format!("insert_one() error: {}", e));
    }

    match runtime.block_on(async { model.get("data_id_not_exist").await }) {
        Err(e) => return Err(format!("model.get() error: {}", e)),
        Ok(data) => match data {
            None => (),
            Some(_) => return Err(format!("should not get not-exist one")),
        },
    };

    let data = match runtime.block_on(async { model.get("data_id_get").await }) {
        Err(e) => return Err(format!("model.get() error: {}", e)),
        Ok(data) => match data {
            None => return Err("should get one".to_string()),
            Some(data) => data,
        },
    };
    expect(data.data_id).to_equal("data_id_get".to_string())?;
    expect(data.unit_id).to_equal("unit_id_get".to_string())?;
    expect(data.unit_code).to_equal("unit_code_get".to_string())?;
    expect(data.application_id).to_equal("application_id_get".to_string())?;
    expect(data.application_code).to_equal("application_code_get".to_string())?;
    expect(data.network_id).to_equal("network_id_get".to_string())?;
    expect(data.network_addr).to_equal("network_addr_get".to_string())?;
    expect(data.device_id).to_equal("device_id_get".to_string())?;
    expect(data.created_at).to_equal(now)?;
    expect(data.expired_at).to_equal(now)
}

/// Test `add()`.
pub fn add(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.mongodb.as_ref().unwrap().dldata_buffer();

    let now = Utc::now().trunc_subsecs(3);
    let data = DlDataBuffer {
        data_id: "data_id_add".to_string(),
        unit_id: "unit_id_add".to_string(),
        unit_code: "unit_code_add".to_string(),
        application_id: "application_id_add".to_string(),
        application_code: "application_code_add".to_string(),
        network_id: "network_id_add".to_string(),
        network_addr: "network_addr_add".to_string(),
        device_id: "device_id_add".to_string(),
        created_at: now,
        expired_at: now,
    };
    if let Err(e) = runtime.block_on(async { model.add(&data).await }) {
        return Err(format!("model.add() error: {}", e));
    }

    let get_data = match runtime.block_on(async { model.get("data_id_add").await }) {
        Err(e) => return Err(format!("model.get() error: {}", e)),
        Ok(data) => match data {
            None => return Err("should get one".to_string()),
            Some(data) => data,
        },
    };
    expect(get_data).to_equal(data)
}

/// Test `add()` with duplicate key.
pub fn add_dup(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.mongodb.as_ref().unwrap().dldata_buffer();

    let now = Utc::now().trunc_subsecs(3);
    let data = DlDataBuffer {
        data_id: "data_id_add".to_string(),
        unit_id: "unit_id_add".to_string(),
        unit_code: "unit_code_add".to_string(),
        application_id: "application_id_add".to_string(),
        application_code: "application_code_add".to_string(),
        network_id: "network_id_add".to_string(),
        network_addr: "network_addr_add".to_string(),
        device_id: "device_id_add".to_string(),
        created_at: now,
        expired_at: now,
    };
    if let Err(e) = runtime.block_on(async { model.add(&data).await }) {
        return Err(format!("model.add() error: {}", e));
    }
    if let Ok(_) = runtime.block_on(async { model.add(&data).await }) {
        return Err("model.add() duplicate data_id should error".to_string());
    }
    Ok(())
}

/// Test `del()` by specifying a data ID.
pub fn del_by_data_id(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.mongodb.as_ref().unwrap().dldata_buffer();

    let now = Utc::now().trunc_subsecs(3);
    let data_id_del = "data_id_del";
    let data_id_not_del = "data_id_not_del";
    let mut data = DlDataBuffer {
        data_id: data_id_del.to_string(),
        unit_id: "unit_id_del".to_string(),
        unit_code: "unit_code_del".to_string(),
        application_id: "application_id_del".to_string(),
        application_code: "application_code_del".to_string(),
        network_id: "network_id_del".to_string(),
        network_addr: "network_addr_del".to_string(),
        device_id: "device_id_del".to_string(),
        created_at: now,
        expired_at: now,
    };
    let cond = QueryCond {
        data_id: Some(data_id_del),
        ..Default::default()
    };
    if let Err(e) = runtime.block_on(async {
        model.add(&data).await?;
        data.data_id = data_id_not_del.to_string();
        model.add(&data).await?;
        model.del(&cond).await
    }) {
        return Err(format!("model.add/del error: {}", e));
    }
    match runtime.block_on(async { model.get(data_id_del).await }) {
        Err(e) => return Err(format!("model.get() error: {}", e)),
        Ok(data) => match data {
            None => (),
            Some(_) => return Err("delete fail".to_string()),
        },
    }
    match runtime.block_on(async { model.get(data_id_not_del).await }) {
        Err(e) => Err(format!("model.get() not delete one error: {}", e)),
        Ok(data) => match data {
            None => Err("delete wrong one".to_string()),
            Some(_) => Ok(()),
        },
    }
}

/// Test `del()` twice.
pub fn del_twice(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.mongodb.as_ref().unwrap().dldata_buffer();

    let now = Utc::now().trunc_subsecs(3);
    let data_id_del = "data_id_del";
    let data = DlDataBuffer {
        data_id: data_id_del.to_string(),
        unit_id: "unit_id_del".to_string(),
        unit_code: "unit_code_del".to_string(),
        application_id: "application_id_del".to_string(),
        application_code: "application_code_del".to_string(),
        network_id: "network_id_del".to_string(),
        network_addr: "network_addr_del".to_string(),
        device_id: "device_id_del".to_string(),
        created_at: now,
        expired_at: now,
    };
    let cond = QueryCond {
        data_id: Some(data_id_del),
        ..Default::default()
    };
    if let Err(e) = runtime.block_on(async {
        model.add(&data).await?;
        model.del(&cond).await?;
        model.del(&cond).await
    }) {
        return Err(format!("model.add/del error: {}", e));
    }
    Ok(())
}

/// Test `del()` by specifying a unit ID.
pub fn del_by_unit_id(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.mongodb.as_ref().unwrap().dldata_buffer();

    let now = Utc::now().trunc_subsecs(3);
    let data_id_del1 = "data_id_del1";
    let data_id_del2 = "data_id_del2";
    let data_id_not_del = "data_id_not_del";
    let data_id_not_del2 = "data_id_not_del2";
    let mut data = DlDataBuffer {
        data_id: data_id_del1.to_string(),
        unit_id: "unit_id_del".to_string(),
        unit_code: "unit_code_del".to_string(),
        application_id: "application_id_del".to_string(),
        application_code: "application_code_del".to_string(),
        network_id: "network_id_del".to_string(),
        network_addr: "network_addr_del".to_string(),
        device_id: "device_id_del".to_string(),
        created_at: now,
        expired_at: now,
    };
    let cond = QueryCond {
        unit_id: Some("unit_id_del"),
        ..Default::default()
    };
    if let Err(e) = runtime.block_on(async {
        model.add(&data).await?;
        data.data_id = data_id_del2.to_string();
        model.add(&data).await?;
        data.data_id = data_id_not_del.to_string();
        data.unit_id = "unit_id_not_del".to_string();
        data.unit_code = "unit_code_not_del".to_string();
        model.add(&data).await?;
        data.data_id = data_id_not_del2.to_string();
        model.add(&data).await?;
        model.del(&cond).await
    }) {
        return Err(format!("model.add/del error: {}", e));
    }
    match runtime.block_on(async { model.get(data_id_del1).await }) {
        Err(e) => return Err(format!("model.get() delete data1 error: {}", e)),
        Ok(data) => match data {
            None => (),
            Some(_) => return Err("delete data1 fail".to_string()),
        },
    }
    match runtime.block_on(async { model.get(data_id_del2).await }) {
        Err(e) => return Err(format!("model.get() delete data2 error: {}", e)),
        Ok(data) => match data {
            None => (),
            Some(_) => return Err("delete data2 fail".to_string()),
        },
    }
    match runtime.block_on(async { model.get(data_id_not_del).await }) {
        Err(e) => return Err(format!("model.get() not delete one error: {}", e)),
        Ok(data) => match data {
            None => return Err("delete wrong one".to_string()),
            Some(_) => (),
        },
    }
    match runtime.block_on(async { model.get(data_id_not_del2).await }) {
        Err(e) => Err(format!("model.get() not delete one2 error: {}", e)),
        Ok(data) => match data {
            None => Err("delete wrong one2".to_string()),
            Some(_) => Ok(()),
        },
    }
}

/// Test `del()` by specifying a pair of unit ID and data ID.
pub fn del_by_unit_data(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.mongodb.as_ref().unwrap().dldata_buffer();

    let now = Utc::now().trunc_subsecs(3);
    let data_id_del = "data_id_del";
    let data_id_not_del = "data_id_not_del";
    let mut data = DlDataBuffer {
        data_id: data_id_del.to_string(),
        unit_id: "unit_id_del".to_string(),
        unit_code: "unit_code_del".to_string(),
        application_id: "application_id_del".to_string(),
        application_code: "application_code_del".to_string(),
        network_id: "network_id_del".to_string(),
        network_addr: "network_addr_del".to_string(),
        device_id: "device_id_del".to_string(),
        created_at: now,
        expired_at: now,
    };
    let cond = QueryCond {
        unit_id: Some("unit_id_del"),
        data_id: Some(data_id_del),
        ..Default::default()
    };
    if let Err(e) = runtime.block_on(async {
        model.add(&data).await?;
        data.data_id = data_id_not_del.to_string();
        model.add(&data).await?;
        model.del(&cond).await
    }) {
        return Err(format!("model.add/del error: {}", e));
    }
    match runtime.block_on(async { model.get(data_id_del).await }) {
        Err(e) => return Err(format!("model.get() error: {}", e)),
        Ok(data) => match data {
            None => (),
            Some(_) => return Err("delete fail".to_string()),
        },
    }
    match runtime.block_on(async { model.get(data_id_not_del).await }) {
        Err(e) => Err(format!("model.get() not delete one error: {}", e)),
        Ok(data) => match data {
            None => Err("delete wrong one".to_string()),
            Some(_) => Ok(()),
        },
    }
}

/// Test `del()` by specifying a application ID.
pub fn del_by_application_id(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.mongodb.as_ref().unwrap().dldata_buffer();

    let now = Utc::now().trunc_subsecs(3);
    let data_id_del1 = "data_id_del1";
    let data_id_del2 = "data_id_del2";
    let data_id_not_del = "data_id_not_del";
    let data_id_not_del2 = "data_id_not_del2";
    let mut data = DlDataBuffer {
        data_id: data_id_del1.to_string(),
        unit_id: "unit_id_del".to_string(),
        unit_code: "unit_code_del".to_string(),
        application_id: "application_id_del".to_string(),
        application_code: "application_code_del".to_string(),
        network_id: "network_id_del".to_string(),
        network_addr: "network_addr_del".to_string(),
        device_id: "device_id_del".to_string(),
        created_at: now,
        expired_at: now,
    };
    let cond = QueryCond {
        application_id: Some("application_id_del"),
        ..Default::default()
    };
    if let Err(e) = runtime.block_on(async {
        model.add(&data).await?;
        data.data_id = data_id_del2.to_string();
        model.add(&data).await?;
        data.data_id = data_id_not_del.to_string();
        data.application_id = "application_id_not_del".to_string();
        model.add(&data).await?;
        data.data_id = data_id_not_del2.to_string();
        model.add(&data).await?;
        model.del(&cond).await
    }) {
        return Err(format!("model.add/del error: {}", e));
    }
    match runtime.block_on(async { model.get(data_id_del1).await }) {
        Err(e) => return Err(format!("model.get() delete data1 error: {}", e)),
        Ok(data) => match data {
            None => (),
            Some(_) => return Err("delete data1 fail".to_string()),
        },
    }
    match runtime.block_on(async { model.get(data_id_del2).await }) {
        Err(e) => return Err(format!("model.get() delete data2 error: {}", e)),
        Ok(data) => match data {
            None => (),
            Some(_) => return Err("delete data2 fail".to_string()),
        },
    }
    match runtime.block_on(async { model.get(data_id_not_del).await }) {
        Err(e) => return Err(format!("model.get() not delete one error: {}", e)),
        Ok(data) => match data {
            None => return Err("delete wrong one".to_string()),
            Some(_) => (),
        },
    }
    match runtime.block_on(async { model.get(data_id_not_del2).await }) {
        Err(e) => Err(format!("model.get() not delete one2 error: {}", e)),
        Ok(data) => match data {
            None => Err("delete wrong one2".to_string()),
            Some(_) => Ok(()),
        },
    }
}

/// Test `del()` by specifying a network ID.
pub fn del_by_network_id(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.mongodb.as_ref().unwrap().dldata_buffer();

    let now = Utc::now().trunc_subsecs(3);
    let data_id_del1 = "data_id_del1";
    let data_id_del2 = "data_id_del2";
    let data_id_not_del = "data_id_not_del";
    let data_id_not_del2 = "data_id_not_del2";
    let mut data = DlDataBuffer {
        data_id: data_id_del1.to_string(),
        unit_id: "unit_id_del".to_string(),
        unit_code: "unit_code_del".to_string(),
        application_id: "application_id_del".to_string(),
        application_code: "application_code_del".to_string(),
        network_id: "network_id_del".to_string(),
        network_addr: "network_addr_del".to_string(),
        device_id: "device_id_del".to_string(),
        created_at: now,
        expired_at: now,
    };
    let cond = QueryCond {
        network_id: Some("network_id_del"),
        ..Default::default()
    };
    if let Err(e) = runtime.block_on(async {
        model.add(&data).await?;
        data.data_id = data_id_del2.to_string();
        model.add(&data).await?;
        data.data_id = data_id_not_del.to_string();
        data.network_id = "network_id_not_del".to_string();
        model.add(&data).await?;
        data.data_id = data_id_not_del2.to_string();
        model.add(&data).await?;
        model.del(&cond).await
    }) {
        return Err(format!("model.add/del error: {}", e));
    }
    match runtime.block_on(async { model.get(data_id_del1).await }) {
        Err(e) => return Err(format!("model.get() delete data1 error: {}", e)),
        Ok(data) => match data {
            None => (),
            Some(_) => return Err("delete data1 fail".to_string()),
        },
    }
    match runtime.block_on(async { model.get(data_id_del2).await }) {
        Err(e) => return Err(format!("model.get() delete data2 error: {}", e)),
        Ok(data) => match data {
            None => (),
            Some(_) => return Err("delete data2 fail".to_string()),
        },
    }
    match runtime.block_on(async { model.get(data_id_not_del).await }) {
        Err(e) => return Err(format!("model.get() not delete one error: {}", e)),
        Ok(data) => match data {
            None => return Err("delete wrong one".to_string()),
            Some(_) => (),
        },
    }
    match runtime.block_on(async { model.get(data_id_not_del2).await }) {
        Err(e) => Err(format!("model.get() not delete one2 error: {}", e)),
        Ok(data) => match data {
            None => Err("delete wrong one2".to_string()),
            Some(_) => Ok(()),
        },
    }
}

/// Test `del()` by specifying a device ID.
pub fn del_by_device_id(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.mongodb.as_ref().unwrap().dldata_buffer();

    let now = Utc::now().trunc_subsecs(3);
    let data_id_del1 = "data_id_del1";
    let data_id_del2 = "data_id_del2";
    let data_id_not_del = "data_id_not_del";
    let data_id_not_del2 = "data_id_not_del2";
    let mut data = DlDataBuffer {
        data_id: data_id_del1.to_string(),
        unit_id: "unit_id_del".to_string(),
        unit_code: "unit_code_del".to_string(),
        application_id: "application_id_del".to_string(),
        application_code: "application_code_del".to_string(),
        network_id: "network_id_del".to_string(),
        network_addr: "network_addr_del".to_string(),
        device_id: "device_id_del".to_string(),
        created_at: now,
        expired_at: now,
    };
    let cond = QueryCond {
        device_id: Some("device_id_del"),
        ..Default::default()
    };
    if let Err(e) = runtime.block_on(async {
        model.add(&data).await?;
        data.data_id = data_id_del2.to_string();
        model.add(&data).await?;
        data.data_id = data_id_not_del.to_string();
        data.device_id = "device_id_not_del".to_string();
        model.add(&data).await?;
        data.data_id = data_id_not_del2.to_string();
        model.add(&data).await?;
        model.del(&cond).await
    }) {
        return Err(format!("model.add/del error: {}", e));
    }
    match runtime.block_on(async { model.get(data_id_del1).await }) {
        Err(e) => return Err(format!("model.get() delete data1 error: {}", e)),
        Ok(data) => match data {
            None => (),
            Some(_) => return Err("delete data1 fail".to_string()),
        },
    }
    match runtime.block_on(async { model.get(data_id_del2).await }) {
        Err(e) => return Err(format!("model.get() delete data2 error: {}", e)),
        Ok(data) => match data {
            None => (),
            Some(_) => return Err("delete data2 fail".to_string()),
        },
    }
    match runtime.block_on(async { model.get(data_id_not_del).await }) {
        Err(e) => return Err(format!("model.get() not delete one error: {}", e)),
        Ok(data) => match data {
            None => return Err("delete wrong one".to_string()),
            Some(_) => (),
        },
    }
    match runtime.block_on(async { model.get(data_id_not_del2).await }) {
        Err(e) => Err(format!("model.get() not delete one2 error: {}", e)),
        Ok(data) => match data {
            None => Err("delete wrong one2".to_string()),
            Some(_) => Ok(()),
        },
    }
}

/// Test `del()` by specifying network addresses.
pub fn del_by_network_addrs(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.mongodb.as_ref().unwrap().dldata_buffer();

    for i in 0..100 {
        let now = Utc::now().trunc_subsecs(3);
        let data = DlDataBuffer {
            data_id: format!("data_id{:#03}", i),
            unit_id: "unit_id_bulk".to_string(),
            unit_code: "unit_code_bulk".to_string(),
            application_id: "application_id_bulk".to_string(),
            application_code: "application_code_bulk".to_string(),
            network_id: "network_id_bulk".to_string(),
            network_addr: format!("network_del{:#03}", i),
            device_id: format!("device_id{:#03}", i),
            created_at: now,
            expired_at: now,
        };
        if let Err(e) = runtime.block_on(async { model.add(&data).await }) {
            return Err(format!("model.add() error: {}", e));
        }
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
        key: SortKey::CreatedAt,
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

/// Test `count()`.
pub fn count(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.mongodb.as_ref().unwrap().dldata_buffer();

    let now = Utc::now().trunc_subsecs(3);
    let mut data = DlDataBuffer {
        data_id: "data_id_count1_1".to_string(),
        unit_id: "unit_id_count".to_string(),
        unit_code: "unit_code_count".to_string(),
        application_id: "application_id_count".to_string(),
        application_code: "application_code_count".to_string(),
        network_id: "network_id_count".to_string(),
        network_addr: "network_addr_count".to_string(),
        device_id: "device_id_count1".to_string(),
        created_at: now,
        expired_at: now,
    };
    if let Err(e) = runtime.block_on(async {
        model.add(&data).await?;
        data.data_id = "data_id_count1_2".to_string();
        data.device_id = "device_id_count1_2".to_string();
        model.add(&data).await?;
        data.data_id = "data_id_count1_3".to_string();
        data.device_id = "device_id_count1_3".to_string();
        data.network_id = "network_id_count1_3".to_string();
        model.add(&data).await?;
        data.data_id = "data_id_count2_1".to_string();
        data.application_id = "application_id_count2".to_string();
        data.device_id = "device_id_count1".to_string();
        model.add(&data).await?;
        data.data_id = "data_id_count3_1".to_string();
        data.unit_id = "unit_id_count3".to_string();
        data.unit_code = "unit_code_count3".to_string();
        data.application_id = "application_id_count3".to_string();
        data.device_id = "device_id_count1".to_string();
        data.network_id = "network_id_count3".to_string();
        model.add(&data).await
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
        unit_id: Some("unit_id_count"),
        ..Default::default()
    };
    let count = match runtime.block_on(async { model.count(&cond).await }) {
        Err(e) => return Err(format!("count unit_id result error: {}", e)),
        Ok(count) => count,
    };
    expect(count).to_equal(4)?;

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
        network_id: Some("network_id_count"),
        ..Default::default()
    };
    let count = match runtime.block_on(async { model.count(&cond).await }) {
        Err(e) => return Err(format!("count network_id result error: {}", e)),
        Ok(count) => count,
    };
    expect(count).to_equal(2)?;

    let cond = ListQueryCond {
        device_id: Some("device_id_count1"),
        ..Default::default()
    };
    let count = match runtime.block_on(async { model.count(&cond).await }) {
        Err(e) => return Err(format!("count device_id result error: {}", e)),
        Ok(count) => count,
    };
    expect(count).to_equal(3)
}

/// Test `list()`.
pub fn list(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.mongodb.as_ref().unwrap().dldata_buffer();

    let now = Utc::now().trunc_subsecs(3);
    let mut data = DlDataBuffer {
        data_id: "data_id_list1_1".to_string(),
        unit_id: "unit_id_list".to_string(),
        unit_code: "unit_code_list".to_string(),
        application_id: "application_id_list".to_string(),
        application_code: "application_code_list".to_string(),
        network_id: "network_id_list".to_string(),
        network_addr: "network_addr_list".to_string(),
        device_id: "device_id_list1".to_string(),
        created_at: now,
        expired_at: now,
    };
    if let Err(e) = runtime.block_on(async {
        model.add(&data).await?;
        data.data_id = "data_id_list1_2".to_string();
        data.device_id = "device_id_list1_2".to_string();
        model.add(&data).await?;
        data.data_id = "data_id_list1_3".to_string();
        data.device_id = "device_id_list1_3".to_string();
        data.network_id = "network_id_list1_3".to_string();
        model.add(&data).await?;
        data.data_id = "data_id_list2_1".to_string();
        data.application_id = "application_id_list2".to_string();
        data.device_id = "device_id_list1".to_string();
        model.add(&data).await?;
        data.data_id = "data_id_list3_1".to_string();
        data.unit_id = "unit_id_list3".to_string();
        data.unit_code = "unit_code_list3".to_string();
        data.application_id = "application_id_list3".to_string();
        data.device_id = "device_id_list1".to_string();
        data.network_id = "network_id_list3".to_string();
        model.add(&data).await
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
        device_id: Some("device_id_list1"),
        ..Default::default()
    };
    opts.cond = &cond;
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list device_id result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(3)
}

/// Test `list()` with sorting.
pub fn list_sort(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.mongodb.as_ref().unwrap().dldata_buffer();

    let mut now = Utc::now().trunc_subsecs(3);
    let mut data = DlDataBuffer {
        data_id: "data_id_list1_1".to_string(),
        unit_id: "unit_id_list".to_string(),
        unit_code: "unit_code_list".to_string(),
        application_id: "application_id_list".to_string(),
        application_code: "application_code_list1".to_string(),
        network_id: "network_id_list".to_string(),
        network_addr: "network_addr_list".to_string(),
        device_id: "device_id_list".to_string(),
        created_at: now,
        expired_at: now,
    };
    if let Err(e) = runtime.block_on(async {
        model.add(&data).await?;
        now = now + Duration::seconds(1);
        data.data_id = "data_id_list1_2".to_string();
        data.application_code = "application_code_list1_2".to_string();
        data.created_at = now;
        data.expired_at = now;
        model.add(&data).await?;
        now = now + Duration::seconds(1);
        data.data_id = "data_id_list2_1".to_string();
        data.application_code = "application_code_list2_1".to_string();
        data.created_at = now;
        data.expired_at = now;
        model.add(&data).await?;
        now = now + Duration::seconds(1);
        data.data_id = "data_id_list3_1".to_string();
        data.application_code = "application_code_list3_1".to_string();
        data.created_at = now;
        data.expired_at = now;
        model.add(&data).await
    }) {
        return Err(format!("model.add() error: {}", e));
    }

    let cond = ListQueryCond {
        ..Default::default()
    };
    let sort_cond = vec![SortCond {
        key: SortKey::ApplicationCode,
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
    expect(list[0].data_id.as_str()).to_equal("data_id_list1_1")?;
    expect(list[1].data_id.as_str()).to_equal("data_id_list1_2")?;
    expect(list[2].data_id.as_str()).to_equal("data_id_list2_1")?;
    expect(list[3].data_id.as_str()).to_equal("data_id_list3_1")?;

    let sort_cond = vec![SortCond {
        key: SortKey::ApplicationCode,
        asc: false,
    }];
    opts.sort = Some(sort_cond.as_slice());
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list code-desc result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(4)?;
    expect(list[0].data_id.as_str()).to_equal("data_id_list3_1")?;
    expect(list[1].data_id.as_str()).to_equal("data_id_list2_1")?;
    expect(list[2].data_id.as_str()).to_equal("data_id_list1_2")?;
    expect(list[3].data_id.as_str()).to_equal("data_id_list1_1")?;

    let sort_cond = vec![SortCond {
        key: SortKey::CreatedAt,
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
        Err(e) => return Err(format!("list created-asc result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(4)?;
    expect(list[0].data_id.as_str()).to_equal("data_id_list1_1")?;
    expect(list[1].data_id.as_str()).to_equal("data_id_list1_2")?;
    expect(list[2].data_id.as_str()).to_equal("data_id_list2_1")?;
    expect(list[3].data_id.as_str()).to_equal("data_id_list3_1")?;

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
    expect(list[0].data_id.as_str()).to_equal("data_id_list3_1")?;
    expect(list[1].data_id.as_str()).to_equal("data_id_list2_1")?;
    expect(list[2].data_id.as_str()).to_equal("data_id_list1_2")?;
    expect(list[3].data_id.as_str()).to_equal("data_id_list1_1")?;

    let sort_cond = vec![SortCond {
        key: SortKey::ExpiredAt,
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
        Err(e) => return Err(format!("list expired-asc result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(4)?;
    expect(list[0].data_id.as_str()).to_equal("data_id_list1_1")?;
    expect(list[1].data_id.as_str()).to_equal("data_id_list1_2")?;
    expect(list[2].data_id.as_str()).to_equal("data_id_list2_1")?;
    expect(list[3].data_id.as_str()).to_equal("data_id_list3_1")?;

    let sort_cond = vec![SortCond {
        key: SortKey::ExpiredAt,
        asc: false,
    }];
    opts.sort = Some(sort_cond.as_slice());
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list expired-desc result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(4)?;
    expect(list[0].data_id.as_str()).to_equal("data_id_list3_1")?;
    expect(list[1].data_id.as_str()).to_equal("data_id_list2_1")?;
    expect(list[2].data_id.as_str()).to_equal("data_id_list1_2")?;
    expect(list[3].data_id.as_str()).to_equal("data_id_list1_1")?;

    let sort_cond = vec![];
    opts.sort = Some(sort_cond.as_slice());
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list empty result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(4)
}

/// Test `list()` with offset/limit.
pub fn list_offset_limit(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.mongodb.as_ref().unwrap().dldata_buffer();

    let now = Utc::now().trunc_subsecs(3);
    let mut data = DlDataBuffer {
        data_id: "data_id_list1_1".to_string(),
        unit_id: "unit_id_list".to_string(),
        unit_code: "unit_code_list".to_string(),
        application_id: "application_id_list".to_string(),
        application_code: "application_code_list1_1".to_string(),
        network_id: "network_id_list".to_string(),
        network_addr: "network_addr_list".to_string(),
        device_id: "device_id_list".to_string(),
        created_at: now,
        expired_at: now,
    };
    if let Err(e) = runtime.block_on(async {
        model.add(&data).await?;
        data.data_id = "data_id_list1_2".to_string();
        data.application_code = "data_id_list1_2".to_string();
        model.add(&data).await?;
        data.data_id = "data_id_list2_1".to_string();
        data.application_code = "data_id_list1_2".to_string();
        model.add(&data).await?;
        data.data_id = "data_id_list3_1".to_string();
        data.application_code = "data_id_list1_2".to_string();
        model.add(&data).await
    }) {
        return Err(format!("model.add() error: {}", e));
    }

    let cond = ListQueryCond {
        ..Default::default()
    };
    let sort_cond = vec![SortCond {
        key: SortKey::ApplicationCode,
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
    expect(list[0].data_id.as_str()).to_equal("data_id_list1_1")?;
    expect(list[1].data_id.as_str()).to_equal("data_id_list1_2")?;
    expect(list[2].data_id.as_str()).to_equal("data_id_list2_1")?;

    opts.limit = Some(5);
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list limit-5 result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(4)?;
    expect(list[0].data_id.as_str()).to_equal("data_id_list1_1")?;
    expect(list[1].data_id.as_str()).to_equal("data_id_list1_2")?;
    expect(list[2].data_id.as_str()).to_equal("data_id_list2_1")?;
    expect(list[3].data_id.as_str()).to_equal("data_id_list3_1")?;

    opts.limit = None;
    opts.offset = Some(2);
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list offset-2 result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(2)?;
    expect(list[0].data_id.as_str()).to_equal("data_id_list2_1")?;
    expect(list[1].data_id.as_str()).to_equal("data_id_list3_1")?;

    opts.limit = Some(0);
    opts.offset = Some(0);
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list offset-limit0 result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(4)?;
    expect(list[0].data_id.as_str()).to_equal("data_id_list1_1")?;
    expect(list[1].data_id.as_str()).to_equal("data_id_list1_2")?;
    expect(list[2].data_id.as_str()).to_equal("data_id_list2_1")?;
    expect(list[3].data_id.as_str()).to_equal("data_id_list3_1")?;

    opts.limit = Some(3);
    opts.offset = Some(3);
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list offset-limit3 result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(1)?;
    expect(list[0].data_id.as_str()).to_equal("data_id_list3_1")
}

/// Test `list()` with cursors.
pub fn list_cursor(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.mongodb.as_ref().unwrap().dldata_buffer();

    let now = Utc::now().trunc_subsecs(3);
    let mut data = DlDataBuffer {
        data_id: "data_id_list1_1".to_string(),
        unit_id: "unit_id_list".to_string(),
        unit_code: "unit_code_list".to_string(),
        application_id: "application_id_list".to_string(),
        application_code: "application_code_list1_1".to_string(),
        network_id: "network_id_list".to_string(),
        network_addr: "network_addr_list".to_string(),
        device_id: "device_id_list".to_string(),
        created_at: now,
        expired_at: now,
    };
    if let Err(e) = runtime.block_on(async {
        model.add(&data).await?;
        data.data_id = "data_id_list1_2".to_string();
        data.application_code = "data_id_list1_2".to_string();
        model.add(&data).await?;
        data.data_id = "data_id_list2_1".to_string();
        data.application_code = "data_id_list1_2".to_string();
        model.add(&data).await?;
        data.data_id = "data_id_list3_1".to_string();
        data.application_code = "data_id_list1_2".to_string();
        model.add(&data).await
    }) {
        return Err(format!("model.add() error: {}", e));
    }

    let cond = ListQueryCond {
        ..Default::default()
    };
    let sort_cond = vec![SortCond {
        key: SortKey::ApplicationCode,
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
    expect(list[0].data_id.as_str()).to_equal("data_id_list1_1")?;
    expect(list[1].data_id.as_str()).to_equal("data_id_list1_2")?;
    expect(list[2].data_id.as_str()).to_equal("data_id_list2_1")?;
    expect(cursor.is_some()).to_equal(true)?;
    expect(cursor.as_ref().unwrap().offset()).to_equal(3)?;
    let (list, cursor) = match runtime.block_on(async { model.list(&opts, cursor).await }) {
        Err(e) => return Err(format!("list cursor-3-2 result error: {}", e)),
        Ok((list, cursor)) => (list, cursor),
    };
    expect(list.len()).to_equal(1)?;
    expect(list[0].data_id.as_str()).to_equal("data_id_list3_1")?;
    expect(cursor.is_none()).to_equal(true)?;

    opts.limit = Some(3);
    opts.cursor_max = Some(2);
    let (list, cursor) = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list cursor-2-1 result error: {}", e)),
        Ok((list, cursor)) => (list, cursor),
    };
    expect(list.len()).to_equal(2)?;
    expect(list[0].data_id.as_str()).to_equal("data_id_list1_1")?;
    expect(list[1].data_id.as_str()).to_equal("data_id_list1_2")?;
    expect(cursor.is_some()).to_equal(true)?;
    expect(cursor.as_ref().unwrap().offset()).to_equal(2)?;
    let (list, cursor) = match runtime.block_on(async { model.list(&opts, cursor).await }) {
        Err(e) => return Err(format!("list cursor-2-2 result error: {}", e)),
        Ok((list, cursor)) => (list, cursor),
    };
    expect(list.len()).to_equal(1)?;
    expect(list[0].data_id.as_str()).to_equal("data_id_list2_1")?;
    expect(cursor.is_none()).to_equal(true)?;

    opts.limit = Some(4);
    opts.cursor_max = Some(2);
    let (list, cursor) = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list cursor-2-2-1 result error: {}", e)),
        Ok((list, cursor)) => (list, cursor),
    };
    expect(list.len()).to_equal(2)?;
    expect(list[0].data_id.as_str()).to_equal("data_id_list1_1")?;
    expect(list[1].data_id.as_str()).to_equal("data_id_list1_2")?;
    expect(cursor.is_some()).to_equal(true)?;
    expect(cursor.as_ref().unwrap().offset()).to_equal(2)?;
    let (list, cursor) = match runtime.block_on(async { model.list(&opts, cursor).await }) {
        Err(e) => return Err(format!("list cursor-2-2-2 result error: {}", e)),
        Ok((list, cursor)) => (list, cursor),
    };
    expect(list.len()).to_equal(2)?;
    expect(list[0].data_id.as_str()).to_equal("data_id_list2_1")?;
    expect(list[1].data_id.as_str()).to_equal("data_id_list3_1")?;
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
    expect(list[0].data_id.as_str()).to_equal("data_id_list2_1")?;
    expect(list[1].data_id.as_str()).to_equal("data_id_list3_1")?;
    expect(cursor.is_none()).to_equal(true)
}
