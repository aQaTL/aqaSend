use log::info;
use std::collections::HashMap;
use std::fs::File;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::{OwnedRwLockReadGuard, OwnedRwLockWriteGuard, RwLock};
use uuid::Uuid;

use crate::files::InitAppFolderStructureError;
use crate::{files, FileEntry, DB_DIR};

const DB_FILE: &str = "index";

pub fn init(working_dir: &Path) -> Result<Db, DbError> {
	files::init_app_directory_structure(working_dir)?;

	let db_path = working_dir.join(DB_DIR);
	let db_file_path = db_path.join(DB_FILE);

	let db_config = Box::leak(Box::new(DbConfig {
		db_path,
		db_file_path: db_file_path.clone(),
	}));

	if !db_file_path.exists() {
		info!("DB file doesn't exist. Creating.");
		File::create(&db_file_path).map_err(DbError::DbFileOperation)?;
		return Ok(Db {
			data: Default::default(),
			config: db_config,
		});
	}

	let mut db_file = File::open(&db_file_path).map_err(DbError::DbFileOperation)?;
	let db: HashMap<Uuid, FileEntry> = serde_json::from_reader(&mut db_file)?;

	Ok(Db {
		data: Arc::new(RwLock::new(db)),
		config: db_config,
	})
}

#[derive(Debug, Error)]
pub enum DbError {
	#[error(transparent)]
	DirectoryInit(#[from] InitAppFolderStructureError),

	#[error(transparent)]
	DbFileOperation(#[from] io::Error),

	#[error(transparent)]
	DbFileDeserialization(#[from] serde_json::Error),

	#[error("Db didn't contain requested item")]
	UpdateFail,
}

pub type DbDataHM = HashMap<Uuid, FileEntry>;

#[derive(Debug)]
pub struct Db {
	data: Arc<RwLock<DbDataHM>>,
	pub config: &'static DbConfig,
}

impl Clone for Db {
	fn clone(&self) -> Self {
		Db {
			data: Arc::clone(&self.data),
			config: self.config,
		}
	}
}

impl Db {
	pub async fn get(&self, uuid: &Uuid) -> Option<FileEntry> {
		self.data.read().await.get(uuid).map(ToOwned::to_owned)
	}

	pub async fn reader(&self) -> OwnedRwLockReadGuard<DbDataHM> {
		self.data.clone().read_owned().await
	}

	pub async fn writer(&self) -> OwnedRwLockWriteGuard<DbDataHM> {
		self.data.clone().write_owned().await
	}

	pub async fn put(&self, uuid: Uuid, file_entry: FileEntry) {
		self.data.write().await.insert(uuid, file_entry);
	}

	pub async fn update(&self, uuid: &Uuid, new_file_entry: FileEntry) -> Result<(), DbError> {
		let mut write_guard = self.data.write().await;
		let file_entry = write_guard.get_mut(uuid).ok_or(DbError::UpdateFail)?;
		*file_entry = new_file_entry;
		Ok(())
	}

	pub async fn save(&self) -> Result<(), DbError> {
		info!("Serializing db to disk");

		let guard = self.data.read().await;
		let hm: &DbDataHM = &*guard;

		let mut file = File::create(&self.config.db_file_path)?;
		serde_json::to_writer(&mut file, hm)?;

		Ok(())
	}
}

#[derive(Debug)]
pub struct DbConfig {
	pub db_path: PathBuf,
	pub db_file_path: PathBuf,
}
