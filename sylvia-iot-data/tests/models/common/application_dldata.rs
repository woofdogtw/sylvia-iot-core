use std::error::Error as StdError;

use async_trait::async_trait;
use chrono::{SubsecRound, TimeDelta, Utc};
use laboratory::expect;
use serde_json::{Map, Value};
use tokio::runtime::Runtime;

use sylvia_iot_corelib::strings;
use sylvia_iot_data::models::application_dldata::{
    ApplicationDlData, ApplicationDlDataModel, ListOptions, ListQueryCond, QueryCond, SortCond,
    SortKey, UpdateQueryCond, Updates,
};

#[async_trait]
/// Database operations.
pub trait Db {
    /// To get the data which ID is `data_id`.
    async fn get(&self, data_id: &str) -> Result<Option<ApplicationDlData>, Box<dyn StdError>>;
}

/// Test `add()`.
pub fn add(
    runtime: &Runtime,
    model: &dyn ApplicationDlDataModel,
    db: &dyn Db,
) -> Result<(), String> {
    let now = Utc::now().trunc_subsecs(3);
    let mut extension = Map::<String, Value>::new();
    extension.insert("key".to_string(), Value::String("value".to_string()));
    let data = ApplicationDlData {
        data_id: strings::random_id(&now, 8),
        proc: now,
        resp: None,
        status: 1,
        unit_id: "unit_id1".to_string(),
        device_id: Some("device_id1".to_string()),
        network_code: None,
        network_addr: None,
        profile: "profile1".to_string(),
        data: "data1".to_string(),
        extension: Some(extension),
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
    let data = ApplicationDlData {
        data_id: strings::random_id(&now, 8),
        proc: now + TimeDelta::try_milliseconds(1).unwrap(),
        resp: Some(now),
        status: -1,
        unit_id: "unit_id2".to_string(),
        device_id: None,
        network_code: Some("network_code2".to_string()),
        network_addr: Some("network_addr2".to_string()),
        profile: "profile2".to_string(),
        data: "data2".to_string(),
        extension: None,
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
pub fn add_dup(runtime: &Runtime, model: &dyn ApplicationDlDataModel) -> Result<(), String> {
    let now = Utc::now().trunc_subsecs(3);
    let data = ApplicationDlData {
        data_id: strings::random_id(&now, 8),
        proc: now,
        resp: None,
        status: 1,
        unit_id: "unit_id".to_string(),
        device_id: Some("device_id".to_string()),
        network_code: None,
        network_addr: None,
        profile: "profile".to_string(),
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
pub fn del_by_unit(
    runtime: &Runtime,
    model: &dyn ApplicationDlDataModel,
    db: &dyn Db,
) -> Result<(), String> {
    let now = Utc::now().trunc_subsecs(3);
    let mut data = ApplicationDlData {
        data_id: "data_id1_1".to_string(),
        proc: now,
        resp: None,
        status: 1,
        unit_id: "unit_id1".to_string(),
        device_id: Some("device_id1_1".to_string()),
        network_code: None,
        network_addr: None,
        profile: "profile".to_string(),
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
        data.unit_id = "unit_id2".to_string();
        data.data_id = "data_id2".to_string();
        data.device_id = Some("device_id2".to_string());
        model.add(&data).await?;
        model.del(&cond).await
    }) {
        return Err(format!("model.add/del error: {}", e));
    }
    match runtime.block_on(async { db.get("data_id1_1").await }) {
        Err(e) => return Err(format!("model.get() 1_1 error: {}", e)),
        Ok(data) => match data {
            None => (),
            Some(_) => return Err("delete 1_1 fail".to_string()),
        },
    }
    match runtime.block_on(async { db.get("data_id1_2").await }) {
        Err(e) => return Err(format!("model.get() 1_2 error: {}", e)),
        Ok(data) => match data {
            None => (),
            Some(_) => return Err("delete 1_2 fail".to_string()),
        },
    }
    match runtime.block_on(async { db.get("data_id2").await }) {
        Err(e) => Err(format!("model.get() 2 error: {}", e)),
        Ok(data) => match data {
            None => Err("delete wrong one".to_string()),
            Some(_) => Ok(()),
        },
    }
}

/// Test `del()` twice.
pub fn del_twice(runtime: &Runtime, model: &dyn ApplicationDlDataModel) -> Result<(), String> {
    let now = Utc::now().trunc_subsecs(3);
    let data = ApplicationDlData {
        data_id: "data_id1_1".to_string(),
        proc: now,
        resp: None,
        status: 1,
        unit_id: "unit_id1".to_string(),
        device_id: Some("device_id1_1".to_string()),
        network_code: None,
        network_addr: None,
        profile: "profile".to_string(),
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
pub fn del_by_device_id(
    runtime: &Runtime,
    model: &dyn ApplicationDlDataModel,
    db: &dyn Db,
) -> Result<(), String> {
    let now = Utc::now().trunc_subsecs(3);
    let mut data = ApplicationDlData {
        data_id: "data_id1_1".to_string(),
        proc: now,
        resp: None,
        status: 1,
        unit_id: "unit_id".to_string(),
        device_id: Some("device_id1".to_string()),
        network_code: None,
        network_addr: None,
        profile: "profile".to_string(),
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
    match runtime.block_on(async { db.get("data_id1_1").await }) {
        Err(e) => return Err(format!("model.get() 1_1 error: {}", e)),
        Ok(data) => match data {
            None => (),
            Some(_) => return Err("delete 1_1 fail".to_string()),
        },
    }
    match runtime.block_on(async { db.get("data_id1_2").await }) {
        Err(e) => return Err(format!("model.get() 1_2 error: {}", e)),
        Ok(data) => match data {
            None => (),
            Some(_) => return Err("delete 1_2 fail".to_string()),
        },
    }
    match runtime.block_on(async { db.get("data_id2").await }) {
        Err(e) => Err(format!("model.get() 2 error: {}", e)),
        Ok(data) => match data {
            None => Err("delete wrong one".to_string()),
            Some(_) => Ok(()),
        },
    }
}

/// Test `del()` by specifying network address.
pub fn del_by_network_addr(
    runtime: &Runtime,
    model: &dyn ApplicationDlDataModel,
    db: &dyn Db,
) -> Result<(), String> {
    let now = Utc::now().trunc_subsecs(3);
    let mut data = ApplicationDlData {
        data_id: "data_id1_1".to_string(),
        proc: now,
        resp: None,
        status: 1,
        unit_id: "unit_id".to_string(),
        device_id: None,
        network_code: Some("network_code".to_string()),
        network_addr: Some("network_addr1".to_string()),
        profile: "profile".to_string(),
        data: "data".to_string(),
        extension: None,
    };
    let cond = QueryCond {
        network_code: Some("network_code"),
        network_addr: Some("network_addr1"),
        ..Default::default()
    };
    if let Err(e) = runtime.block_on(async {
        model.add(&data).await?;
        data.data_id = "data_id1_2".to_string();
        model.add(&data).await?;
        data.data_id = "data_id2".to_string();
        data.network_addr = Some("network_addr2".to_string());
        model.add(&data).await?;
        model.del(&cond).await
    }) {
        return Err(format!("model.add/del error: {}", e));
    }
    match runtime.block_on(async { db.get("data_id1_1").await }) {
        Err(e) => return Err(format!("model.get() 1_1 error: {}", e)),
        Ok(data) => match data {
            None => (),
            Some(_) => return Err("delete 1_1 fail".to_string()),
        },
    }
    match runtime.block_on(async { db.get("data_id1_2").await }) {
        Err(e) => return Err(format!("model.get() 1_2 error: {}", e)),
        Ok(data) => match data {
            None => (),
            Some(_) => return Err("delete 1_2 fail".to_string()),
        },
    }
    match runtime.block_on(async { db.get("data_id2").await }) {
        Err(e) => Err(format!("model.get() 2 error: {}", e)),
        Ok(data) => match data {
            None => Err("delete wrong one".to_string()),
            Some(_) => Ok(()),
        },
    }
}

/// Test `del()` by specifying processed time.
pub fn del_by_proc(
    runtime: &Runtime,
    model: &dyn ApplicationDlDataModel,
    db: &dyn Db,
) -> Result<(), String> {
    let now = Utc::now().trunc_subsecs(3);
    let mut data = ApplicationDlData {
        data_id: "data_id1".to_string(),
        proc: now,
        resp: None,
        status: 1,
        unit_id: "unit_id".to_string(),
        device_id: None,
        network_code: Some("network_code".to_string()),
        network_addr: Some("network_addr".to_string()),
        profile: "profile".to_string(),
        data: "data".to_string(),
        extension: None,
    };
    let cond = QueryCond {
        proc_gte: Some(now + TimeDelta::try_milliseconds(1).unwrap()),
        proc_lte: Some(now + TimeDelta::try_milliseconds(2).unwrap()),
        ..Default::default()
    };
    if let Err(e) = runtime.block_on(async {
        model.add(&data).await?;
        data.data_id = "data_id2".to_string();
        data.proc = now + TimeDelta::try_milliseconds(1).unwrap();
        model.add(&data).await?;
        data.data_id = "data_id3".to_string();
        data.proc = now + TimeDelta::try_milliseconds(2).unwrap();
        model.add(&data).await?;
        data.data_id = "data_id4".to_string();
        data.proc = now + TimeDelta::try_milliseconds(3).unwrap();
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

/// Test `update()` status with zero.
pub fn update_with_zero(
    runtime: &Runtime,
    model: &dyn ApplicationDlDataModel,
    db: &dyn Db,
) -> Result<(), String> {
    let now = Utc::now().trunc_subsecs(3);
    let mut data = ApplicationDlData {
        data_id: "data_id1".to_string(),
        proc: now,
        resp: None,
        status: -3,
        unit_id: "unit_id".to_string(),
        device_id: None,
        network_code: Some("network_code".to_string()),
        network_addr: Some("network_addr".to_string()),
        profile: "profile".to_string(),
        data: "data".to_string(),
        extension: None,
    };
    let mut cond = UpdateQueryCond {
        data_id: "data_id1",
    };
    let updates = Updates {
        resp: now + TimeDelta::try_milliseconds(1).unwrap(),
        status: 0,
    };
    if let Err(e) = runtime.block_on(async {
        model.add(&data).await?;
        data.data_id = "data_id2".to_string();
        data.status = 2;
        model.add(&data).await?;
        data.data_id = "data_id3".to_string();
        data.status = 0;
        model.add(&data).await?;
        model.update(&cond, &updates).await?;
        cond.data_id = "data_id2";
        model.update(&cond, &updates).await?;
        cond.data_id = "data_id3";
        model.update(&cond, &updates).await
    }) {
        return Err(format!("model.add/update error: {}", e));
    }

    match runtime.block_on(async { db.get("data_id1").await }) {
        Err(e) => return Err(format!("model.get() 1 error: {}", e)),
        Ok(new_data) => match new_data {
            None => return Err("delete wrong one 1".to_string()),
            Some(new_data) => {
                expect(new_data.resp).to_equal(Some(updates.resp))?;
                expect(new_data.status).to_equal(0)?;
            }
        },
    }
    match runtime.block_on(async { db.get("data_id2").await }) {
        Err(e) => return Err(format!("model.get() 2 error: {}", e)),
        Ok(data) => match data {
            None => return Err("delete wrong one 2".to_string()),
            Some(data) => {
                expect(data.resp).to_equal(Some(updates.resp))?;
                expect(data.status).to_equal(0)?;
            }
        },
    }
    match runtime.block_on(async { db.get("data_id3").await }) {
        Err(e) => return Err(format!("model.get() 3 error: {}", e)),
        Ok(new_data) => match new_data {
            None => return Err("delete wrong one 3".to_string()),
            Some(new_data) => {
                expect(new_data.resp).to_equal(data.resp)?;
                expect(new_data.status).to_equal(0)?;
            }
        },
    }

    Ok(())
}

/// Test `update()` status with positive.
pub fn update_with_positive(
    runtime: &Runtime,
    model: &dyn ApplicationDlDataModel,
    db: &dyn Db,
) -> Result<(), String> {
    let now = Utc::now().trunc_subsecs(3);
    let mut data = ApplicationDlData {
        data_id: "data_id1".to_string(),
        proc: now,
        resp: None,
        status: -3,
        unit_id: "unit_id".to_string(),
        device_id: None,
        network_code: Some("network_code".to_string()),
        network_addr: Some("network_addr".to_string()),
        profile: "profile".to_string(),
        data: "data".to_string(),
        extension: None,
    };
    let mut cond = UpdateQueryCond {
        data_id: "data_id1",
    };
    let updates = Updates {
        resp: now + TimeDelta::try_milliseconds(1).unwrap(),
        status: 1,
    };
    if let Err(e) = runtime.block_on(async {
        model.add(&data).await?;
        data.data_id = "data_id2".to_string();
        data.status = 2;
        model.add(&data).await?;
        data.data_id = "data_id3".to_string();
        data.status = 0;
        model.add(&data).await?;
        model.update(&cond, &updates).await?;
        cond.data_id = "data_id2";
        model.update(&cond, &updates).await?;
        cond.data_id = "data_id3";
        model.update(&cond, &updates).await
    }) {
        return Err(format!("model.add/update error: {}", e));
    }

    match runtime.block_on(async { db.get("data_id1").await }) {
        Err(e) => return Err(format!("model.get() 1 error: {}", e)),
        Ok(new_data) => match new_data {
            None => return Err("delete wrong one 1".to_string()),
            Some(new_data) => {
                expect(new_data.resp).to_equal(Some(updates.resp))?;
                expect(new_data.status).to_equal(updates.status)?;
            }
        },
    }
    match runtime.block_on(async { db.get("data_id2").await }) {
        Err(e) => return Err(format!("model.get() 2 error: {}", e)),
        Ok(data) => match data {
            None => return Err("delete wrong one 2".to_string()),
            Some(data) => {
                expect(data.resp).to_equal(Some(updates.resp))?;
                expect(data.status).to_equal(updates.status)?;
            }
        },
    }
    match runtime.block_on(async { db.get("data_id3").await }) {
        Err(e) => return Err(format!("model.get() 3 error: {}", e)),
        Ok(new_data) => match new_data {
            None => return Err("delete wrong one 3".to_string()),
            Some(new_data) => {
                expect(new_data.resp).to_equal(data.resp)?;
                expect(new_data.status).to_equal(0)?;
            }
        },
    }

    Ok(())
}

/// Test `update()` status with negative.
pub fn update_with_negative(
    runtime: &Runtime,
    model: &dyn ApplicationDlDataModel,
    db: &dyn Db,
) -> Result<(), String> {
    let now = Utc::now().trunc_subsecs(3);
    let mut data = ApplicationDlData {
        data_id: "data_id1".to_string(),
        proc: now,
        resp: None,
        status: -3,
        unit_id: "unit_id".to_string(),
        device_id: None,
        network_code: Some("network_code".to_string()),
        network_addr: Some("network_addr".to_string()),
        profile: "profile".to_string(),
        data: "data".to_string(),
        extension: None,
    };
    let mut cond = UpdateQueryCond {
        data_id: "data_id1",
    };
    let updates = Updates {
        resp: now + TimeDelta::try_milliseconds(1).unwrap(),
        status: -2,
    };
    if let Err(e) = runtime.block_on(async {
        model.add(&data).await?;
        data.data_id = "data_id2".to_string();
        data.status = 2;
        model.add(&data).await?;
        data.data_id = "data_id3".to_string();
        data.status = 0;
        model.add(&data).await?;
        data.data_id = "data_id4".to_string();
        data.status = -1;
        model.add(&data).await?;
        model.update(&cond, &updates).await?;
        cond.data_id = "data_id2";
        model.update(&cond, &updates).await?;
        cond.data_id = "data_id3";
        model.update(&cond, &updates).await?;
        cond.data_id = "data_id4";
        model.update(&cond, &updates).await
    }) {
        return Err(format!("model.add/update error: {}", e));
    }

    match runtime.block_on(async { db.get("data_id1").await }) {
        Err(e) => return Err(format!("model.get() 1 error: {}", e)),
        Ok(new_data) => match new_data {
            None => return Err("delete wrong one 1".to_string()),
            Some(new_data) => {
                expect(new_data.resp).to_equal(Some(updates.resp))?;
                expect(new_data.status).to_equal(updates.status)?;
            }
        },
    }
    match runtime.block_on(async { db.get("data_id2").await }) {
        Err(e) => return Err(format!("model.get() 2 error: {}", e)),
        Ok(data) => match data {
            None => return Err("delete wrong one 2".to_string()),
            Some(data) => {
                expect(data.resp).to_equal(data.resp)?;
                expect(data.status).to_equal(2)?;
            }
        },
    }
    match runtime.block_on(async { db.get("data_id3").await }) {
        Err(e) => return Err(format!("model.get() 3 error: {}", e)),
        Ok(new_data) => match new_data {
            None => return Err("delete wrong one 3".to_string()),
            Some(new_data) => {
                expect(new_data.resp).to_equal(data.resp)?;
                expect(new_data.status).to_equal(0)?;
            }
        },
    }
    match runtime.block_on(async { db.get("data_id4").await }) {
        Err(e) => return Err(format!("model.get() 4 error: {}", e)),
        Ok(new_data) => match new_data {
            None => return Err("delete wrong one 4".to_string()),
            Some(new_data) => {
                expect(new_data.resp).to_equal(data.resp)?;
                expect(new_data.status).to_equal(-1)?;
            }
        },
    }

    Ok(())
}

/// Test `update()` with a non-exist condition.
pub fn update_not_exist(
    runtime: &Runtime,
    model: &dyn ApplicationDlDataModel,
) -> Result<(), String> {
    let cond = UpdateQueryCond {
        data_id: "data_id_not_exist",
    };
    let updates = Updates {
        resp: Utc::now(),
        status: 0,
    };
    if let Err(e) = runtime.block_on(async { model.update(&cond, &updates).await }) {
        return Err(format!("model.update() error: {}", e));
    }
    Ok(())
}

/// Test `count()`.
pub fn count(runtime: &Runtime, model: &dyn ApplicationDlDataModel) -> Result<(), String> {
    let now = Utc::now().trunc_subsecs(3);
    let mut data = ApplicationDlData {
        data_id: "data_id1".to_string(),
        proc: now,
        resp: None,
        status: -3,
        unit_id: "unit_id1".to_string(),
        device_id: None,
        network_code: Some("network_code1_1".to_string()),
        network_addr: Some("network_addr1_1".to_string()),
        profile: "profile1".to_string(),
        data: "data".to_string(),
        extension: None,
    };
    if let Err(e) = runtime.block_on(async {
        model.add(&data).await?;
        data.data_id = "data_id2".to_string();
        data.proc = now + TimeDelta::try_milliseconds(1).unwrap();
        data.resp = Some(now + TimeDelta::try_milliseconds(4).unwrap());
        model.add(&data).await?;
        data.data_id = "data_id3".to_string();
        data.proc = now + TimeDelta::try_milliseconds(2).unwrap();
        data.resp = Some(now + TimeDelta::try_milliseconds(3).unwrap());
        data.network_addr = Some("network_addr1_2".to_string());
        model.add(&data).await?;
        data.data_id = "data_id4".to_string();
        data.proc = now + TimeDelta::try_milliseconds(3).unwrap();
        data.resp = Some(now + TimeDelta::try_milliseconds(2).unwrap());
        data.network_code = Some("network_code2".to_string());
        data.network_addr = Some("network_addr2".to_string());
        data.profile = "profile2".to_string();
        model.add(&data).await?;
        data.data_id = "data_id5".to_string();
        data.unit_id = "unit_id2".to_string();
        data.proc = now + TimeDelta::try_milliseconds(4).unwrap();
        data.resp = Some(now + TimeDelta::try_milliseconds(1).unwrap());
        data.device_id = Some("device_id3".to_string());
        data.network_code = None;
        data.network_addr = None;
        data.profile = "profile3".to_string();
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
    expect(count).to_equal(4)?;

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
        profile: Some("profile1"),
        ..Default::default()
    };
    let count = match runtime.block_on(async { model.count(&cond).await }) {
        Err(e) => return Err(format!("count profile result error: {}", e)),
        Ok(count) => count,
    };
    expect(count).to_equal(3)?;

    let cond = ListQueryCond {
        proc_gte: Some(now + TimeDelta::try_milliseconds(1).unwrap()),
        proc_lte: Some(now + TimeDelta::try_milliseconds(3).unwrap()),
        ..Default::default()
    };
    let count = match runtime.block_on(async { model.count(&cond).await }) {
        Err(e) => return Err(format!("count proc result error: {}", e)),
        Ok(count) => count,
    };
    expect(count).to_equal(3)?;

    let cond = ListQueryCond {
        resp_gte: Some(now + TimeDelta::try_milliseconds(2).unwrap()),
        resp_lte: Some(now + TimeDelta::try_milliseconds(3).unwrap()),
        ..Default::default()
    };
    let count = match runtime.block_on(async { model.count(&cond).await }) {
        Err(e) => return Err(format!("count resp result error: {}", e)),
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
pub fn list(runtime: &Runtime, model: &dyn ApplicationDlDataModel) -> Result<(), String> {
    let now = Utc::now().trunc_subsecs(3);
    let mut data = ApplicationDlData {
        data_id: "data_id1".to_string(),
        proc: now,
        resp: None,
        status: -3,
        unit_id: "unit_id1".to_string(),
        device_id: None,
        network_code: Some("network_code1_1".to_string()),
        network_addr: Some("network_addr1_1".to_string()),
        profile: "profile1".to_string(),
        data: "data".to_string(),
        extension: None,
    };
    if let Err(e) = runtime.block_on(async {
        model.add(&data).await?;
        data.data_id = "data_id2".to_string();
        data.proc = now + TimeDelta::try_milliseconds(1).unwrap();
        data.resp = Some(now + TimeDelta::try_milliseconds(4).unwrap());
        model.add(&data).await?;
        data.data_id = "data_id3".to_string();
        data.proc = now + TimeDelta::try_milliseconds(2).unwrap();
        data.resp = Some(now + TimeDelta::try_milliseconds(3).unwrap());
        data.network_addr = Some("network_addr1_2".to_string());
        model.add(&data).await?;
        data.data_id = "data_id4".to_string();
        data.proc = now + TimeDelta::try_milliseconds(3).unwrap();
        data.resp = Some(now + TimeDelta::try_milliseconds(2).unwrap());
        data.network_code = Some("network_code2".to_string());
        data.network_addr = Some("network_addr2".to_string());
        data.profile = "profile2".to_string();
        model.add(&data).await?;
        data.data_id = "data_id5".to_string();
        data.unit_id = "unit_id2".to_string();
        data.proc = now + TimeDelta::try_milliseconds(4).unwrap();
        data.resp = Some(now + TimeDelta::try_milliseconds(1).unwrap());
        data.device_id = Some("device_id3".to_string());
        data.network_code = None;
        data.network_addr = None;
        data.profile = "profile3".to_string();
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
    expect(list.len()).to_equal(4)?;

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
        profile: Some("profile1"),
        ..Default::default()
    };
    opts.cond = &cond;
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list profile result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(3)?;

    let cond = ListQueryCond {
        proc_gte: Some(now + TimeDelta::try_milliseconds(1).unwrap()),
        proc_lte: Some(now + TimeDelta::try_milliseconds(3).unwrap()),
        ..Default::default()
    };
    opts.cond = &cond;
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list proc result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(3)?;

    let cond = ListQueryCond {
        resp_gte: Some(now + TimeDelta::try_milliseconds(2).unwrap()),
        resp_lte: Some(now + TimeDelta::try_milliseconds(3).unwrap()),
        ..Default::default()
    };
    opts.cond = &cond;
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list resp result error: {}", e)),
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
pub fn list_sort(runtime: &Runtime, model: &dyn ApplicationDlDataModel) -> Result<(), String> {
    let now = Utc::now().trunc_subsecs(3);
    let mut data = ApplicationDlData {
        data_id: "data_id1".to_string(),
        proc: now,
        resp: None,
        status: -3,
        unit_id: "unit_id1".to_string(),
        device_id: None,
        network_code: Some("network_code1_1".to_string()),
        network_addr: Some("network_addr1_1".to_string()),
        profile: "profile".to_string(),
        data: "data".to_string(),
        extension: None,
    };
    if let Err(e) = runtime.block_on(async {
        model.add(&data).await?;
        data.data_id = "data_id2".to_string();
        data.proc = now + TimeDelta::try_milliseconds(1).unwrap();
        data.resp = Some(now + TimeDelta::try_milliseconds(4).unwrap());
        model.add(&data).await?;
        data.data_id = "data_id3".to_string();
        data.proc = now + TimeDelta::try_milliseconds(2).unwrap();
        data.resp = Some(now + TimeDelta::try_milliseconds(3).unwrap());
        data.network_addr = Some("network_addr1_2".to_string());
        model.add(&data).await?;
        data.data_id = "data_id4".to_string();
        data.proc = now + TimeDelta::try_milliseconds(3).unwrap();
        data.resp = Some(now + TimeDelta::try_milliseconds(2).unwrap());
        data.network_code = Some("network_code2".to_string());
        data.network_addr = Some("network_addr2".to_string());
        model.add(&data).await?;
        data.data_id = "data_id5".to_string();
        data.unit_id = "unit_id2".to_string();
        data.proc = now + TimeDelta::try_milliseconds(4).unwrap();
        data.resp = Some(now + TimeDelta::try_milliseconds(1).unwrap());
        data.device_id = Some("device_id3".to_string());
        data.network_code = None;
        data.network_addr = None;
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
        key: SortKey::Resp,
        asc: true,
    }];
    opts.sort = Some(sort_cond.as_slice());
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list resp-asc result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(5)?;
    expect(list[0].data_id.as_str()).to_equal("data_id1")?;
    expect(list[1].data_id.as_str()).to_equal("data_id5")?;
    expect(list[2].data_id.as_str()).to_equal("data_id4")?;
    expect(list[3].data_id.as_str()).to_equal("data_id3")?;
    expect(list[4].data_id.as_str()).to_equal("data_id2")?;

    let sort_cond = vec![SortCond {
        key: SortKey::Resp,
        asc: false,
    }];
    opts.sort = Some(sort_cond.as_slice());
    let list = match runtime.block_on(async { model.list(&opts, None).await }) {
        Err(e) => return Err(format!("list resp-desc result error: {}", e)),
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(5)?;
    expect(list[0].data_id.as_str()).to_equal("data_id2")?;
    expect(list[1].data_id.as_str()).to_equal("data_id3")?;
    expect(list[2].data_id.as_str()).to_equal("data_id4")?;
    expect(list[3].data_id.as_str()).to_equal("data_id5")?;
    expect(list[4].data_id.as_str()).to_equal("data_id1")?;

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
            ));
        }
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(5)?;
    expect(list[0].data_id.as_str()).to_equal("data_id5")?;
    expect(list[1].data_id.as_str()).to_equal("data_id1")?;
    expect(list[2].data_id.as_str()).to_equal("data_id2")?;
    expect(list[3].data_id.as_str()).to_equal("data_id3")?;
    expect(list[4].data_id.as_str()).to_equal("data_id4")?;

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
            ));
        }
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(5)?;
    expect(list[0].data_id.as_str()).to_equal("data_id5")?;
    expect(list[1].data_id.as_str()).to_equal("data_id2")?;
    expect(list[2].data_id.as_str()).to_equal("data_id1")?;
    expect(list[3].data_id.as_str()).to_equal("data_id3")?;
    expect(list[4].data_id.as_str()).to_equal("data_id4")?;

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
            ));
        }
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(5)?;
    expect(list[0].data_id.as_str()).to_equal("data_id4")?;
    expect(list[1].data_id.as_str()).to_equal("data_id1")?;
    expect(list[2].data_id.as_str()).to_equal("data_id2")?;
    expect(list[3].data_id.as_str()).to_equal("data_id3")?;
    expect(list[4].data_id.as_str()).to_equal("data_id5")?;

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
            ));
        }
        Ok((list, _)) => list,
    };
    expect(list.len()).to_equal(5)?;
    expect(list[0].data_id.as_str()).to_equal("data_id4")?;
    expect(list[1].data_id.as_str()).to_equal("data_id3")?;
    expect(list[2].data_id.as_str()).to_equal("data_id1")?;
    expect(list[3].data_id.as_str()).to_equal("data_id2")?;
    expect(list[4].data_id.as_str()).to_equal("data_id5")?;

    Ok(())
}

/// Test `list()` with offset/limit.
pub fn list_offset_limit(
    runtime: &Runtime,
    model: &dyn ApplicationDlDataModel,
) -> Result<(), String> {
    let now = Utc::now().trunc_subsecs(3);
    let mut data = ApplicationDlData {
        data_id: "data_id1".to_string(),
        proc: now,
        resp: None,
        status: -3,
        unit_id: "unit_id1".to_string(),
        device_id: None,
        network_code: Some("network_code1_1".to_string()),
        network_addr: Some("network_addr1_1".to_string()),
        profile: "profile".to_string(),
        data: "data".to_string(),
        extension: None,
    };
    if let Err(e) = runtime.block_on(async {
        model.add(&data).await?;
        data.data_id = "data_id2".to_string();
        data.proc = now + TimeDelta::try_milliseconds(1).unwrap();
        data.resp = Some(now + TimeDelta::try_milliseconds(4).unwrap());
        model.add(&data).await?;
        data.data_id = "data_id3".to_string();
        data.proc = now + TimeDelta::try_milliseconds(2).unwrap();
        data.resp = Some(now + TimeDelta::try_milliseconds(3).unwrap());
        data.network_addr = Some("network_addr1_2".to_string());
        model.add(&data).await?;
        data.data_id = "data_id4".to_string();
        data.proc = now + TimeDelta::try_milliseconds(3).unwrap();
        data.resp = Some(now + TimeDelta::try_milliseconds(2).unwrap());
        data.network_code = Some("network_code2".to_string());
        data.network_addr = Some("network_addr2".to_string());
        model.add(&data).await?;
        data.data_id = "data_id5".to_string();
        data.unit_id = "unit_id2".to_string();
        data.proc = now + TimeDelta::try_milliseconds(4).unwrap();
        data.resp = Some(now + TimeDelta::try_milliseconds(1).unwrap());
        data.device_id = Some("device_id3".to_string());
        data.network_code = None;
        data.network_addr = None;
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
pub fn list_cursor(runtime: &Runtime, model: &dyn ApplicationDlDataModel) -> Result<(), String> {
    let now = Utc::now().trunc_subsecs(3);
    let mut data = ApplicationDlData {
        data_id: "data_id1".to_string(),
        proc: now,
        resp: None,
        status: -3,
        unit_id: "unit_id1".to_string(),
        device_id: None,
        network_code: Some("network_code1_1".to_string()),
        network_addr: Some("network_addr1_1".to_string()),
        profile: "profile".to_string(),
        data: "data".to_string(),
        extension: None,
    };
    if let Err(e) = runtime.block_on(async {
        model.add(&data).await?;
        data.data_id = "data_id2".to_string();
        data.proc = now + TimeDelta::try_milliseconds(1).unwrap();
        data.resp = Some(now + TimeDelta::try_milliseconds(4).unwrap());
        model.add(&data).await?;
        data.data_id = "data_id3".to_string();
        data.proc = now + TimeDelta::try_milliseconds(2).unwrap();
        data.resp = Some(now + TimeDelta::try_milliseconds(3).unwrap());
        data.network_addr = Some("network_addr1_2".to_string());
        model.add(&data).await?;
        data.data_id = "data_id4".to_string();
        data.proc = now + TimeDelta::try_milliseconds(3).unwrap();
        data.resp = Some(now + TimeDelta::try_milliseconds(2).unwrap());
        data.network_code = Some("network_code2".to_string());
        data.network_addr = Some("network_addr2".to_string());
        model.add(&data).await?;
        data.data_id = "data_id5".to_string();
        data.unit_id = "unit_id2".to_string();
        data.proc = now + TimeDelta::try_milliseconds(4).unwrap();
        data.resp = Some(now + TimeDelta::try_milliseconds(1).unwrap());
        data.device_id = Some("device_id3".to_string());
        data.network_code = None;
        data.network_addr = None;
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
