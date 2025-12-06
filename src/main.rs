use unwire_gtfs_rt::{FeedId, fetch_all_feed_trip_updates, fetch_feed_vehicles};

#[tokio::main]
async fn main() {
    env_logger::init();

    let feed = FeedId::Dart;

    println!("--- Fetching Vehicles ---");
    match fetch_feed_vehicles(feed).await {
        Ok(feed) => {
            println!("Successfully fetched {} vehicles", feed.entity.len());
            for entity in feed.entity.iter().take(5) {
                println!("Vehicle: {:?}", entity.vehicle);
            }
        }
        Err(e) => eprintln!("Error fetching vehicles: {}", e),
    }

    println!("\n--- Fetching All Trip Updates ---");
    match fetch_all_feed_trip_updates(feed).await {
        Ok(feed) => {
            println!("Successfully fetched {} trip updates", feed.entity.len());
            for entity in feed.entity.iter().take(5) {
                println!("Trip Update: {:?}", entity.trip_update);
            }
        }
        Err(e) => eprintln!("Error fetching trip updates: {}", e),
    }
}
