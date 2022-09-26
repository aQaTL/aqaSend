use log::info;
use std::collections::HashMap;
use std::fs::File;
use std::io;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::{OwnedRwLockReadGuard, OwnedRwLockWriteGuard, RwLock};
use tokio::task::JoinError;
use uuid::Uuid;

use crate::db_stuff::Account;
use crate::files::InitAppFolderStructureError;
use crate::{files, FileEntry, DB_DIR};

const DB_FILE: &str = "index";
const ACCOUNTS_FILE: &str = "accounts";

pub fn init(working_dir: &Path) -> Result<Db, DbError> {
	files::init_app_directory_structure(working_dir)?;

	let db_path = working_dir.join(DB_DIR);
	let db_file_path = db_path.join(DB_FILE);
	let accounts_path = db_path.join(ACCOUNTS_FILE);

	let db_config = Box::leak(Box::new(DbConfig {
		db_path,
		db_file_path: db_file_path.clone(),
		accounts_path: accounts_path.clone(),
	}));

	let db: HashMap<Uuid, FileEntry> = match File::open(&db_file_path) {
		Ok(mut db_file) => serde_json::from_reader(&mut db_file)?,
		Err(err) if err.kind() == ErrorKind::NotFound => Default::default(),
		Err(err) => return Err(DbError::DbFileOperation(err)),
	};

	let accounts: HashMap<String, Account> = match File::open(&accounts_path) {
		Ok(mut accounts_file) => serde_json::from_reader(&mut accounts_file)?,
		Err(err) if err.kind() == ErrorKind::NotFound => Default::default(),
		Err(err) => return Err(DbError::DbFileOperation(err)),
	};

	Ok(Db {
		data: Arc::new(RwLock::new(db)),
		accounts: Arc::new(RwLock::new(accounts)),
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

	#[error("Blocking task failed")]
	BlockingTaskJoinError(#[from] JoinError),
}

pub type DbDataHM = HashMap<Uuid, FileEntry>;
pub type AccountsHM = HashMap<String, Account>;

#[derive(Debug)]
pub struct Db {
	data: Arc<RwLock<DbDataHM>>,
	accounts: Arc<RwLock<AccountsHM>>,
	pub config: &'static DbConfig,
}

impl Clone for Db {
	fn clone(&self) -> Self {
		Db {
			data: Arc::clone(&self.data),
			accounts: Arc::clone(&self.accounts),
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

	pub async fn accounts_reader(&self) -> OwnedRwLockReadGuard<AccountsHM> {
		self.accounts.clone().read_owned().await
	}

	pub async fn accounts_writer(&self) -> OwnedRwLockWriteGuard<AccountsHM> {
		self.accounts.clone().write_owned().await
	}

	pub async fn update(&self, uuid: &Uuid, new_file_entry: FileEntry) -> Result<(), DbError> {
		let mut write_guard = self.data.write().await;
		let file_entry = write_guard.get_mut(uuid).ok_or(DbError::UpdateFail)?;
		*file_entry = new_file_entry;
		Ok(())
	}

	pub async fn save(&self) -> Result<(), DbError> {
		info!("Serializing db to disk");

		let data_hm: DbDataHM = {
			let data_guard = self.data.read().await;
			data_guard.clone()
		};

		let accounts_hm: AccountsHM = {
			let accounts_guard = self.accounts.read().await;
			accounts_guard.clone()
		};

		let config: &'static DbConfig = self.config;

		tokio::task::spawn_blocking(move || {
			let mut file = File::create(&config.db_file_path)?;
			serde_json::to_writer(&mut file, &data_hm)?;

			let mut file = File::create(&config.accounts_path)?;
			serde_json::to_writer(&mut file, &accounts_hm)?;

			Result::<(), DbError>::Ok(())
		})
		.await??;

		info!("Db serialized and saved to disk");

		Ok(())
	}
}

#[derive(Debug)]
pub struct DbConfig {
	pub db_path: PathBuf,
	pub db_file_path: PathBuf,
	pub accounts_path: PathBuf,
}
