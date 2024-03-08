use actix_web::{
    http::{header, StatusCode},
    middleware::NormalizePath,
    test::{self, TestRequest},
    App,
};
use chrono::{DateTime, TimeDelta, Utc};
use laboratory::{expect, SpecContext};
use serde_json::{Map, Value};
use serde_urlencoded;
use tokio::runtime::Runtime;

use sylvia_iot_corelib::err;
use sylvia_iot_data::{models::coremgr_opdata::CoremgrOpData, routes};

use super::{
    super::{
        super::libs::{test_get_400, test_invalid_token, ApiError},
        TestState, STATE, TOKEN_MANAGER, TOKEN_MEMBER, TOKEN_OWNER,
    },
    request, response,
};

pub fn get_count(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    let now = Utc::now();
    count_list_dataset(runtime, routes_state, now)?;

    test_get_count(runtime, routes_state, TOKEN_MANAGER, None, 5)?;
    test_get_count(runtime, routes_state, TOKEN_OWNER, None, 3)?;
    test_get_count(runtime, routes_state, TOKEN_MEMBER, None, 0)?;

    let mut param = request::GetCount {
        user: Some("".to_string()),
        ..Default::default()
    };
    test_get_count(runtime, routes_state, TOKEN_MANAGER, Some(&param), 5)?;
    test_get_count(runtime, routes_state, TOKEN_OWNER, Some(&param), 3)?;
    test_get_count(runtime, routes_state, TOKEN_MEMBER, Some(&param), 0)?;
    param.user = Some("owner".to_string());
    test_get_count(runtime, routes_state, TOKEN_MANAGER, Some(&param), 3)?;
    test_get_count(runtime, routes_state, TOKEN_OWNER, Some(&param), 3)?;
    test_get_count(runtime, routes_state, TOKEN_MEMBER, Some(&param), 0)?;
    param.user = Some("user_id2".to_string());
    test_get_count(runtime, routes_state, TOKEN_MANAGER, Some(&param), 1)?;
    test_get_count(runtime, routes_state, TOKEN_OWNER, Some(&param), 3)?;
    test_get_count(runtime, routes_state, TOKEN_MEMBER, Some(&param), 0)
}

pub fn get_count_not_exist_user(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    let uri = "/data/api/v1/coremgr-opdata/count";
    let code = "err_data_user_not_exist";

    let mut query = Map::<String, Value>::new();
    query.insert("user".to_string(), Value::String("unit_id3".to_string()));
    test_get_400(runtime, routes_state, TOKEN_MANAGER, uri, &query, code)
}

pub fn get_count_invalid_param(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    let uri = "/data/api/v1/coremgr-opdata/count";
    let code = err::E_PARAM;

    let mut query = Map::<String, Value>::new();
    query.insert("tstart".to_string(), Value::String("0".to_string()));
    test_get_400(runtime, routes_state, TOKEN_MANAGER, uri, &query, code)?;
    test_get_400(runtime, routes_state, TOKEN_OWNER, uri, &query, code)?;
    test_get_400(runtime, routes_state, TOKEN_MEMBER, uri, &query, code)?;

    let mut query = Map::<String, Value>::new();
    query.insert("tend".to_string(), Value::String("0".to_string()));
    test_get_400(runtime, routes_state, TOKEN_MANAGER, uri, &query, code)?;
    test_get_400(runtime, routes_state, TOKEN_OWNER, uri, &query, code)?;
    test_get_400(runtime, routes_state, TOKEN_MEMBER, uri, &query, code)?;

    let mut query = Map::<String, Value>::new();
    query.insert("tstart".to_string(), Value::String("-1".to_string()));
    test_get_400(runtime, routes_state, TOKEN_MANAGER, uri, &query, code)?;
    test_get_400(runtime, routes_state, TOKEN_OWNER, uri, &query, code)?;
    test_get_400(runtime, routes_state, TOKEN_MEMBER, uri, &query, code)?;

    let mut query = Map::<String, Value>::new();
    query.insert("tend".to_string(), Value::String("1.1".to_string()));
    test_get_400(runtime, routes_state, TOKEN_MANAGER, uri, &query, code)?;
    test_get_400(runtime, routes_state, TOKEN_OWNER, uri, &query, code)?;
    test_get_400(runtime, routes_state, TOKEN_MEMBER, uri, &query, code)?;

    let mut query = Map::<String, Value>::new();
    query.insert("tfield".to_string(), Value::String("invalid".to_string()));
    query.insert("tstart".to_string(), Value::String("0".to_string()));
    test_get_400(runtime, routes_state, TOKEN_MANAGER, uri, &query, code)?;
    test_get_400(runtime, routes_state, TOKEN_OWNER, uri, &query, code)?;
    test_get_400(runtime, routes_state, TOKEN_MEMBER, uri, &query, code)?;

    let mut query = Map::<String, Value>::new();
    query.insert("tfield".to_string(), Value::String("invalid".to_string()));
    query.insert("tend".to_string(), Value::String("0".to_string()));
    test_get_400(runtime, routes_state, TOKEN_MANAGER, uri, &query, code)?;
    test_get_400(runtime, routes_state, TOKEN_OWNER, uri, &query, code)?;
    test_get_400(runtime, routes_state, TOKEN_MEMBER, uri, &query, code)
}

