use std::{error::Error as StdError, num::NonZeroUsize, sync::Arc};

use async_lock::RwLock;
use async_trait::async_trait;
use lru::LruCache;

use super::super::{
    network_route::{ListOptions, ListQueryCond, NetworkRouteCache, NetworkRouteCacheUlData},
    Model,
};

pub struct Cache {
    model: Arc<dyn Model>,
    uldata: Arc<RwLock<LruCache<String, Option<NetworkRouteCacheUlData>>>>,
}

pub struct Options {
    pub uldata_size: usize,
}

const DEF_SIZE: usize = 10_000;

impl Cache {
    pub fn new(opts: &Options, model: Arc<dyn Model>) -> Self {
        let uldata = unsafe { NonZeroUsize::new_unchecked(opts.uldata_size) };
        Cache {
            model,
            uldata: Arc::new(RwLock::new(LruCache::new(uldata))),
        }
    }
}

#[async_trait]
impl NetworkRouteCache for Cache {
    async fn clear(&self) -> Result<(), Box<dyn StdError>> {
        let mut lock = self.uldata.write().await;
        lock.clear();
        Ok(())
    }

    async fn get_uldata(
        &self,
        network_id: &str,
    ) -> Result<Option<NetworkRouteCacheUlData>, Box<dyn StdError>> {
        {
            let mut lock = self.uldata.write().await;
            if let Some(value) = lock.get(network_id) {
                match value {
                    None => return Ok(None),
                    Some(value) => return Ok(Some(value.clone())),
                }
            }
        }

        let opts = ListOptions {
            cond: &ListQueryCond {
                network_id: Some(network_id),
                ..Default::default()
            },
            offset: None,
            limit: None,
            sort: None,
            cursor_max: None,
        };
        let (routes, _) = self.model.network_route().list(&opts, None).await?;
        let data = match routes.len() {
            0 => None,
            _ => {
                let mut routes_data = vec![];
                for r in routes.iter() {
                    routes_data.push(format!("{}.{}", r.unit_code, r.application_code))
                }
                Some(NetworkRouteCacheUlData {
                    app_mgr_keys: routes_data,
                })
            }
        };
        let _ = self.set_uldata(network_id, data.as_ref()).await;
        Ok(data)
    }

    async fn set_uldata(
        &self,
        network_id: &str,
        value: Option<&NetworkRouteCacheUlData>,
    ) -> Result<(), Box<dyn StdError>> {
        let key = network_id.to_string();
        let mut lock = self.uldata.write().await;
        let _ = match value {
            None => lock.push(key, None),
            Some(value) => lock.push(key, Some(value.clone())),
        };
        Ok(())
    }

    async fn del_uldata(&self, network_id: &str) -> Result<(), Box<dyn StdError>> {
        let mut lock = self.uldata.write().await;
        lock.pop(network_id);
        Ok(())
    }
}

impl Default for Options {
    fn default() -> Self {
        Options {
            uldata_size: DEF_SIZE,
        }
    }
}
