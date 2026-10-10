//! Durable cancellation intent; accounting remains owned by the common settlement engine.
use crate::{financial::db_failure, postgres::PgStore};
use aihub_domain::{
    admission::{CancellationReceipt, RequestOwner},
    error::HubError,
    financial::Acceptance,
    settlement::{SettlementAuthority, SettlementFact, TerminalState},
};
use sqlx::Row;
use uuid::Uuid;

impl PgStore {
    pub(crate) async fn cancel_request(
        &self,
        owner: &RequestOwner,
        request_id: Uuid,
    ) -> Result<CancellationReceipt, HubError> {
        if owner.client_id.is_nil() || owner.principal_id.is_empty() || request_id.is_nil() {
            return Err(HubError::Invalid("cancellation owner"));
        }
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| db_failure(e, line!()))?;
        let context=sqlx::query("SELECT grant_id FROM requests WHERE installation_id=$1 AND id=$2 AND client_id=$3 AND principal_id=$4")
            .bind(self.installation_id).bind(request_id).bind(owner.client_id).bind(&owner.principal_id).fetch_optional(&mut *tx).await.map_err(|e|db_failure(e,line!()))?.ok_or(HubError::NotFound)?;
        // Revocation forbids new sends but does not prevent the issuer stopping an owned request.
        sqlx::query("SELECT id FROM clients WHERE installation_id=$1 AND id=$2 FOR UPDATE")
            .bind(self.installation_id)
            .bind(owner.client_id)
            .fetch_one(&mut *tx)
            .await
            .map_err(|e| db_failure(e, line!()))?;
        sqlx::query("SELECT id FROM grants WHERE installation_id=$1 AND id=$2 FOR UPDATE")
            .bind(self.installation_id)
            .bind(context.get::<Uuid, _>("grant_id"))
            .fetch_one(&mut *tx)
            .await
            .map_err(|e| db_failure(e, line!()))?;
        let request=sqlx::query("SELECT state,cancel_requested,operation_id,namespace_binding_id FROM requests WHERE installation_id=$1 AND id=$2 FOR UPDATE")
            .bind(self.installation_id).bind(request_id).fetch_one(&mut *tx).await.map_err(|e|db_failure(e,line!()))?;
        let state: String = request.get("state");
        let requested: bool = request.get("cancel_requested");
        if matches!(state.as_str(), "completed" | "failed" | "cancelled") {
            return Ok(CancellationReceipt {
                request_id,
                state,
                cancel_requested: requested,
            });
        }
        let attempts=sqlx::query("SELECT id,state,dispatch_fence FROM attempts WHERE installation_id=$1 AND request_id=$2 ORDER BY ordinal FOR UPDATE")
            .bind(self.installation_id).bind(request_id).fetch_all(&mut *tx).await.map_err(|e|db_failure(e,line!()))?;
        let before_dispatch = state == "admitted"
            && attempts.len() == 1
            && attempts[0].get::<String, _>("state") == "intended"
            && attempts[0]
                .get::<Option<Uuid>, _>("dispatch_fence")
                .is_none();
        if !requested {
            sqlx::query("UPDATE requests SET cancel_requested=true,version=version+1 WHERE installation_id=$1 AND id=$2").bind(self.installation_id).bind(request_id).execute(&mut *tx).await.map_err(|e|db_failure(e,line!()))?;
            sqlx::query("INSERT INTO audit_events(id,installation_id,actor,action,object_id,operation_id,namespace_binding_id,reason) VALUES($1,$2,$3,'request.cancel.intent',$4,$5,$6,'Owned cancellation; uncertainty retains reserve')")
                .bind(Uuid::new_v4()).bind(self.installation_id).bind(&owner.principal_id).bind(request_id.to_string()).bind(request.get::<Uuid,_>("operation_id")).bind(request.get::<Option<Uuid>,_>("namespace_binding_id")).execute(&mut *tx).await.map_err(|e|db_failure(e,line!()))?;
        }
        tx.commit().await.map_err(|e| db_failure(e, line!()))?;
        if before_dispatch {
            // A crash between these transactions leaves a durable intent, held reserve and no new claim.
            self.settle_fact(&SettlementFact {
                attempt_id: attempts[0].get("id"),
                authority: SettlementAuthority::BeforeDispatchCancellation {
                    cancellation: owner.clone(),
                },
                source: "hub-cancel-before-dispatch".into(),
                source_event_id: attempts[0].get::<Uuid, _>("id").to_string(),
                acceptance: Acceptance::NotAccepted,
                terminal: TerminalState::Cancelled,
                usage: None,
                receipt: None,
            })
            .await?;
            return Ok(CancellationReceipt {
                request_id,
                state: "cancelled".into(),
                cancel_requested: true,
            });
        }
        Ok(CancellationReceipt {
            request_id,
            state,
            cancel_requested: true,
        })
    }

    pub(crate) async fn recover_unclaimed_intents(&self) -> Result<u64, HubError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| db_failure(e, line!()))?;
        let rows=sqlx::query("SELECT id,operation_id,principal_id,namespace_binding_id FROM requests WHERE installation_id=$1 AND state='admitted' AND (intent_deadline IS NULL OR intent_deadline<=clock_timestamp()) ORDER BY id LIMIT 100 FOR UPDATE SKIP LOCKED")
            .bind(self.installation_id).fetch_all(&mut *tx).await.map_err(|e|db_failure(e,line!()))?;
        let mut recovered = 0;
        for request in rows {
            recovered+=sqlx::query("UPDATE attempts SET state='unknown',accepted='unknown',version=version+1 WHERE installation_id=$1 AND request_id=$2 AND state='intended' AND dispatch_fence IS NULL")
                .bind(self.installation_id).bind(request.get::<Uuid,_>("id")).execute(&mut *tx).await.map_err(|e|db_failure(e,line!()))?.rows_affected();
            sqlx::query("UPDATE requests SET state='unknown',version=version+1 WHERE installation_id=$1 AND id=$2").bind(self.installation_id).bind(request.get::<Uuid,_>("id")).execute(&mut *tx).await.map_err(|e|db_failure(e,line!()))?;
            sqlx::query("INSERT INTO audit_events(id,installation_id,actor,action,object_id,operation_id,namespace_binding_id,reason) VALUES($1,$2,$3,'request.recover.unclaimed',$4,$5,$6,'Expired unclaimed intent; reserve held; no redispatch')")
                .bind(Uuid::new_v4()).bind(self.installation_id).bind(request.get::<String,_>("principal_id")).bind(request.get::<Uuid,_>("id").to_string()).bind(request.get::<Uuid,_>("operation_id")).bind(request.get::<Option<Uuid>,_>("namespace_binding_id")).execute(&mut *tx).await.map_err(|e|db_failure(e,line!()))?;
        }
        tx.commit().await.map_err(|e| db_failure(e, line!()))?;
        Ok(recovered)
    }
}