pub fn get_count_invalid_token(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    let req = TestRequest::get().uri("/data/api/v1/coremgr-opdata/count");
    test_invalid_token(runtime, &routes_state, req)
}

pub fn get_list(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    let now = Utc::now();
    count_list_dataset(runtime, routes_state, now)?;
    let max_req = now.timestamp_millis() + 4;

    test_get_list(runtime, routes_state, TOKEN_MANAGER, None, 5, max_req)?;
    test_get_list(runtime, routes_state, TOKEN_OWNER, None, 3, max_req)?;
    test_get_list(runtime, routes_state, TOKEN_MEMBER, None, 0, max_req)?;

    let mut param = request::GetList {
        user: Some("".to_string()),
        ..Default::default()
    };
    test_get_list(
        runtime,
        routes_state,
        TOKEN_MANAGER,
        Some(&param),
        5,
        max_req,
    )?;
    test_get_list(runtime, routes_state, TOKEN_OWNER, Some(&param), 3, max_req)?;
    test_get_list(
        runtime,
        routes_state,
        TOKEN_MEMBER,
        Some(&param),
        0,
        max_req,
    )?;
    param.user = Some("owner".to_string());
    test_get_list(
        runtime,
        routes_state,
        TOKEN_MANAGER,
        Some(&param),
        3,
        max_req,
    )?;
    test_get_list(runtime, routes_state, TOKEN_OWNER, Some(&param), 3, max_req)?;
    test_get_list(
        runtime,
        routes_state,
        TOKEN_MEMBER,
        Some(&param),
        0,
        max_req,
    )?;
    param.user = Some("user_id2".to_string());
    test_get_list(
        runtime,
        routes_state,
        TOKEN_MANAGER,
        Some(&param),
        1,
        max_req,
    )?;
    test_get_list(runtime, routes_state, TOKEN_OWNER, Some(&param), 3, max_req)?;
    test_get_list(
        runtime,
        routes_state,
        TOKEN_MEMBER,
        Some(&param),
        0,
        max_req,
    )
}

