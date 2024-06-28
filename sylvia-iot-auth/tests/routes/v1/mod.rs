use std::collections::HashMap;

use laboratory::{describe, Suite};

use super::{libs::new_state, remove_sqlite};
use crate::TestState;

mod auth;
mod client;
mod libs;
mod user;

use auth::api as authapi;
use client::api as clientapi;
use user::api as userapi;

pub const STATE: &'static str = "routes/v1";

pub fn suite(db_engine: &'static str) -> Suite<TestState> {
    describe(format!("routes.v1 - {}", db_engine), move |context| {
        context.describe("auth", |context| {
            context.it("GET /tokeninfo", authapi::get_tokeninfo);
            context.it("POST /logout", authapi::post_logout);

            context
                .before_all(authapi::before_all_fn)
                .after_all(authapi::after_all_fn)
                .after_each(authapi::after_each_fn);
        });

        context.describe("user", |context| {
            context.it("GET /user", userapi::get);
            context.it("GET /user with invalid token", userapi::get_invalid_token);
            context.it("PATCH /user", userapi::patch);
            context.it("PATCH /user with password", userapi::patch_password);
            context.it(
                "PATCH /user with invalid parameters",
                userapi::patch_invalid_param,
            );
            context.it(
                "PATCH /user with invalid token",
                userapi::patch_invalid_token,
            );
            context.it("POST /user", userapi::post_admin);
            context.it("POST /user with duplicate account", userapi::post_admin_dup);
            context.it(
                "POST /user with invalid parameters",
                userapi::post_admin_invalid_param,
            );
            context.it(
                "POST /user with invalid token",
                userapi::post_admin_invalid_token,
            );
            context.it(
                "POST /user with invalid permission",
                userapi::post_admin_invalid_perm,
            );
            context.it("GET /user/count", userapi::get_admin_count);
            context.it(
                "GET /user/count with invalid token",
                userapi::get_admin_count_invalid_token,
            );
            context.it(
                "GET /user/count with invalid permission",
                userapi::get_admin_count_invalid_perm,
            );
            context.it("GET /user/list", userapi::get_admin_list);
            context.it("GET /user/list sort", userapi::get_admin_list_sort);
            context.it(
                "GET /user/list offset limit",
                userapi::get_admin_list_offset_limit,
            );
            context.it(
                "GET /user/list array format",
                userapi::get_admin_list_format_array,
            );
            context.it(
                "GET /user/list with invalid parameters",
                userapi::get_admin_list_invalid_param,
            );
            context.it(
                "GET /user/list with invalid token",
                userapi::get_admin_list_invalid_token,
            );
            context.it(
                "GET /user/list with invalid permission",
                userapi::get_admin_list_invalid_perm,
            );
            context.it("GET /user/{userId}", userapi::get_admin);
            context.it(
                "GET /user/{userId} with wrong ID",
                userapi::get_admin_wrong_id,
            );
            context.it(
                "GET /user/{userId} with invalid token",
                userapi::get_admin_invalid_token,
            );
            context.it(
                "GET /user/{userId} with invalid permission",
                userapi::get_admin_invalid_perm,
            );
            context.it("PATCH /user/{userId}", userapi::patch_admin);
            context.it(
                "PATCH /user/{userId} with password",
                userapi::patch_admin_password,
            );
            context.it(
                "PATCH /user/{userId} with wrong ID",
                userapi::patch_admin_wrong_id,
            );
            context.it(
                "PATCH /user/{userId} with invalid parameters",
                userapi::patch_admin_invalid_param,
            );
            context.it(
                "PATCH /user/{userId} with invalid token",
                userapi::patch_admin_invalid_token,
            );
            context.it(
                "PATCH /user/{userId} with invalid permission",
                userapi::patch_admin_invalid_perm,
            );
            context.it("DELETE /user/{userId}", userapi::delete_admin);
            context.it(
                "DELETE /user/{userId} with invalid token",
                userapi::delete_admin_invalid_token,
            );
            context.it(
                "DELETE /user/{userId} with invalid permission",
                userapi::delete_admin_invalid_perm,
            );

            context
                .before_each(userapi::before_each_fn)
                .after_each(userapi::after_each_fn);
        });

        context.describe("client", |context| {
            context.it("POST /client", clientapi::post);
            context.it(
                "POST /client with duplicate uris and scopes",
                clientapi::post_dup,
            );
            context.it(
                "POST /client with not exist user",
                clientapi::post_not_exist_user,
            );
            context.it(
                "POST /client with invalid parameters",
                clientapi::post_invalid_param,
            );
            context.it(
                "POST /client with invalid token",
                clientapi::post_invalid_token,
            );
            context.it(
                "POST /client with invalid permission",
                clientapi::post_invalid_perm,
            );
            context.it("GET /client/count", clientapi::get_count);
            context.it(
                "GET /client/count with invalid token",
                clientapi::get_count_invalid_token,
            );
            context.it(
                "GET /client/count with invalid permission",
                clientapi::get_count_invalid_perm,
            );
            context.it("GET /client/list", clientapi::get_list);
            context.it("GET /client/list sort", clientapi::get_list_sort);
            context.it(
                "GET /client/list offset limit",
                clientapi::get_list_offset_limit,
            );
            context.it(
                "GET /client/list array format",
                clientapi::get_list_format_array,
            );
            context.it(
                "GET /client/list with invalid parameters",
                clientapi::get_list_invalid_param,
            );
            context.it(
                "GET /client/list with invalid token",
                clientapi::get_list_invalid_token,
            );
            context.it(
                "GET /client/list with invalid permission",
                clientapi::get_list_invalid_perm,
            );
            context.it("GET /client/{clientId}", clientapi::get);
            context.it(
                "GET /client/{clientId} with wrong ID",
                clientapi::get_wrong_id,
            );
            context.it(
                "GET /client/{clientId} with invalid token",
                clientapi::get_invalid_token,
            );
            context.it(
                "GET /client/{clientId} with invalid permission",
                clientapi::get_invalid_perm,
            );
            context.it("PATCH /client/{clientId}", clientapi::patch);
            context.it(
                "PATCH /client/{clientId} with wrong ID",
                clientapi::patch_wrong_id,
            );
            context.it(
                "PATCH /client/{clientId} with invalid parameters",
                clientapi::patch_invalid_param,
            );
            context.it(
                "PATCH /client/{clientId} with invalid token",
                clientapi::patch_invalid_token,
            );
            context.it(
                "PATCH /client/{clientId} with invalid permission",
                clientapi::patch_invalid_perm,
            );
            context.it("DELETE /client/{clientId}", clientapi::delete);
            context.it(
                "DELETE /client/{clientId} with invalid token",
                clientapi::delete_invalid_token,
            );
            context.it(
                "DELETE /client/{clientId} with invalid permission",
                clientapi::delete_invalid_perm,
            );
            context.it("DELETE /client/user/{userId}", clientapi::delete_user);
            context.it(
                "DELETE /client/user/{userId} with invalid token",
                clientapi::delete_user_invalid_token,
            );
            context.it(
                "DELETE /client/user/{userId} with invalid permission",
                clientapi::delete_user_invalid_perm,
            );

            context
                .before_all(clientapi::before_all_fn)
                .after_all(clientapi::after_all_fn)
                .before_each(clientapi::before_each_fn)
                .after_each(clientapi::after_each_fn);
        });

        context
            .before_all(move |state| {
                state.insert(STATE, new_state(Some(db_engine)));
            })
            .after_all(after_all_fn);
    })
}

fn after_all_fn(state: &mut HashMap<&'static str, TestState>) -> () {
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();

    if let Some(model) = state.mongodb.as_ref() {
        runtime.block_on(async {
            if let Err(e) = model.get_connection().drop().await {
                println!("remove mongodb database error: {}", e);
            }
        })
    }
    let mut path = std::env::temp_dir();
    path.push(crate::TEST_SQLITE_PATH);
    remove_sqlite(path.to_str().unwrap());
}
