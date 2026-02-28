use axum::response::{IntoResponse, Response};
use serde::Serialize;

use crate::http::{Json, Query};

/// Query parameters for `GET /version`
#[derive(serde::Deserialize)]
pub struct GetVersionQuery {
    pub q: Option<String>,
}

#[derive(Serialize)]
struct GetVersionRes<'a> {
    data: GetVersionResData<'a>,
}

#[derive(Serialize)]
struct GetVersionResData<'a> {
    name: &'a str,
    version: &'a str,
}

/// Creates an axum handler for `GET /version`.
///
/// Returns a handler that responds with the service name and version. Supports the optional
/// query parameter `q`:
/// - `q=name`: returns the service name as plain text
/// - `q=version`: returns the service version as plain text
/// - otherwise: returns `{"data":{"name":"...","version":"..."}}` as JSON
pub fn gen_get_version(
    name: &'static str,
    version: &'static str,
) -> impl Fn(Query<GetVersionQuery>) -> std::future::Ready<Response> + Clone + Send + 'static {
    move |Query(query): Query<GetVersionQuery>| {
        let resp = if let Some(q) = query.q.as_ref() {
            match q.as_str() {
                "name" => name.into_response(),
                "version" => version.into_response(),
                _ => Json(GetVersionRes {
                    data: GetVersionResData { name, version },
                })
                .into_response(),
            }
        } else {
            Json(GetVersionRes {
                data: GetVersionResData { name, version },
            })
            .into_response()
        };
        std::future::ready(resp)
    }
}
