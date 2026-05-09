use serde::{Deserialize, Serialize};

use super::ConfigFile;

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct SpotifyAuth {
    pub client_id: Option<String>,
    pub access_token: Option<String>,
    pub refresh_token: Option<String>,
}

impl SpotifyAuth {
    pub fn is_connected(&self) -> bool {
        self.client_id.is_some() && self.refresh_token.is_some()
    }
}

impl ConfigFile for SpotifyAuth {
    fn get_filename() -> &'static str {
        "spotify_auth.json"
    }
}
