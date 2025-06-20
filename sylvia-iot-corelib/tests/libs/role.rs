use std::collections::HashMap;

use laboratory::{SpecContext, expect};

use sylvia_iot_corelib::role::Role;

use crate::TestState;

/// Test [`Role::is_role`].
pub fn is_role(_context: &mut SpecContext<TestState>) -> Result<(), String> {
    let mut roles = HashMap::<String, bool>::new();
    roles.insert("role1".to_string(), true);
    roles.insert("role2".to_string(), false);

    expect(Role::is_role(&roles, "role1")).to_equal(true)?;
    expect(Role::is_role(&roles, "role2")).to_equal(false)?;
    expect(Role::is_role(&roles, "role3")).to_equal(false)
}
