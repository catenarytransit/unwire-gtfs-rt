use crate::auth::Authenticator;
use crate::model::{TripUpdateResponse, VehicleSnapshotResponse};
use anyhow::{Context, Result};
use reqwest::Client;
use std::fs::OpenOptions;
use std::io::Write;

const BASE_URL: &str = "https://ssge-ticketing.us.unwire.com/api-gateway";
const BASE_PATH: &str = "/api-gateway";
const VEHICLES_ENDPOINT: &str = "/v3/api/ttools/vehicles/snapshot";
const TRIPS_ENDPOINT_PREFIX: &str = "/v5/api/ttools/trips/";
const TRIPS_ENDPOINT_SUFFIX: &str = "/timetable";

fn log_debug(msg: &str) {
    if let Ok(mut file) = OpenOptions::new()
        .create(true)
        .append(true)
        .open("debug.log")
    {
        let _ = writeln!(file, "{}", msg);
    }
}

#[derive(Clone)]
pub struct UnwireClient {
    client: Client,
    authenticator: Authenticator,
}

impl UnwireClient {
    pub async fn new() -> Result<Self> {
        let mut authenticator = Authenticator::new()?;
        authenticator
            .register()
            .await
            .context("failed to register app instance")?;

        Ok(Self {
            client: Client::new(),
            authenticator,
        })
    }

    pub async fn fetch_vehicles(&self) -> Result<VehicleSnapshotResponse> {
        let full_path = format!("{}{}", BASE_PATH, VEHICLES_ENDPOINT);
        let headers = self
            .authenticator
            .get_auth_headers("GET", &full_path, None, None)?;

        let url = format!("{}{}", BASE_URL, VEHICLES_ENDPOINT);
        let mut req = self.client.get(&url);

        for (k, v) in headers {
            req = req.header(k, v);
        }

        req = self.add_common_headers(req);

        let resp = req.send().await.context("failed to send vehicle request")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            anyhow::bail!("Vehicle fetch failed: {} - {}", status, text);
        }

        let snapshot: VehicleSnapshotResponse = resp
            .json()
            .await
            .context("failed to parse vehicle response")?;

        Ok(snapshot)
    }

    pub async fn fetch_trip_updates(&self, trip_id: &str) -> Result<TripUpdateResponse> {
        // Endpoint: /v5/api/ttools/trips/{trip_id}/timetable
        let endpoint = format!(
            "{}{}{}",
            TRIPS_ENDPOINT_PREFIX, trip_id, TRIPS_ENDPOINT_SUFFIX
        );
        let full_path = format!("{}{}", BASE_PATH, endpoint);

        let headers = self
            .authenticator
            .get_auth_headers("GET", &full_path, None, None)?;

        let url = format!("{}{}", BASE_URL, endpoint);
        let mut req = self.client.get(&url);

        for (k, v) in headers {
            req = req.header(k, v);
        }

        req = self.add_common_headers(req);

        let resp = req
            .send()
            .await
            .context("failed to send trip update request")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await.unwrap_or_default();
            anyhow::bail!("Trip update fetch failed: {} - {}", status, text);
        }

        let text = resp.text().await.context("failed to read response text")?;
        // log_debug(&format!("Trip update response for {}: {}", trip_id, text));

        let response: TripUpdateResponse =
            serde_json::from_str(&text).context("failed to parse trip update response")?;

        Ok(response)
    }

    fn add_common_headers(&self, req: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        req.header(
            "User-Agent",
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:145.0) Gecko/20100101 Firefox/145.0",
        )
        .header("Accept", "application/json")
        .header("Accept-Language", "en-CA,en-US;q=0.7,en;q=0.3")
        .header("Accept-Encoding", "gzip, deflate, br, zstd")
        .header("Content-Type", "application/json;charset=UTF-8")
        .header("Referer", "https://dart.mygopass.org/")
        .header("Origin", "https://dart.mygopass.org")
        .header("Connection", "keep-alive")
        .header("Sec-Fetch-Dest", "empty")
        .header("Sec-Fetch-Mode", "cors")
        .header("Sec-Fetch-Site", "cross-site")
        .header("DNT", "1")
        .header("Sec-GPC", "1")
        .header("Pragma", "no-cache")
        .header("Cache-Control", "no-cache")
        .header("TE", "trailers")
    }
}