pub fn get_list_sort(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    let now = Utc::now();
    count_list_dataset(runtime, &routes_state, now)?;

    let mut param = request::GetList {
        ..Default::default()
    };
    test_get_list_sort(
        runtime,
        &routes_state,
        TOKEN_MANAGER,
        &mut param,
        &["data_id5", "data_id4", "data_id3", "data_id2", "data_id1"],
    )?;

    param.sort_vec = Some(vec![("req", true)]);
    test_get_list_sort(
        runtime,
        &routes_state,
        TOKEN_MANAGER,
        &mut param,
        &["data_id1", "data_id2", "data_id3", "data_id4", "data_id5"],
    )?;
    param.sort_vec = Some(vec![("req", false)]);
    test_get_list_sort(
        runtime,
        &routes_state,
        TOKEN_MANAGER,
        &mut param,
        &["data_id5", "data_id4", "data_id3", "data_id2", "data_id1"],
    )?;

    param.sort_vec = Some(vec![("res", true)]);
    test_get_list_sort(
        runtime,
        &routes_state,
        TOKEN_MANAGER,
        &mut param,
        &["data_id5", "data_id4", "data_id3", "data_id2", "data_id1"],
    )?;
    param.sort_vec = Some(vec![("res", false)]);
    test_get_list_sort(
        runtime,
        &routes_state,
        TOKEN_MANAGER,
        &mut param,
        &["data_id1", "data_id2", "data_id3", "data_id4", "data_id5"],
    )?;

    param.sort_vec = Some(vec![("latency", true)]);
    test_get_list_sort(
        runtime,
        &routes_state,
        TOKEN_MANAGER,
        &mut param,
        &["data_id5", "data_id4", "data_id3", "data_id2", "data_id1"],
    )?;
    param.sort_vec = Some(vec![("latency", false)]);
    test_get_list_sort(
        runtime,
        &routes_state,
        TOKEN_MANAGER,
        &mut param,
        &["data_id1", "data_id2", "data_id3", "data_id4", "data_id5"],
    )
}

pub fn get_list_offset_limit(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    let now = Utc::now();

    for i in 100..201 {
        add_offset_limit_data(
            runtime,
            &routes_state,
            format!("data_id{}", i).as_str(),
            now + TimeDelta::try_milliseconds(i).unwrap(),
            false,
        )?;
    }
    for i in 201..302 {
        add_offset_limit_data(
            runtime,
            &routes_state,
            format!("data_id{}", i).as_str(),
            now + TimeDelta::try_milliseconds(i).unwrap(),
            true,
        )?;
    }

    let mut param = request::GetList {
        tfield: Some("req".to_string()),
        tstart: Some(now.timestamp_millis()),
        sort: Some("req:asc".to_string()),
        ..Default::default()
    };
    test_get_list_offset_limit(
        runtime,
        &routes_state,
        TOKEN_MANAGER,
        &param,
        (100..200).collect(),
    )?;

    param.limit = Some(0);
    test_get_list_offset_limit(
        runtime,
        &routes_state,
        TOKEN_MANAGER,
        &param,
        (100..302).collect(),
    )?;

    param.offset = Some(0);
    param.limit = Some(5);
    test_get_list_offset_limit(
        runtime,
        &routes_state,
        TOKEN_MANAGER,
        &param,
        (100..105).collect(),
    )?;

    param.offset = Some(5);
    param.limit = Some(0);
    test_get_list_offset_limit(
        runtime,
        &routes_state,
        TOKEN_MANAGER,
        &param,
        (105..302).collect(),
    )?;

    param.offset = Some(198);
    param.limit = Some(50);
    test_get_list_offset_limit(
        runtime,
        &routes_state,
        TOKEN_MANAGER,
        &param,
        (298..302).collect(),
    )?;

    param.offset = Some(2);
    param.limit = Some(105);
    test_get_list_offset_limit(
        runtime,
        &routes_state,
        TOKEN_MANAGER,
        &param,
        (102..207).collect(),
    )?;

    param.offset = Some(2);
    param.limit = None;
    test_get_list_offset_limit(
        runtime,
        &routes_state,
        TOKEN_MANAGER,
        &param,
        (102..202).collect(),
    )
}

pub fn get_list_format_array_csv(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    let now = Utc::now();

    for i in 100..201 {
        add_offset_limit_data(
            runtime,
            &routes_state,
            format!("data_id{}", i).as_str(),
            now + TimeDelta::try_milliseconds(i).unwrap(),
            false,
        )?;
    }
    for i in 201..302 {
        add_offset_limit_data(
            runtime,
            &routes_state,
            format!("data_id{}", i).as_str(),
            now + TimeDelta::try_milliseconds(i).unwrap(),
            true,
        )?;
    }

    let mut param = request::GetList {
        tfield: Some("req".to_string()),
        tstart: Some(now.timestamp_millis()),
        sort: Some("req:asc".to_string()),
        limit: Some(5),
        ..Default::default()
    };
    test_get_list_format_array(
        runtime,
        &routes_state,
        TOKEN_MANAGER,
        &mut param,
        (100..105).collect(),
    )?;
    test_get_list_format_csv(
        runtime,
        &routes_state,
        TOKEN_MANAGER,
        &mut param,
        (100..105).collect(),
    )?;

    param.offset = Some(2);
    param.limit = Some(105);
    test_get_list_format_array(
        runtime,
        &routes_state,
        TOKEN_MANAGER,
        &mut param,
        (102..207).collect(),
    )?;
    test_get_list_format_csv(
        runtime,
        &routes_state,
        TOKEN_MANAGER,
        &mut param,
        (102..207).collect(),
    )
}

