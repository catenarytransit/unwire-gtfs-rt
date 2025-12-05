use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct VehicleSnapshotResponse {
    pub content: Vec<VehicleContent>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VehicleContent {
    pub id: String,
    pub transit_mode: Option<String>,
    pub orientation: Option<f64>,
    pub coordinate: Coordinate,
    pub stop: Option<EntityRef>,
    pub route: Option<EntityRef>,
    pub trip: Option<EntityRef>,
    pub short_code: Option<String>,
    pub head_sign: Option<String>,
    pub direction_id: Option<i32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Coordinate {
    pub lat: f64,
    pub lng: f64,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EntityRef {
    pub id: String,
    pub feed_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TripUpdateResponse {
    pub state: Option<String>,
    pub entries: Vec<TripUpdateEntry>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TripUpdateEntry {
    pub stop: StopInfo,
    pub arrival: Option<TimeInfo>,
    pub departure: Option<TimeInfo>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StopInfo {
    pub id: String,
    pub name: Option<String>,
    pub index: u32,
    pub coordinate: Option<Coordinate>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimeInfo {
    pub state: Option<String>,
    pub scheduled: Option<String>,
    pub real: Option<String>,
}
