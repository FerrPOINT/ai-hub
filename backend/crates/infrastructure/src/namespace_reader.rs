//! Fixed owner reader; human bearer tokens and caller URLs never enter this port.
use aihub_domain::{NamespaceRef, error::HubError};
use chrono::{DateTime, Utc};
use sdlc_shared::resource_context::{ResourceKind, ResourceRef};
use serde::Deserialize;
use url::Url;
use uuid::Uuid;
use zeroize::Zeroizing;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OwnerNamespace {
    pub id: Uuid,
    pub registry_instance_id: Uuid,
    pub slug: String,
    pub name: String,
    pub description: String,
    pub responsible_subject: String,
    pub state: String,
    pub revision: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OwnerBinding {
    pub namespace: NamespaceRef,
    pub resource: ResourceRef,
    pub operation_id: Uuid,
    pub generation: i64,
    pub desired_state: String,
    pub confirmed: bool,
    pub create_spec: Option<serde_json::Value>,
    pub last_error: Option<String>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OwnerContext {
    pub namespace: OwnerNamespace,
    pub bindings: Vec<OwnerBinding>,
}
pub struct RegistryObservation {
    pub namespace: NamespaceRef,
    pub tracker: ResourceRef,
    pub generation: i64,
    pub state: String,
    pub label: String,
    pub observed_at: DateTime<Utc>,
}
impl OwnerContext {
    pub fn verify(
        self,
        expected: &NamespaceRef,
        tracker_instance: Uuid,
    ) -> Result<RegistryObservation, HubError> {
        if expected.registry_instance_id.is_nil()
            || expected.namespace_id.is_nil()
            || tracker_instance.is_nil()
            || self.namespace.id != expected.namespace_id
            || self.namespace.registry_instance_id != expected.registry_instance_id
            || self.namespace.revision < 1
            || self.namespace.name.is_empty()
            || self.namespace.name.chars().count() > 200
            || self.bindings.len() > 3
            || self.bindings.iter().any(|binding| {
                binding.namespace != *expected
                    || binding.generation < 1
                    || binding.resource.resource_id.is_nil()
                    || binding.resource.instance_id.is_nil()
                    || binding.operation_id.is_nil()
            })
        {
            return Err(HubError::Invalid("owner Namespace identity/readback"));
        }
        let mut trackers = self
            .bindings
            .into_iter()
            .filter(|binding| binding.resource.kind == ResourceKind::TrackerProject);
        let binding = trackers.next().ok_or(HubError::Unavailable)?;
        if trackers.next().is_some() || binding.resource.instance_id != tracker_instance {
            return Err(HubError::Invalid("owner Tracker identity"));
        }
        let state = match (
            self.namespace.state.as_str(),
            binding.confirmed,
            binding.desired_state.as_str(),
        ) {
            ("active", true, "active") => "active",
            ("archived", true, "archived") => "archived",
            _ => "unavailable",
        };
        Ok(RegistryObservation {
            namespace: expected.clone(),
            tracker: binding.resource,
            generation: binding.generation,
            state: state.into(),
            label: self.namespace.name,
            observed_at: Utc::now(),
        })
    }
}
pub struct RegistryReader {
    origin: Url,
    registry_instance: Uuid,
    tracker_instance: Uuid,
    credential: Zeroizing<String>,
    http: reqwest::Client,
}
impl RegistryReader {
    pub fn new(
        origin: Url,
        registry_instance: Uuid,
        tracker_instance: Uuid,
        credential: Zeroizing<String>,
    ) -> Result<Self, HubError> {
        let origin = crate::config::trusted_origin(origin.as_str())?;
        if origin.path() != "/"
            || registry_instance.is_nil()
            || tracker_instance.is_nil()
            || credential.is_empty()
            || credential.len() > 16384
            || credential.contains(['\r', '\n'])
        {
            return Err(HubError::Invalid("registry reader configuration"));
        }
        let http = reqwest::Client::builder()
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(std::time::Duration::from_secs(10))
            .connect_timeout(std::time::Duration::from_secs(3))
            .build()
            .map_err(|_| HubError::Unavailable)?;
        Ok(Self {
            origin,
            registry_instance,
            tracker_instance,
            credential,
            http,
        })
    }
    pub async fn read(&self, namespace: &NamespaceRef) -> Result<RegistryObservation, HubError> {
        if namespace.registry_instance_id != self.registry_instance
            || namespace.namespace_id.is_nil()
        {
            return Err(HubError::Forbidden);
        }
        let url = self
            .origin
            .join(&format!(
                "api/v1/namespaces/{}/context",
                namespace.namespace_id
            ))
            .map_err(|_| HubError::Unavailable)?;
        let mut reply = self
            .http
            .get(url)
            .bearer_auth(self.credential.as_str())
            .send()
            .await
            .map_err(|_| HubError::Unavailable)?;
        if reply.status() == reqwest::StatusCode::NOT_FOUND {
            return Err(HubError::NotFound);
        }
        if !reply.status().is_success() || reply.content_length().is_some_and(|n| n > 65536) {
            return Err(HubError::Unavailable);
        }
        let mut bytes = Vec::new();
        while let Some(chunk) = reply.chunk().await.map_err(|_| HubError::Unavailable)? {
            if bytes.len() + chunk.len() > 65536 {
                return Err(HubError::Unavailable);
            }
            bytes.extend_from_slice(&chunk);
        }
        let context: OwnerContext =
            serde_json::from_slice(&bytes).map_err(|_| HubError::Unavailable)?;
        context.verify(namespace, self.tracker_instance)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture(namespace: &NamespaceRef, tracker: Uuid) -> OwnerContext {
        OwnerContext {
            namespace: OwnerNamespace {
                id: namespace.namespace_id,
                registry_instance_id: namespace.registry_instance_id,
                slug: "same-name".into(),
                name: "Same name".into(),
                description: "".into(),
                responsible_subject: Uuid::new_v4().to_string(),
                state: "active".into(),
                revision: 2,
                created_at: Utc::now(),
                updated_at: Utc::now(),
            },
            bindings: vec![OwnerBinding {
                namespace: namespace.clone(),
                resource: ResourceRef {
                    kind: ResourceKind::TrackerProject,
                    instance_id: tracker,
                    resource_id: Uuid::new_v4(),
                },
                operation_id: Uuid::new_v4(),
                generation: 2,
                desired_state: "active".into(),
                confirmed: true,
                create_spec: None,
                last_error: None,
            }],
        }
    }
    #[test]
    fn registry_identity_and_confirmed_state_are_independent_of_names() {
        let namespace = NamespaceRef {
            registry_instance_id: Uuid::new_v4(),
            namespace_id: Uuid::new_v4(),
        };
        let tracker = Uuid::new_v4();
        assert_eq!(
            fixture(&namespace, tracker)
                .verify(&namespace, tracker)
                .unwrap()
                .state,
            "active"
        );
        let foreign = NamespaceRef {
            registry_instance_id: Uuid::new_v4(),
            ..namespace.clone()
        };
        assert!(
            fixture(&namespace, tracker)
                .verify(&foreign, tracker)
                .is_err()
        );
        assert!(
            fixture(&namespace, tracker)
                .verify(&namespace, Uuid::new_v4())
                .is_err()
        );
        let mut pending = fixture(&namespace, tracker);
        pending.bindings[0].confirmed = false;
        assert_eq!(
            pending.verify(&namespace, tracker).unwrap().state,
            "unavailable"
        );
        let mut archived = fixture(&namespace, tracker);
        archived.namespace.state = "archived".into();
        archived.bindings[0].desired_state = "archived".into();
        assert_eq!(
            archived.verify(&namespace, tracker).unwrap().state,
            "archived"
        );
        let mut duplicate = fixture(&namespace, tracker);
        duplicate
            .bindings
            .push(fixture(&namespace, tracker).bindings.remove(0));
        assert!(duplicate.verify(&namespace, tracker).is_err());
    }
}
