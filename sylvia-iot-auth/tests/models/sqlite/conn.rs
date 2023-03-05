use laboratory::{expect, SpecContext};

use sylvia_iot_auth::models::{self, sqlite_conn, ConnOptions, SqliteOptions};

use super::{TestState, STATE};

/// Generate spec for models::conn::connect().
pub fn conn(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();

    let result = runtime.block_on(async {
        let mut path = std::env::temp_dir();
        path.push(crate::TEST_SQLITE_PATH);
        sqlite_conn::connect(&SqliteOptions {
            path: path.to_str().unwrap().to_string(),
        })
        .await
    });
    expect(result.is_ok()).to_equal(true)?;
    runtime.block_on(async { result.unwrap().close().await });
    Ok(())
}

/// Generate spec for models::new().
pub fn models_new(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();

    let model = match runtime.block_on(async {
        let mut path = std::env::temp_dir();
        path.push(crate::TEST_SQLITE_PATH);
        models::new(&ConnOptions::Sqlite(SqliteOptions {
            path: path.to_str().unwrap().to_string(),
        }))
        .await
    }) {
        Err(e) => return Err(format!("new model error: {}", e)),
        Ok(model) => model,
    };
    match runtime.block_on(async { model.close().await }) {
        Err(e) => return Err(format!("close model error: {}", e)),
        Ok(_) => return Ok(()),
    }
}
