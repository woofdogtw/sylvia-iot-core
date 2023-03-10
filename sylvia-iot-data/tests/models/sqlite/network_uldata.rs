use std::{collections::HashMap, error::Error as StdError};

use chrono::{Duration, SubsecRound, TimeZone, Utc};
use laboratory::{expect, SpecContext};
use serde_json::{Map, Value};
use sql_builder::{quote, SqlBuilder};
use sqlx::SqlitePool;

use sylvia_iot_data::models::{
    network_uldata::{ListOptions, ListQueryCond, NetworkUlData, QueryCond, SortCond, SortKey},
    Model,
};
use sylvia_iot_corelib::strings;

use super::{TestState, STATE};

/// SQLite schema.
#[derive(sqlx::FromRow)]
struct Schema {
    pub data_id: String,
    /// i64 as time tick from Epoch in milliseconds.
    pub proc: i64,
    /// use empty string as NULL.
    pub unit_code: String,
    pub network_code: String,
    pub network_addr: String,
    /// use empty string as NULL.
    pub unit_id: String,
    /// use empty string as NULL.
    pub device_id: String,
    /// i64 as time tick from Epoch in milliseconds.
    pub time: i64,
    pub data: String,
    pub extension: String,
}

const TABLE_NAME: &'static str = "network_uldata";
const FIELDS: &'static [&'static str] = &[
    "data_id",
    "proc",
    "unit_code",
    "network_code",
    "network_addr",
    "unit_id",
    "device_id",
    "time",
    "data",
    "extension",
];