pub fn get_list_not_exist_user(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    let uri = "/data/api/v1/coremgr-opdata/list";
    let code = "err_data_user_not_exist";

    let mut query = Map::<String, Value>::new();
    query.insert("user".to_string(), Value::String("unit_id3".to_string()));
    test_get_400(runtime, routes_state, TOKEN_MANAGER, uri, &query, code)
}

pub fn get_list_invalid_param(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    let uri = "/data/api/v1/coremgr-opdata/list";
    let code = err::E_PARAM;

    let mut query = Map::<String, Value>::new();
    query.insert("tstart".to_string(), Value::String("0".to_string()));
    test_get_400(runtime, routes_state, TOKEN_MANAGER, uri, &query, code)?;
    test_get_400(runtime, routes_state, TOKEN_OWNER, uri, &query, code)?;
    test_get_400(runtime, routes_state, TOKEN_MEMBER, uri, &query, code)?;

    let mut query = Map::<String, Value>::new();
    query.insert("tend".to_string(), Value::String("0".to_string()));
    test_get_400(runtime, routes_state, TOKEN_MANAGER, uri, &query, code)?;
    test_get_400(runtime, routes_state, TOKEN_OWNER, uri, &query, code)?;
    test_get_400(runtime, routes_state, TOKEN_MEMBER, uri, &query, code)?;

    let mut query = Map::<String, Value>::new();
    query.insert("tstart".to_string(), Value::String("-1".to_string()));
    test_get_400(runtime, routes_state, TOKEN_MANAGER, uri, &query, code)?;
    test_get_400(runtime, routes_state, TOKEN_OWNER, uri, &query, code)?;
    test_get_400(runtime, routes_state, TOKEN_MEMBER, uri, &query, code)?;

    let mut query = Map::<String, Value>::new();
    query.insert("tend".to_string(), Value::String("1.1".to_string()));
    test_get_400(runtime, routes_state, TOKEN_MANAGER, uri, &query, code)?;
    test_get_400(runtime, routes_state, TOKEN_OWNER, uri, &query, code)?;
    test_get_400(runtime, routes_state, TOKEN_MEMBER, uri, &query, code)?;

    let mut query = Map::<String, Value>::new();
    query.insert("tfield".to_string(), Value::String("invalid".to_string()));
    query.insert("tstart".to_string(), Value::String("0".to_string()));
    test_get_400(runtime, routes_state, TOKEN_MANAGER, uri, &query, code)?;
    test_get_400(runtime, routes_state, TOKEN_OWNER, uri, &query, code)?;
    test_get_400(runtime, routes_state, TOKEN_MEMBER, uri, &query, code)?;

    let mut query = Map::<String, Value>::new();
    query.insert("tfield".to_string(), Value::String("invalid".to_string()));
    query.insert("tend".to_string(), Value::String("0".to_string()));
    test_get_400(runtime, routes_state, TOKEN_MANAGER, uri, &query, code)?;
    test_get_400(runtime, routes_state, TOKEN_OWNER, uri, &query, code)?;
    test_get_400(runtime, routes_state, TOKEN_MEMBER, uri, &query, code)?;

    let mut query = Map::<String, Value>::new();
    query.insert("sort".to_string(), Value::String("".to_string()));
    test_get_400(runtime, &routes_state, TOKEN_MANAGER, uri, &query, code)?;

    let mut query = Map::<String, Value>::new();
    query.insert("sort".to_string(), Value::String("req".to_string()));
    test_get_400(runtime, &routes_state, TOKEN_MANAGER, uri, &query, code)?;

    let mut query = Map::<String, Value>::new();
    query.insert("sort".to_string(), Value::String("req:asc:c".to_string()));
    test_get_400(runtime, &routes_state, TOKEN_MANAGER, uri, &query, code)?;

    let mut query = Map::<String, Value>::new();
    query.insert(
        "sort".to_string(),
        Value::String("req:asc,res:true".to_string()),
    );
    test_get_400(runtime, &routes_state, TOKEN_MANAGER, uri, &query, code)?;

    let mut query = Map::<String, Value>::new();
    query.insert("sort".to_string(), Value::String("a:asc".to_string()));
    test_get_400(runtime, &routes_state, TOKEN_MANAGER, uri, &query, code)
}

