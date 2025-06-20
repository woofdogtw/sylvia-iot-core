use laboratory::{Suite, describe};

use crate::TestState;

mod auth;

pub fn suite() -> Suite<TestState> {
    describe("middlewares", |context| {
        context.describe_import(auth::suite());
    })
}