pub fn after_each_fn(state: &mut HashMap<&'static str, TestState>) -> () {
    let state = state.get_mut(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let conn = state.sqlite.as_ref().unwrap().get_connection();
    let sql = SqlBuilder::delete_from(TABLE_NAME).sql().unwrap();
    let _ = runtime.block_on(async { sqlx::query(sql.as_str()).execute(conn).await });
}

/// Test table initialization.
pub fn init(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.sqlite.as_ref().unwrap().network_uldata();

    let result = runtime.block_on(async { model.init().await });
    expect(result.is_ok()).to_equal(true)
}

/// Test `add()`.
pub fn add(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.sqlite.as_ref().unwrap().network_uldata();
    let conn = state.sqlite.as_ref().unwrap().get_connection();

    let now = Utc::now().trunc_subsecs(3);
    let mut extension = Map::<String, Value>::new();
    extension.insert("key".to_string(), Value::String("value".to_string()));
    let data = NetworkUlData {
        data_id: strings::random_id(&now, 8),
        proc: now,
        unit_code: Some("unit_code1".to_string()),
        network_code: "network_code1".to_string(),
        network_addr: "network_addr1".to_string(),
        unit_id: Some("unit_id1".to_string()),
        device_id: Some("device_id1".to_string()),
        time: now + Duration::milliseconds(2),
        data: "data1".to_string(),
        extension: Some(extension),
    };
    if let Err(e) = runtime.block_on(async { model.add(&data).await }) {
        return Err(format!("model.add() 1 error: {}", e));
    }
    let get_data = match runtime.block_on(async { get(&conn, data.data_id.as_str()).await }) {
        Err(e) => return Err(format!("model.get() 1 error: {}", e)),
        Ok(get_data) => match get_data {
            None => return Err("should get 1 one".to_string()),
            Some(get_data) => get_data,
        },
    };
    expect(get_data).to_equal(data)?;

    let now = Utc::now().trunc_subsecs(3);
    let data = NetworkUlData {
        data_id: strings::random_id(&now, 8),
        proc: now + Duration::milliseconds(1),
        unit_code: None,
        network_code: "public_code".to_string(),
        network_addr: "public_addr".to_string(),
        unit_id: Some("unit_id2".to_string()),
        device_id: Some("device_public".to_string()),
        time: now + Duration::milliseconds(2),
        data: "data2".to_string(),
        extension: None,
    };
    if let Err(e) = runtime.block_on(async { model.add(&data).await }) {
        return Err(format!("model.add() 2 error: {}", e));
    }
    let get_data = match runtime.block_on(async { get(&conn, data.data_id.as_str()).await }) {
        Err(e) => return Err(format!("model.get() 2 error: {}", e)),
        Ok(get_data) => match get_data {
            None => return Err("should get 2 one".to_string()),
            Some(get_data) => get_data,
        },
    };
    expect(get_data).to_equal(data)
}

/// Test `add()` with duplicate key.
pub fn add_dup(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.sqlite.as_ref().unwrap().network_uldata();

    let now = Utc::now().trunc_subsecs(3);
    let data = NetworkUlData {
        data_id: strings::random_id(&now, 8),
        proc: now,
        unit_code: Some("unit_code".to_string()),
        network_code: "network_code".to_string(),
        network_addr: "network_addr".to_string(),
        unit_id: Some("unit_id".to_string()),
        device_id: Some("device_id".to_string()),
        time: now,
        data: "data".to_string(),
        extension: None,
    };
    if let Err(e) = runtime.block_on(async { model.add(&data).await }) {
        return Err(format!("model.add() error: {}", e));
    }
    if let Ok(_) = runtime.block_on(async { model.add(&data).await }) {
        return Err("model.add() duplicate data_id should error".to_string());
    }
    Ok(())
}

/// Test `del()` by specifying unit ID.
pub fn del_by_unit(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.sqlite.as_ref().unwrap().network_uldata();
    let conn = state.sqlite.as_ref().unwrap().get_connection();

    let now = Utc::now().trunc_subsecs(3);
    let mut data = NetworkUlData {
        data_id: "data_id1_1".to_string(),
        proc: now,
        unit_code: Some("unit_code1".to_string()),
        network_code: "network_code1".to_string(),
        network_addr: "network_addr1_1".to_string(),
        unit_id: Some("unit_id1".to_string()),
        device_id: Some("device_id1_1".to_string()),
        time: now,
        data: "data".to_string(),
        extension: None,
    };
    let cond = QueryCond {
        unit_id: Some("unit_id1"),
        ..Default::default()
    };
    if let Err(e) = runtime.block_on(async {
        model.add(&data).await?;
        data.data_id = "data_id1_2".to_string();
        data.device_id = Some("device_id1_2".to_string());
        model.add(&data).await?;
        data.unit_code = None;
        data.unit_id = None;
        data.data_id = "data_id2".to_string();
        data.device_id = None;
        model.add(&data).await?;
        model.del(&cond).await
    }) {
        return Err(format!("model.add/del error: {}", e));
    }
    match runtime.block_on(async { get(conn, "data_id1_1").await }) {
        Err(e) => return Err(format!("model.get() 1_1 error: {}", e)),
        Ok(data) => match data {
            None => (),
            Some(_) => return Err("delete 1_1 fail".to_string()),
        },
    }
    match runtime.block_on(async { get(conn, "data_id1_2").await }) {
        Err(e) => return Err(format!("model.get() 1_2 error: {}", e)),
        Ok(data) => match data {
            None => (),
            Some(_) => return Err("delete 1_2 fail".to_string()),
        },
    }
    match runtime.block_on(async { get(conn, "data_id2").await }) {
        Err(e) => Err(format!("model.get() 2 error: {}", e)),
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
    let model = state.sqlite.as_ref().unwrap().network_uldata();

    let now = Utc::now().trunc_subsecs(3);
    let data = NetworkUlData {
        data_id: "data_id1_1".to_string(),
        proc: now,
        unit_code: Some("unit_code1".to_string()),
        network_code: "network_code1".to_string(),
        network_addr: "network_addr1_1".to_string(),
        unit_id: Some("unit_id1".to_string()),
        device_id: Some("device_id1_1".to_string()),
        time: now,
        data: "data".to_string(),
        extension: None,
    };
    let cond = QueryCond {
        unit_id: Some("unit_id1"),
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

/// Test `del()` by specifying device ID.
pub fn del_by_device_id(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.sqlite.as_ref().unwrap().network_uldata();
    let conn = state.sqlite.as_ref().unwrap().get_connection();

    let now = Utc::now().trunc_subsecs(3);
    let mut data = NetworkUlData {
        data_id: "data_id1_1".to_string(),
        proc: now,
        unit_code: Some("unit_code1".to_string()),
        network_code: "network_code1".to_string(),
        network_addr: "network_addr1".to_string(),
        unit_id: Some("unit_id1".to_string()),
        device_id: Some("device_id1".to_string()),
        time: now,
        data: "data".to_string(),
        extension: None,
    };
    let cond = QueryCond {
        device_id: Some("device_id1"),
        ..Default::default()
    };
    if let Err(e) = runtime.block_on(async {
        model.add(&data).await?;
        data.data_id = "data_id1_2".to_string();
        model.add(&data).await?;
        data.data_id = "data_id2".to_string();
        data.device_id = Some("device_id2".to_string());
        model.add(&data).await?;
        model.del(&cond).await
    }) {
        return Err(format!("model.add/del error: {}", e));
    }
    match runtime.block_on(async { get(conn, "data_id1_1").await }) {
        Err(e) => return Err(format!("model.get() 1_1 error: {}", e)),
        Ok(data) => match data {
            None => (),
            Some(_) => return Err("delete 1_1 fail".to_string()),
        },
    }
    match runtime.block_on(async { get(conn, "data_id1_2").await }) {
        Err(e) => return Err(format!("model.get() 1_2 error: {}", e)),
        Ok(data) => match data {
            None => (),
            Some(_) => return Err("delete 1_2 fail".to_string()),
        },
    }
    match runtime.block_on(async { get(conn, "data_id2").await }) {
        Err(e) => Err(format!("model.get() 2 error: {}", e)),
        Ok(data) => match data {
            None => Err("delete wrong one".to_string()),
            Some(_) => Ok(()),
        },
    }
}

/// Test `del()` by specifying processed time.
pub fn del_by_proc(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.sqlite.as_ref().unwrap().network_uldata();
    let conn = state.sqlite.as_ref().unwrap().get_connection();

    let now = Utc::now().trunc_subsecs(3);
    let mut data = NetworkUlData {
        data_id: "data_id1".to_string(),
        proc: now,
        unit_code: Some("unit_code".to_string()),
        network_code: "network_code".to_string(),
        network_addr: "network_addr".to_string(),
        unit_id: Some("unit_id".to_string()),
        device_id: Some("device_id".to_string()),
        time: now,
        data: "data".to_string(),
        extension: None,
    };
    let cond = QueryCond {
        proc_gte: Some(now + Duration::milliseconds(1)),
        proc_lte: Some(now + Duration::milliseconds(2)),
        ..Default::default()
    };
    if let Err(e) = runtime.block_on(async {
        model.add(&data).await?;
        data.data_id = "data_id2".to_string();
        data.proc = now + Duration::milliseconds(1);
        model.add(&data).await?;
        data.data_id = "data_id3".to_string();
        data.proc = now + Duration::milliseconds(2);
        model.add(&data).await?;
        data.data_id = "data_id4".to_string();
        data.proc = now + Duration::milliseconds(3);
        model.add(&data).await?;
        model.del(&cond).await
    }) {
        return Err(format!("model.add/del error: {}", e));
    }
    match runtime.block_on(async { get(conn, "data_id1").await }) {
        Err(e) => return Err(format!("model.get() 1 error: {}", e)),
        Ok(data) => match data {
            None => return Err("delete wrong one 1".to_string()),
            Some(_) => (),
        },
    }
    match runtime.block_on(async { get(conn, "data_id2").await }) {
        Err(e) => return Err(format!("model.get() 2 error: {}", e)),
        Ok(data) => match data {
            None => (),
            Some(_) => return Err("delete 2 fail".to_string()),
        },
    }
    match runtime.block_on(async { get(conn, "data_id3").await }) {
        Err(e) => return Err(format!("model.get() 3 error: {}", e)),
        Ok(data) => match data {
            None => (),
            Some(_) => return Err("delete 3 fail".to_string()),
        },
    }
    match runtime.block_on(async { get(conn, "data_id4").await }) {
        Err(e) => Err(format!("model.get() 4 error: {}", e)),
        Ok(data) => match data {
            None => Err("delete wrong one 4".to_string()),
            Some(_) => Ok(()),
        },
    }
}

/// Test `count()`.
pub fn count(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.sqlite.as_ref().unwrap().network_uldata();

    let now = Utc::now().trunc_subsecs(3);
    let mut data = NetworkUlData {
        data_id: "data_id1".to_string(),
        proc: now,
        unit_code: Some("unit_code1".to_string()),
        device_id: Some("device_id1_1".to_string()),
        unit_id: Some("unit_id1".to_string()),
        network_code: "network_code1_1".to_string(),
        network_addr: "network_addr1_1".to_string(),
        time: now + Duration::milliseconds(5),
        data: "data".to_string(),
        extension: None,
    };
    if let Err(e) = runtime.block_on(async {
        model.add(&data).await?;
        data.data_id = "data_id2".to_string();
        data.proc = now + Duration::milliseconds(1);
        data.time = now + Duration::milliseconds(4);
        model.add(&data).await?;
        data.data_id = "data_id3".to_string();
        data.proc = now + Duration::milliseconds(2);
        data.time = now + Duration::milliseconds(3);
        data.device_id = Some("device_id1_2".to_string());
        data.network_addr = "network_addr1_2".to_string();
        model.add(&data).await?;
        data.data_id = "data_id4".to_string();
        data.proc = now + Duration::milliseconds(3);
        data.time = now + Duration::milliseconds(2);
        data.unit_id = None;
        data.device_id = None;
        data.network_code = "network_code2".to_string();
        data.network_addr = "network_addr2".to_string();
        model.add(&data).await?;
        data.data_id = "data_id5".to_string();
        data.unit_code = None;
        data.unit_id = Some("unit_id2".to_string());
        data.proc = now + Duration::milliseconds(4);
        data.time = now + Duration::milliseconds(1);
        data.device_id = Some("device_id3".to_string());
        data.network_code = "network_code3".to_string();
        data.network_addr = "network_addr3".to_string();
        data.extension = Some(Map::new());
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
        unit_id: Some("unit_id1"),
        ..Default::default()
    };
    let count = match runtime.block_on(async { model.count(&cond).await }) {
        Err(e) => return Err(format!("count unit_id result error: {}", e)),
        Ok(count) => count,
    };
    expect(count).to_equal(3)?;

    let cond = ListQueryCond {
        device_id: Some("device_id3"),
        ..Default::default()
    };
    let count = match runtime.block_on(async { model.count(&cond).await }) {
        Err(e) => return Err(format!("count device_id result error: {}", e)),
        Ok(count) => count,
    };
    expect(count).to_equal(1)?;

    let cond = ListQueryCond {
        network_code: Some("network_code1_1"),
        network_addr: Some("network_addr1_1"),
        ..Default::default()
    };
    let count = match runtime.block_on(async { model.count(&cond).await }) {
        Err(e) => return Err(format!("count network_code_addr result error: {}", e)),
        Ok(count) => count,
    };
    expect(count).to_equal(2)?;

    let cond = ListQueryCond {
        proc_gte: Some(now + Duration::milliseconds(1)),
        proc_lte: Some(now + Duration::milliseconds(3)),
        ..Default::default()
    };
    let count = match runtime.block_on(async { model.count(&cond).await }) {
        Err(e) => return Err(format!("count proc result error: {}", e)),
        Ok(count) => count,
    };
    expect(count).to_equal(3)?;

    let cond = ListQueryCond {
        time_gte: Some(now + Duration::milliseconds(2)),
        time_lte: Some(now + Duration::milliseconds(3)),
        ..Default::default()
    };
    let count = match runtime.block_on(async { model.count(&cond).await }) {
        Err(e) => return Err(format!("count time result error: {}", e)),
        Ok(count) => count,
    };
    expect(count).to_equal(2)?;

    let cond = ListQueryCond {
        unit_id: Some("unit_id1"),
        device_id: Some("device_id3"),
        ..Default::default()
    };
    let count = match runtime.block_on(async { model.count(&cond).await }) {
        Err(e) => return Err(format!("count unit_id_device_id result error: {}", e)),
        Ok(count) => count,
    };
    expect(count).to_equal(0)
}

/// Test `list()`.
pub fn list(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.sqlite.as_ref().unwrap().network_uldata();

    let now = Utc::now().trunc_subsecs(3);
    let mut data = NetworkUlData {
        data_id: "data_id1".to_string(),
        proc: now,
        unit_code: Some("unit_code1".to_string()),
        device_id: Some("device_id1_1".to_string()),
        unit_id: Some("unit_id1".to_string()),
        network_code: "network_code1_1".to_string(),
        network_addr: "network_addr1_1".to_string(),
        time: now + Duration::milliseconds(5),
        data: "data".to_string(),
        extension: None,
    };
    if let Err(e) = runtime.block_on(async {
        model.add(&data).await?;
        data.data_id = "data_id2".to_string();
        data.proc = now + Duration::milliseconds(1);
        data.time = now + Duration::milliseconds(4);
        model.add(&data).await?;
        data.data_id = "data_id3".to_string();
        data.proc = now + Duration::milliseconds(2);
        data.time = now + Duration::milliseconds(3);
        data.device_id = Some("device_id1_2".to_string());
        data.network_addr = "network_addr1_2".to_string();
        model.add(&data).await?;
        data.data_id = "data_id4".to_string();
        data.proc = now + Duration::milliseconds(3);
        data.time = now + Duration::milliseconds(2);
        data.unit_id = None;
        data.device_id = None;
        data.network_code = "network_code2".to_string();
        data.network_addr = "network_addr2".to_string();
        model.add(&data).await?;
        data.data_id = "data_id5".to_string();
        data.unit_code = None;
        data.unit_id = Some("unit_id2".to_string());
        data.proc = now + Duration::milliseconds(4);
        data.time = now + Duration::milliseconds(1);
        data.device_id = Some("device_id3".to_string());
        data.network_code = "network_code3".to_string();
        data.network_addr = "network_addr3".to_string();
        data.extension = Some(Map::new());
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
        unit_id: Some("unit_id1"),
        ..Default::default()
    };
    opts.cond = &cond;
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list unit_id result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(3)?;

    let cond = ListQueryCond {
        device_id: Some("device_id3"),
        ..Default::default()
    };
    opts.cond = &cond;
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list device_id result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(1)?;

    let cond = ListQueryCond {
        network_code: Some("network_code1_1"),
        network_addr: Some("network_addr1_1"),
        ..Default::default()
    };
    opts.cond = &cond;
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list network_code_addr result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(2)?;

    let cond = ListQueryCond {
        proc_gte: Some(now + Duration::milliseconds(1)),
        proc_lte: Some(now + Duration::milliseconds(3)),
        ..Default::default()
    };
    opts.cond = &cond;
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list proc result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(3)?;

    let cond = ListQueryCond {
        time_gte: Some(now + Duration::milliseconds(2)),
        time_lte: Some(now + Duration::milliseconds(3)),
        ..Default::default()
    };
    opts.cond = &cond;
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list time result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(2)?;

    let cond = ListQueryCond {
        unit_id: Some("unit_id1"),
        device_id: Some("device_id3"),
        ..Default::default()
    };
    opts.cond = &cond;
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list unit_id_device_id result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(0)
}

/// Test `list()` with sorting.
pub fn list_sort(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.sqlite.as_ref().unwrap().network_uldata();

    let now = Utc::now().trunc_subsecs(3);
    let mut data = NetworkUlData {
        data_id: "data_id1".to_string(),
        proc: now,
        unit_code: Some("unit_code1".to_string()),
        device_id: Some("device_id1_1".to_string()),
        unit_id: Some("unit_id1".to_string()),
        network_code: "network_code1_1".to_string(),
        network_addr: "network_addr1_1".to_string(),
        time: now + Duration::milliseconds(5),
        data: "data".to_string(),
        extension: None,
    };
    if let Err(e) = runtime.block_on(async {
        model.add(&data).await?;
        data.data_id = "data_id2".to_string();
        data.proc = now + Duration::milliseconds(1);
        data.time = now + Duration::milliseconds(4);
        model.add(&data).await?;
        data.data_id = "data_id3".to_string();
        data.proc = now + Duration::milliseconds(2);
        data.time = now + Duration::milliseconds(3);
        data.device_id = Some("device_id1_2".to_string());
        data.network_addr = "network_addr1_2".to_string();
        model.add(&data).await?;
        data.data_id = "data_id4".to_string();
        data.proc = now + Duration::milliseconds(3);
        data.time = now + Duration::milliseconds(2);
        data.unit_id = None;
        data.device_id = None;
        data.network_code = "network_code2".to_string();
        data.network_addr = "network_addr2".to_string();
        model.add(&data).await?;
        data.data_id = "data_id5".to_string();
        data.unit_code = None;
        data.unit_id = Some("unit_id2".to_string());
        data.proc = now + Duration::milliseconds(4);
        data.time = now + Duration::milliseconds(1);
        data.device_id = Some("device_id3".to_string());
        data.network_code = "network_code3".to_string();
        data.network_addr = "network_addr3".to_string();
        data.extension = Some(Map::new());
        model.add(&data).await
    }) {
        return Err(format!("model.add() error: {}", e));
    }

    let cond = ListQueryCond {
        ..Default::default()
    };
    let sort_cond = vec![SortCond {
        key: SortKey::Proc,
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
        Err(e) => return Err(format!("list proc-asc result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(5)?;
    expect(list[0].data_id.as_str()).to_equal("data_id1")?;
    expect(list[1].data_id.as_str()).to_equal("data_id2")?;
    expect(list[2].data_id.as_str()).to_equal("data_id3")?;
    expect(list[3].data_id.as_str()).to_equal("data_id4")?;
    expect(list[4].data_id.as_str()).to_equal("data_id5")?;

    let sort_cond = vec![SortCond {
        key: SortKey::Proc,
        asc: false,
    }];
    opts.sort = Some(sort_cond.as_slice());
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list proc-desc result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(5)?;
    expect(list[0].data_id.as_str()).to_equal("data_id5")?;
    expect(list[1].data_id.as_str()).to_equal("data_id4")?;
    expect(list[2].data_id.as_str()).to_equal("data_id3")?;
    expect(list[3].data_id.as_str()).to_equal("data_id2")?;
    expect(list[4].data_id.as_str()).to_equal("data_id1")?;

    let sort_cond = vec![SortCond {
        key: SortKey::Time,
        asc: true,
    }];
    opts.sort = Some(sort_cond.as_slice());
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list time-asc result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(5)?;
    expect(list[0].data_id.as_str()).to_equal("data_id5")?;
    expect(list[1].data_id.as_str()).to_equal("data_id4")?;
    expect(list[2].data_id.as_str()).to_equal("data_id3")?;
    expect(list[3].data_id.as_str()).to_equal("data_id2")?;
    expect(list[4].data_id.as_str()).to_equal("data_id1")?;

    let sort_cond = vec![SortCond {
        key: SortKey::Time,
        asc: false,
    }];
    opts.sort = Some(sort_cond.as_slice());
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list time-desc result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(5)?;
    expect(list[0].data_id.as_str()).to_equal("data_id1")?;
    expect(list[1].data_id.as_str()).to_equal("data_id2")?;
    expect(list[2].data_id.as_str()).to_equal("data_id3")?;
    expect(list[3].data_id.as_str()).to_equal("data_id4")?;
    expect(list[4].data_id.as_str()).to_equal("data_id5")?;

    let sort_cond = vec![
        SortCond {
            key: SortKey::NetworkCode,
            asc: true,
        },
        SortCond {
            key: SortKey::NetworkAddr,
            asc: true,
        },
        SortCond {
            key: SortKey::Proc,
            asc: true,
        },
    ];
    opts.sort = Some(sort_cond.as_slice());
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => {
            return Err(format!(
                "list code-asc-addr-asc-proc-asc result error: {}",
                e
            ))
        }
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(5)?;
    expect(list[0].data_id.as_str()).to_equal("data_id1")?;
    expect(list[1].data_id.as_str()).to_equal("data_id2")?;
    expect(list[2].data_id.as_str()).to_equal("data_id3")?;
    expect(list[3].data_id.as_str()).to_equal("data_id4")?;
    expect(list[4].data_id.as_str()).to_equal("data_id5")?;

    let sort_cond = vec![
        SortCond {
            key: SortKey::NetworkCode,
            asc: true,
        },
        SortCond {
            key: SortKey::NetworkAddr,
            asc: true,
        },
        SortCond {
            key: SortKey::Proc,
            asc: false,
        },
    ];
    opts.sort = Some(sort_cond.as_slice());
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => {
            return Err(format!(
                "list code-asc-addr-asc-proc-desc result error: {}",
                e
            ))
        }
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(5)?;
    expect(list[0].data_id.as_str()).to_equal("data_id2")?;
    expect(list[1].data_id.as_str()).to_equal("data_id1")?;
    expect(list[2].data_id.as_str()).to_equal("data_id3")?;
    expect(list[3].data_id.as_str()).to_equal("data_id4")?;
    expect(list[4].data_id.as_str()).to_equal("data_id5")?;

    let sort_cond = vec![
        SortCond {
            key: SortKey::NetworkCode,
            asc: false,
        },
        SortCond {
            key: SortKey::NetworkAddr,
            asc: true,
        },
        SortCond {
            key: SortKey::Proc,
            asc: true,
        },
    ];
    opts.sort = Some(sort_cond.as_slice());
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => {
            return Err(format!(
                "list code-desc-addr-asc-proc-asc result error: {}",
                e
            ))
        }
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(5)?;
    expect(list[0].data_id.as_str()).to_equal("data_id5")?;
    expect(list[1].data_id.as_str()).to_equal("data_id4")?;
    expect(list[2].data_id.as_str()).to_equal("data_id1")?;
    expect(list[3].data_id.as_str()).to_equal("data_id2")?;
    expect(list[4].data_id.as_str()).to_equal("data_id3")?;

    let sort_cond = vec![
        SortCond {
            key: SortKey::NetworkCode,
            asc: false,
        },
        SortCond {
            key: SortKey::NetworkAddr,
            asc: false,
        },
        SortCond {
            key: SortKey::Proc,
            asc: true,
        },
    ];
    opts.sort = Some(sort_cond.as_slice());
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => {
            return Err(format!(
                "list code-desc-addr-desc-proc-asc result error: {}",
                e
            ))
        }
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(5)?;
    expect(list[0].data_id.as_str()).to_equal("data_id5")?;
    expect(list[1].data_id.as_str()).to_equal("data_id4")?;
    expect(list[2].data_id.as_str()).to_equal("data_id3")?;
    expect(list[3].data_id.as_str()).to_equal("data_id1")?;
    expect(list[4].data_id.as_str()).to_equal("data_id2")?;

    Ok(())
}

/// Test `list()` with offset/limit.
pub fn list_offset_limit(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.sqlite.as_ref().unwrap().network_uldata();

    let now = Utc::now().trunc_subsecs(3);
    let mut data = NetworkUlData {
        data_id: "data_id1".to_string(),
        proc: now,
        unit_code: Some("unit_code1".to_string()),
        device_id: Some("device_id1_1".to_string()),
        unit_id: Some("unit_id1".to_string()),
        network_code: "network_code1_1".to_string(),
        network_addr: "network_addr1_1".to_string(),
        time: now + Duration::milliseconds(5),
        data: "data".to_string(),
        extension: None,
    };
    if let Err(e) = runtime.block_on(async {
        model.add(&data).await?;
        data.data_id = "data_id2".to_string();
        data.proc = now + Duration::milliseconds(1);
        data.time = now + Duration::milliseconds(4);
        model.add(&data).await?;
        data.data_id = "data_id3".to_string();
        data.proc = now + Duration::milliseconds(2);
        data.time = now + Duration::milliseconds(3);
        data.device_id = Some("device_id1_2".to_string());
        data.network_addr = "network_addr1_2".to_string();
        model.add(&data).await?;
        data.data_id = "data_id4".to_string();
        data.proc = now + Duration::milliseconds(3);
        data.time = now + Duration::milliseconds(2);
        data.unit_id = None;
        data.device_id = None;
        data.network_code = "network_code2".to_string();
        data.network_addr = "network_addr2".to_string();
        model.add(&data).await?;
        data.data_id = "data_id5".to_string();
        data.unit_code = None;
        data.unit_id = Some("unit_id2".to_string());
        data.proc = now + Duration::milliseconds(4);
        data.time = now + Duration::milliseconds(1);
        data.device_id = Some("device_id3".to_string());
        data.network_code = "network_code3".to_string();
        data.network_addr = "network_addr3".to_string();
        data.extension = Some(Map::new());
        model.add(&data).await
    }) {
        return Err(format!("model.add() error: {}", e));
    }

    let cond = ListQueryCond {
        ..Default::default()
    };
    let sort_cond = vec![SortCond {
        key: SortKey::Proc,
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
    expect(list[0].data_id.as_str()).to_equal("data_id1")?;
    expect(list[1].data_id.as_str()).to_equal("data_id2")?;
    expect(list[2].data_id.as_str()).to_equal("data_id3")?;

    opts.limit = Some(6);
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list limit-6 result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(5)?;
    expect(list[0].data_id.as_str()).to_equal("data_id1")?;
    expect(list[1].data_id.as_str()).to_equal("data_id2")?;
    expect(list[2].data_id.as_str()).to_equal("data_id3")?;
    expect(list[3].data_id.as_str()).to_equal("data_id4")?;
    expect(list[4].data_id.as_str()).to_equal("data_id5")?;

    opts.limit = None;
    opts.offset = Some(2);
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list offset-2 result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(3)?;
    expect(list[0].data_id.as_str()).to_equal("data_id3")?;
    expect(list[1].data_id.as_str()).to_equal("data_id4")?;
    expect(list[2].data_id.as_str()).to_equal("data_id5")?;

    opts.limit = Some(0);
    opts.offset = Some(0);
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list offset-limit0 result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(5)?;
    expect(list[0].data_id.as_str()).to_equal("data_id1")?;
    expect(list[1].data_id.as_str()).to_equal("data_id2")?;
    expect(list[2].data_id.as_str()).to_equal("data_id3")?;
    expect(list[3].data_id.as_str()).to_equal("data_id4")?;
    expect(list[4].data_id.as_str()).to_equal("data_id5")?;

    opts.limit = Some(3);
    opts.offset = Some(3);
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list offset-limit3 result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(2)?;
    expect(list[0].data_id.as_str()).to_equal("data_id4")?;
    expect(list[1].data_id.as_str()).to_equal("data_id5")
}

/// Test `list()` with cursors.
pub fn list_cursor(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let model = state.sqlite.as_ref().unwrap().network_uldata();

    let now = Utc::now().trunc_subsecs(3);
    let mut data = NetworkUlData {
        data_id: "data_id1".to_string(),
        proc: now,
        unit_code: Some("unit_code1".to_string()),
        device_id: Some("device_id1_1".to_string()),
        unit_id: Some("unit_id1".to_string()),
        network_code: "network_code1_1".to_string(),
        network_addr: "network_addr1_1".to_string(),
        time: now + Duration::milliseconds(5),
        data: "data".to_string(),
        extension: None,
    };
    if let Err(e) = runtime.block_on(async {
        model.add(&data).await?;
        data.data_id = "data_id2".to_string();
        data.proc = now + Duration::milliseconds(1);
        data.time = now + Duration::milliseconds(4);
        model.add(&data).await?;
        data.data_id = "data_id3".to_string();
        data.proc = now + Duration::milliseconds(2);
        data.time = now + Duration::milliseconds(3);
        data.device_id = Some("device_id1_2".to_string());
        data.network_addr = "network_addr1_2".to_string();
        model.add(&data).await?;
        data.data_id = "data_id4".to_string();
        data.proc = now + Duration::milliseconds(3);
        data.time = now + Duration::milliseconds(2);
        data.unit_id = None;
        data.device_id = None;
        data.network_code = "network_code2".to_string();
        data.network_addr = "network_addr2".to_string();
        model.add(&data).await?;
        data.data_id = "data_id5".to_string();
        data.unit_code = None;
        data.unit_id = Some("unit_id2".to_string());
        data.proc = now + Duration::milliseconds(4);
        data.time = now + Duration::milliseconds(1);
        data.device_id = Some("device_id3".to_string());
        data.network_code = "network_code3".to_string();
        data.network_addr = "network_addr3".to_string();
        data.extension = Some(Map::new());
        model.add(&data).await
    }) {
        return Err(format!("model.add() error: {}", e));
    }

    let cond = ListQueryCond {
        ..Default::default()
    };
    let sort_cond = vec![SortCond {
        key: SortKey::Proc,
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
    expect(list[0].data_id.as_str()).to_equal("data_id1")?;
    expect(list[1].data_id.as_str()).to_equal("data_id2")?;
    expect(list[2].data_id.as_str()).to_equal("data_id3")?;
    expect(cursor.is_some()).to_equal(true)?;
    expect(cursor.as_ref().unwrap().offset()).to_equal(3)?;
    let (list, cursor) = match runtime.block_on(async { model.list(&opts, cursor).await }) {
        Err(e) => return Err(format!("list cursor-3-2 result error: {}", e)),
        Ok((list, cursor)) => (list, cursor),
    };
    expect(list.len()).to_equal(2)?;
    expect(list[0].data_id.as_str()).to_equal("data_id4")?;
    expect(list[1].data_id.as_str()).to_equal("data_id5")?;
    expect(cursor.is_none()).to_equal(true)?;

    opts.limit = Some(3);
    opts.cursor_max = Some(2);
    let (list, cursor) = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list cursor-2-1 result error: {}", e)),
        Ok((list, cursor)) => (list, cursor),
    };
    expect(list.len()).to_equal(2)?;
    expect(list[0].data_id.as_str()).to_equal("data_id1")?;
    expect(list[1].data_id.as_str()).to_equal("data_id2")?;
    expect(cursor.is_some()).to_equal(true)?;
    expect(cursor.as_ref().unwrap().offset()).to_equal(2)?;
    let (list, cursor) = match runtime.block_on(async { model.list(&opts, cursor).await }) {
        Err(e) => return Err(format!("list cursor-2-2 result error: {}", e)),
        Ok((list, cursor)) => (list, cursor),
    };
    expect(list.len()).to_equal(1)?;
    expect(list[0].data_id.as_str()).to_equal("data_id3")?;
    expect(cursor.is_none()).to_equal(true)?;

    opts.limit = Some(3);
    opts.cursor_max = Some(2);
    let (list, cursor) = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list cursor-2-1 result error: {}", e)),
        Ok((list, cursor)) => (list, cursor),
    };
    expect(list.len()).to_equal(2)?;
    expect(list[0].data_id.as_str()).to_equal("data_id1")?;
    expect(list[1].data_id.as_str()).to_equal("data_id2")?;
    expect(cursor.is_some()).to_equal(true)?;
    expect(cursor.as_ref().unwrap().offset()).to_equal(2)?;
    let (list, cursor) = match runtime.block_on(async { model.list(&opts, cursor).await }) {
        Err(e) => return Err(format!("list cursor-2-2 result error: {}", e)),
        Ok((list, cursor)) => (list, cursor),
    };
    expect(list.len()).to_equal(1)?;
    expect(list[0].data_id.as_str()).to_equal("data_id3")?;
    expect(cursor.is_none()).to_equal(true)?;

    opts.offset = Some(1);
    opts.limit = Some(4);
    opts.cursor_max = Some(2);
    let (list, cursor) = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list cursor-2-2-1 result error: {}", e)),
        Ok((list, cursor)) => (list, cursor),
    };
    expect(list.len()).to_equal(2)?;
    expect(list[0].data_id.as_str()).to_equal("data_id2")?;
    expect(list[1].data_id.as_str()).to_equal("data_id3")?;
    expect(cursor.is_some()).to_equal(true)?;
    expect(cursor.as_ref().unwrap().offset()).to_equal(2)?;
    let (list, cursor) = match runtime.block_on(async { model.list(&opts, cursor).await }) {
        Err(e) => return Err(format!("list cursor-2-2-2 result error: {}", e)),
        Ok((list, cursor)) => (list, cursor),
    };
    expect(list.len()).to_equal(2)?;
    expect(list[0].data_id.as_str()).to_equal("data_id4")?;
    expect(list[1].data_id.as_str()).to_equal("data_id5")?;
    expect(cursor.is_some()).to_equal(true)?;
    expect(cursor.as_ref().unwrap().offset()).to_equal(4)?;
    let (list, cursor) = match runtime.block_on(async { model.list(&opts, cursor).await }) {
        Err(e) => return Err(format!("list cursor-2-2-3 result error: {}", e)),
        Ok((list, cursor)) => (list, cursor),
    };
    expect(list.len()).to_equal(0)?;
    expect(cursor.is_none()).to_equal(true)?;

    opts.offset = Some(3);
    opts.limit = Some(3);
    opts.cursor_max = Some(5);
    let (list, cursor) = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list cursor-5 result error: {}", e)),
        Ok((list, cursor)) => (list, cursor),
    };
    expect(list.len()).to_equal(2)?;
    expect(list[0].data_id.as_str()).to_equal("data_id4")?;
    expect(list[1].data_id.as_str()).to_equal("data_id5")?;
    expect(cursor.is_none()).to_equal(true)
}

async fn get(conn: &SqlitePool, data_id: &str) -> Result<Option<NetworkUlData>, Box<dyn StdError>> {
    let sql = SqlBuilder::select_from(TABLE_NAME)
        .fields(FIELDS)
        .and_where_eq("data_id", quote(data_id))
        .sql()?;

    let result: Result<Schema, sqlx::Error> = sqlx::query_as(sql.as_str()).fetch_one(conn).await;

    let row = match result {
        Err(e) => match e {
            sqlx::Error::RowNotFound => return Ok(None),
            _ => return Err(Box::new(e)),
        },
        Ok(row) => row,
    };

    Ok(Some(NetworkUlData {
        data_id: row.data_id,
        proc: Utc.timestamp_nanos(row.proc * 1000000),
        unit_code: match row.unit_code.len() {
            0 => None,
            _ => Some(row.unit_code),
        },
        network_code: row.network_code,
        network_addr: row.network_addr,
        unit_id: match row.unit_id.len() {
            0 => None,
            _ => Some(row.unit_id),
        },
        device_id: match row.device_id.len() {
            0 => None,
            _ => Some(row.device_id),
        },
        time: Utc.timestamp_nanos(row.time * 1000000),
        data: row.data,
        extension: match row.extension.len() {
            0 => None,
            _ => serde_json::from_str(row.extension.as_str())?,
        },
    }))
}
