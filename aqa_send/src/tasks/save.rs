use crate::db::DbError;
use crate::Db;
use log::{error, info};
use std::time::Duration;
use tokio::time::Instant;

pub const DEFAULT_SAVE_INTERVAL: Duration = Duration::from_secs(60 * 5);
pub const DEFAULT_START_LAG: Duration = Duration::from_secs(60);

pub async fn save_task(db: Db, save_interval: Duration, start_lag: Duration) {
	let mut save_tick = tokio::time::interval_at(Instant::now() + start_lag, save_interval);

	loop {
		let _ = save_tick.tick().await;

		let result: Result<(), DbError> = db.save().await;

		match result {
			Ok(()) => info!("DB serialized to disk successfully"),
			Err(err) => error!("Failed to serialize db to disk: {err:?}"),
		}
	}
}
