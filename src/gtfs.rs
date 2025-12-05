use crate::model::{VehicleContent, TripUpdateResponse};
use prost::Message;
use std::time::{SystemTime, UNIX_EPOCH};
use chrono::{DateTime, FixedOffset};

// Manual definitions of GTFS Realtime structs using prost macros

#[derive(Clone, PartialEq, Message)]
pub struct FeedMessage {
    #[prost(message, required, tag = "1")]
    pub header: FeedHeader,
    #[prost(message, repeated, tag = "2")]
    pub entity: Vec<FeedEntity>,
}

#[derive(Clone, PartialEq, Message)]
pub struct FeedHeader {
    #[prost(string, required, tag = "1")]
    pub gtfs_realtime_version: String,
    #[prost(enumeration = "Incrementality", optional, tag = "2")]
    pub incrementality: Option<i32>,
    #[prost(uint64, optional, tag = "3")]
    pub timestamp: Option<u64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, prost::Enumeration)]
#[repr(i32)]
pub enum Incrementality {
    FullDataset = 0,
    Differential = 1,
}

#[derive(Clone, PartialEq, Message)]
pub struct FeedEntity {
    #[prost(string, required, tag = "1")]
    pub id: String,
    #[prost(bool, optional, tag = "2")]
    pub is_deleted: Option<bool>,
    #[prost(message, optional, tag = "3")]
    pub trip_update: Option<TripUpdate>,
    #[prost(message, optional, tag = "4")]
    pub vehicle: Option<VehiclePosition>,
}

#[derive(Clone, PartialEq, Message)]
pub struct TripUpdate {
    #[prost(message, required, tag = "1")]
    pub trip: TripDescriptor,
    #[prost(message, optional, tag = "2")]
    pub vehicle: Option<VehicleDescriptor>,
    #[prost(message, repeated, tag = "3")]
    pub stop_time_update: Vec<StopTimeUpdate>,
}

#[derive(Clone, PartialEq, Message)]
pub struct StopTimeUpdate {
    #[prost(uint32, optional, tag = "1")]
    pub stop_sequence: Option<u32>,
    #[prost(string, optional, tag = "4")]
    pub stop_id: Option<String>,
    #[prost(message, optional, tag = "2")]
    pub arrival: Option<StopTimeEvent>,
    #[prost(message, optional, tag = "3")]
    pub departure: Option<StopTimeEvent>,
}

#[derive(Clone, PartialEq, Message)]
pub struct StopTimeEvent {
    #[prost(int32, optional, tag = "1")]
    pub delay: Option<i32>,
    #[prost(int64, optional, tag = "2")]
    pub time: Option<i64>,
    #[prost(int32, optional, tag = "3")]
    pub uncertainty: Option<i32>,
}

#[derive(Clone, PartialEq, Message)]
pub struct VehiclePosition {
    #[prost(message, optional, tag = "1")]
    pub trip: Option<TripDescriptor>,
    #[prost(message, optional, tag = "8")]
    pub vehicle: Option<VehicleDescriptor>,
    #[prost(message, optional, tag = "2")]
    pub position: Option<Position>,
    #[prost(uint32, optional, tag = "3")]
    pub current_stop_sequence: Option<u32>,
    #[prost(string, optional, tag = "4")]
    pub stop_id: Option<String>,
    #[prost(enumeration = "VehicleStopStatus", optional, tag = "5")]
    pub current_status: Option<i32>,
    #[prost(uint64, optional, tag = "6")]
    pub timestamp: Option<u64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, prost::Enumeration)]
#[repr(i32)]
pub enum VehicleStopStatus {
    IncomingAt = 0,
    StoppedAt = 1,
    InTransitTo = 2,
}

#[derive(Clone, PartialEq, Message)]
pub struct TripDescriptor {
    #[prost(string, optional, tag = "1")]
    pub trip_id: Option<String>,
    #[prost(string, optional, tag = "5")]
    pub route_id: Option<String>,
    #[prost(uint32, optional, tag = "6")]
    pub direction_id: Option<u32>,
}

