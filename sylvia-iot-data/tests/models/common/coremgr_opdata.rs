use std::error::Error as StdError;

use async_trait::async_trait;
use chrono::{SubsecRound, TimeDelta, Utc};
use laboratory::expect;
use serde_json::{Map, Value};
use tokio::runtime::Runtime;

use sylvia_iot_corelib::strings;
use sylvia_iot_data::models::coremgr_opdata::{
    CoremgrOpData, CoremgrOpDataModel, ListOptions, ListQueryCond, QueryCond, SortCond, SortKey,
};

#[async_trait]
/// Database operations.
pub trait Db {
    /// To get the data which ID is `data_id`.
    async fn get(&self, data_id: &str) -> Result<Option<CoremgrOpData>, Box<dyn StdError>>;
}

/// Test `add()`.
pub fn add(runtime: &Runtime, model: &dyn CoremgrOpDataModel, db: &dyn Db) -> Result<(), String> {
    let now = Utc::now().trunc_subsecs(3);
    let mut body = Map::<String, Value>::new();
    body.insert("key".to_string(), Value::String("value".to_string()));
    let data = CoremgrOpData {
        data_id: strings::random_id(&now, 8),
        req_time: now,
        res_time: now + TimeDelta::try_milliseconds(2).unwrap(),
        latency_ms: 2,
        status: 200,
        source_ip: "::1".to_string(),
        method: "GET".to_string(),
        path: "/path".to_string(),
        body: Some(body),
        user_id: "user_id1".to_string(),
        client_id: "client_id1".to_string(),
        err_code: None,
        err_message: None,
    };
    if let Err(e) = runtime.block_on(async { model.add(&data).await }) {
        return Err(format!("model.add() 1 error: {}", e));
    }
    let get_data = match runtime.block_on(async { db.get(data.data_id.as_str()).await }) {
        Err(e) => return Err(format!("model.get() 1 error: {}", e)),
        Ok(get_data) => match get_data {
            None => return Err("should get 1 one".to_string()),
            Some(get_data) => get_data,
        },
    };
    expect(get_data).to_equal(data)?;

    let now = Utc::now().trunc_subsecs(3);
    let data = CoremgrOpData {
        data_id: strings::random_id(&now, 8),
        req_time: now,
        res_time: now + TimeDelta::try_milliseconds(2).unwrap(),
        latency_ms: 2,
        status: 400,
        source_ip: "::1".to_string(),
        method: "GET".to_string(),
        path: "/path".to_string(),
        body: None,
        user_id: "user_id2".to_string(),
        client_id: "client_id2".to_string(),
        err_code: Some("err_param".to_string()),
        err_message: Some("error parameter".to_string()),
    };
    if let Err(e) = runtime.block_on(async { model.add(&data).await }) {
        return Err(format!("model.add() 2 error: {}", e));
    }
    let get_data = match runtime.block_on(async { db.get(data.data_id.as_str()).await }) {
        Err(e) => return Err(format!("model.get() 2 error: {}", e)),
        Ok(get_data) => match get_data {
            None => return Err("should get 2 one".to_string()),
            Some(get_data) => get_data,
        },
    };
    expect(get_data).to_equal(data)
}

/// Test `add()` with duplicate key.
pub fn add_dup(runtime: &Runtime, model: &dyn CoremgrOpDataModel) -> Result<(), String> {
    let now = Utc::now().trunc_subsecs(3);
    let data = CoremgrOpData {
        data_id: strings::random_id(&now, 8),
        req_time: now,
        res_time: now + TimeDelta::try_milliseconds(2).unwrap(),
        latency_ms: 2,
        status: 200,
        source_ip: "::1".to_string(),
        method: "GET".to_string(),
        path: "/path".to_string(),
        body: None,
        user_id: "user_id".to_string(),
        client_id: "client_id".to_string(),
        err_code: None,
        err_message: None,
    };
    if let Err(e) = runtime.block_on(async { model.add(&data).await }) {
        return Err(format!("model.add() error: {}", e));
    }
    if let Ok(_) = runtime.block_on(async { model.add(&data).await }) {
        return Err("model.add() duplicate data_id should error".to_string());
    }
    Ok(())
}

