use crate::FileEntry;
use hyper::{Body, Request, Response, StatusCode};
use rocksdb::{IteratorMode, DB};
use std::sync::Arc;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ListError {
	#[error(transparent)]
	Http(#[from] hyper::http::Error),
	#[error(transparent)]
	Json(#[from] serde_json::Error),
}

pub async fn list(_req: Request<Body>, db: Arc<DB>) -> Result<Response<Body>, ListError> {
	let list: Vec<FileEntry> = db
		.iterator(IteratorMode::Start)
		.map(|(_key, value)| bincode::deserialize(&value))
		.filter_map(|result| result.ok())
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
