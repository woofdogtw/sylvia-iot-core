//! This module provides common module/functions for different kind of databases.
//!
//! Most of model test cases only use [`tokio::runtime::Runtime`] and [`sylvia_iot_data::models`]
//! arguments, this module can reduce code size and lines.

pub mod application_dldata;
pub mod application_uldata;
pub mod coremgr_opdata;
pub mod network_dldata;
pub mod network_uldata;
