use std::collections::HashMap;

/// Role definitions in sylvia-iot platform.
pub struct Role;

impl Role {
    /// The system administrator who has all privileges.
    pub const ADMIN: &'static str = "admin";
    /// The developer who can create clients.
    pub const DEV: &'static str = "dev";
    /// The system manager who has much of priviledges than normal user.
    pub const MANAGER: &'static str = "manager";
    /// The service (process, program, client).
    pub const SERVICE: &'static str = "service";

    /// To check if a user who has `role_map` with the specific `role_name`.
    pub fn is_role(role_map: &HashMap<String, bool>, role_name: &'static str) -> bool {
        match role_map.get(role_name) {
            None => false,
            Some(map) => *map,
        }
    }
}