/// Test `del()` by specifying user ID.
pub fn del_by_user(
    runtime: &Runtime,
    model: &dyn CoremgrOpDataModel,
    db: &dyn Db,
) -> Result<(), String> {
    let now = Utc::now().trunc_subsecs(3);
    let mut data = CoremgrOpData {
        data_id: "data_id1".to_string(),
        req_time: now,
        res_time: now + TimeDelta::try_milliseconds(2).unwrap(),
        latency_ms: 2,
        status: 200,
        source_ip: "::1".to_string(),
        method: "GET".to_string(),
        path: "/path".to_string(),
        body: None,
        user_id: "user_id1".to_string(),
        client_id: "client_id1".to_string(),
        err_code: None,
        err_message: None,
    };
    let cond = QueryCond {
        user_id: Some("user_id1"),
        ..Default::default()
    };
    if let Err(e) = runtime.block_on(async {
        model.add(&data).await?;
        data.data_id = "data_id2".to_string();
        model.add(&data).await?;
        data.data_id = "data_id3".to_string();
        data.user_id = "user_id2".to_string();
        model.add(&data).await?;
        model.del(&cond).await
    }) {
        return Err(format!("model.add/del error: {}", e));
    }
    match runtime.block_on(async { db.get("data_id1").await }) {
        Err(e) => return Err(format!("model.get() 1 error: {}", e)),
        Ok(data) => match data {
            None => (),
            Some(_) => return Err("delete 1 fail".to_string()),
        },
    }
    match runtime.block_on(async { db.get("data_id2").await }) {
        Err(e) => return Err(format!("model.get() 2 error: {}", e)),
        Ok(data) => match data {
            None => (),
            Some(_) => return Err("delete 2 fail".to_string()),
        },
    }
    match runtime.block_on(async { db.get("data_id3").await }) {
        Err(e) => Err(format!("model.get() 3 error: {}", e)),
        Ok(data) => match data {
            None => Err("delete wrong one".to_string()),
            Some(_) => Ok(()),
        },
    }
}

