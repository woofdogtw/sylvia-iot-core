use axum::{
    extract::{
        FromRequest, FromRequestParts, Json as AxumJson, Path as AxumPath, Query as AxumQuery,
        Request, rejection::JsonRejection,
    },
    http::{header, request::Parts},
    response::{IntoResponse, Response},
};
use bytes::{BufMut, BytesMut};
use serde::{Serialize, de::DeserializeOwned};

use crate::{constants::ContentType, err::ErrResp};

/// JSON Extractor / Response.
///
/// This is the customized [`axum::extract::Json`] version to respose error with [`ErrResp`].
pub struct Json<T>(pub T);

/// Path Extractor / Response.
///
/// This is the customized [`axum::extract::Path`] version to respose error with [`ErrResp`].
pub struct Path<T>(pub T);

/// Query Extractor / Response.
///
/// This is the customized [`axum::extract::Query`] version to respose error with [`ErrResp`].
pub struct Query<T>(pub T);

impl<S, T> FromRequest<S> for Json<T>
where
    AxumJson<T>: FromRequest<S, Rejection = JsonRejection>,
    S: Send + Sync,
{
    type Rejection = ErrResp;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        match AxumJson::<T>::from_request(req, state).await {
            Err(e) => Err(ErrResp::ErrParam(Some(e.to_string()))),
            Ok(value) => Ok(Self(value.0)),
        }
    }
}

impl<T> IntoResponse for Json<T>
where
    T: Serialize,
{
    fn into_response(self) -> Response {
        // Use a small initial capacity of 128 bytes like serde_json::to_vec
        // https://docs.rs/serde_json/1.0.82/src/serde_json/ser.rs.html#2189
        let mut buf = BytesMut::with_capacity(128).writer();
        match serde_json::to_writer(&mut buf, &self.0) {
            Err(e) => ErrResp::ErrUnknown(Some(e.to_string())).into_response(),
            Ok(()) => (
                [(header::CONTENT_TYPE, ContentType::JSON)],
                buf.into_inner().freeze(),
            )
                .into_response(),
        }
    }
}

impl<T, S> FromRequestParts<S> for Path<T>
where
    T: DeserializeOwned + Send,
    S: Send + Sync,
{
    type Rejection = ErrResp;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        match AxumPath::from_request_parts(parts, state).await {
            Err(e) => Err(ErrResp::ErrParam(Some(e.to_string()))),
            Ok(value) => Ok(Self(value.0)),
        }
    }
}

impl<T, S> FromRequestParts<S> for Query<T>
where
    T: DeserializeOwned,
    S: Send + Sync,
{
    type Rejection = ErrResp;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        match AxumQuery::from_request_parts(parts, state).await {
            Err(e) => Err(ErrResp::ErrParam(Some(e.to_string()))),
            Ok(value) => Ok(Self(value.0)),
        }
    }
}

/// Parse Authorization header content. Returns `None` means no Authorization header.
pub fn parse_header_auth(req: &Request) -> Result<Option<String>, ErrResp> {
    let mut auth_all = req.headers().get_all(header::AUTHORIZATION).iter();
    let auth = match auth_all.next() {
        None => return Ok(None),
        Some(auth) => match auth.to_str() {
            Err(e) => return Err(ErrResp::ErrParam(Some(e.to_string()))),
            Ok(auth) => auth,
        },
    };
    if auth_all.next() != None {
        return Err(ErrResp::ErrParam(Some(
            "invalid multiple Authorization header".to_string(),
        )));
    }
    Ok(Some(auth.to_string()))
}
