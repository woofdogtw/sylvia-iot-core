use std::{error::Error as StdError, num::NonZeroUsize, sync::Arc};

use async_lock::RwLock;
use async_trait::async_trait;
use lru::LruCache;

use super::super::{
    device::{
        DelCacheQueryCond, DeviceCache, DeviceCacheItem, GetCacheQueryCond, QueryCond, QueryOneCond,
    },
    Model,
};

pub struct Cache {
    model: Arc<dyn Model>,
    uldata: Arc<RwLock<LruCache<String, Option<DeviceCacheItem>>>>,
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
impl DeviceCache for Cache {
    async fn clear(&self) -> Result<(), Box<dyn StdError>> {
        // To collect all locks before clearing cache.
        let mut lock = self.uldata.write().await;
        lock.clear();
        Ok(())
    }

    async fn get(
        &self,
        cond: &GetCacheQueryCond,
    ) -> Result<Option<DeviceCacheItem>, Box<dyn StdError>> {
        // Try to hit cache first, or returns a model query condition.
        let model_cond = match cond {
            GetCacheQueryCond::CodeAddr(cond) => {
                let key = match cond.unit_code {
                    None => format!(".{}.{}", cond.network_code, cond.network_addr),
                    Some(unit) => format!("{}.{}.{}", unit, cond.network_code, cond.network_addr),
                };
                {
                    let mut lock = self.uldata.write().await;
                    if let Some(value) = lock.get(&key) {
                        return Ok(value.clone());
                    }
                }
                QueryCond {
                    device: Some(QueryOneCond {
                        unit_code: cond.unit_code,
                        network_code: cond.network_code,
                        network_addr: cond.network_addr,
                    }),
                    ..Default::default()
                }
            }
        };

        let item = match self.model.device().get(&model_cond).await? {
            None => None,
            Some(device) => Some(DeviceCacheItem {
                device_id: device.device_id,
            }),
        };
        let _ = self.set(cond, item.as_ref()).await;
        Ok(item)
    }

    async fn set(
        &self,
        cond: &GetCacheQueryCond,
        value: Option<&DeviceCacheItem>,
    ) -> Result<(), Box<dyn StdError>> {
        match cond {
            GetCacheQueryCond::CodeAddr(cond) => {
                let key = match cond.unit_code {
                    None => format!(".{}.{}", cond.network_code, cond.network_addr),
                    Some(unit) => format!("{}.{}.{}", unit, cond.network_code, cond.network_addr),
                };
                {
                    let mut lock = self.uldata.write().await;
                    let _ = match value {
                        None => lock.push(key, None),
                        Some(value) => lock.push(key, Some(value.clone())),
                    };
                }
            }
        }
        Ok(())
    }

    async fn del(&self, cond: &DelCacheQueryCond) -> Result<(), Box<dyn StdError>> {
        let key = match cond.network_code {
            None => {
                // Disallow deleting all devices of public networks.
                if cond.unit_code.len() == 0 {
                    return Ok(());
                }

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
                    {
                        let mut lock = self.uldata.write().await;
                        lock.pop(&key);
                    }
                    return Ok(());
                }
            },
        };
        {
            let mut lock = self.uldata.write().await;
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
        }
    }
}
