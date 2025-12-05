use anyhow::{Context, Result};
use hmac::{Hmac, Mac};
use rand::rngs::OsRng;
use reqwest::blocking::Client;
use rsa::{
    pkcs1::{EncodeRsaPrivateKey, EncodeRsaPublicKey},
    pkcs8::{DecodePrivateKey, EncodePublicKey},
    PaddingScheme, RsaPrivateKey, RsaPublicKey,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::time::{SystemTime, UNIX_EPOCH};
use std::fs::OpenOptions;
use std::io::Write;

const DEVELOPER_KEY: &str = "057e903f-48b3-4461-b9fc-39d14ca829ce";
const DEVELOPER_SECRET: &str = "35a425ad968fbe5e4ec8364e8e500420";
const BASE_URL: &str = "https://ssge-ticketing.us.unwire.com/api-gateway";
const APP_INSTANCE_URL: &str = "/v1/appinstance";
const TENANT_ID: &str = "205";

#[derive(Debug, Serialize, Deserialize)]
struct AppInstanceRequest {
    #[serde(rename = "tenantId")]
    tenant_id: String,
    #[serde(rename = "osType")]
    os_type: String,
    #[serde(rename = "hardwareId")]
    hardware_id: String,
    #[serde(rename = "publicKey")]
    public_key: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct AppInstanceResponseWrapper {
    #[serde(rename = "appInstance")]
    app_instance: AppInstanceResponse,
}

#[derive(Debug, Serialize, Deserialize)]
struct AppInstanceResponse {
    #[serde(rename = "encryptedSecret")]
    encrypted_secret: String,
    #[serde(rename = "appInstanceId")]
    instance_id: String,
}

pub struct Authenticator {
    client: Client,
    private_key: RsaPrivateKey,
    decrypted_secret: Option<String>,
    instance_id: Option<String>,
}

fn log_debug(msg: &str) {
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open("debug.log") {
        let _ = writeln!(file, "{}", msg);
    }
}

impl Authenticator {
    pub fn new() -> Result<Self> {
        let mut rng = OsRng;
        let bits = 1024;
        let private_key = RsaPrivateKey::new(&mut rng, bits).context("failed to generate a key")?;

        Ok(Self {
            client: Client::new(),
            private_key,
            decrypted_secret: None,
            instance_id: None,
        })
    }

    pub fn register(&mut self) -> Result<()> {
        let public_key = RsaPublicKey::from(&self.private_key);
        let public_key_der = public_key
            .to_public_key_der()
            .context("failed to convert to der")?;
        
        let public_key_b64 = base64::encode(&public_key_der);

        let req_body = AppInstanceRequest {
            tenant_id: TENANT_ID.to_string(),
            os_type: "Android".to_string(),
            hardware_id: "1234".to_string(),
            public_key: public_key_b64.clone(),
        };

        type HmacSha256 = Hmac<Sha256>;
        let mut mac = HmacSha256::new_from_slice(DEVELOPER_SECRET.as_bytes())
            .expect("HMAC can take key of any size");
        mac.update(req_body.public_key.as_bytes());
        let signature = base64::encode(mac.finalize().into_bytes());
        let api_developer_key = format!("{}:{}", DEVELOPER_KEY, signature);

        let url = format!("{}{}", BASE_URL, APP_INSTANCE_URL);
        let resp = self
            .client
            .post(&url)
            .header("api-developer-key", api_developer_key)
            .header("Content-Type", "application/json")
            .json(&req_body)
            .send()
            .context("failed to send registration request")?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().unwrap_or_default();
            anyhow::bail!("Registration failed: {} - {}", status, text);
        }

        let resp_json: AppInstanceResponseWrapper = resp.json().context("failed to parse response")?;
        let app_instance = resp_json.app_instance;

        let encrypted_bytes = base64::decode(&app_instance.encrypted_secret)
            .context("failed to decode encrypted secret")?;
        
        let padding = PaddingScheme::new_pkcs1v15_encrypt();
        let decrypted_bytes = self
            .private_key
            .decrypt(padding, &encrypted_bytes)
            .context("failed to decrypt secret")?;
        
        let decrypted_secret = String::from_utf8(decrypted_bytes)
            .context("decrypted secret is not valid utf8")?;

        log_debug(&format!("Decrypted secret: {}", decrypted_secret));
        log_debug(&format!("Instance ID: {}", app_instance.instance_id));

        self.decrypted_secret = Some(decrypted_secret);
        self.instance_id = Some(app_instance.instance_id);

        Ok(())
    }

    pub fn get_auth_headers(
        &self,
        method: &str,
        path: &str,
        body: Option<&str>,
        query: Option<&str>,
    ) -> Result<Vec<(String, String)>> {
        let decrypted_secret = self
            .decrypted_secret
            .as_ref()
            .context("not registered")?;
        let instance_id = self.instance_id.as_ref().context("not registered")?;

        let content_sha256 = if let Some(b) = body {
            let mut hasher = Sha256::new();
            hasher.update(b.as_bytes());
            base64::encode(hasher.finalize())
        } else {
            "".to_string()
        };

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let jti = format!("{}-{}", now, SystemTime::now().duration_since(UNIX_EPOCH).unwrap().subsec_millis());
        
        #[derive(Serialize)]
        struct JwtClaim {
            iat: u64,
            jti: String,
        }
        let claim = JwtClaim { iat: now, jti };
        let claim_json = serde_json::to_string(&claim)?;
        let api_jwt_claim = base64::encode(claim_json);

        let newline = "\n";
        let qs_part = if let Some(q) = query {
            format!("{}{}", newline, q)
        } else {
            "".to_string()
        };

        let sig_input = format!(
            "{}{}{}{}{}{}{}{}{}{}",
            method,
            newline,
            path,
            newline,
            "content-sha256=",
            content_sha256,
            newline,
            "api-jwt-claim=",
            api_jwt_claim,
            qs_part
        );

        log_debug(&format!("Sig Input:\n{:?}", sig_input));

        type HmacSha256 = Hmac<Sha256>;
        let mut mac = HmacSha256::new_from_slice(decrypted_secret.as_bytes())
            .expect("HMAC can take key of any size");
        mac.update(sig_input.as_bytes());
        let signature = base64::encode(mac.finalize().into_bytes());
        
        let ssg_instance_hmac = format!("{}:{}", instance_id, signature);

        Ok(vec![
            ("Content-SHA256".to_string(), content_sha256),
            ("SSG-Instance-HMAC".to_string(), ssg_instance_hmac),
            ("api-jwt-claim".to_string(), api_jwt_claim),
        ])
    }
}
