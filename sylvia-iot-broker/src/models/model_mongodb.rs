//! Pure MongoDB model.

use std::{error::Error as StdError, sync::Arc};

use async_trait::async_trait;
use mongodb::Database;

use super::{
    application, device, device_route, dldata_buffer,
    mongodb::{
        application::Model as ApplicationModel,
        conn::{self, Options},
        device::Model as DeviceModel,
        device_route::Model as DeviceRouteModel,
        dldata_buffer::Model as DlDataBufferModel,
        network::Model as NetworkModel,
        network_route::Model as NetworkRouteModel,
        unit::Model as UnitModel,
    },
    network, network_route, unit,
};

/// Pure MongoDB model.
#[derive(Clone)]
pub struct Model {
    conn: Arc<Database>,
    unit: Arc<UnitModel>,
    application: Arc<ApplicationModel>,
    network: Arc<NetworkModel>,
    device: Arc<DeviceModel>,
    device_route: Arc<DeviceRouteModel>,
    network_route: Arc<NetworkRouteModel>,
    dldata_buffer: Arc<DlDataBufferModel>,
}

impl Model {
    /// Create an instance.
    pub async fn new(opts: &Options) -> Result<Self, Box<dyn StdError>> {
        let conn = Arc::new(conn::connect(opts).await?);
        Ok(Model {
            conn: conn.clone(),
            unit: Arc::new(UnitModel::new(conn.clone()).await?),
            application: Arc::new(ApplicationModel::new(conn.clone()).await?),
            network: Arc::new(NetworkModel::new(conn.clone()).await?),
            device: Arc::new(DeviceModel::new(conn.clone()).await?),
            device_route: Arc::new(DeviceRouteModel::new(conn.clone()).await?),
            network_route: Arc::new(NetworkRouteModel::new(conn.clone()).await?),
            dldata_buffer: Arc::new(DlDataBufferModel::new(conn.clone()).await?),
        })
    }

    /// Get the raw database connection ([`Database`]).
    pub fn get_connection(&self) -> &Database {
        &self.conn
    }
}

#[async_trait]
impl super::Model for Model {
    async fn close(&self) -> Result<(), Box<dyn StdError>> {
        Ok(())
    }

    fn unit(&self) -> &dyn unit::UnitModel {
        self.unit.as_ref()
    }

    fn application(&self) -> &dyn application::ApplicationModel {
        self.application.as_ref()
    }

    fn network(&self) -> &dyn network::NetworkModel {
        self.network.as_ref()
    }

    fn device(&self) -> &dyn device::DeviceModel {
        self.device.as_ref()
    }

    fn device_route(&self) -> &dyn device_route::DeviceRouteModel {
        self.device_route.as_ref()
    }

    fn network_route(&self) -> &dyn network_route::NetworkRouteModel {
        self.network_route.as_ref()
    }

    fn dldata_buffer(&self) -> &dyn dldata_buffer::DlDataBufferModel {
        self.dldata_buffer.as_ref()
    }
}
