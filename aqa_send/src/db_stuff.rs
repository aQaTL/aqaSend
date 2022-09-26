use serde::{Deserialize, Serialize};
use std::str::FromStr;
use std::time::SystemTime;
use thiserror::Error;
// use uuid::Uuid;

use crate::headers::{DownloadCount, Lifetime, Password, Visibility};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FileEntry {
	// uuid: Uuid,
	pub filename: String,
	pub content_type: String,
	pub uploader_username: Option<String>,

	pub download_count_type: DownloadCount,
	pub download_count: u64,

	pub visibility: Visibility,
	pub password: Option<Password>,

	pub lifetime: Lifetime,
	pub upload_date: SystemTime,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Account {
	//username: String,
	pub password_hash: String,
	pub acc_type: AccountType,
}

#[derive(Serialize, Deserialize, Debug, Copy, Clone)]
pub enum AccountType {
	Admin,
	User,
}

#[derive(Error, Debug)]
#[error("Possible account types: [admin|user]")]
pub struct AccountTypeParseError;

impl FromStr for AccountType {
	type Err = AccountTypeParseError;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		match s {
			"admin" => Ok(AccountType::Admin),
			"user" => Ok(AccountType::User),
			_ => Err(AccountTypeParseError),
		}
	}
}
