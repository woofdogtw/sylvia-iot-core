use laboratory::{describe, LabResult};

mod libs;

use libs::{err, http, logger, role, server_config, strings};

pub struct TestState;

#[test]
pub fn integration_test() -> LabResult {
    describe("full test", |context| {
        context.describe("err", |context| {
            context.it("to_json", err::to_json);
            context.it("ErrResp::fmt", err::fmt);
            context.it("ErrResp::status_code", err::status_code);
            context.it("ErrResp::error_response", err::error_response);
        });

        context.describe("http", |context| {
            context.it("parse_header_auth", http::parse_header_auth);
        });

        context.describe("logger", |context| {
            context.it("apply_default", logger::apply_default);
            context.it("reg_args", logger::reg_args);
            context.it("read_args", logger::read_args);
        });

        context.describe("role", |context| {
            context.it("is_role", role::is_role);
        });

        context.describe("server_config", |context| {
            context.it("apply_default", server_config::apply_default);
            context.it("reg_args", server_config::reg_args);
            context.it("read_args", server_config::read_args);
        });

        context.describe("strings", |context| {
            context.it("hex_addr_to_u128", strings::hex_addr_to_u128);
            context.it("is_account", strings::is_account);
            context.it("is_code", strings::is_code);
            context.it("is_scope", strings::is_scope);
            context.it("is_uri", strings::is_uri);
            context.it("password_hash", strings::password_hash);
            context.it("random_id", strings::random_id);
            context.it("random_id_sha", strings::random_id_sha);
            context.it("randomstring", strings::randomstring);
            context.it("time_str", strings::time_str);
            context.it("u128_to_addr", strings::u128_to_addr);
        });
    })
    .run()
}
