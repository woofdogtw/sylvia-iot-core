use serde::Deserialize;

#[derive(Deserialize)]
pub struct OAuth2Error {
    pub error: String,
    #[serde(rename = "error_description")]
    pub _error_description: Option<String>,
}

#[derive(Deserialize)]
pub struct PostLoginLocation {
    #[serde(rename = "state")]
    pub _state: String,
    #[serde(rename = "session_id")]
    pub _session_id: String,
}

#[derive(Deserialize)]
pub struct PostAuthorizeLocation {
    pub code: String,
    pub state: Option<String>,
}

#[derive(Deserialize)]
pub struct AccessToken {
    pub access_token: String,
    pub refresh_token: String,
    pub token_type: String,
    pub expires_in: u64,
}
