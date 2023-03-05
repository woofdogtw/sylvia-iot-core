use laboratory::{expect, SpecContext};

use sylvia_iot_auth::models::{self, mongodb_conn, ConnOptions, MongoDbOptions};

use super::{TestState, STATE};

/// Generate spec for models::conn::connect().
pub fn conn(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();

    let result = runtime.block_on(async {
        mongodb_conn::connect(&MongoDbOptions {
            url: crate::TEST_MONGODB_URL.to_string(),
            db: crate::TEST_MONGODB_DB.to_string(),
            pool_size: None,
        })
        .await
    });
    expect(result.is_ok()).to_equal(true)?;

    let result = runtime.block_on(async {
        mongodb_conn::connect(&MongoDbOptions {
            url: crate::TEST_MONGODB_URL.to_string(),
            db: crate::TEST_MONGODB_DB.to_string(),
            pool_size: Some(10),
        })
        .await
    });
    expect(result.is_ok()).to_equal(true)
}

/// Generate spec for models::new().
pub fn models_new(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();

    let model = match runtime.block_on(async {
        models::new(&ConnOptions::MongoDB(MongoDbOptions {
            url: crate::TEST_MONGODB_URL.to_string(),
            db: crate::TEST_MONGODB_DB.to_string(),
            pool_size: None,
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
