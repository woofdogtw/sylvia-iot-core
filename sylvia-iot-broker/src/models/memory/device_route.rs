use std::{error::Error as StdError, num::NonZeroUsize, sync::Arc};

use async_trait::async_trait;
use lru::LruCache;
use tokio::sync::RwLock;

use super::super::{
    device::{QueryCond as DeviceQueryCond, QueryOneCond as DeviceQueryOneCond},
    device_route::{
        DelCachePubQueryCond, DelCacheQueryCond, DeviceRouteCache, DeviceRouteCacheDlData,
        DeviceRouteCacheUlData, GetCachePubQueryCond, GetCacheQueryCond, ListOptions,
        ListQueryCond,
    },
    Model,
};

pub struct Cache {
    model: Arc<dyn Model>,
    uldata: Arc<RwLock<LruCache<String, Option<DeviceRouteCacheUlData>>>>,
    dldata: Arc<RwLock<LruCache<String, Option<DeviceRouteCacheDlData>>>>,
    dldata_pub: Arc<RwLock<LruCache<String, Option<DeviceRouteCacheDlData>>>>,
}

pub struct Options {
    pub uldata_size: usize,
    pub dldata_size: usize,
    pub dldata_pub_size: usize,
}

const DEF_SIZE: usize = 10_000;

impl Cache {
    pub fn new(opts: &Options, model: Arc<dyn Model>) -> Self {
        let (uldata, dldata, dldata_pub) = unsafe {
            (
                NonZeroUsize::new_unchecked(opts.uldata_size),
                NonZeroUsize::new_unchecked(opts.dldata_size),
                NonZeroUsize::new_unchecked(opts.dldata_pub_size),
            )
        };
        Cache {
            model,
            uldata: Arc::new(RwLock::new(LruCache::new(uldata))),
            dldata: Arc::new(RwLock::new(LruCache::new(dldata))),
            dldata_pub: Arc::new(RwLock::new(LruCache::new(dldata_pub))),
        }
    }
}

#[async_trait]
impl DeviceRouteCache for Cache {
    async fn clear(&self) -> Result<(), Box<dyn StdError>> {
        // To collect all locks before clearing cache.
        let mut lock1 = self.uldata.write().await;
        let mut lock2 = self.dldata.write().await;
        let mut lock3 = self.dldata_pub.write().await;
        lock1.clear();
        lock2.clear();
        lock3.clear();
        Ok(())
    }

    async fn get_uldata(
        &self,
        device_id: &str,
    ) -> Result<Option<DeviceRouteCacheUlData>, Box<dyn StdError>> {
        {
            let mut lock = self.uldata.write().await;
            if let Some(value) = lock.get(device_id) {
                return Ok(value.clone());
            }
        }

        let opts = ListOptions {
            cond: &ListQueryCond {
                device_id: Some(device_id),
                ..Default::default()
            },
            offset: None,
            limit: None,
            sort: None,
            cursor_max: None,
        };
        let (mut routes, _) = self.model.device_route().list(&opts, None).await?;
        let data: Option<DeviceRouteCacheUlData> = match routes.len() {
            0 => None,
            _ => {
                let mut routes_data = vec![];
                for r in routes.iter() {
                    routes_data.push(format!("{}.{}", r.unit_code, r.application_code))
                }
                let route = routes.pop().unwrap();
                Some(DeviceRouteCacheUlData {
                    app_mgr_keys: routes_data,
                    unit_id: route.unit_id,
                    application_id: route.application_id,
                    network_id: route.network_id,
                })
            }
        };
        let _ = self.set_uldata(device_id, data.as_ref()).await;
        Ok(data)
    }

    async fn set_uldata(
        &self,
        device_id: &str,
        value: Option<&DeviceRouteCacheUlData>,
    ) -> Result<(), Box<dyn StdError>> {
        let key = device_id.to_string();
        let mut lock = self.uldata.write().await;
        let _ = match value {
            None => lock.push(key, None),
            Some(value) => lock.push(key, Some(value.clone())),
        };
        Ok(())
    }

    async fn del_uldata(&self, device_id: &str) -> Result<(), Box<dyn StdError>> {
        let mut lock = self.uldata.write().await;
        lock.pop(device_id);
        Ok(())
    }

    async fn get_dldata(
        &self,
        cond: &GetCacheQueryCond,
    ) -> Result<Option<DeviceRouteCacheDlData>, Box<dyn StdError>> {
        let key = format!(
            "{}.{}.{}",
            cond.unit_code, cond.network_code, cond.network_addr
        );

        {
            let mut lock = self.dldata.write().await;
            if let Some(value) = lock.get(&key) {
                match value {
                    None => return Ok(None),
                    Some(value) => return Ok(Some(value.clone())),
                }
            }
        }

        let dev_cond = DeviceQueryCond {
            device: Some(DeviceQueryOneCond {
                unit_code: Some(cond.unit_code),
                network_code: cond.network_code,
                network_addr: cond.network_addr,
            }),
            ..Default::default()
        };
        let device = self.model.device().get(&dev_cond).await?;
        let data = match device {
            None => None,
            Some(device) => match device.unit_code.as_ref() {
                // This should not occur!
                None => None,
                Some(unit_code) => Some(DeviceRouteCacheDlData {
                    net_mgr_key: format!("{}.{}", unit_code, cond.network_code),
                    network_id: device.network_id,
                    network_addr: device.network_addr,
                    device_id: device.device_id,
                }),
            },
        };
        let _ = self.set_dldata(cond, data.as_ref()).await;
        Ok(data)
    }