pub fn get_list_invalid_token(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();
    let routes_state = state.routes_state.as_ref().unwrap();

    let req = TestRequest::get().uri("/data/api/v1/coremgr-opdata/list");
    test_invalid_token(runtime, &routes_state, req)
}

fn test_get_count(
    runtime: &Runtime,
    state: &routes::State,
    token: &str,
    param: Option<&request::GetCount>,
    expect_count: usize,
) -> Result<(), String> {
    let mut app = runtime.block_on(async {
        test::init_service(
            App::new()
                .wrap(NormalizePath::trim())
                .service(routes::new_service(state)),
        )
        .await
    });

    let uri = match param {
        None => "/data/api/v1/coremgr-opdata/count".to_string(),
        Some(param) => format!(
            "/data/api/v1/coremgr-opdata/count?{}",
            serde_urlencoded::to_string(&param).unwrap()
        ),
    };
    let req = TestRequest::get()
        .uri(uri.as_str())
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", token)))
        .to_request();
    let resp = runtime.block_on(async { test::call_service(&mut app, req).await });
    expect(resp.status()).to_equal(StatusCode::OK)?;
    let body: response::GetCount = runtime.block_on(async { test::read_body_json(resp).await });
    expect(body.data.count).to_equal(expect_count)
}

fn test_get_list(
    runtime: &Runtime,
    state: &routes::State,
    token: &str,
    param: Option<&request::GetList>,
    expect_count: usize,
    max_req: i64,
) -> Result<(), String> {
    let mut app = runtime.block_on(async {
        test::init_service(
            App::new()
                .wrap(NormalizePath::trim())
                .service(routes::new_service(state)),
        )
        .await
    });

    let uri = match param {
        None => "/data/api/v1/coremgr-opdata/list".to_string(),
        Some(param) => format!(
            "/data/api/v1/coremgr-opdata/list?{}",
            serde_urlencoded::to_string(&param).unwrap()
        ),
    };
    let req = TestRequest::get()
        .uri(uri.as_str())
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", token)))
        .to_request();
    let resp = runtime.block_on(async { test::call_service(&mut app, req).await });
    expect(resp.status()).to_equal(StatusCode::OK)?;
    let body: response::GetList = runtime.block_on(async { test::read_body_json(resp).await });
    expect(body.data.len()).to_equal(expect_count)?;

    let mut prev_req = max_req;
    for info in body.data.iter() {
        let req_time = match DateTime::parse_from_rfc3339(info.req_time.as_str()) {
            Err(_) => return Err(format!("reqTime {} format error", info.req_time.as_str())),
            Ok(req_time) => req_time.timestamp_millis(),
        };
        if let Err(_) = DateTime::parse_from_rfc3339(info.res_time.as_str()) {
            return Err(format!("resTime {} format error", info.res_time.as_str()));
        }
        if req_time > prev_req {
            return Err(format!(
                "reqTime order error: {} - {}",
                prev_req,
                info.req_time.as_str()
            ));
        }
        prev_req = req_time;
    }
    Ok(())
}

