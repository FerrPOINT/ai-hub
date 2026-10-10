//! Installation-owned recovery/retention. This worker has no provider transport capability.
use crate::{financial::db_failure, postgres::PgStore, replay::ProtectedResults, vault::Vault};
use aihub_application::{FinancialAdmission, FoundationStore, ResultDelivery};
use aihub_domain::error::HubError;
use std::{sync::Arc, time::Duration};
use tokio::sync::watch;

pub struct MaintenanceWorker {
    store: Arc<PgStore>,
    results: ProtectedResults,
}
#[derive(Debug, Default)]
pub struct MaintenanceStats {
    pub cancellations: u64,
    pub expired_intents: u64,
    pub expired_dispatches: u64,
    pub expired_payloads: u64,
    pub expired_snapshots: u64,
    pub failed_tasks: u32,
}
fn record(stats: &mut MaintenanceStats, task: &'static str, result: Result<u64, HubError>) -> u64 {
    match result {
        Ok(count) => count,
        Err(_) => {
            stats.failed_tasks += 1;
            tracing::warn!(task, "AI Hub maintenance task failed; retry on next cycle");
            0
        }
    }
}
impl MaintenanceWorker {
    pub fn new(store: Arc<PgStore>, vault: Arc<Vault>) -> Result<Self, HubError> {
        let results = ProtectedResults::new(store.clone(), vault)?;
        Ok(Self { store, results })
    }
    pub async fn run_once(&self) -> Result<MaintenanceStats, HubError> {
        self.store.ready().await?;
        let mut stats = MaintenanceStats::default();
        // Persisted explicit no-send cancellation is settled before uncertainty recovery.
        match self.store.resume_pending_cancellations().await {
            Ok((count, failed)) => {
                stats.cancellations = count;
                stats.failed_tasks += u32::from(failed > 0)
            }
            Err(_) => {
                stats.failed_tasks += 1;
                tracing::warn!(task = "cancel", "AI Hub cancellation scan unavailable");
            }
        }
        stats.expired_intents = record(
            &mut stats,
            "intent",
            self.store.recover_expired_intents().await,
        );
        stats.expired_dispatches = record(
            &mut stats,
            "dispatch",
            self.store.recover_expired_dispatches().await,
        );
        stats.expired_payloads = record(
            &mut stats,
            "payload",
            self.results.purge_expired_results(100).await,
        );
        let snapshots=sqlx::query("DELETE FROM read_snapshots WHERE id IN (SELECT id FROM read_snapshots WHERE installation_id=$1 AND expires_at<=clock_timestamp() ORDER BY expires_at,id LIMIT 100 FOR UPDATE SKIP LOCKED)")
            .bind(self.store.installation_id).execute(&self.store.pool).await.map(|r|r.rows_affected()).map_err(|e|db_failure(e,line!()));
        stats.expired_snapshots = record(&mut stats, "snapshot", snapshots);
        Ok(stats)
    }
    pub async fn run(
        self,
        tick: Duration,
        mut stop: watch::Receiver<bool>,
    ) -> Result<(), HubError> {
        if !(Duration::from_secs(1)..=Duration::from_secs(60)).contains(&tick) {
            return Err(HubError::Invalid("maintenance interval"));
        }
        let mut interval = tokio::time::interval(tick);
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            if *stop.borrow() {
                break;
            }
            tokio::select! {biased;
                changed=stop.changed()=>{if changed.is_err() || *stop.borrow() {break}},
                _=interval.tick()=>{
                    match self.run_once().await {
                        Ok(stats) if stats.cancellations+stats.expired_intents+stats.expired_dispatches+stats.expired_payloads+stats.expired_snapshots>0 || stats.failed_tasks>0=>tracing::info!(?stats,"AI Hub maintenance cycle"),
                        Ok(_)=>(),
                        Err(_)=>tracing::warn!(task="readiness","AI Hub maintenance unavailable; no unchecked mutation"),
                    }
                }
            }
        }
        Ok(())
    }
}
