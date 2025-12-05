pub mod auth;
pub mod client;
pub mod gtfs;
pub mod model;

pub use client::UnwireClient;
pub use gtfs::{convert_to_gtfs, convert_trip_update};

use anyhow::Result;
use futures::{StreamExt, stream};
use gtfs::{FeedEntity, FeedHeader, FeedMessage, Incrementality};
use model::VehicleContent;
use std::collections::HashSet;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FeedId {
    Dart,
    Ccrta,
    Mcallen,
    Fwta,
}

impl FeedId {
    pub const fn as_str(&self) -> &'static str {
        match self {
            FeedId::Dart => "DART",
            FeedId::Ccrta => "CCRTA",
            FeedId::Mcallen => "MCALLEN",
            FeedId::Fwta => "FWTA",
        }
    }

    pub const fn all() -> [FeedId; 4] {
        [FeedId::Dart, FeedId::Ccrta, FeedId::Mcallen, FeedId::Fwta]
    }
}

pub fn strip_prefix(id: &str) -> String {
    for feed in FeedId::all().iter() {
        let prefix = feed.as_str();
        if let Some(rest) = id.strip_prefix(prefix) {
            if let Some(stripped) = rest.strip_prefix(':').or_else(|| rest.strip_prefix('-')) {
                return stripped.to_string();
            }
        }
    }
    id.to_string()
}

fn vehicle_matches_feed(vehicle: &VehicleContent, feed: FeedId) -> bool {
    let prefix = feed.as_str();
    vehicle
        .id
        .strip_prefix(prefix)
        .map(|rest| rest.starts_with('-') || rest.starts_with(':'))
        .unwrap_or(false)
}

fn normalize_trip_id(feed: FeedId, trip_id: &str) -> String {
    if trip_id.starts_with(feed.as_str()) {
        trip_id.to_string()
    } else {
        format!("{}:{}", feed.as_str(), trip_id)
    }
}

pub async fn fetch_dart_vehicles() -> Result<FeedMessage> {
    fetch_feed_vehicles(FeedId::Dart).await
}

pub async fn fetch_feed_vehicles(feed: FeedId) -> Result<FeedMessage> {
    let client = UnwireClient::new().await?;
    let snapshot = client.fetch_vehicles().await?;
    let filtered: Vec<VehicleContent> = snapshot
        .content
        .into_iter()
        .filter(|vehicle| vehicle_matches_feed(vehicle, feed))
        .collect();

    let feed_message = convert_to_gtfs(filtered);
    Ok(feed_message)
}

pub async fn fetch_feed_trip_update(feed: FeedId, trip_id: &str) -> Result<FeedMessage> {
    let client = UnwireClient::new().await?;
    let normalized_trip_id = normalize_trip_id(feed, trip_id);
    let update_response = client.fetch_trip_updates(&normalized_trip_id).await?;
    let trip_update = convert_trip_update(normalized_trip_id.clone(), update_response);

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let entity = FeedEntity {
        id: strip_prefix(&normalized_trip_id),
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

pub async fn fetch_dart_trip_updates(trip_id: &str) -> Result<FeedMessage> {
    fetch_feed_trip_update(FeedId::Dart, trip_id).await
}

pub async fn fetch_all_dart_trip_updates() -> Result<FeedMessage> {
    fetch_all_feed_trip_updates(FeedId::Dart).await
}

pub async fn fetch_all_feed_trip_updates(feed: FeedId) -> Result<FeedMessage> {
    let client = UnwireClient::new().await?;
    let snapshot = client.fetch_vehicles().await?;

    let mut trip_ids: HashSet<String> = HashSet::new();
    for vehicle in snapshot
        .content
        .into_iter()
        .filter(|v| vehicle_matches_feed(v, feed))
    {
        if let Some(trip) = vehicle.trip {
            let full_trip_id = format!("{}:{}", trip.feed_id, trip.id);
            if full_trip_id.starts_with(feed.as_str()) {
                trip_ids.insert(full_trip_id);
            }
        }
    }

    println!("Fetching trip updates for {} trips...", trip_ids.len());

    let concurrency = trip_ids.len().clamp(4, 16);

    let mut stream = stream::iter(trip_ids.into_iter().map(|trip_id| {
        let client = client.clone();
        async move {
            match client.fetch_trip_updates(&trip_id).await {
                Ok(update_response) => {
                    let trip_update = convert_trip_update(trip_id.clone(), update_response);
                    let entity = FeedEntity {
                        id: strip_prefix(&trip_id),
                        is_deleted: Some(false),
                        trip_update: Some(trip_update),
                        vehicle: None,
                    };
                    Some(entity)
                }
                Err(e) => {
                    eprintln!("Failed to fetch update for trip {}: {}", trip_id, e);
                    None
                }
            }
        }
    }))
    .buffer_unordered(concurrency);

    let mut entities = Vec::new();
    while let Some(entity) = stream.next().await {
        if let Some(entity) = entity {
            entities.push(entity);
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
