use laboratory::{expect, SpecContext};

use sylvia_iot_auth::models::redis::conn::{self, Options};

use super::{get_test_db_path, TestState, STATE};

/// Generate spec for models::conn::connect().
pub fn conn(context: &mut SpecContext<TestState>) -> Result<(), String> {
    let state = context.state.borrow();
    let state = state.get(STATE).unwrap();
    let runtime = state.runtime.as_ref().unwrap();

    let result = runtime.block_on(async {
        conn::connect(&Options {
            url: get_test_db_path().to_string(),
        })
        .await
    });
    expect(result.is_ok()).to_equal(true)?;
    Ok(())
}