#[derive(Clone, PartialEq, Message)]
pub struct VehicleDescriptor {
    #[prost(string, optional, tag = "1")]
    pub id: Option<String>,
    #[prost(string, optional, tag = "2")]
    pub label: Option<String>,
    #[prost(string, optional, tag = "3")]
    pub license_plate: Option<String>,
}

#[derive(Clone, PartialEq, Message)]
pub struct Position {
    #[prost(float, required, tag = "1")]
    pub latitude: f32,
    #[prost(float, required, tag = "2")]
    pub longitude: f32,
    #[prost(float, optional, tag = "3")]
    pub bearing: Option<f32>,
    #[prost(double, optional, tag = "4")]
    pub odometer: Option<f64>,
    #[prost(float, optional, tag = "5")]
    pub speed: Option<f32>,
}

fn strip_prefix(id: &str) -> String {
    if let Some(stripped) = id.strip_prefix("DART:") {
        stripped.to_string()
    } else if let Some(stripped) = id.strip_prefix("DART-") {
        stripped.to_string()
    } else {
        id.to_string()
    }
}

pub fn convert_to_gtfs(vehicles: Vec<VehicleContent>) -> FeedMessage {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let entities = vehicles
        .into_iter()
        .map(|v| {
            let trip_desc = if let Some(trip) = v.trip {
                Some(TripDescriptor {
                    trip_id: Some(strip_prefix(&trip.id)),
                    route_id: v.route.map(|r| strip_prefix(&r.id)),
                    direction_id: v.direction_id.map(|d| d as u32),
                })
            } else {
                None
            };

            let position = Position {
                latitude: v.coordinate.lat as f32,
                longitude: v.coordinate.lng as f32,
                bearing: v.orientation.map(|o| o as f32),
                odometer: None,
                speed: None,
            };

            let vehicle_id = strip_prefix(&v.id);
            let vehicle_desc = VehicleDescriptor {
                id: Some(vehicle_id.clone()),
                label: v.head_sign,
                license_plate: None,
            };

            let vehicle_pos = VehiclePosition {
                trip: trip_desc,
                vehicle: Some(vehicle_desc),
                position: Some(position),
                current_stop_sequence: None,
                stop_id: v.stop.map(|s| strip_prefix(&s.id)),
                current_status: None,
                timestamp: None,
            };

            FeedEntity {
                id: vehicle_id,
                is_deleted: Some(false),
                trip_update: None,
                vehicle: Some(vehicle_pos),
            }
        })
        .collect();

    FeedMessage {
        header: FeedHeader {
            gtfs_realtime_version: "2.0".to_string(),
            incrementality: Some(Incrementality::FullDataset as i32),
            timestamp: Some(timestamp),
        },
        entity: entities,
    }
}

fn parse_time(t: &Option<String>) -> Option<i64> {
    if let Some(s) = t {
        if let Ok(dt) = DateTime::parse_from_rfc3339(s) {
            return Some(dt.timestamp());
        }
    }
    None
}

pub fn convert_trip_update(trip_id: String, update: TripUpdateResponse) -> TripUpdate {
    let stop_time_updates = update.entries.into_iter().map(|entry| {
        let arrival = if let Some(arr) = entry.arrival {
            Some(StopTimeEvent {
                delay: None,
                time: parse_time(&arr.real).or_else(|| parse_time(&arr.scheduled)),
                uncertainty: None,
            })
        } else {
            None
        };

        let departure = if let Some(dep) = entry.departure {
            Some(StopTimeEvent {
                delay: None,
                time: parse_time(&dep.real).or_else(|| parse_time(&dep.scheduled)),
                uncertainty: None,
            })
        } else {
            None
        };

        StopTimeUpdate {
            stop_sequence: Some(entry.stop.index),
            stop_id: Some(strip_prefix(&entry.stop.id)),
            arrival,
            departure,
        }
    }).collect();

    TripUpdate {
        trip: TripDescriptor {
            trip_id: Some(strip_prefix(&trip_id)),
            route_id: None, // Not available in TripUpdateResponse
            direction_id: None,
        },
        vehicle: None,
        stop_time_update: stop_time_updates,
    }
}
