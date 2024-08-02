use std::path::Path;

use axum::{extract::State, response::IntoResponse};
use chrono::Utc;
use sylvia_iot_sdk::util::{err::ErrResp, http::Json, strings};
use tokio::task;

use super::{super::super::State as AppState, response};

/// `GET /{base}/api/v1/sys/usage`
pub async fn get_usage(State(state): State<AppState>) -> impl IntoResponse {
    let sys_info = state.sys_info.clone();
    let disk_info = state.disk_info.clone();

    let result = task::spawn_blocking(move || {
        let mut sys = sys_info.lock().unwrap();
        let mut disk = disk_info.lock().unwrap();
        sys.refresh_cpu_all();
        sys.refresh_memory();
        disk.refresh_list();

        let mut cpu_usages = vec![];
        for cpu in sys.cpus() {
            cpu_usages.push(cpu.cpu_usage().round() as usize);
        }
        let mem_total = sys.total_memory();
        let mem_used = sys.used_memory();
        let mut disk_total = 0;
        let mut disk_used = 0;
        for disk in disk.list() {
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
        Ok(result) => Ok(Json(result)),
    }
}

/// `GET /{base}/api/v1/sys/time`
pub async fn get_time() -> impl IntoResponse {
    Json(response::GetTime {
        data: response::GetTimeData {
            time: strings::time_str(&Utc::now()),
        },
    })
}
