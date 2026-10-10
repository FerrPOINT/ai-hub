use aihub_application::CentralAuthentication;
use aihub_domain::{
    access::{HumanAuthentication, HumanPrincipal},
    error::HubError,
};
use async_trait::async_trait;
use sdlc_auth_core::service_bridge::{BridgeOutcome, ServiceBridge};
use uuid::Uuid;

pub struct CentralAuth {
    bridge: ServiceBridge,
}

impl Default for CentralAuth {
    fn default() -> Self {
        Self {
            bridge: ServiceBridge::new("AIHUB_AUTH"),
        }
    }
}

#[async_trait]
impl CentralAuthentication for CentralAuth {
    async fn authenticate(&self, token: &str) -> Result<HumanPrincipal, HubError> {
        let (outcome, name) = self.bridge.try_token_with_name(token).await;
        match outcome {
            BridgeOutcome::Validated(context) => {
                // The pinned bridge validates crypto plus current session/PAT status.
                // Require the live profile too: a malformed /auth/me response is not authority.
                if name.is_none() || Uuid::parse_str(&context.user_id).is_err() {
                    return Err(HubError::Unauthenticated);
                }
                let authentication = match context.session_id {
                    Some(_) if !token.starts_with("sdlc_pat_") => {
                        HumanAuthentication::BrowserSession
                    }
                    None if token.starts_with("sdlc_pat_") => HumanAuthentication::PersonalToken {
                        scopes: context.scopes.into_iter().collect(),
                    },
                    _ => return Err(HubError::Forbidden),
                };
                Ok(HumanPrincipal {
                    subject: context.user_id,
                    authentication,
                })
            }
            BridgeOutcome::Unavailable | BridgeOutcome::NotConfigured => Err(HubError::Unavailable),
            // There is deliberately no local credential/issuer fallback.
            BridgeOutcome::NotOurs | BridgeOutcome::Expired | BridgeOutcome::Invalid(_) => {
                Err(HubError::Unauthenticated)
            }
        }
    }
}
