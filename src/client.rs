use crate::auth::Authenticator;
use crate::model::{VehicleSnapshotResponse, TripUpdateResponse};
use anyhow::{Context, Result};
use reqwest::blocking::Client;
use std::fs::OpenOptions;
use std::io::Write;

const BASE_URL: &str = "https://ssge-ticketing.us.unwire.com/api-gateway";
const BASE_PATH: &str = "/api-gateway";
const VEHICLES_ENDPOINT: &str = "/v3/api/ttools/vehicles/snapshot";
const TRIPS_ENDPOINT_PREFIX: &str = "/v5/api/ttools/trips/";
const TRIPS_ENDPOINT_SUFFIX: &str = "/timetable";

fn log_debug(msg: &str) {
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open("debug.log") {
        let _ = writeln!(file, "{}", msg);
    }
}

pub struct DartClient {
    client: Client,
    authenticator: Authenticator,
}

impl DartClient {
    pub fn new() -> Result<Self> {
        let mut authenticator = Authenticator::new()?;
        authenticator.register().context("failed to register app instance")?;
        
        Ok(Self {
            client: Client::new(),
            authenticator,
        })
    }

    pub fn fetch_vehicles(&self) -> Result<VehicleSnapshotResponse> {
        let params = vec![
            ("lat", "32.78132590720856"),
            ("lng", "-96.7982304096222"),
            ("radius", "616.1041125022343"),
            ("provider", "2"),
            ("onlyFeatured", "false"),
        ];
        
        let mut signing_params: Vec<(String, String)> = params.iter()
            .map(|(k, v)| (k.to_lowercase(), v.to_string()))
            .collect();
        signing_params.sort_by(|a, b| a.0.cmp(&b.0));
        
        let signing_query_parts: Vec<String> = signing_params.iter()
            .map(|(k, v)| {
                let encoded_v = urlencoding::encode(v);
                format!("{}={}", k, encoded_v).to_lowercase()
            })
            .collect();
        let signing_query = signing_query_parts.join("&");
        
        let full_path = format!("{}{}", BASE_PATH, VEHICLES_ENDPOINT);
        let headers = self.authenticator.get_auth_headers(
            "GET",
            &full_path,
            None,
            Some(&signing_query),
        )?;

        let url = format!("{}{}", BASE_URL, VEHICLES_ENDPOINT);
        let mut req = self.client.get(&url)
            .query(&params);
            
        for (k, v) in headers {
            req = req.header(k, v);
        }
        
        req = self.add_common_headers(req);

        let resp = req.send().context("failed to send vehicle request")?;
        
        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().unwrap_or_default();
            anyhow::bail!("Vehicle fetch failed: {} - {}", status, text);
        }
        
        let snapshot: VehicleSnapshotResponse = resp.json().context("failed to parse vehicle response")?;
        
        Ok(snapshot)
    }

    pub fn fetch_trip_updates(&self, trip_id: &str) -> Result<TripUpdateResponse> {
        // Endpoint: /v5/api/ttools/trips/{trip_id}/timetable
        let endpoint = format!("{}{}{}", TRIPS_ENDPOINT_PREFIX, trip_id, TRIPS_ENDPOINT_SUFFIX);
        let full_path = format!("{}{}", BASE_PATH, endpoint);
        
        let headers = self.authenticator.get_auth_headers(
            "GET",
            &full_path,
            None,
            None,
        )?;

        let url = format!("{}{}", BASE_URL, endpoint);
        let mut req = self.client.get(&url);
            
        for (k, v) in headers {
            req = req.header(k, v);
        }
        
        req = self.add_common_headers(req);

        let resp = req.send().context("failed to send trip update request")?;
        
        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().unwrap_or_default();
            anyhow::bail!("Trip update fetch failed: {} - {}", status, text);
        }

        let text = resp.text().context("failed to read response text")?;
        // log_debug(&format!("Trip update response for {}: {}", trip_id, text));
        
        let response: TripUpdateResponse = serde_json::from_str(&text).context("failed to parse trip update response")?;
        
        Ok(response)
    }

    fn add_common_headers(&self, req: reqwest::blocking::RequestBuilder) -> reqwest::blocking::RequestBuilder {
        req
            .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:145.0) Gecko/20100101 Firefox/145.0")
            .header("Accept", "*/*")
            .header("Accept-Language", "en-CA,en-US;q=0.7,en;q=0.3")
            .header("Accept-Encoding", "gzip, deflate, br, zstd")
            .header("Referer", "https://dart.mygopass.org/")
            .header("Origin", "https://dart.mygopass.org")
            .header("Connection", "keep-alive")
            .header("Sec-Fetch-Dest", "empty")
            .header("Sec-Fetch-Mode", "cors")
            .header("Sec-Fetch-Site", "cross-site")
            .header("DNT", "1")
            .header("Sec-GPC", "1")
            .header("Priority", "u=4")
            .header("Pragma", "no-cache")
            .header("Cache-Control", "no-cache")
    }
}
