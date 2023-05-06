//! This module provides common module/functions for different kind of databases.
//!
//! Most of model test cases only use [`tokio::runtime::Runtime`] and [`sylvia_iot_broker::models`]
//! arguments, this module can reduce code size and lines.

pub mod application;
pub mod device;
pub mod device_route;
pub mod dldata_buffer;
pub mod network;
pub mod network_route;
pub mod unit;
