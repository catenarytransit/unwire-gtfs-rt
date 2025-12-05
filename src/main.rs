use dart_fetcher::{fetch_dart_vehicles, fetch_all_dart_trip_updates};

fn main() {
    env_logger::init();

    println!("--- Fetching Vehicles ---");
    match fetch_dart_vehicles() {
        Ok(feed) => {
            println!("Successfully fetched {} vehicles", feed.entity.len());
            for entity in feed.entity.iter().take(5) {
                println!("Vehicle: {:?}", entity.vehicle);
            }
        }
        Err(e) => eprintln!("Error fetching vehicles: {}", e),
    }

    println!("\n--- Fetching All Trip Updates ---");
    match fetch_all_dart_trip_updates() {
        Ok(feed) => {
            println!("Successfully fetched {} trip updates", feed.entity.len());
            for entity in feed.entity.iter().take(5) {
                println!("Trip Update: {:?}", entity.trip_update);
            }
        }
        Err(e) => eprintln!("Error fetching trip updates: {}", e),
    }
}
