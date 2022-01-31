use crate::headers::DownloadCount;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct FileEntry {
	pub download_count_type: DownloadCount,
	pub download_count: u64,
	pub filename: String,
	pub content_type: String,
}
