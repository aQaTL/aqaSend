use crate::{FileEntry, Lifetime};
use hyper::{Body, Request, Response, StatusCode};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::error;
use uuid::Uuid;

use crate::db::Db;
use crate::headers::Visibility;

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

#[derive(Deserialize)]
pub struct OwnedFileModel {
	pub id: Uuid,
	#[serde(flatten)]
	pub file_entry: FileEntry,
}

pub async fn list(_req: Request<Body>, db: Db) -> Result<Response<Body>, ListError> {
	let db_reader = db.reader().await;
	let list: Vec<FileModel> = db_reader
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
		.filter(|(_key, entry)| matches!(entry.visibility, Visibility::Public))
		.filter(|(_key, entry): &(_, &FileEntry)| match entry.lifetime {
			Lifetime::Duration(lifetime) => match entry.upload_date.elapsed() {
				Ok(elapsed) => elapsed <= lifetime,
				Err(err) => {
					error!(
						"Failed to get elapsed upload time. Diff: {:?}",
						err.duration()
					);
					false
				}
			},
			Lifetime::Infinite => true,
		})
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
