use actix_web::{
    http::{header, StatusCode},
    middleware::NormalizePath,
    test::{self, TestRequest},
    App,
};
use mongodb::bson::Document;
use sql_builder::SqlBuilder;
use sqlx;
use tokio::runtime::Runtime;

use sylvia_iot_broker::routes;

use super::{application, device, device_route, network, network_route, unit};
use crate::TestState;

pub fn clear_all_data(runtime: &Runtime, state: &TestState) -> () {
    const UNIT_NAME: &'static str = "unit";
    const APPLICATION_NAME: &'static str = "application";
    const NETWORK_NAME: &'static str = "network";
    const DEVICE_NAME: &'static str = "device";
    const NETWORK_ROUTE_NAME1: &'static str = "networkRoute";
    const NETWORK_ROUTE_NAME2: &'static str = "network_route";
    const DEVICE_ROUTE_NAME1: &'static str = "deviceRoute";
    const DEVICE_ROUTE_NAME2: &'static str = "device_route";
    const DLDATA_BUFFER_NAME1: &'static str = "dldataBuffer";
    const DLDATA_BUFFER_NAME2: &'static str = "dldata_buffer";

    if let Some(model) = state.mongodb.as_ref() {
        runtime.block_on(async {
            struct Doc;

            let conn = model.get_connection();
            let _ = conn
                .collection::<Doc>(UNIT_NAME)
                .delete_many(Document::new(), None)
                .await;
            let _ = conn
                .collection::<Doc>(APPLICATION_NAME)
                .delete_many(Document::new(), None)
                .await;
            let _ = conn
                .collection::<Doc>(NETWORK_NAME)
                .delete_many(Document::new(), None)
                .await;
            let _ = conn
                .collection::<Doc>(DEVICE_NAME)
                .delete_many(Document::new(), None)
                .await;
            let _ = conn
                .collection::<Doc>(NETWORK_ROUTE_NAME1)
                .delete_many(Document::new(), None)
                .await;
            let _ = conn
                .collection::<Doc>(DEVICE_ROUTE_NAME1)
                .delete_many(Document::new(), None)
                .await;
            let _ = conn
                .collection::<Doc>(DLDATA_BUFFER_NAME1)
                .delete_many(Document::new(), None)
                .await;
        });
    }
    if let Some(model) = state.sqlite.as_ref() {
        runtime.block_on(async {
            let conn = model.get_connection();
            let sql = SqlBuilder::delete_from(UNIT_NAME).sql().unwrap();
            let _ = sqlx::query(sql.as_str()).execute(conn).await;
            let sql = SqlBuilder::delete_from(APPLICATION_NAME).sql().unwrap();
            let _ = sqlx::query(sql.as_str()).execute(conn).await;
            let sql = SqlBuilder::delete_from(NETWORK_NAME).sql().unwrap();
            let _ = sqlx::query(sql.as_str()).execute(conn).await;
            let sql = SqlBuilder::delete_from(DEVICE_NAME).sql().unwrap();
            let _ = sqlx::query(sql.as_str()).execute(conn).await;
            let sql = SqlBuilder::delete_from(NETWORK_ROUTE_NAME2).sql().unwrap();
            let _ = sqlx::query(sql.as_str()).execute(conn).await;
            let sql = SqlBuilder::delete_from(DEVICE_ROUTE_NAME2).sql().unwrap();
            let _ = sqlx::query(sql.as_str()).execute(conn).await;
            let sql = SqlBuilder::delete_from(DLDATA_BUFFER_NAME2).sql().unwrap();
            let _ = sqlx::query(sql.as_str()).execute(conn).await;
        });
    }
    if let Some(state) = state.routes_state.as_ref() {
        runtime.block_on(async {
            let mgrs = { state.application_mgrs.lock().unwrap().clone() };
            for (_, mgr) in mgrs {
                if let Err(e) = mgr.close().await {
                    println!("close ApplicationMgr error: {}", e);
                }
            }
            {
                state.application_mgrs.lock().unwrap().clear();
            }
            let mgrs = { state.network_mgrs.lock().unwrap().clone() };
            for (_, mgr) in mgrs {
                if let Err(e) = mgr.close().await {
                    println!("close NetworkMgr error: {}", e);
                }
            }
            {
                state.network_mgrs.lock().unwrap().clear();
            }
        });
    }
}

pub fn create_unit(
    runtime: &Runtime,
    state: &routes::State,
    token: &str,
    param: &unit::request::PostUnit,
) -> Result<String, String> {
    let mut app = runtime.block_on(async {
        test::init_service(
            App::new()
                .wrap(NormalizePath::trim())
                .service(routes::new_service(state)),
        )
        .await
    });

    let req = TestRequest::post()
        .uri("/broker/api/v1/unit")
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", token)))
        .set_json(param)
        .to_request();
    let resp = runtime.block_on(async { test::call_service(&mut app, req).await });
    if resp.status() != StatusCode::OK {
        let status = resp.status();
        let body = runtime.block_on(async { test::read_body(resp).await });
        return Err(format!(
            "create unit resp status {}, body: {:?}",
            status, body
        ));
    }
    let body: unit::response::PostUnit =
        runtime.block_on(async { test::read_body_json(resp).await });

    Ok(body.data.unit_id)
}

