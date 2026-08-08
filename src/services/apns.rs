use crate::db::{models::TeamMessage, repository::Repository};
use anyhow::Context;
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use std::{fs, time::{SystemTime, UNIX_EPOCH}};

#[derive(Clone)]
pub struct PushNotificationService {
    apns: Option<ApnsClient>,
}

impl PushNotificationService {
    pub fn from_env() -> Self {
        Self {
            apns: ApnsClient::from_env().ok(),
        }
    }

    pub fn is_configured(&self) -> bool {
        self.apns.is_some()
    }

    pub async fn deliver_team_message(
        &self,
        repo: &Repository,
        thread_id: &str,
        message: &TeamMessage,
    ) -> anyhow::Result<()> {
        let Some(apns) = &self.apns else {
            tracing::debug!("APNs delivery skipped: APNs credentials are not configured");
            return Ok(());
        };

        let targets = repo
            .get_push_targets_for_thread(thread_id, &message.sender_device_id)
            .await?;

        if targets.is_empty() {
            tracing::debug!(thread_id, "APNs delivery skipped: no registered recipient tokens");
            return Ok(());
        }

        for target in targets {
            let unread = repo
                .get_team_unread_summary(&target.device_id)
                .await
                .map(|summary| summary.total_unread)
                .unwrap_or(1);

            let delivery = apns.send_team_message(&target, message, unread).await;
            match delivery {
                Ok(()) => {
                    repo.log_push_delivery(
                        Some(&message.uuid),
                        Some(thread_id),
                        &target.device_id,
                        Some(target.token_id),
                        "sent",
                        None,
                    ).await?;
                }
                Err(error) => {
                    let error_message = error.to_string();
                    repo.log_push_delivery(
                        Some(&message.uuid),
                        Some(thread_id),
                        &target.device_id,
                        Some(target.token_id),
                        "failed",
                        Some(&error_message),
                    ).await?;

                    if should_disable_token(&error_message) {
                        repo.disable_push_token(target.token_id).await?;
                    }
                }
            }
        }

        Ok(())
    }
}

#[derive(Clone)]
struct ApnsClient {
    client: reqwest::Client,
    key_id: String,
    team_id: String,
    bundle_id: String,
    encoding_key: EncodingKey,
}

impl ApnsClient {
    fn from_env() -> anyhow::Result<Self> {
        let key_id = std::env::var("APNS_KEY_ID").context("APNS_KEY_ID is required")?;
        let team_id = std::env::var("APNS_TEAM_ID").context("APNS_TEAM_ID is required")?;
        let bundle_id = std::env::var("APNS_BUNDLE_ID").context("APNS_BUNDLE_ID is required")?;
        let private_key = match std::env::var("APNS_PRIVATE_KEY") {
            Ok(value) => value.replace("\\n", "\n"),
            Err(_) => {
                let path = std::env::var("APNS_PRIVATE_KEY_PATH")
                    .context("APNS_PRIVATE_KEY or APNS_PRIVATE_KEY_PATH is required")?;
                fs::read_to_string(path).context("failed to read APNs private key")?
            }
        };

        let encoding_key = EncodingKey::from_ec_pem(private_key.as_bytes())
            .context("failed to parse APNs ES256 private key")?;

        Ok(Self {
            client: reqwest::Client::builder()
                .http2_prior_knowledge()
                .build()
                .context("failed to create APNs HTTP client")?,
            key_id,
            team_id,
            bundle_id,
            encoding_key,
        })
    }

    async fn send_team_message(
        &self,
        target: &crate::db::models::PushDeliveryTarget,
        message: &TeamMessage,
        badge_count: i64,
    ) -> anyhow::Result<()> {
        let authorization = self.authorization_token()?;
        let topic = target.bundle_id.as_deref().unwrap_or(&self.bundle_id);
        let endpoint = apns_endpoint(target.environment.as_deref().unwrap_or("production"));
        let url = format!("{}/3/device/{}", endpoint, target.token);
        let title = message.sender_name.as_deref().unwrap_or("CabNet Team");
        let payload = ApnsTeamPayload::new(title, &message.body, badge_count, &message.thread_id, &message.uuid);

        let response = self.client
            .post(url)
            .bearer_auth(authorization)
            .header("apns-topic", topic)
            .header("apns-push-type", "alert")
            .header("apns-priority", "10")
            .json(&payload)
            .send()
            .await
            .context("failed to send APNs request")?;

        if response.status().is_success() {
            return Ok(());
        }

        let status = response.status();
        let error_body = response
            .json::<ApnsErrorResponse>()
            .await
            .map(|body| body.reason)
            .unwrap_or_else(|_| status.to_string());

        Err(anyhow::anyhow!("APNs {}: {}", status.as_u16(), error_body))
    }

    fn authorization_token(&self) -> anyhow::Result<String> {
        let mut header = Header::new(Algorithm::ES256);
        header.kid = Some(self.key_id.clone());

        let issued_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .context("system clock is before Unix epoch")?
            .as_secs() as usize;

        encode(
            &header,
            &ApnsClaims { iss: &self.team_id, iat: issued_at },
            &self.encoding_key,
        ).context("failed to sign APNs provider token")
    }
}

#[derive(Serialize)]
struct ApnsClaims<'a> {
    iss: &'a str,
    iat: usize,
}

#[derive(Serialize)]
struct ApnsTeamPayload<'a> {
    aps: ApsPayload<'a>,
    #[serde(rename = "cabnet")]
    data: CabNetPushData<'a>,
}

impl<'a> ApnsTeamPayload<'a> {
    fn new(title: &'a str, body: &'a str, badge: i64, thread_id: &'a str, message_id: &'a str) -> Self {
        Self {
            aps: ApsPayload {
                alert: ApsAlert { title, body },
                sound: "default",
                badge,
            },
            data: CabNetPushData {
                kind: "team_message",
                thread_id,
                message_id,
            },
        }
    }
}

#[derive(Serialize)]
struct ApsPayload<'a> {
    alert: ApsAlert<'a>,
    sound: &'a str,
    badge: i64,
}

#[derive(Serialize)]
struct ApsAlert<'a> {
    title: &'a str,
    body: &'a str,
}

#[derive(Serialize)]
struct CabNetPushData<'a> {
    #[serde(rename = "type")]
    kind: &'a str,
    thread_id: &'a str,
    message_id: &'a str,
}

#[derive(Deserialize)]
struct ApnsErrorResponse {
    reason: String,
}

fn apns_endpoint(environment: &str) -> &'static str {
    match environment {
        "sandbox" | "development" | "debug" => "https://api.sandbox.push.apple.com",
        _ => "https://api.push.apple.com",
    }
}

fn should_disable_token(error_message: &str) -> bool {
    error_message.contains(&StatusCode::GONE.as_u16().to_string())
        || error_message.contains("BadDeviceToken")
        || error_message.contains("Unregistered")
        || error_message.contains("DeviceTokenNotForTopic")
}