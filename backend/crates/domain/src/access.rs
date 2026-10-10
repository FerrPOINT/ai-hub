use crate::{NamespaceRef, error::HubError};
use std::collections::BTreeSet;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HumanAuthentication {
    BrowserSession,
    PersonalToken { scopes: BTreeSet<String> },
}

/// Only a live central-auth result constructs this value; metadata is never authority.
#[derive(Debug, Clone)]
pub struct HumanPrincipal {
    pub subject: String,
    pub authentication: HumanAuthentication,
}

impl HumanPrincipal {
    pub fn permits_config(&self, write: bool) -> bool {
        match &self.authentication {
            HumanAuthentication::BrowserSession => true,
            HumanAuthentication::PersonalToken { scopes } => {
                scopes.contains(if write { "ai-hub:write" } else { "ai-hub:read" })
            }
        }
    }

    pub fn require_config(&self, write: bool) -> Result<(), HubError> {
        if self.permits_config(write) {
            Ok(())
        } else {
            Err(HubError::Forbidden)
        }
    }
}

/// Reject incomplete, duplicate and nil identities before querying authorized data.
pub fn namespace_filter(query: &str) -> Result<Option<NamespaceRef>, HubError> {
    let mut registry = None;
    let mut namespace = None;
    for (key, value) in url::form_urlencoded::parse(query.as_bytes()) {
        let slot = match key.as_ref() {
            "registry_instance_id" => &mut registry,
            "namespace_id" => &mut namespace,
            _ => continue,
        };
        if slot.is_some() {
            return Err(HubError::Invalid("повторяющаяся UUID-пара Namespace"));
        }
        let id = Uuid::parse_str(&value).map_err(|_| HubError::Invalid("UUID Namespace"))?;
        if id.is_nil() {
            return Err(HubError::Invalid("пустой UUID Namespace"));
        }
        *slot = Some(id);
    }
    match (registry, namespace) {
        (None, None) => Ok(None),
        (Some(registry_instance_id), Some(namespace_id)) => Ok(Some(NamespaceRef {
            registry_instance_id,
            namespace_id,
        })),
        _ => Err(HubError::Invalid("неполная UUID-пара Namespace")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn personal_write_does_not_imply_read_or_inference() {
        let principal = HumanPrincipal {
            subject: Uuid::new_v4().to_string(),
            authentication: HumanAuthentication::PersonalToken {
                scopes: ["ai-hub:write".into()].into(),
            },
        };
        assert!(principal.permits_config(true));
        assert!(!principal.permits_config(false));
        let foreign = HumanPrincipal {
            authentication: HumanAuthentication::PersonalToken {
                scopes: ["admin:write".into()].into(),
            },
            ..principal
        };
        assert!(!foreign.permits_config(true));
    }

    #[test]
    fn namespace_pair_is_complete_and_never_repaired_from_a_name() {
        let a = Uuid::new_v4();
        let b = Uuid::new_v4();
        let pair = format!("registry_instance_id={a}&namespace_id={b}");
        assert_eq!(namespace_filter(&pair).unwrap().unwrap().namespace_id, b);
        assert!(namespace_filter(&format!("{pair}&namespace_id={b}")).is_err());
        assert!(namespace_filter(&format!("registry_instance_id={a}&name=valid")).is_err());
        assert!(
            namespace_filter(&format!(
                "registry_instance_id={a}&namespace_id={}",
                Uuid::nil()
            ))
            .is_err()
        );
        assert!(namespace_filter("namespace_id=bad").is_err());
        assert!(namespace_filter("q=project").unwrap().is_none());
    }
}
