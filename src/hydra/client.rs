use axum::http::HeaderMap;
use reqwest::Url;
use serde::{de, Deserialize};

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
        Self {
            base_url,
            client: reqwest::Client::builder()
                .default_headers(headers)
                .build()
                .unwrap(),
        }
    }

    pub async fn push(&self, project: &str, jobset: &str) -> anyhow::Result<()> {
        let mut url = self.base_url.join("api/push")?;
        url.query_pairs_mut()
            .append_pair("jobsets", &format!("{project}:{jobset}"));
        self.client.put(url).send().await?;
        Ok(())
    }

    pub async fn get_queue(&self) -> anyhow::Result<Vec<Build>> {
        let url = self.base_url.join("api/queue")?;
        let response = self.client.get(url).send().await?.json().await?;
        Ok(response)
    }
}

#[derive(Deserialize, Debug)]
pub struct Build {
    pub project: String,
    pub jobset: String,
    #[serde(deserialize_with = "int_to_bool")]
    pub finished: bool,
    pub builder: String,
    pub system: String,
}

fn int_to_bool<'de, D>(deserializer: D) -> Result<bool, D::Error>
where
    D: de::Deserializer<'de>,
{
    let s: u32 = de::Deserialize::deserialize(deserializer)?;
    Ok(s != 0)
}