pub fn create_application(
    runtime: &Runtime,
    state: &routes::State,
    token: &str,
    param: &application::request::PostApplication,
) -> Result<String, String> {
    let mut app = runtime.block_on(async {
        test::init_service(
            App::new()
                .wrap(NormalizePath::trim())
                .service(routes::new_service(state)),
        )
        .await
    });

    let req = TestRequest::post()
        .uri("/broker/api/v1/application")
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", token)))
        .set_json(param)
        .to_request();
    let resp = runtime.block_on(async { test::call_service(&mut app, req).await });
    if resp.status() != StatusCode::OK {
        let status = resp.status();
        let body = runtime.block_on(async { test::read_body(resp).await });
        return Err(format!(
            "create application resp status {}, body: {:?}",
            status, body
        ));
    }
    let body: application::response::PostApplication =
        runtime.block_on(async { test::read_body_json(resp).await });

    Ok(body.data.application_id)
}

pub fn create_network(
    runtime: &Runtime,
    state: &routes::State,
    token: &str,
    param: &network::request::PostNetwork,
) -> Result<String, String> {
    let mut app = runtime.block_on(async {
        test::init_service(
            App::new()
                .wrap(NormalizePath::trim())
                .service(routes::new_service(state)),
        )
        .await
    });

    let req = TestRequest::post()
        .uri("/broker/api/v1/network")
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", token)))
        .set_json(param)
        .to_request();
    let resp = runtime.block_on(async { test::call_service(&mut app, req).await });
    if resp.status() != StatusCode::OK {
        let status = resp.status();
        let body = runtime.block_on(async { test::read_body(resp).await });
        return Err(format!(
            "create network resp status {}, body: {:?}",
            status, body
        ));
    }
    let body: network::response::PostNetwork =
        runtime.block_on(async { test::read_body_json(resp).await });

    Ok(body.data.network_id)
}

pub fn create_device(
    runtime: &Runtime,
    state: &routes::State,
    token: &str,
    param: &device::request::PostDevice,
) -> Result<String, String> {
    let mut app = runtime.block_on(async {
        test::init_service(
            App::new()
                .wrap(NormalizePath::trim())
                .service(routes::new_service(state)),
        )
        .await
    });

    let req = TestRequest::post()
        .uri("/broker/api/v1/device")
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", token)))
        .set_json(param)
        .to_request();
    let resp = runtime.block_on(async { test::call_service(&mut app, req).await });
    if resp.status() != StatusCode::OK {
        let status = resp.status();
        let body = runtime.block_on(async { test::read_body(resp).await });
        return Err(format!(
            "create device resp status {}, body: {:?}",
            status, body
        ));
    }
    let body: device::response::PostDevice =
        runtime.block_on(async { test::read_body_json(resp).await });

    Ok(body.data.device_id)
}

pub fn create_device_route(
    runtime: &Runtime,
    state: &routes::State,
    token: &str,
    param: &device_route::request::PostDeviceRoute,
) -> Result<String, String> {
    let mut app = runtime.block_on(async {
        test::init_service(
            App::new()
                .wrap(NormalizePath::trim())
                .service(routes::new_service(state)),
        )
        .await
    });

    let req = TestRequest::post()
        .uri("/broker/api/v1/device-route")
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", token)))
        .set_json(param)
        .to_request();
    let resp = runtime.block_on(async { test::call_service(&mut app, req).await });
    if resp.status() != StatusCode::OK {
        let status = resp.status();
        let body = runtime.block_on(async { test::read_body(resp).await });
        return Err(format!(
            "create device route resp status {}, body: {:?}",
            status, body
        ));
    }
    let body: device_route::response::PostDeviceRoute =
        runtime.block_on(async { test::read_body_json(resp).await });

    Ok(body.data.route_id)
}

pub fn delete_device_route(
    runtime: &Runtime,
    state: &routes::State,
    token: &str,
    route_id: &str,
) -> Result<(), String> {
    let mut app = runtime.block_on(async {
        test::init_service(
            App::new()
                .wrap(NormalizePath::trim())
                .service(routes::new_service(state)),
        )
        .await
    });

    let req = TestRequest::delete()
        .uri(format!("/broker/api/v1/device-route/{}", route_id).as_str())
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", token)))
        .to_request();
    let resp = runtime.block_on(async { test::call_service(&mut app, req).await });
    if resp.status() != StatusCode::NO_CONTENT {
        let status = resp.status();
        let body = runtime.block_on(async { test::read_body(resp).await });
        return Err(format!(
            "delete device route resp status {}, body: {:?}",
            status, body
        ));
    }

    Ok(())
}

pub fn create_network_route(
    runtime: &Runtime,
    state: &routes::State,
    token: &str,
    param: &network_route::request::PostNetworkRoute,
) -> Result<String, String> {
    let mut app = runtime.block_on(async {
        test::init_service(
            App::new()
                .wrap(NormalizePath::trim())
                .service(routes::new_service(state)),
        )
        .await
    });

    let req = TestRequest::post()
        .uri("/broker/api/v1/network-route")
        .insert_header((header::AUTHORIZATION, format!("Bearer {}", token)))
        .set_json(param)
        .to_request();
    let resp = runtime.block_on(async { test::call_service(&mut app, req).await });
    if resp.status() != StatusCode::OK {
        let status = resp.status();
        let body = runtime.block_on(async { test::read_body(resp).await });
        return Err(format!(
            "create network route resp status {}, body: {:?}",
            status, body
        ));
    }
    let body: network_route::response::PostNetworkRoute =
        runtime.block_on(async { test::read_body_json(resp).await });

    Ok(body.data.route_id)
}
