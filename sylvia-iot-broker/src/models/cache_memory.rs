//! Pure memory cache.

use std::{error::Error as StdError, sync::Arc};

use async_trait::async_trait;

use super::{
    Model, device, device_route,
    memory::{
        device::{Cache as DeviceCache, Options as DeviceOptions},
        device_route::{Cache as DeviceRouteCache, Options as DeviceRouteOptions},
        network_route::{Cache as NetworkRouteCache, Options as NetworkRouteOptions},
    },
    network_route,
};

/// Pure memory cache.
#[derive(Clone)]
pub struct Cache {
    device: Arc<DeviceCache>,
    device_route: Arc<DeviceRouteCache>,
    network_route: Arc<NetworkRouteCache>,
}

pub struct Options<'a> {
    pub device: &'a DeviceOptions,
    pub device_route: &'a DeviceRouteOptions,
    pub network_route: &'a NetworkRouteOptions,
}

impl Cache {
    /// Create an instance.
    pub fn new(opts: &Options, model: &Arc<dyn Model>) -> Self {
        Cache {
            device: Arc::new(DeviceCache::new(opts.device, model.clone())),
            device_route: Arc::new(DeviceRouteCache::new(opts.device_route, model.clone())),
            network_route: Arc::new(NetworkRouteCache::new(opts.network_route, model.clone())),
        }
    }
}

#[async_trait]
impl super::Cache for Cache {
    async fn close(&self) -> Result<(), Box<dyn StdError>> {
        let _ = self.device_route().clear().await;
        let _ = self.network_route().clear().await;
        Ok(())
    }

    fn device(&self) -> &dyn device::DeviceCache {
        self.device.as_ref()
    }

    fn device_route(&self) -> &dyn device_route::DeviceRouteCache {
        self.device_route.as_ref()
    }

    fn network_route(&self) -> &dyn network_route::NetworkRouteCache {
        self.network_route.as_ref()
    }
}
