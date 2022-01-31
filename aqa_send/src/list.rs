use crate::FileEntry;
use hyper::{Body, Request, Response, StatusCode};
use rocksdb::{IteratorMode, DB};
use serde::Serialize;
use std::sync::Arc;
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum ListError {
	#[error(transparent)]
	Http(#[from] hyper::http::Error),
	#[error(transparent)]
	Json(#[from] serde_json::Error),
}

#[derive(Serialize)]
struct FileModel {
	id: Uuid,
	#[serde(flatten)]
	file_entry: FileEntry,
}

pub async fn list(_req: Request<Body>, db: Arc<DB>) -> Result<Response<Body>, ListError> {
	let list: Vec<FileModel> = db
		.iterator(IteratorMode::Start)
		.map(|(key, value)| {
			(
				Uuid::from_slice(&key).unwrap(),
				bincode::deserialize(&value),
			)
		})
		.filter_map(|(id, file_entry)| match file_entry {
			Ok(file_entry) => Some(FileModel { id, file_entry }),
			Err(_) => None,
		})
		.collect();

	let resp = if cfg!(debug_assertions) {
		serde_json::to_vec_pretty(&list)
	} else {
		serde_json::to_vec(&list)
	}?;

	Ok(Response::builder()
		.status(StatusCode::OK)
		.header("Content-Type", "application/json")
		.body(Body::from(resp))?)
}
