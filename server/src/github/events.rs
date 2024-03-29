use serde::Deserialize;

#[derive(Deserialize, Debug)]
pub struct PushEvent {
    pub r#ref: String,
    pub repository: Repository,
}

#[derive(Deserialize, Debug)]
pub struct Repository {
    pub name: String,
}

impl PushEvent {
    pub fn branch(&self) -> Option<&str> {
        self.r#ref.strip_prefix("refs/heads/")
    }
}
