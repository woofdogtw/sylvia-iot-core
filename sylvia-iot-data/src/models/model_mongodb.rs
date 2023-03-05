//! Pure MongoDB model.

use std::{error::Error as StdError, sync::Arc};

use async_trait::async_trait;
use mongodb::Database;

use super::{
    application_dldata, application_uldata, coremgr_opdata,
    mongodb::{
        application_dldata::Model as ApplicationDlDataModel,
        application_uldata::Model as ApplicationUlDataModel,
        conn::{self, Options},
        coremgr_opdata::Model as CoremgrOpDataModel,
        network_dldata::Model as NetworkDlDataModel,
        network_uldata::Model as NetworkUlDataModel,
    },
    network_dldata, network_uldata,
};

/// Pure MongoDB model.
#[derive(Clone)]
pub struct Model {
    conn: Arc<Database>,
    application_dldata: Arc<ApplicationDlDataModel>,
    application_uldata: Arc<ApplicationUlDataModel>,
    coremgr_opdata: Arc<CoremgrOpDataModel>,
    network_dldata: Arc<NetworkDlDataModel>,
    network_uldata: Arc<NetworkUlDataModel>,
}

impl Model {
    /// Create an instance.
    pub async fn new(opts: &Options) -> Result<Self, Box<dyn StdError>> {
        let conn = Arc::new(conn::connect(opts).await?);
        Ok(Model {
            conn: conn.clone(),
            application_dldata: Arc::new(ApplicationDlDataModel::new(conn.clone()).await?),
            application_uldata: Arc::new(ApplicationUlDataModel::new(conn.clone()).await?),
            coremgr_opdata: Arc::new(CoremgrOpDataModel::new(conn.clone()).await?),
            network_dldata: Arc::new(NetworkDlDataModel::new(conn.clone()).await?),
            network_uldata: Arc::new(NetworkUlDataModel::new(conn.clone()).await?),
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

    fn application_dldata(&self) -> &dyn application_dldata::ApplicationDlDataModel {
        self.application_dldata.as_ref()
    }

    fn application_uldata(&self) -> &dyn application_uldata::ApplicationUlDataModel {
        self.application_uldata.as_ref()
    }

    fn coremgr_opdata(&self) -> &dyn coremgr_opdata::CoremgrOpDataModel {
        self.coremgr_opdata.as_ref()
    }

    fn network_dldata(&self) -> &dyn network_dldata::NetworkDlDataModel {
        self.network_dldata.as_ref()
    }

    fn network_uldata(&self) -> &dyn network_uldata::NetworkUlDataModel {
        self.network_uldata.as_ref()
    }
}
