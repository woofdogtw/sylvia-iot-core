use axum::{
    Router,
    extract::{Request, State},
    http::header,
    response::IntoResponse,
    routing,
};
use serde::Serialize;

use sylvia_iot_corelib::{err::ErrResp, http::Json};

use super::{super::State as AppState, get_tokeninfo_inner};
use crate::libs::config::MqHost;

#[derive(Serialize)]
struct GetHosts {
    data: Vec<GetHostsItem>,
}

#[derive(Serialize)]
struct GetHostsItem {
    /// `amqp` or `mqtt`.
    #[serde(rename = "type")]
    q_type: &'static str,
    name: String,
    external: String,
    active: bool,
}

pub fn new_service(scope_path: &str, state: &AppState) -> Router {
    Router::new().nest(
        scope_path,
        Router::new()
            .route("/hosts", routing::get(get_hosts))
            .with_state(state.clone()),
    )
}

/// `GET /{base}/api/v1/mq/hosts`
///
/// Lists the message broker hosts that units can use. The internal hosts are not exposed.
async fn get_hosts(state: State<AppState>, req: Request) -> impl IntoResponse {
    const FN_NAME: &'static str = "get_hosts";
    let token = match req.headers().get(header::AUTHORIZATION) {
        None => return ErrResp::ErrAuth(None).into_response(),
        Some(value) => value.clone(),
    };
    if let Err(e) =
        get_tokeninfo_inner(FN_NAME, &state.client, state.auth_base.as_str(), &token).await
    {
        return e;
    }

    let mut data = vec![];
    data.extend(transform("amqp", &state.mq_hosts.amqp));
    data.extend(transform("mqtt", &state.mq_hosts.mqtt));
    Json(&GetHosts { data }).into_response()
}

fn transform(q_type: &'static str, hosts: &Vec<MqHost>) -> Vec<GetHostsItem> {
    hosts
        .iter()
        .map(|item| GetHostsItem {
            q_type,
            name: item.name.clone(),
            external: item.external.clone(),
            active: item.active,
        })
        .collect()
}
