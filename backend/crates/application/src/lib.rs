use aihub_domain::{
    NamespaceRef,
    access::HumanPrincipal,
    error::HubError,
    records::{AuditEvent, Identity, NamespaceBinding, Operation, Page},
};
use async_trait::async_trait;
use std::sync::Arc;
use uuid::Uuid;

#[async_trait]
pub trait FinancialAdmission: Send + Sync {
    async fn reserve(
        &self,
        intent: &aihub_domain::admission::AdmissionIntent,
    ) -> Result<aihub_domain::admission::AdmissionReceipt, HubError>;
    async fn claim_dispatch(
        &self,
        attempt_id: Uuid,
        owner_id: Uuid,
        lease_seconds: i32,
    ) -> Result<aihub_domain::admission::DispatchClaim, HubError>;
    async fn record_uncertain(
        &self,
        claim: &aihub_domain::admission::DispatchClaim,
    ) -> Result<(), HubError>;
    async fn recover_expired_dispatches(&self) -> Result<u64, HubError>;
    async fn settle(
        &self,
        fact: &aihub_domain::settlement::SettlementFact,
    ) -> Result<aihub_domain::settlement::SettlementReceipt, HubError>;
}

#[async_trait]
pub trait CentralAuthentication: Send + Sync {
    async fn authenticate(&self, token: &str) -> Result<HumanPrincipal, HubError>;
}

#[async_trait]
pub trait FoundationStore: Send + Sync {
    async fn ready(&self) -> Result<(), HubError>;
    async fn project_grants(&self, subject: &str, action: &str) -> Result<Vec<String>, HubError>;
    async fn namespaces(
        &self,
        subject: &str,
        limit: i64,
    ) -> Result<Vec<NamespaceBinding>, HubError>;
    async fn require_namespace(
        &self,
        subject: &str,
        namespace: &NamespaceRef,
        action: &str,
    ) -> Result<(), HubError>;
    async fn audit(
        &self,
        subject: &str,
        namespace: Option<&NamespaceRef>,
        limit: i64,
    ) -> Result<Vec<AuditEvent>, HubError>;
    async fn operation(&self, subject: &str, id: Uuid) -> Result<Operation, HubError>;
    async fn namespace_page(
        &self,
        subject: &str,
        limit: i64,
        cursor: Option<Uuid>,
    ) -> Result<Page<NamespaceBinding>, HubError> {
        if cursor.is_some() {
            return Err(HubError::Invalid("cursor"));
        }
        let items = self.namespaces(subject, limit + 1).await?;
        if items.len() > limit as usize {
            return Err(HubError::Unavailable);
        }
        Ok(Page {
            items,
            next_cursor: None,
        })
    }
    async fn audit_page(
        &self,
        subject: &str,
        namespace: Option<&NamespaceRef>,
        limit: i64,
        cursor: Option<Uuid>,
    ) -> Result<Page<AuditEvent>, HubError> {
        if cursor.is_some() {
            return Err(HubError::Invalid("cursor"));
        }
        let items = self.audit(subject, namespace, limit + 1).await?;
        if items.len() > limit as usize {
            return Err(HubError::Unavailable);
        }
        Ok(Page {
            items,
            next_cursor: None,
        })
    }
}

#[derive(Clone)]
pub struct Foundation {
    pub installation_id: Uuid,
    pub auth: Arc<dyn CentralAuthentication>,
    pub store: Arc<dyn FoundationStore>,
}

impl Foundation {
    pub async fn identity(&self, principal: &HumanPrincipal) -> Result<Identity, HubError> {
        // A write-only PAT may inspect its own capabilities without gaining config read access.
        if !principal.permits_config(false) && !principal.permits_config(true) {
            return Err(HubError::Forbidden);
        }
        let capabilities = [(false, "config.read"), (true, "config.write")]
            .into_iter()
            .filter(|(write, _)| principal.permits_config(*write))
            .map(|(_, name)| name.to_owned())
            .collect();
        Ok(Identity {
            subject: principal.subject.clone(),
            installation_id: self.installation_id,
            capabilities,
            project_grants: self
                .store
                .project_grants(&principal.subject, "metadata.read")
                .await?,
        })
    }

    pub async fn namespaces(
        &self,
        principal: &HumanPrincipal,
        limit: i64,
        cursor: Option<Uuid>,
    ) -> Result<Page<NamespaceBinding>, HubError> {
        principal.require_config(false)?;
        self.store
            .namespace_page(&principal.subject, bounded_limit(limit)?, cursor)
            .await
    }

    pub async fn audit(
        &self,
        principal: &HumanPrincipal,
        namespace: Option<&NamespaceRef>,
        limit: i64,
        cursor: Option<Uuid>,
    ) -> Result<Page<AuditEvent>, HubError> {
        principal.require_config(false)?;
        if let Some(namespace) = namespace {
            self.store
                .require_namespace(&principal.subject, namespace, "audit.read")
                .await?;
        }
        self.store
            .audit_page(&principal.subject, namespace, bounded_limit(limit)?, cursor)
            .await
    }

    pub async fn operation(
        &self,
        principal: &HumanPrincipal,
        id: Uuid,
    ) -> Result<Operation, HubError> {
        if !principal.permits_config(false) && !principal.permits_config(true) {
            return Err(HubError::Forbidden);
        }
        self.store.operation(&principal.subject, id).await
    }
}

pub fn bounded_limit(limit: i64) -> Result<i64, HubError> {
    if (1..=100).contains(&limit) {
        Ok(limit)
    } else {
        Err(HubError::Invalid("limit должен быть от 1 до 100"))
    }
}
