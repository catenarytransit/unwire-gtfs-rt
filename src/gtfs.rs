use crate::model::{TripUpdateResponse, VehicleContent};
use crate::strip_prefix;
use chrono::DateTime;
use std::time::{SystemTime, UNIX_EPOCH};

pub use gtfs_realtime::{
    feed_header::Incrementality,
    trip_update::{StopTimeEvent, StopTimeUpdate},
    vehicle_position::VehicleStopStatus,
    FeedEntity,
    FeedHeader,
    FeedMessage,
    Position,
    TripDescriptor,
    TripUpdate,
    VehicleDescriptor,
    VehiclePosition,
};

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
                    start_time: None,
                    start_date: None,
                    schedule_relationship: None,
                    modified_trip: None,
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
                wheelchair_accessible: None,
            };

            let vehicle_pos = VehiclePosition {
                trip: trip_desc,
                vehicle: Some(vehicle_desc),
                position: Some(position),
                current_stop_sequence: None,
                stop_id: v.stop.map(|s| strip_prefix(&s.id)),
                current_status: None,
                timestamp: None,
                congestion_level: None,
                occupancy_status: None,
                occupancy_percentage: None,
                multi_carriage_details: Vec::new(),
            };

            FeedEntity {
                id: vehicle_id,
                is_deleted: Some(false),
                trip_update: None,
                vehicle: Some(vehicle_pos),
                alert: None,
                shape: None,
                stop: None,
                trip_modifications: None,
            }
        })
        .collect();

    FeedMessage {
        header: FeedHeader {
            gtfs_realtime_version: "2.0".to_string(),
            incrementality: Some(Incrementality::FullDataset as i32),
            timestamp: Some(timestamp),
            feed_version: None,
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
    let stop_time_updates = update
        .entries
        .into_iter()
        .map(|entry| {
            let arrival = if let Some(arr) = entry.arrival {
                Some(StopTimeEvent {
                    delay: None,
                    time: parse_time(&arr.real).or_else(|| parse_time(&arr.scheduled)),
                    uncertainty: None,
                    scheduled_time: None,
                })
            } else {
                None
            };

            let departure = if let Some(dep) = entry.departure {
                Some(StopTimeEvent {
                    delay: None,
                    time: parse_time(&dep.real).or_else(|| parse_time(&dep.scheduled)),
                    uncertainty: None,
                    scheduled_time: None,
                })
            } else {
                None
            };

            StopTimeUpdate {
                stop_sequence: Some(entry.stop.index),
                stop_id: Some(strip_prefix(&entry.stop.id)),
                arrival,
                departure,
                departure_occupancy_status: None,
                schedule_relationship: None,
                stop_time_properties: None,
            }
        })
        .collect();

    TripUpdate {
        trip: TripDescriptor {
            trip_id: Some(strip_prefix(&trip_id)),
            route_id: None, // Not available in TripUpdateResponse
            direction_id: None,
            start_time: None,
            start_date: None,
            schedule_relationship: None,
            modified_trip: None,
        },
        vehicle: None,
        stop_time_update: stop_time_updates,
        timestamp: None,
        delay: None,
        trip_properties: None,
    }
}
