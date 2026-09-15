use crate::persist::Persist;
use data::api_client::{Account, Browser, BrowserProfile, GeckoContainer};
use error::{YError, YResult};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct ClientState {
    pub browser: Option<Browser>,
    pub profile: Option<BrowserProfile>,
    pub gecko_container: Option<GeckoContainer>,
    pub account: Option<Account>,
}

impl Persist for ClientState {
    const FILE_NAME: &'static str = "client_state.json";
}

impl ClientState {
    pub fn get_validated_fields(
        &self,
    ) -> YResult<(
        &Browser,
        &BrowserProfile,
        Option<&GeckoContainer>,
        Option<&Account>,
    )> {
        let browser = self
            .browser
            .as_ref()
            .ok_or_else(|| YError::MissApiClientContext("Browser".to_string()))?;
        let profile = self
            .profile
            .as_ref()
            .ok_or_else(|| YError::MissApiClientContext("Profile".to_string()))?;
        Ok((
            browser,
            profile,
            self.gecko_container.as_ref(),
            self.account.as_ref(),
        ))
    }
}