fn test_get_list_sort(
    runtime: &Runtime,
    state: &routes::State,
    token: &str,
    param: &mut request::GetList,
    expect_ids: &[&str],
) -> Result<(), String> {
    let mut app = runtime.block_on(async {
        test::init_service(
            App::new()
                .wrap(NormalizePath::trim())
                .service(routes::new_service(state)),
        )
        .await
    });

    if let Some(sorts) = param.sort_vec.as_ref() {
        let sorts: Vec<String> = sorts
            .iter()
            .map(|(k, v)| {
                format!(
                    "{}:{}",
                    k,
                    match v {
                        false => "desc",
                        true => "asc",
                    }
                )
            })
            .collect();
        if sorts.len() > 0 {
            param.sort = Some(sorts.join(","));
        }
    }

    let uri = format!(
        "/data/api/v1/coremgr-opdata/list?{}",
        serde_urlencoded::to_string(&param).unwrap()
    );
    let req = TestRequest::get()
        .uri(uri.as_str())
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", token)))
        .to_request();
    let resp = runtime.block_on(async { test::call_service(&mut app, req).await });
    if let Err(_) = expect(resp.status()).to_equal(StatusCode::OK) {
        let body: ApiError = runtime.block_on(async { test::read_body_json(resp).await });
        let message = match body.message.as_ref() {
            None => "",
            Some(message) => message.as_str(),
        };
        return Err(format!(
            "response not 200: {}, {}, {}",
            uri.as_str(),
            body.code,
            message
        ));
    }
    let body: response::GetList = runtime.block_on(async { test::read_body_json(resp).await });
    expect(body.data.len()).to_equal(expect_ids.len())?;

    let mut index = 0;
    for data in body.data.iter() {
        expect(data.data_id.as_str()).to_equal(expect_ids[index])?;
        index += 1;
    }
    Ok(())
}

fn test_get_list_offset_limit(
    runtime: &Runtime,
    state: &routes::State,
    token: &str,
    param: &request::GetList,
    expect_ids: Vec<i32>,
) -> Result<(), String> {
    let mut app = runtime.block_on(async {
        test::init_service(
            App::new()
                .wrap(NormalizePath::trim())
                .service(routes::new_service(state)),
        )
        .await
    });

    let uri = format!(
        "/data/api/v1/coremgr-opdata/list?{}",
        serde_urlencoded::to_string(&param).unwrap()
    );
    let req = TestRequest::get()
        .uri(uri.as_str())
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", token)))
        .to_request();
    let resp = runtime.block_on(async { test::call_service(&mut app, req).await });
    expect(resp.status()).to_equal(StatusCode::OK)?;
    let body: response::GetList = runtime.block_on(async { test::read_body_json(resp).await });
    expect(body.data.len()).to_equal(expect_ids.len())?;

    let mut index = 0;
    for data in body.data.iter() {
        expect(data.data_id.as_str()).to_equal(format!("data_id{}", expect_ids[index]).as_str())?;
        index += 1;
    }
    Ok(())
}

fn test_get_list_format_array(
    runtime: &Runtime,
    state: &routes::State,
    token: &str,
    param: &mut request::GetList,
    expect_ids: Vec<i32>,
) -> Result<(), String> {
    let mut app = runtime.block_on(async {
        test::init_service(
            App::new()
                .wrap(NormalizePath::trim())
                .service(routes::new_service(state)),
        )
        .await
    });

    param.format = Some("array".to_string());
    let uri = format!(
        "/data/api/v1/coremgr-opdata/list?{}",
        serde_urlencoded::to_string(&param).unwrap()
    );
    let req = TestRequest::get()
        .uri(uri.as_str())
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", token)))
        .to_request();
    let resp = runtime.block_on(async { test::call_service(&mut app, req).await });
    expect(resp.status()).to_equal(StatusCode::OK)?;
    let body: Vec<response::GetListData> =
        runtime.block_on(async { test::read_body_json(resp).await });
    expect(body.len()).to_equal(expect_ids.len())?;

    let mut index = 0;
    for data in body.iter() {
        expect(data.data_id.as_str()).to_equal(format!("data_id{}", expect_ids[index]).as_str())?;
        index += 1;
    }
    Ok(())
}

