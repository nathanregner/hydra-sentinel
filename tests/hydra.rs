use crate::hydra::apis::{configuration::Configuration, default_api::api_push_put};

#[tokio::test]
async fn hydra() {
    let mut configuration = Configuration::default();
    configuration.base_path = "https://hydra.nregner.net".to_owned();
    let client = api_push_put(&configuration, Some("nix-config:bogus"))
        .await
        .unwrap();
    dbg!(client);
}
