use std::collections::HashMap;
use std::ops::Deref;
use std::path::Path;
use std::sync::Arc;
use uuid::Uuid;
use thiserror::Error;
use std::fs::File;
use tokio::task::spawn_blocking;
use crate::FileEntry;

/// hddd stands for HashMap Dump Database Directory 😂
const DB_DIR: &str = "hddd";
/// hddf stands for HashMap Dump Database File 😂
const DB_FILE: &str = "hddf";

pub async fn init(working_dir: &Path) -> Result<DbHandle, DbError> {
    let db_dir = working_dir.join(DB_DIR);
    let db_file = db_dir.join(DB_FILE);

    let db = spawn_blocking(move || load_db(&db_file)).await??;

    tokio::spawn(db_worker_task(db));

    let handle = DbHandle {
    };

    Ok(handle)
}

struct DbView {

}

impl DbView {

}

fn load_db(db_file: &Path) -> Result<Db, DbError> {
    let mut file = File::create(&db_file).map_err(DbError::DbFileOpen)?;
    let hm = bincode::deserialize_from(&mut file)?;
    Ok(Db { hm, file })
}

async fn db_worker_task(db: Db) {

}

pub struct Db {
    hm: HashMap<Uuid, FileEntry>,
    file: File,
}

#[derive(Debug, Error)]
pub enum DbError {
    #[error("Failed to open {} File: {:?}", DB_FILE, .0)]
    DbFileOpen(std::io::Error),
    #[error("Tokio error: {:?}", .0)]
    JoinError(#[from] tokio::task::JoinError),
    #[error(transparent)]
    Serialization(#[from] bincode::Error),
}

pub struct DbHandle {
}

impl Clone for DbHandle {
    fn clone(&self) -> Self {
        DbHandle {
            handle: self.handle.clone(),
        }
    }
}

impl Deref for DbHandle {
    type Target = Db;

    fn deref(&self) -> &Self::Target {
        self.handle.deref()
    }
}

impl DbHandle {
    pub async fn insert(&self, key: Uuid, value: FileEntry) -> Result<(), DbError> {
        todo!()
    }

    pub async fn remove(&self, key: Uuid) -> Result<FileEntry, DbError> {
        todo!()
    }

    pub async fn get(&self, key: Uuid) -> Result<Option<FileEntry>, DbError> {
        todo!()
    }

    pub fn iter(&self) -> impl Iterator<Item = FileEntry> {
        todo!();
        std::iter::empty()
    }
}

struct DbIter {

}

impl Iterator for Db {
    type Item = FileEntry;

    fn next(&mut self) -> Option<Self::Item> {
        todo!()
    }
}