fn test_get_list_format_csv(
    runtime: &Runtime,
    state: &routes::State,
    token: &str,
    param: &mut request::GetList,
    expect_ids: Vec<i32>,
) -> Result<(), String> {
    let mut app = runtime.block_on(async {
        test::init_service(
            App::new()
                .wrap(NormalizePath::trim())
                .service(routes::new_service(state)),
        )
        .await
    });

    param.format = Some("csv".to_string());
    let uri = format!(
        "/data/api/v1/coremgr-opdata/list?{}",
        serde_urlencoded::to_string(&param).unwrap()
    );
    let req = TestRequest::get()
        .uri(uri.as_str())
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", token)))
        .to_request();
    let resp = runtime.block_on(async { test::call_service(&mut app, req).await });
    expect(resp.status()).to_equal(StatusCode::OK)?;
    let body = runtime.block_on(async { test::read_body(resp).await });
    let body = match String::from_utf8(body.to_vec()) {
        Err(e) => return Err(format!("list not csv string: {}", e)),
        Ok(body) => body,
    };

    let fields =
        b"dataId,reqTime,resTime,latencyMs,status,method,path,body,userId,clientId,errCode,errMessage";
    let mut count = 0;
    for line in body.lines() {
        if count == 0 {
            let mut fields_line: Vec<u8> = vec![0xEF, 0xBB, 0xBF];
            fields_line.extend_from_slice(fields);
            expect(fields_line.as_slice()).to_equal(line.as_bytes())?;
        } else {
            expect(line.starts_with(format!("data_id{},", expect_ids[count - 1]).as_str()))
                .to_equal(true)?;
        }
        count += 1;
    }
    expect(expect_ids.len() + 1).to_equal(count)
}

fn count_list_dataset(
    runtime: &Runtime,
    state: &routes::State,
    now: DateTime<Utc>,
) -> Result<(), String> {
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
        user_id: "owner".to_string(),
        client_id: "client".to_string(),
        err_code: None,
        err_message: None,
    };
    if let Err(e) = runtime.block_on(async {
        state.model.coremgr_opdata().add(&data).await?;
        data.data_id = "data_id2".to_string();
        data.req_time = now + TimeDelta::try_milliseconds(1).unwrap();
        data.res_time = now + TimeDelta::try_milliseconds(9).unwrap();
        data.latency_ms = data.res_time.timestamp_millis() - data.req_time.timestamp_millis();
        state.model.coremgr_opdata().add(&data).await?;
        data.data_id = "data_id3".to_string();
        data.req_time = now + TimeDelta::try_milliseconds(2).unwrap();
        data.res_time = now + TimeDelta::try_milliseconds(8).unwrap();
        data.latency_ms = data.res_time.timestamp_millis() - data.req_time.timestamp_millis();
        data.client_id = "client_id2".to_string();
        state.model.coremgr_opdata().add(&data).await?;
        data.data_id = "data_id4".to_string();
        data.req_time = now + TimeDelta::try_milliseconds(3).unwrap();
        data.res_time = now + TimeDelta::try_milliseconds(7).unwrap();
        data.latency_ms = data.res_time.timestamp_millis() - data.req_time.timestamp_millis();
        data.user_id = "user_id2".to_string();
        data.body = Some(Map::new());
        state.model.coremgr_opdata().add(&data).await?;
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
        state.model.coremgr_opdata().add(&data).await
    }) {
        return Err(format!("model.add() error: {}", e));
    }

    Ok(())
}

fn add_offset_limit_data(
    runtime: &Runtime,
    state: &routes::State,
    data_id: &str,
    req_time: DateTime<Utc>,
    use_some: bool,
) -> Result<(), String> {
    match runtime.block_on(async {
        let data = CoremgrOpData {
            data_id: data_id.to_string(),
            req_time,
            res_time: req_time,
            latency_ms: 0,
            status: 200,
            source_ip: "::1".to_string(),
            method: "GET".to_string(),
            path: "/path".to_string(),
            body: match use_some {
                false => None,
                true => Some(Map::new()),
            },
            user_id: "manager".to_string(),
            client_id: "manager".to_string(),
            err_code: match use_some {
                false => None,
                true => Some("err_code".to_string()),
            },
            err_message: match use_some {
                false => None,
                true => Some("err_message".to_string()),
            },
        };
        state.model.coremgr_opdata().add(&data).await
    }) {
        Err(e) => Err(format!("add data model error: {}", e)),
        Ok(_) => Ok(()),
    }
}
