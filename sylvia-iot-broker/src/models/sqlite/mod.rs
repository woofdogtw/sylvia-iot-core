//! SQLite model implementation.
//!
//! # Notes
//!
//! The cursor is the **simulated** implementation. It only works when there are no add/delete
//! operations during a list operation.

use sql_builder::{esc, SqlBuilder};

pub mod application;
pub mod conn;
pub mod device;
pub mod device_route;
pub mod dldata_buffer;
pub mod network;
pub mod network_route;
pub mod unit;

fn build_where_like<S, T>(builder: &mut SqlBuilder, field: S, mask: T) -> &mut SqlBuilder
where
    S: ToString,
    T: ToString,
{
    let mut use_escape = false;
    let mask = mask.to_string();
    let like_str = mask
        .replace("\\", "\\\\")
        .replace("%", "\\%")
        .replace("_", "\\_");
    if like_str.len() > mask.len() {
        use_escape = true;
    }

    let mut cond = field.to_string();
    cond.push_str(" LIKE '%");
    cond.push_str(&esc(like_str.as_str()));
    cond.push_str("%'");
    if use_escape {
        cond.push_str(" ESCAPE '\\'");
    }
    builder.and_where(&cond)
}
