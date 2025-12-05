pub mod auth;
pub mod client;
pub mod gtfs;
pub mod model;

pub use client::DartClient;
pub use gtfs::{convert_to_gtfs, convert_trip_update};

use anyhow::Result;
use gtfs::{FeedMessage, FeedEntity, FeedHeader, Incrementality};
use std::time::{SystemTime, UNIX_EPOCH};

pub fn fetch_dart_vehicles() -> Result<FeedMessage> {
    let client = DartClient::new()?;
    let snapshot = client.fetch_vehicles()?;
    let feed = convert_to_gtfs(snapshot.content);
    Ok(feed)
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

pub fn fetch_dart_trip_updates(trip_id: &str) -> Result<FeedMessage> {
    let client = DartClient::new()?;
    let update_response = client.fetch_trip_updates(trip_id)?;
    let trip_update = convert_trip_update(trip_id.to_string(), update_response);

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let entity = FeedEntity {
        id: strip_prefix(trip_id),
        is_deleted: Some(false),
        trip_update: Some(trip_update),
        vehicle: None,
    };

    Ok(FeedMessage {
        header: FeedHeader {
            gtfs_realtime_version: "2.0".to_string(),
            incrementality: Some(Incrementality::FullDataset as i32),
            timestamp: Some(timestamp),
        },
        entity: vec![entity],
    })
}

pub fn fetch_all_dart_trip_updates() -> Result<FeedMessage> {
    let client = DartClient::new()?;
    let snapshot = client.fetch_vehicles()?;
    
    let mut entities = Vec::new();
    
    // Collect unique trip IDs to avoid duplicate requests if any
    let mut trip_ids = std::collections::HashSet::new();
    for vehicle in snapshot.content {
        if let Some(trip) = vehicle.trip {
            // Construct the full trip ID as "FEED_ID:TRIP_ID"
            // Assuming feed_id is "DART" based on observation, but using the one from response
            let full_trip_id = format!("{}:{}", trip.feed_id, trip.id);
            trip_ids.insert(full_trip_id);
        }
    }

    println!("Fetching trip updates for {} trips...", trip_ids.len());

    for trip_id in trip_ids {
        // We ignore errors for individual trips to allow partial success
        match client.fetch_trip_updates(&trip_id) {
            Ok(update_response) => {
                let trip_update = convert_trip_update(trip_id.clone(), update_response);
                let entity = FeedEntity {
                    id: strip_prefix(&trip_id),
                    is_deleted: Some(false),
                    trip_update: Some(trip_update),
                    vehicle: None,
                };
                entities.push(entity);
            }
            Err(e) => {
                eprintln!("Failed to fetch update for trip {}: {}", trip_id, e);
            }
        }
    }

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    Ok(FeedMessage {
        header: FeedHeader {
            gtfs_realtime_version: "2.0".to_string(),
            incrementality: Some(Incrementality::FullDataset as i32),
            timestamp: Some(timestamp),
        },
        entity: entities,
    })
}
