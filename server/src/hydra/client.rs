use axum::http::HeaderMap;
use reqwest::Url;
use serde::{de, Deserialize};
use serde_json::Value;

use crate::model::System;

/// https://editor.swagger.io/?url=https://raw.githubusercontent.com/NixOS/hydra/master/hydra-api.yaml
#[derive(Clone)]
pub struct HydraClient {
    base_url: Url,
    client: reqwest::Client,
}

impl HydraClient {
    pub fn new(base_url: Url) -> Self {
        let mut headers = HeaderMap::new();
        headers.insert("Accept", "application/json".parse().unwrap());
        headers.insert("Referer", base_url.to_string().parse().unwrap()); // bypass XSRF check
        Self {
            base_url,
            client: reqwest::Client::builder()
                .default_headers(headers)
                .build()
                .unwrap(),
        }
    }

    pub async fn push(&self, event: Value) -> anyhow::Result<Value> {
        let mut url = self.base_url.join("/api/push-github")?;
        // https://github.com/NixOS/hydra/commit/916531dc9ccee52e6dab256232933fcf6d198158
        let response = self
            .client
            .post(url)
            .body(event)
            .send()
            .await?
            .error_for_status_with_body()
            .await?;
        Ok(response.json().await?)
    }

    pub async fn get_queue(&self) -> anyhow::Result<Vec<Build>> {
        let url = self.base_url.join("queue")?;
        let response = self
            .client
            .get(url)
            .send()
            .await?
            .error_for_status_with_body()
            .await?;
        let body = response.json().await?;
        Ok(body)
    }
}

trait ResponseExt {
    async fn error_for_status_with_body(self) -> anyhow::Result<Self>
    where
        Self: Sized;
}

impl ResponseExt for reqwest::Response {
    async fn error_for_status_with_body(self) -> anyhow::Result<Self>
    where
        Self: Sized,
    {
        if self.status().is_server_error() || self.status().is_client_error() {
            let url = self.url().clone();
            let status = self.status();
            let body = self.text().await;
            return Err(anyhow::anyhow!(
                "{url} returned HTTP {status} with body {body:#?}",
            ));
        }

        Ok(self)
    }
}

#[derive(Deserialize, Debug)]
pub struct Build {
    pub project: String,
    pub jobset: String,
    #[serde(deserialize_with = "int_to_bool")]
    pub finished: bool,
    pub starttime: Option<u32>,
    pub stoptime: Option<u32>,
    pub buildstatus: Option<u32>,
    pub system: System,
}

fn int_to_bool<'de, D>(deserializer: D) -> Result<bool, D::Error>
where
    D: de::Deserializer<'de>,
{
    let s: u32 = de::Deserialize::deserialize(deserializer)?;
    Ok(s != 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserialize_build() {
        let build = serde_json::from_str::<Vec<Build>>(include_str!("../../test/hydra-queue.json"))
            .unwrap();

        dbg!(build);
    }

    // TODO: integration tests
    // #[tokio::test]
    // async fn push() {
    //     let client = HydraClient::new(Url::parse("https://localhost:3000").unwrap());
    //     client.push("project", "jobset").await.unwrap();
    // }
}
