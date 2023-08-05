use log::{debug, info};
use std::collections::hash_map::Entry;
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
const REGISTRATION_CODES_PATH: &str = "registration_codes";

pub fn init(working_dir: &Path) -> Result<Db, DbError> {
	files::init_app_directory_structure(working_dir)?;

	let db_path = working_dir.join(DB_DIR);
	let db_file_path = db_path.join(DB_FILE);
	let accounts_path = db_path.join(ACCOUNTS_FILE);
	let registration_codes_path = db_path.join(REGISTRATION_CODES_PATH);

	let db_config = Box::leak(Box::new(DbConfig {
		db_path,
		db_file_path: db_file_path.clone(),
		accounts_path: accounts_path.clone(),
		registration_codes_path: registration_codes_path.clone(),
	}));

	debug!("Reading {DB_FILE} file");
	let db: HashMap<Uuid, FileEntry> = match File::open(&db_file_path) {
		Ok(mut file) => serde_json::from_reader(&mut file).map_err(|err| DbError::Io {
			error: err.into(),
			file: DB_FILE,
		})?,
		Err(err) if err.kind() == ErrorKind::NotFound => Default::default(),
		Err(err) => {
			return Err(DbError::Io {
				error: DbIoError::DbFileOperation(err),
				file: DB_FILE,
			})
		}
	};

	debug!("Reading {ACCOUNTS_FILE} file");
	let accounts: HashMap<Uuid, Account> = match File::open(&accounts_path) {
		Ok(mut file) => serde_json::from_reader(&mut file).map_err(|err| DbError::Io {
			error: err.into(),
			file: ACCOUNTS_FILE,
		})?,
		Err(err) if err.kind() == ErrorKind::NotFound => Default::default(),
		Err(err) => {
			return Err(DbError::Io {
				error: DbIoError::DbFileOperation(err),
				file: ACCOUNTS_FILE,
			})
		}
	};

	let mut account_uuids = HashMap::<String, Uuid>::with_capacity(accounts.len());
	for account in accounts.values() {
		account_uuids.insert(account.username.clone(), account.uuid);
	}

	debug!("Reading {REGISTRATION_CODES_PATH} file");
	let registration_codes: Vec<String> = match File::open(&registration_codes_path) {
		Ok(mut file) => serde_json::from_reader(&mut file).map_err(|err| DbError::Io {
			error: err.into(),
			file: REGISTRATION_CODES_PATH,
		})?,
		Err(err) if err.kind() == ErrorKind::NotFound => Default::default(),
		Err(err) => {
			return Err(DbError::Io {
				error: DbIoError::DbFileOperation(err),
				file: REGISTRATION_CODES_PATH,
			})
		}
	};

	Ok(Db {
		file_entries: Arc::new(RwLock::new(db)),
		accounts: Arc::new(RwLock::new(accounts)),
		account_uuids: Arc::new(RwLock::new(account_uuids)),
		registration_codes: Arc::new(RwLock::new(registration_codes)),
		config: db_config,
	})
}

