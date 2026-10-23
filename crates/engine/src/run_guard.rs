//! Finalize abandoned runs, including a cancelled collection future.
use sqlx::PgPool;
use std::time::Duration;

pub(crate) struct RunGuard {
    pool: PgPool,
    id: Option<i64>,
}

impl RunGuard {
    pub(crate) fn new(pool: &PgPool, id: Option<i64>) -> Self {
        Self {
            pool: pool.clone(),
            id,
        }
    }

    pub(crate) fn disarm(&mut self) {
        self.id = None;
    }

    pub(crate) async fn fail(&mut self, reason: &'static str) -> Result<(), sqlx::Error> {
        if let Some(id) = self.id {
            finish(&self.pool, id, "failed", reason).await?;
        }
        self.disarm();
        Ok(())
    }
}

async fn finish(pool: &PgPool, id: i64, status: &str, reason: &str) -> Result<(), sqlx::Error> {
    let errors = serde_json::json!([{ "reason": reason }]);
    for attempt in 0..3 {
        let update = sqlx::query!("UPDATE ingestion_runs SET status=$2,finished_at=now(),errors=$3 WHERE id=$1 AND status='running'", id, status, errors).execute(pool);
        let result = tokio::time::timeout(Duration::from_secs(5), update)
            .await
            .unwrap_or(Err(sqlx::Error::PoolTimedOut));
        match result {
            Ok(_) => return Ok(()),
            Err(error) if attempt == 2 => return Err(error),
            Err(_) => tokio::time::sleep(Duration::from_millis(100)).await,
        }
    }
    Ok(())
}

impl Drop for RunGuard {
    fn drop(&mut self) {
        if let Some(id) = self.id.take()
            && let Ok(runtime) = tokio::runtime::Handle::try_current()
        {
            let pool = self.pool.clone();
            runtime.spawn(async move {
                if finish(
                    &pool,
                    id,
                    "interrupted",
                    "run interrupted before completion",
                )
                .await
                .is_err()
                {
                    eprintln!("Cannot finalize interrupted collection run: database unavailable");
                }
            });
        }
    }
}
