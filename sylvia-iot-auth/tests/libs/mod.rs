use laboratory::{Suite, describe};

use crate::TestState;

mod config;

pub fn suite() -> Suite<TestState> {
    describe("libs", |context| {
        context.describe("config", |context| {
            context.it("apply_default", config::apply_default);
            context.it("reg_args", config::reg_args);
            context.it("read_args", config::read_args);
        });
    })
}