#[derive(Debug, Error)]
pub enum DbError {
	#[error(transparent)]
	DirectoryInit(#[from] InitAppFolderStructureError),

	#[error("Io error for \"{file}\": {error}")]
	Io {
		error: DbIoError,

		file: &'static str,
	},

	#[error("Db didn't contain requested item")]
	UpdateFail,

	#[error("Blocking task failed")]
	BlockingTaskJoinError(#[from] JoinError),

	#[error("Tried to add account with username that already exists")]
	AccountAlreadyExists,
}

#[derive(Debug, Error)]
pub enum DbIoError {
	#[error(transparent)]
	DbFileOperation(#[from] io::Error),

	#[error(transparent)]
	DbFileDeserialization(#[from] serde_json::Error),
}

pub type DbDataHM = HashMap<Uuid, FileEntry>;
pub type AccountsHM = HashMap<Uuid, Account>;
pub type AccountUuidsHM = HashMap<String, Uuid>;
pub type RegistrationCodesVec = Vec<String>;

#[derive(Debug)]
pub struct Db {
	file_entries: Arc<RwLock<DbDataHM>>,
	accounts: Arc<RwLock<AccountsHM>>,
	registration_codes: Arc<RwLock<Vec<String>>>,

	account_uuids: Arc<RwLock<AccountUuidsHM>>,

	pub config: &'static DbConfig,
}

impl Clone for Db {
	fn clone(&self) -> Self {
		Db {
			file_entries: Arc::clone(&self.file_entries),
			accounts: Arc::clone(&self.accounts),
			account_uuids: Arc::clone(&self.account_uuids),
			registration_codes: Arc::clone(&self.registration_codes),
			config: self.config,
		}
	}
}

impl Db {
	pub async fn get(&self, uuid: &Uuid) -> Option<FileEntry> {
		self.file_entries
			.read()
			.await
			.get(uuid)
			.map(ToOwned::to_owned)
	}

	pub async fn reader(&self) -> OwnedRwLockReadGuard<DbDataHM> {
		self.file_entries.clone().read_owned().await
	}

	pub async fn writer(&self) -> OwnedRwLockWriteGuard<DbDataHM> {
		self.file_entries.clone().write_owned().await
	}

	pub async fn put(&self, uuid: Uuid, file_entry: FileEntry) {
		self.file_entries.write().await.insert(uuid, file_entry);
	}

	pub async fn get_account(&self, uuid: &Uuid) -> Option<Account> {
		self.accounts.read().await.get(uuid).map(ToOwned::to_owned)
	}

	pub async fn accounts_reader(&self) -> OwnedRwLockReadGuard<AccountsHM> {
		self.accounts.clone().read_owned().await
	}

	pub async fn accounts_writer(&self) -> OwnedRwLockWriteGuard<AccountsHM> {
		self.accounts.clone().write_owned().await
	}

	pub async fn account_uuids_reader(&self) -> OwnedRwLockReadGuard<AccountUuidsHM> {
		self.account_uuids.clone().read_owned().await
	}

	pub async fn account_uuids_writer(&self) -> OwnedRwLockWriteGuard<AccountUuidsHM> {
		self.account_uuids.clone().write_owned().await
	}

	pub async fn registration_codes_reader(&self) -> OwnedRwLockReadGuard<RegistrationCodesVec> {
		self.registration_codes.clone().read_owned().await
	}

	pub async fn registration_codes_writer(&self) -> OwnedRwLockWriteGuard<RegistrationCodesVec> {
		self.registration_codes.clone().write_owned().await
	}

	pub async fn update(&self, uuid: &Uuid, new_file_entry: FileEntry) -> Result<(), DbError> {
		let mut write_guard = self.file_entries.write().await;
		let file_entry = write_guard.get_mut(uuid).ok_or(DbError::UpdateFail)?;
		*file_entry = new_file_entry;
		Ok(())
	}

	pub async fn save(&self) -> Result<(), DbError> {
		info!("Serializing db to disk");

		let data_hm: DbDataHM = {
			let data_guard = self.file_entries.read().await;
			data_guard.clone()
		};

		let accounts_hm: AccountsHM = {
			let accounts_guard = self.accounts.read().await;
			accounts_guard.clone()
		};

		let registration_codes_vec: RegistrationCodesVec = {
			let registration_codes_guard = self.registration_codes.read().await;
			registration_codes_guard.clone()
		};

		let config: &'static DbConfig = self.config;

		tokio::task::spawn_blocking(move || {
			let mut file = File::create(&config.db_file_path).map_err(|err| DbError::Io {
				error: err.into(),
				file: DB_FILE,
			})?;
			serde_json::to_writer(&mut file, &data_hm).map_err(|err| DbError::Io {
				error: err.into(),
				file: DB_FILE,
			})?;

			let mut file = File::create(&config.accounts_path).map_err(|err| DbError::Io {
				error: err.into(),
				file: ACCOUNTS_FILE,
			})?;
			serde_json::to_writer(&mut file, &accounts_hm).map_err(|err| DbError::Io {
				error: err.into(),
				file: ACCOUNTS_FILE,
			})?;

			let mut file =
				File::create(&config.registration_codes_path).map_err(|err| DbError::Io {
					error: err.into(),
					file: REGISTRATION_CODES_PATH,
				})?;
			serde_json::to_writer(&mut file, &registration_codes_vec).map_err(|err| {
				DbError::Io {
					error: err.into(),
					file: REGISTRATION_CODES_PATH,
				}
			})?;

			Result::<(), DbError>::Ok(())
		})
		.await??;

		info!("Db serialized and saved to disk");

		Ok(())
	}

	pub async fn add_account(&self, uuid: Uuid, account: Account) -> Result<(), DbError> {
		match self.accounts_writer().await.entry(uuid) {
			Entry::Occupied(_) => Err(DbError::AccountAlreadyExists),
			Entry::Vacant(entry) => {
				self.account_uuids_writer()
					.await
					.insert(account.username.clone(), uuid);
				entry.insert(account);
				Ok(())
			}
		}
	}
}

#[derive(Debug)]
pub struct DbConfig {
	pub db_path: PathBuf,
	pub db_file_path: PathBuf,
	pub accounts_path: PathBuf,
	pub registration_codes_path: PathBuf,
}
