use serde::{Deserialize, Serialize};
use std::time::SystemTime;
// use uuid::Uuid;

use crate::headers::{DownloadCount, Lifetime, Password, Visibility};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FileEntry {
	// uuid: Uuid,
	pub filename: String,
	pub content_type: String,

	pub download_count_type: DownloadCount,
	pub download_count: u64,

	pub visibility: Visibility,
	pub password: Option<Password>,

	pub lifetime: Lifetime,
	pub upload_date: SystemTime,
}
