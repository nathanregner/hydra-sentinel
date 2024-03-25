mod state;
pub mod websocket;

use reqwest::Url;

#[derive(Clone)]
pub struct HydraClient {
    base_url: Url,
    client: reqwest::Client,
}

impl HydraClient {
    pub fn new(base_url: Url) -> Self {
        Self {
            base_url,
            client: reqwest::Client::new(),
        }
    }

    pub async fn push(&self, project: &str, jobset: &str) -> anyhow::Result<()> {
        let mut url = self.base_url.join("api/push")?;
        url.query_pairs_mut()
            .append_pair("jobsets", &format!("{project}:{jobset}"));
        self.client.put(url).send().await?;
        Ok(())
    }
}
