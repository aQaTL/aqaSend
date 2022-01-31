use serde::{Deserialize, Serialize};
use std::time::SystemTime;

use crate::headers::{DownloadCount, Lifetime, Password, Visibility};

#[derive(Serialize, Deserialize)]
pub struct FileEntry {
	pub filename: String,
	pub content_type: String,

	pub download_count_type: DownloadCount,
	pub download_count: u64,

	pub visibility: Visibility,
	pub password: Password,

	pub lifetime: Lifetime,
	pub upload_date: SystemTime,
}
