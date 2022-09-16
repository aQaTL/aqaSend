use log::info;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum InitAppFolderStructureError {
	#[error(transparent)]
	Io(#[from] std::io::Error),
}

pub const DB_DIR: &str = "DB";
pub const DIRS_BY_DOWNLOAD_COUNT: [&str; 5] = ["1", "5", "10", "100", "infinite"];

pub fn init_app_directory_structure(dir: &Path) -> Result<(), InitAppFolderStructureError> {
	let db_dir = dir.join(DB_DIR);
	if !db_dir.exists() {
		info!(
			"Database directory (\"{}\"), doesn't exist. Initializing app directory structure.",
			db_dir.display()
		);
		std::fs::create_dir(&db_dir)?;
	}

	for dir in DIRS_BY_DOWNLOAD_COUNT {
		let dir: PathBuf = db_dir.join(dir);
		if !dir.exists() {
			std::fs::create_dir(&dir)?;
		}
	}

	info!("Directory structure initialized");
	Ok(())
}
