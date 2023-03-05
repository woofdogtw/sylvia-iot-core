use actix_web::{http::header, HttpRequest};

use crate::err::ErrResp;

/// Parse Authorization header content. Returns `None` means no Authorization header.
pub fn parse_header_auth(req: &HttpRequest) -> Result<Option<String>, ErrResp> {
    let mut auth_all = req.headers().get_all(header::AUTHORIZATION);
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
