use std::path::Path;
use thiserror::Error;
use uuid::Uuid;

use crate::FileEntry;

pub async fn init(db_dir: &Path) -> Result<Db, DbError> {
	Ok(Db {})
}

#[derive(Debug, Error)]
pub enum DbError {}

pub struct Db {}

impl Clone for Db {
	fn clone(&self) -> Self {
		Db {}
	}
}

impl Db {
	pub fn get(&self, uuid: &Uuid) -> Result<Option<FileEntry>, DbError> {
		todo!()
	}

	// Interior mutability incoming
	pub fn put(&self, uuid: &Uuid, file_entry: FileEntry) -> Result<(), DbError> {
		todo!()
	}

	pub fn iter(&self) -> impl Iterator<Item = (&Uuid, &FileEntry)> {
		#[allow(unreachable_code)]
		{
			todo!();
			std::iter::empty()
		}
	}
}
