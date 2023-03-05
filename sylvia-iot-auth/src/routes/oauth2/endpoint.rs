use std::sync::Arc;

use oxide_auth::primitives::scope::Scope;
use oxide_auth_async::{
    code_grant::{
        access_token::{Endpoint as AccessTokenEndpoint, Extension as AccessTokenExtension},
        authorization::{Endpoint as AuthorizationEndpoint, Extension as AuthorizationExtension},
        refresh::Endpoint as RefreshEndpoint,
        resource::Endpoint as ResourceEndpoint,
    },
    primitives::{Authorizer, Issuer, Registrar},
};

use super::primitive::Primitive;
use crate::models::Model;

#[derive(Clone)]
pub struct Endpoint {
    primitive: Primitive,
    extension_fallback: (),
    scopes: Vec<Scope>,
}

impl Endpoint {
    pub fn new(model: Arc<dyn Model>, resource_scopes: Option<&str>) -> Self {
        Endpoint {
            primitive: Primitive::new(model.clone()),
            extension_fallback: (),
            scopes: match resource_scopes {
                None => vec![],
                Some(scopes) => vec![scopes.parse().unwrap()],
            },
        }
    }
}

impl AccessTokenEndpoint for Endpoint {
    fn registrar(&self) -> &(dyn Registrar + Sync) {
        &self.primitive
    }

    fn authorizer(&mut self) -> &mut (dyn Authorizer + Send) {
        &mut self.primitive
    }

    fn issuer(&mut self) -> &mut (dyn Issuer + Send) {
        &mut self.primitive
    }

    fn extension(&mut self) -> &mut (dyn AccessTokenExtension + Send) {
        &mut self.extension_fallback
    }
}

impl AuthorizationEndpoint for Endpoint {
    fn registrar(&self) -> &(dyn Registrar + Sync) {
        &self.primitive
    }

    fn authorizer(&mut self) -> &mut (dyn Authorizer + Send) {
        &mut self.primitive
    }

    fn extension(&mut self) -> &mut (dyn AuthorizationExtension + Send) {
        &mut self.extension_fallback
    }
}

impl RefreshEndpoint for Endpoint {
    fn registrar(&self) -> &(dyn Registrar + Sync) {
        &self.primitive
    }

    fn issuer(&mut self) -> &mut (dyn Issuer + Send) {
        &mut self.primitive
    }
}

impl ResourceEndpoint for Endpoint {
    fn scopes(&mut self) -> &[Scope] {
        self.scopes.as_slice()
    }

    fn issuer(&mut self) -> &mut (dyn Issuer + Send) {
        &mut self.primitive
    }
}
