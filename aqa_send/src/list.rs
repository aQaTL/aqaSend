use crate::FileEntry;
use hyper::{Body, Request, Response, StatusCode};
use serde::Serialize;
use thiserror::Error;
use uuid::Uuid;

use crate::db::Db;

#[derive(Debug, Error)]
pub enum ListError {
	#[error(transparent)]
	Http(#[from] hyper::http::Error),
	#[error(transparent)]
	Json(#[from] serde_json::Error),
}

#[derive(Serialize)]
struct FileModel<'a> {
	id: &'a Uuid,
	#[serde(flatten)]
	file_entry: &'a FileEntry,
}

pub async fn list(_req: Request<Body>, db: Db) -> Result<Response<Body>, ListError> {
	let list: Vec<FileModel> = db
		.iter()
		// .map(|(key, value)| {
		// 	(
		// 		Uuid::from_slice(&key).unwrap(),
		// 		bincode::deserialize(&value),
		// 	)
		// })
		// .map(|(id, file_entry)| match file_entry {
		// 	Ok(file_entry) => Some(FileModel { id: id.clone(), file_entry: file_entry.to_owned() }),
		// 	Err(_) => None,
		// })
		.map(|(key, value)| FileModel {
			id: key,
			file_entry: value,
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
