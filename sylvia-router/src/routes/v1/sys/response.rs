use serde::Serialize;

#[derive(Serialize)]
pub struct GetUsage {
    pub data: GetUsageData,
}

#[derive(Serialize)]
pub struct GetUsageData {
    pub cpu: Vec<usize>,
    pub mem: Usage,
    pub disk: Usage,
}

#[derive(Serialize)]
pub struct GetTime {
    pub data: GetTimeData,
}

#[derive(Serialize)]
pub struct GetTimeData {
    pub time: String,
}

#[derive(Serialize)]
pub struct Usage {
    pub total: u64,
    pub used: u64,
}