    async fn set_dldata(
        &self,
        cond: &GetCacheQueryCond,
        value: Option<&DeviceRouteCacheDlData>,
    ) -> Result<(), Box<dyn StdError>> {
        let key = format!(
            "{}.{}.{}",
            cond.unit_code, cond.network_code, cond.network_addr
        );
        let mut lock = self.dldata.write().await;
        let _ = match value {
            None => lock.push(key, None),
            Some(value) => lock.push(key, Some(value.clone())),
        };
        Ok(())
    }

    async fn del_dldata(&self, cond: &DelCacheQueryCond) -> Result<(), Box<dyn StdError>> {
        let key = match cond.network_code {
            None => {
                // Remove all routes of the unit.
                cond.unit_code.to_string()
            }
            Some(code) => match cond.network_addr {
                None => {
                    // Remove all routes of the network.
                    format!("{}.{}", cond.unit_code, code)
                }
                Some(addr) => {
                    let key = format!("{}.{}.{}", cond.unit_code, code, addr);
                    let mut lock = self.dldata.write().await;
                    let _ = lock.pop(&key);
                    return Ok(());
                }
            },
        };
        {
            let mut lock = self.dldata.write().await;
            loop {
                let mut rm_key = None;
                for (k, _) in lock.iter() {
                    if k.starts_with(key.as_str()) {
                        rm_key = Some(k.clone());
                        break;
                    }
                }
                match rm_key {
                    None => break,
                    Some(key) => {
                        let _ = lock.pop(&key);
                    }
                }
            }
        }
        Ok(())
    }

    async fn get_dldata_pub(
        &self,
        cond: &GetCachePubQueryCond,
    ) -> Result<Option<DeviceRouteCacheDlData>, Box<dyn StdError>> {
        let key = format!("{}.{}", cond.unit_id, cond.device_id);

        {
            let mut lock = self.dldata_pub.write().await;
            if let Some(value) = lock.get(&key) {
                match value {
                    None => return Ok(None),
                    Some(value) => return Ok(Some(value.clone())),
                }
            }
        }

        let dev_cond = DeviceQueryCond {
            unit_id: Some(cond.unit_id),
            device_id: Some(cond.device_id),
            ..Default::default()
        };
        let device = self.model.device().get(&dev_cond).await?;
        let data = match device {
            None => None,
            Some(device) => match device.unit_code.as_ref() {
                None => Some(DeviceRouteCacheDlData {
                    net_mgr_key: format!(".{}", device.network_code),
                    network_id: device.network_id,
                    network_addr: device.network_addr,
                    device_id: device.device_id,
                }),
                Some(unit_code) => Some(DeviceRouteCacheDlData {
                    net_mgr_key: format!("{}.{}", unit_code, device.network_code),
                    network_id: device.network_id,
                    network_addr: device.network_addr,
                    device_id: device.device_id,
                }),
            },
        };
        let _ = self.set_dldata_pub(cond, data.as_ref()).await;
        Ok(data)
    }

    async fn set_dldata_pub(
        &self,
        cond: &GetCachePubQueryCond,
        value: Option<&DeviceRouteCacheDlData>,
    ) -> Result<(), Box<dyn StdError>> {
        let key = format!("{}.{}", cond.unit_id, cond.device_id);
        let mut lock = self.dldata_pub.write().await;
        let _ = match value {
            None => lock.push(key, None),
            Some(value) => lock.push(key, Some(value.clone())),
        };
        Ok(())
    }

    async fn del_dldata_pub(&self, cond: &DelCachePubQueryCond) -> Result<(), Box<dyn StdError>> {
        let key = match cond.device_id {
            None => {
                // Remove all routes of the unit.
                cond.unit_id.to_string()
            }
            Some(id) => {
                let key = format!("{}.{}", cond.unit_id, id);
                {
                    let mut lock = self.dldata_pub.write().await;
                    lock.pop(&key);
                }
                return Ok(());
            }
        };
        {
            let mut lock = self.dldata_pub.write().await;
            loop {
                let mut rm_key = None;
                for (k, _) in lock.iter() {
                    if k.starts_with(key.as_str()) {
                        rm_key = Some(k.clone());
                        break;
                    }
                }
                match rm_key {
                    None => break,
                    Some(key) => {
                        let _ = lock.pop(&key);
                    }
                }
            }
        }
        Ok(())
    }
}

impl Default for Options {
    fn default() -> Self {
        Options {
            uldata_size: DEF_SIZE,
            dldata_size: DEF_SIZE,
            dldata_pub_size: DEF_SIZE,
        }
    }
}
