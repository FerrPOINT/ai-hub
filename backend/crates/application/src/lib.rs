use aihub_domain::{
    NamespaceRef,
    access::HumanPrincipal,
    error::HubError,
    records::{AuditEvent, Identity, NamespaceBinding, Operation, Page},
};
use async_trait::async_trait;
use std::sync::Arc;
use uuid::Uuid;

pub trait OperationBinding: Send + Sync {
    fn bind(&self, value: &serde_json::Value) -> Result<[u8; 32], HubError>;
}

#[async_trait]
pub trait ResultDelivery: Send + Sync {
    async fn settle_with_result(
        &self,
        fact: &aihub_domain::settlement::SettlementFact,
        payload: &aihub_domain::replay::ResultPayload,
        ttl_seconds: u32,
    ) -> Result<aihub_domain::settlement::SettlementReceipt, HubError>;
    async fn read_result(
        &self,
        reader: &aihub_domain::replay::ResultReader,
        request_id: Uuid,
    ) -> Result<aihub_domain::replay::ReplayOutcome, HubError>;
    async fn purge_expired_results(&self, limit: i64) -> Result<u64, HubError>;
}

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
    async fn recover_expired_intents(&self) -> Result<u64, HubError>;
    async fn cancel_owned(
        &self,
        owner: &aihub_domain::admission::RequestOwner,
        request_id: Uuid,
    ) -> Result<aihub_domain::admission::CancellationReceipt, HubError>;
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
    async fn pricing_source_page(
        &self,
        _subject: &str,
        _limit: i64,
        _cursor: Option<Uuid>,
    ) -> Result<Page<aihub_domain::pricing_sources::PricingSourceRevision>, HubError> {
        Err(HubError::Unavailable)
    }
    async fn create_pricing_source(
        &self,
        _subject: &str,
        _key: Uuid,
        _binding: [u8; 32],
        _input: &aihub_domain::pricing_sources::PricingSourceInput,
    ) -> Result<aihub_domain::pricing_sources::PricingSourceMutation, HubError> {
        Err(HubError::Unavailable)
    }
    async fn budget_page(
        &self,
        _subject: &str,
        _filter: &BudgetFilter,
        _limit: i64,
        _cursor: Option<Uuid>,
    ) -> Result<Page<aihub_domain::budgets::Budget>, HubError> {
        Err(HubError::Unavailable)
    }
    async fn write_budget(
        &self,
        _subject: &str,
        _key: Uuid,
        _binding: [u8; 32],
        _update: Option<(Uuid, i64)>,
        _policy: &aihub_domain::budgets::BudgetInput,
    ) -> Result<aihub_domain::budgets::BudgetMutation, HubError> {
        Err(HubError::Unavailable)
    }
    async fn price_page(
        &self,
        _subject: &str,
        _limit: i64,
        _cursor: Option<Uuid>,
    ) -> Result<Page<aihub_domain::prices::PriceRevision>, HubError> {
        Err(HubError::Unavailable)
    }
    async fn create_price(
        &self,
        _subject: &str,
        _key: Uuid,
        _binding: [u8; 32],
        _price: &aihub_domain::prices::PriceInput,
    ) -> Result<aihub_domain::prices::PriceMutation, HubError> {
        Err(HubError::Unavailable)
    }
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
    pub bindings: Arc<dyn OperationBinding>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct BudgetFilter {
    pub namespace: Option<NamespaceRef>,
    pub unbound_only: bool,
}

impl Foundation {
    pub async fn pricing_sources(
        &self,
        principal: &HumanPrincipal,
        limit: i64,
        cursor: Option<Uuid>,
    ) -> Result<Page<aihub_domain::pricing_sources::PricingSourceRevision>, HubError> {
        principal.require_config(false)?;
        self.store
            .pricing_source_page(&principal.subject, bounded_limit(limit)?, cursor)
            .await
    }
    pub async fn create_pricing_source(
        &self,
        principal: &HumanPrincipal,
        key: Uuid,
        input: &aihub_domain::pricing_sources::PricingSourceInput,
    ) -> Result<aihub_domain::pricing_sources::PricingSourceMutation, HubError> {
        principal.require_config(true)?;
        input.validate()?;
        if key.is_nil() {
            return Err(HubError::Invalid("pricing source operation key"));
        }
        let binding=self.bindings.bind(&serde_json::json!({"installation_id":self.installation_id,"principal":principal.subject,"action":"pricing-source.create","input":input}))?;
        self.store
            .create_pricing_source(&principal.subject, key, binding, input)
            .await
    }
    pub async fn budgets(
        &self,
        principal: &HumanPrincipal,
        filter: &BudgetFilter,
        limit: i64,
        cursor: Option<Uuid>,
    ) -> Result<Page<aihub_domain::budgets::Budget>, HubError> {
        principal.require_config(false)?;
        if filter.unbound_only && filter.namespace.is_some() {
            return Err(HubError::Invalid("binding filter"));
        }
        if let Some(namespace) = &filter.namespace {
            self.store
                .require_namespace(&principal.subject, namespace, "metadata.read")
                .await?;
        }
        self.store
            .budget_page(&principal.subject, filter, bounded_limit(limit)?, cursor)
            .await
    }
    pub async fn write_budget(
        &self,
        principal: &HumanPrincipal,
        key: Uuid,
        update: Option<(Uuid, i64)>,
        policy: &aihub_domain::budgets::BudgetInput,
    ) -> Result<aihub_domain::budgets::BudgetMutation, HubError> {
        principal.require_config(true)?;
        if key.is_nil() || update.is_some_and(|(id, version)| id.is_nil() || version < 1) {
            return Err(HubError::Invalid("budget operation"));
        }
        policy.validate(self.installation_id)?;
        let action = if update.is_some() {
            "budget.update"
        } else {
            "budget.create"
        };
        let binding=self.bindings.bind(&serde_json::json!({"installation_id":self.installation_id,"principal":principal.subject,"action":action,"update":update,"policy":policy}))?;
        self.store
            .write_budget(&principal.subject, key, binding, update, policy)
            .await
    }
    pub async fn prices(
        &self,
        principal: &HumanPrincipal,
        limit: i64,
        cursor: Option<Uuid>,
    ) -> Result<Page<aihub_domain::prices::PriceRevision>, HubError> {
        principal.require_config(false)?;
        self.store
            .price_page(&principal.subject, bounded_limit(limit)?, cursor)
            .await
    }
    pub async fn create_price(
        &self,
        principal: &HumanPrincipal,
        key: Uuid,
        price: &aihub_domain::prices::PriceInput,
    ) -> Result<aihub_domain::prices::PriceMutation, HubError> {
        principal.require_config(true)?;
        if key.is_nil() {
            return Err(HubError::Invalid("idempotency key"));
        }
        price.validate()?;
        let binding=self.bindings.bind(&serde_json::json!({"installation_id":self.installation_id,"principal":principal.subject,"action":"price.create","price":price}))?;
        self.store
            .create_price(&principal.subject, key, binding, price)
            .await
    }
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
