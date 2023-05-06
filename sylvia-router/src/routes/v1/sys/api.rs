use std::path::Path;

use actix_web::{web, HttpResponse, Responder};
use chrono::Utc;
use sylvia_iot_sdk::util::{err::ErrResp, strings};
use sysinfo::{CpuExt, DiskExt, SystemExt};
use tokio::task;

use super::{super::super::State, response};

/// `GET /{base}/api/v1/sys/usage`
pub async fn get_usage(state: web::Data<State>) -> impl Responder {
    let sys_info = state.sys_info.clone();

    let result = task::spawn_blocking(move || {
        let mut sys = sys_info.lock().unwrap();
        sys.refresh_cpu();
        sys.refresh_memory();
        sys.refresh_disks();

        let mut cpu_usages = vec![];
        for cpu in sys.cpus() {
            cpu_usages.push(cpu.cpu_usage().round() as usize);
        }
        let mem_total = sys.total_memory();
        let mem_used = sys.used_memory();
        let mut disk_total = 0;
        let mut disk_used = 0;
        for disk in sys.disks() {
            if disk.mount_point().eq(Path::new("/")) {
                disk_total = disk.total_space();
                disk_used = disk_total - disk.available_space();
                break;
            }
        }

        response::GetUsage {
            data: response::GetUsageData {
                cpu: cpu_usages,
                mem: response::Usage {
                    total: mem_total,
                    used: mem_used,
                },
                disk: response::Usage {
                    total: disk_total,
                    used: disk_used,
                },
            },
        }
    })
    .await;
    match result {
        Err(e) => Err(ErrResp::ErrRsc(Some(format!("get resource error: {}", e)))),
        Ok(result) => Ok(HttpResponse::Ok().json(&result)),
    }
}

/// `GET /{base}/api/v1/sys/time`
pub async fn get_time() -> impl Responder {
    HttpResponse::Ok().json(response::GetTime {
        data: response::GetTimeData {
            time: strings::time_str(&Utc::now()),
        },
    })
}
