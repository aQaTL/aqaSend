use log::{debug, error, info};
use std::path::PathBuf;
use std::time::Duration;
use tokio::time::Instant;
use uuid::Uuid;

use crate::{Db, DownloadCount, Lifetime, DB_DIR};

/// Default cleanup interval is 1 hour
pub const DEFAULT_CLEANUP_INTERVAL: Duration = Duration::from_secs(60 * 60);
pub const DEFAULT_START_LAG: Duration = Duration::from_secs(60);

pub async fn cleanup_task(db: Db, cleanup_interval: Duration, start_lag: Duration) {
	let mut cleanup_tick = tokio::time::interval_at(Instant::now() + start_lag, cleanup_interval);

	loop {
		let _ = cleanup_tick.tick().await;
		debug!("Cleanup task starting");

		let mut db_entries_to_delete = Vec::<Uuid>::new();
		let mut deleted_files_count: u64 = 0;

		let mut writer_lock = db.writer().await;

		for (uuid, file_entry) in writer_lock.iter_mut() {
			if let DownloadCount::Count(max_count) = file_entry.download_count_type {
				if file_entry.download_count >= max_count {
					db_entries_to_delete.push(*uuid);

					let mut file_path = PathBuf::from(DB_DIR);
					file_path.push(file_entry.download_count_type.to_string());
					file_path.push(uuid.to_string());

					match tokio::fs::remove_file(file_path).await {
						Ok(_) => deleted_files_count += 1,
						Err(err) => error!("DB error when deleting {}: {:?}", uuid, err),
					}
					continue;
				}
			}

			if let Lifetime::Duration(lifetime) = file_entry.lifetime {
				if let Ok(elapsed) = file_entry.upload_date.elapsed() {
					if elapsed > lifetime {
						db_entries_to_delete.push(*uuid);

						//TODO(aqatl): remove lifetime bounded file from disk
						continue;
					}
				}
			}
		}

		for uuid in db_entries_to_delete {
			writer_lock.remove(&uuid);
		}

		debug!("Cleanup task finished");
		info!("Cleanup removed {} files.", deleted_files_count);
	}
}