/// Test `del()` twice.
pub fn del_twice(runtime: &Runtime, model: &dyn CoremgrOpDataModel) -> Result<(), String> {
    let now = Utc::now().trunc_subsecs(3);
    let data = CoremgrOpData {
        data_id: "data_id1".to_string(),
        req_time: now,
        res_time: now + TimeDelta::try_milliseconds(2).unwrap(),
        latency_ms: 2,
        status: 200,
        source_ip: "::1".to_string(),
        method: "GET".to_string(),
        path: "/path".to_string(),
        body: None,
        user_id: "user_id1".to_string(),
        client_id: "client_id1".to_string(),
        err_code: None,
        err_message: None,
    };
    let cond = QueryCond {
        user_id: Some("user_id1"),
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

/// Test `del()` by specifying client ID.
pub fn del_by_client(
    runtime: &Runtime,
    model: &dyn CoremgrOpDataModel,
    db: &dyn Db,
) -> Result<(), String> {
    let now = Utc::now().trunc_subsecs(3);
    let mut data = CoremgrOpData {
        data_id: "data_id1".to_string(),
        req_time: now,
        res_time: now + TimeDelta::try_milliseconds(2).unwrap(),
        latency_ms: 2,
        status: 200,
        source_ip: "::1".to_string(),
        method: "GET".to_string(),
        path: "/path".to_string(),
        body: None,
        user_id: "user_id1".to_string(),
        client_id: "client_id1".to_string(),
        err_code: None,
        err_message: None,
    };
    let cond = QueryCond {
        client_id: Some("client_id1"),
        ..Default::default()
    };
    if let Err(e) = runtime.block_on(async {
        model.add(&data).await?;
        data.data_id = "data_id2".to_string();
        model.add(&data).await?;
        data.data_id = "data_id3".to_string();
        data.client_id = "client_id2".to_string();
        model.add(&data).await?;
        model.del(&cond).await
    }) {
        return Err(format!("model.add/del error: {}", e));
    }
    match runtime.block_on(async { db.get("data_id1").await }) {
        Err(e) => return Err(format!("model.get() 1 error: {}", e)),
        Ok(data) => match data {
            None => (),
            Some(_) => return Err("delete 1 fail".to_string()),
        },
    }
    match runtime.block_on(async { db.get("data_id2").await }) {
        Err(e) => return Err(format!("model.get() 2 error: {}", e)),
        Ok(data) => match data {
            None => (),
            Some(_) => return Err("delete 2 fail".to_string()),
        },
    }
    match runtime.block_on(async { db.get("data_id3").await }) {
        Err(e) => Err(format!("model.get() 3 error: {}", e)),
        Ok(data) => match data {
            None => Err("delete wrong one".to_string()),
            Some(_) => Ok(()),
        },
    }
}

/// Test `del()` by specifying request time.
pub fn del_by_req(
    runtime: &Runtime,
    model: &dyn CoremgrOpDataModel,
    db: &dyn Db,
) -> Result<(), String> {
    let now = Utc::now().trunc_subsecs(3);
    let mut data = CoremgrOpData {
        data_id: "data_id1".to_string(),
        req_time: now,
        res_time: now + TimeDelta::try_milliseconds(2).unwrap(),
        latency_ms: 2,
        status: 200,
        source_ip: "::1".to_string(),
        method: "GET".to_string(),
        path: "/path".to_string(),
        body: None,
        user_id: "user_id1".to_string(),
        client_id: "client_id1".to_string(),
        err_code: None,
        err_message: None,
    };
    let cond = QueryCond {
        req_gte: Some(now + TimeDelta::try_milliseconds(1).unwrap()),
        req_lte: Some(now + TimeDelta::try_milliseconds(2).unwrap()),
        ..Default::default()
    };
    if let Err(e) = runtime.block_on(async {
        model.add(&data).await?;
        data.data_id = "data_id2".to_string();
        data.req_time = now + TimeDelta::try_milliseconds(1).unwrap();
        model.add(&data).await?;
        data.data_id = "data_id3".to_string();
        data.req_time = now + TimeDelta::try_milliseconds(2).unwrap();
        model.add(&data).await?;
        data.data_id = "data_id4".to_string();
        data.req_time = now + TimeDelta::try_milliseconds(3).unwrap();
        model.add(&data).await?;
        model.del(&cond).await
    }) {
        return Err(format!("model.add/del error: {}", e));
    }
    match runtime.block_on(async { db.get("data_id1").await }) {
        Err(e) => return Err(format!("model.get() 1 error: {}", e)),
        Ok(data) => match data {
            None => return Err("delete wrong one 1".to_string()),
            Some(_) => (),
        },
    }
    match runtime.block_on(async { db.get("data_id2").await }) {
        Err(e) => return Err(format!("model.get() 2 error: {}", e)),
        Ok(data) => match data {
            None => (),
            Some(_) => return Err("delete 2 fail".to_string()),
        },
    }
    match runtime.block_on(async { db.get("data_id3").await }) {
        Err(e) => return Err(format!("model.get() 3 error: {}", e)),
        Ok(data) => match data {
            None => (),
            Some(_) => return Err("delete 3 fail".to_string()),
        },
    }
    match runtime.block_on(async { db.get("data_id4").await }) {
        Err(e) => Err(format!("model.get() 4 error: {}", e)),
        Ok(data) => match data {
            None => Err("delete wrong one 4".to_string()),
            Some(_) => Ok(()),
        },
    }
}

/// Test `count()`.
pub fn count(runtime: &Runtime, model: &dyn CoremgrOpDataModel) -> Result<(), String> {
    let now = Utc::now().trunc_subsecs(3);
    let mut data = CoremgrOpData {
        data_id: "data_id1".to_string(),
        req_time: now,
        res_time: now + TimeDelta::try_milliseconds(10).unwrap(),
        latency_ms: 10,
        status: 200,
        source_ip: "::1".to_string(),
        method: "GET".to_string(),
        path: "/path".to_string(),
        body: None,
        user_id: "user_id1".to_string(),
        client_id: "client_id1".to_string(),
        err_code: None,
        err_message: None,
    };
    if let Err(e) = runtime.block_on(async {
        model.add(&data).await?;
        data.data_id = "data_id2".to_string();
        data.req_time = now + TimeDelta::try_milliseconds(1).unwrap();
        data.res_time = now + TimeDelta::try_milliseconds(9).unwrap();
        data.latency_ms = data.res_time.timestamp_millis() - data.req_time.timestamp_millis();
        model.add(&data).await?;
        data.data_id = "data_id3".to_string();
        data.req_time = now + TimeDelta::try_milliseconds(2).unwrap();
        data.res_time = now + TimeDelta::try_milliseconds(8).unwrap();
        data.latency_ms = data.res_time.timestamp_millis() - data.req_time.timestamp_millis();
        data.client_id = "client_id2".to_string();
        model.add(&data).await?;
        data.data_id = "data_id4".to_string();
        data.req_time = now + TimeDelta::try_milliseconds(3).unwrap();
        data.res_time = now + TimeDelta::try_milliseconds(7).unwrap();
        data.latency_ms = data.res_time.timestamp_millis() - data.req_time.timestamp_millis();
        data.user_id = "user_id2".to_string();
        data.body = Some(Map::new());
        model.add(&data).await?;
        data.data_id = "data_id5".to_string();
        data.req_time = now + TimeDelta::try_milliseconds(4).unwrap();
        data.res_time = now + TimeDelta::try_milliseconds(6).unwrap();
        data.latency_ms = data.res_time.timestamp_millis() - data.req_time.timestamp_millis();
        data.user_id = "user_id3".to_string();
        data.client_id = "client_id3".to_string();
        data.status = 400;
        data.body = None;
        data.err_code = Some("err_param".to_string());
        data.err_message = Some("error parameter".to_string());
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
        user_id: Some("user_id1"),
        ..Default::default()
    };
    let count = match runtime.block_on(async { model.count(&cond).await }) {
        Err(e) => return Err(format!("count user_id result error: {}", e)),
        Ok(count) => count,
    };
    expect(count).to_equal(3)?;

    let cond = ListQueryCond {
        client_id: Some("client_id3"),
        ..Default::default()
    };
    let count = match runtime.block_on(async { model.count(&cond).await }) {
        Err(e) => return Err(format!("count client_id result error: {}", e)),
        Ok(count) => count,
    };
    expect(count).to_equal(1)?;

    let cond = ListQueryCond {
        req_gte: Some(now + TimeDelta::try_milliseconds(1).unwrap()),
        req_lte: Some(now + TimeDelta::try_milliseconds(3).unwrap()),
        ..Default::default()
    };
    let count = match runtime.block_on(async { model.count(&cond).await }) {
        Err(e) => return Err(format!("count req_time result error: {}", e)),
        Ok(count) => count,
    };
    expect(count).to_equal(3)?;

    let cond = ListQueryCond {
        res_gte: Some(now + TimeDelta::try_milliseconds(9).unwrap()),
        res_lte: Some(now + TimeDelta::try_milliseconds(11).unwrap()),
        ..Default::default()
    };
    let count = match runtime.block_on(async { model.count(&cond).await }) {
        Err(e) => return Err(format!("count res_time result error: {}", e)),
        Ok(count) => count,
    };
    expect(count).to_equal(2)?;

    let cond = ListQueryCond {
        user_id: Some("user_id1"),
        client_id: Some("client_id3"),
        ..Default::default()
    };
    let count = match runtime.block_on(async { model.count(&cond).await }) {
        Err(e) => return Err(format!("count user_id_client_id result error: {}", e)),
        Ok(count) => count,
    };
    expect(count).to_equal(0)
}

/// Test `list()`.
pub fn list(runtime: &Runtime, model: &dyn CoremgrOpDataModel) -> Result<(), String> {
    let now = Utc::now().trunc_subsecs(3);
    let mut data = CoremgrOpData {
        data_id: "data_id1".to_string(),
        req_time: now,
        res_time: now + TimeDelta::try_milliseconds(10).unwrap(),
        latency_ms: 10,
        status: 200,
        source_ip: "::1".to_string(),
        method: "GET".to_string(),
        path: "/path".to_string(),
        body: None,
        user_id: "user_id1".to_string(),
        client_id: "client_id1".to_string(),
        err_code: None,
        err_message: None,
    };
    if let Err(e) = runtime.block_on(async {
        model.add(&data).await?;
        data.data_id = "data_id2".to_string();
        data.req_time = now + TimeDelta::try_milliseconds(1).unwrap();
        data.res_time = now + TimeDelta::try_milliseconds(9).unwrap();
        data.latency_ms = data.res_time.timestamp_millis() - data.req_time.timestamp_millis();
        model.add(&data).await?;
        data.data_id = "data_id3".to_string();
        data.req_time = now + TimeDelta::try_milliseconds(2).unwrap();
        data.res_time = now + TimeDelta::try_milliseconds(8).unwrap();
        data.latency_ms = data.res_time.timestamp_millis() - data.req_time.timestamp_millis();
        data.client_id = "client_id2".to_string();
        model.add(&data).await?;
        data.data_id = "data_id4".to_string();
        data.req_time = now + TimeDelta::try_milliseconds(3).unwrap();
        data.res_time = now + TimeDelta::try_milliseconds(7).unwrap();
        data.latency_ms = data.res_time.timestamp_millis() - data.req_time.timestamp_millis();
        data.user_id = "user_id2".to_string();
        data.body = Some(Map::new());
        model.add(&data).await?;
        data.data_id = "data_id5".to_string();
        data.req_time = now + TimeDelta::try_milliseconds(4).unwrap();
        data.res_time = now + TimeDelta::try_milliseconds(6).unwrap();
        data.latency_ms = data.res_time.timestamp_millis() - data.req_time.timestamp_millis();
        data.user_id = "user_id3".to_string();
        data.client_id = "client_id3".to_string();
        data.status = 400;
        data.body = None;
        data.err_code = Some("err_param".to_string());
        data.err_message = Some("error parameter".to_string());
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
        user_id: Some("user_id1"),
        ..Default::default()
    };
    opts.cond = &cond;
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list user_id result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(3)?;

    let cond = ListQueryCond {
        client_id: Some("client_id3"),
        ..Default::default()
    };
    opts.cond = &cond;
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list client_id result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(1)?;

    let cond = ListQueryCond {
        req_gte: Some(now + TimeDelta::try_milliseconds(1).unwrap()),
        req_lte: Some(now + TimeDelta::try_milliseconds(3).unwrap()),
        ..Default::default()
    };
    opts.cond = &cond;
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list req_time result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(3)?;

    let cond = ListQueryCond {
        res_gte: Some(now + TimeDelta::try_milliseconds(9).unwrap()),
        res_lte: Some(now + TimeDelta::try_milliseconds(11).unwrap()),
        ..Default::default()
    };
    opts.cond = &cond;
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list res_time result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(2)?;

    let cond = ListQueryCond {
        user_id: Some("user_id1"),
        client_id: Some("client_id3"),
        ..Default::default()
    };
    opts.cond = &cond;
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list user_id_client_id result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(0)
}

/// Test `list()` with sorting.
pub fn list_sort(runtime: &Runtime, model: &dyn CoremgrOpDataModel) -> Result<(), String> {
    let now = Utc::now().trunc_subsecs(3);
    let mut data = CoremgrOpData {
        data_id: "data_id1".to_string(),
        req_time: now,
        res_time: now + TimeDelta::try_milliseconds(10).unwrap(),
        latency_ms: 10,
        status: 200,
        source_ip: "::1".to_string(),
        method: "GET".to_string(),
        path: "/path".to_string(),
        body: None,
        user_id: "user_id1".to_string(),
        client_id: "client_id1".to_string(),
        err_code: None,
        err_message: None,
    };
    if let Err(e) = runtime.block_on(async {
        model.add(&data).await?;
        data.data_id = "data_id2".to_string();
        data.req_time = now + TimeDelta::try_milliseconds(1).unwrap();
        data.res_time = now + TimeDelta::try_milliseconds(9).unwrap();
        data.latency_ms = data.res_time.timestamp_millis() - data.req_time.timestamp_millis();
        model.add(&data).await?;
        data.data_id = "data_id3".to_string();
        data.req_time = now + TimeDelta::try_milliseconds(2).unwrap();
        data.res_time = now + TimeDelta::try_milliseconds(8).unwrap();
        data.latency_ms = data.res_time.timestamp_millis() - data.req_time.timestamp_millis();
        data.client_id = "client_id2".to_string();
        model.add(&data).await?;
        data.data_id = "data_id4".to_string();
        data.req_time = now + TimeDelta::try_milliseconds(3).unwrap();
        data.res_time = now + TimeDelta::try_milliseconds(7).unwrap();
        data.latency_ms = data.res_time.timestamp_millis() - data.req_time.timestamp_millis();
        data.user_id = "user_id2".to_string();
        data.body = Some(Map::new());
        model.add(&data).await?;
        data.data_id = "data_id5".to_string();
        data.req_time = now + TimeDelta::try_milliseconds(4).unwrap();
        data.res_time = now + TimeDelta::try_milliseconds(6).unwrap();
        data.latency_ms = data.res_time.timestamp_millis() - data.req_time.timestamp_millis();
        data.user_id = "user_id3".to_string();
        data.client_id = "client_id3".to_string();
        data.status = 400;
        data.body = None;
        data.err_code = Some("err_param".to_string());
        data.err_message = Some("error parameter".to_string());
        model.add(&data).await
    }) {
        return Err(format!("model.add() error: {}", e));
    }

    let cond = ListQueryCond {
        ..Default::default()
    };
    let sort_cond = vec![SortCond {
        key: SortKey::ReqTime,
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
        Err(e) => return Err(format!("list req-asc result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(5)?;
    expect(list[0].data_id.as_str()).to_equal("data_id1")?;
    expect(list[1].data_id.as_str()).to_equal("data_id2")?;
    expect(list[2].data_id.as_str()).to_equal("data_id3")?;
    expect(list[3].data_id.as_str()).to_equal("data_id4")?;
    expect(list[4].data_id.as_str()).to_equal("data_id5")?;

    let sort_cond = vec![SortCond {
        key: SortKey::ReqTime,
        asc: false,
    }];
    opts.sort = Some(sort_cond.as_slice());
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list req-desc result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(5)?;
    expect(list[0].data_id.as_str()).to_equal("data_id5")?;
    expect(list[1].data_id.as_str()).to_equal("data_id4")?;
    expect(list[2].data_id.as_str()).to_equal("data_id3")?;
    expect(list[3].data_id.as_str()).to_equal("data_id2")?;
    expect(list[4].data_id.as_str()).to_equal("data_id1")?;

    let sort_cond = vec![SortCond {
        key: SortKey::ResTime,
        asc: true,
    }];
    opts.sort = Some(sort_cond.as_slice());
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list res-asc result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(5)?;
    expect(list[0].data_id.as_str()).to_equal("data_id5")?;
    expect(list[1].data_id.as_str()).to_equal("data_id4")?;
    expect(list[2].data_id.as_str()).to_equal("data_id3")?;
    expect(list[3].data_id.as_str()).to_equal("data_id2")?;
    expect(list[4].data_id.as_str()).to_equal("data_id1")?;

    let sort_cond = vec![SortCond {
        key: SortKey::ResTime,
        asc: false,
    }];
    opts.sort = Some(sort_cond.as_slice());
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list res-desc result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(5)?;
    expect(list[0].data_id.as_str()).to_equal("data_id1")?;
    expect(list[1].data_id.as_str()).to_equal("data_id2")?;
    expect(list[2].data_id.as_str()).to_equal("data_id3")?;
    expect(list[3].data_id.as_str()).to_equal("data_id4")?;
    expect(list[4].data_id.as_str()).to_equal("data_id5")?;

    let sort_cond = vec![SortCond {
        key: SortKey::Latency,
        asc: true,
    }];
    opts.sort = Some(sort_cond.as_slice());
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list latency-asc result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(5)?;
    expect(list[0].data_id.as_str()).to_equal("data_id5")?;
    expect(list[1].data_id.as_str()).to_equal("data_id4")?;
    expect(list[2].data_id.as_str()).to_equal("data_id3")?;
    expect(list[3].data_id.as_str()).to_equal("data_id2")?;
    expect(list[4].data_id.as_str()).to_equal("data_id1")?;

    let sort_cond = vec![SortCond {
        key: SortKey::Latency,
        asc: false,
    }];
    opts.sort = Some(sort_cond.as_slice());
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list latency-desc result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(5)?;
    expect(list[0].data_id.as_str()).to_equal("data_id1")?;
    expect(list[1].data_id.as_str()).to_equal("data_id2")?;
    expect(list[2].data_id.as_str()).to_equal("data_id3")?;
    expect(list[3].data_id.as_str()).to_equal("data_id4")?;
    expect(list[4].data_id.as_str()).to_equal("data_id5")?;

    Ok(())
}

/// Test `list()` with offset/limit.
pub fn list_offset_limit(runtime: &Runtime, model: &dyn CoremgrOpDataModel) -> Result<(), String> {
    let now = Utc::now().trunc_subsecs(3);
    let mut data = CoremgrOpData {
        data_id: "data_id1".to_string(),
        req_time: now,
        res_time: now + TimeDelta::try_milliseconds(10).unwrap(),
        latency_ms: 10,
        status: 200,
        source_ip: "::1".to_string(),
        method: "GET".to_string(),
        path: "/path".to_string(),
        body: None,
        user_id: "user_id1".to_string(),
        client_id: "client_id1".to_string(),
        err_code: None,
        err_message: None,
    };
    if let Err(e) = runtime.block_on(async {
        model.add(&data).await?;
        data.data_id = "data_id2".to_string();
        data.req_time = now + TimeDelta::try_milliseconds(1).unwrap();
        data.res_time = now + TimeDelta::try_milliseconds(9).unwrap();
        data.latency_ms = data.res_time.timestamp_millis() - data.req_time.timestamp_millis();
        model.add(&data).await?;
        data.data_id = "data_id3".to_string();
        data.req_time = now + TimeDelta::try_milliseconds(2).unwrap();
        data.res_time = now + TimeDelta::try_milliseconds(8).unwrap();
        data.latency_ms = data.res_time.timestamp_millis() - data.req_time.timestamp_millis();
        data.client_id = "client_id2".to_string();
        model.add(&data).await?;
        data.data_id = "data_id4".to_string();
        data.req_time = now + TimeDelta::try_milliseconds(3).unwrap();
        data.res_time = now + TimeDelta::try_milliseconds(7).unwrap();
        data.latency_ms = data.res_time.timestamp_millis() - data.req_time.timestamp_millis();
        data.user_id = "user_id2".to_string();
        data.body = Some(Map::new());
        model.add(&data).await?;
        data.data_id = "data_id5".to_string();
        data.req_time = now + TimeDelta::try_milliseconds(4).unwrap();
        data.res_time = now + TimeDelta::try_milliseconds(6).unwrap();
        data.latency_ms = data.res_time.timestamp_millis() - data.req_time.timestamp_millis();
        data.user_id = "user_id3".to_string();
        data.client_id = "client_id3".to_string();
        data.status = 400;
        data.body = None;
        data.err_code = Some("err_param".to_string());
        data.err_message = Some("error parameter".to_string());
        model.add(&data).await
    }) {
        return Err(format!("model.add() error: {}", e));
    }

    let cond = ListQueryCond {
        ..Default::default()
    };
    let sort_cond = vec![SortCond {
        key: SortKey::ReqTime,
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
pub fn list_cursor(runtime: &Runtime, model: &dyn CoremgrOpDataModel) -> Result<(), String> {
    let now = Utc::now().trunc_subsecs(3);
    let mut data = CoremgrOpData {
        data_id: "data_id1".to_string(),
        req_time: now,
        res_time: now + TimeDelta::try_milliseconds(10).unwrap(),
        latency_ms: 10,
        status: 200,
        source_ip: "::1".to_string(),
        method: "GET".to_string(),
        path: "/path".to_string(),
        body: None,
        user_id: "user_id1".to_string(),
        client_id: "client_id1".to_string(),
        err_code: None,
        err_message: None,
    };
    if let Err(e) = runtime.block_on(async {
        model.add(&data).await?;
        data.data_id = "data_id2".to_string();
        data.req_time = now + TimeDelta::try_milliseconds(1).unwrap();
        data.res_time = now + TimeDelta::try_milliseconds(9).unwrap();
        data.latency_ms = data.res_time.timestamp_millis() - data.req_time.timestamp_millis();
        model.add(&data).await?;
        data.data_id = "data_id3".to_string();
        data.req_time = now + TimeDelta::try_milliseconds(2).unwrap();
        data.res_time = now + TimeDelta::try_milliseconds(8).unwrap();
        data.latency_ms = data.res_time.timestamp_millis() - data.req_time.timestamp_millis();
        data.client_id = "client_id2".to_string();
        model.add(&data).await?;
        data.data_id = "data_id4".to_string();
        data.req_time = now + TimeDelta::try_milliseconds(3).unwrap();
        data.res_time = now + TimeDelta::try_milliseconds(7).unwrap();
        data.latency_ms = data.res_time.timestamp_millis() - data.req_time.timestamp_millis();
        data.user_id = "user_id2".to_string();
        data.body = Some(Map::new());
        model.add(&data).await?;
        data.data_id = "data_id5".to_string();
        data.req_time = now + TimeDelta::try_milliseconds(4).unwrap();
        data.res_time = now + TimeDelta::try_milliseconds(6).unwrap();
        data.latency_ms = data.res_time.timestamp_millis() - data.req_time.timestamp_millis();
        data.user_id = "user_id3".to_string();
        data.client_id = "client_id3".to_string();
        data.status = 400;
        data.body = None;
        data.err_code = Some("err_param".to_string());
        data.err_message = Some("error parameter".to_string());
        model.add(&data).await
    }) {
        return Err(format!("model.add() error: {}", e));
    }

    let cond = ListQueryCond {
        ..Default::default()
    };
    let sort_cond = vec![SortCond {
        key: SortKey::ReqTime,
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
