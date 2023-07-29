use crate::cookie::parse_cookie;
use crate::{AuthorizedUsers, FileEntry, HandlerError, HttpHandlerError, Lifetime};
use hyper::{Body, Request, Response, StatusCode};
use log::debug;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::error;
use uuid::Uuid;

use crate::db::Db;
use crate::error::{ErrorContentType, Field, IntoHandlerError};
use crate::headers::{DownloadCount, Visibility};

#[derive(Debug, Error)]
pub enum ListError {
	#[error(transparent)]
	Json(#[from] serde_json::Error),

	#[error("malformed session cookie")]
	MalformedSessionCookie(uuid::Error),

	#[error("{0:?} must only contain visible ascii characters")]
	AsciiOnly(Field),

	#[error("Malformed {0:?} data")]
	Malformed(Field),

	#[error("Session expired")]
	SessionExpired,

	#[error("Authorized user doesn't exist")]
	UnknownUser,
}

impl HttpHandlerError for ListError {
	fn code(&self) -> StatusCode {
		match self {
			ListError::Json(_) => StatusCode::INTERNAL_SERVER_ERROR,
			ListError::MalformedSessionCookie(_) => StatusCode::BAD_REQUEST,
			ListError::AsciiOnly(_) => StatusCode::BAD_REQUEST,
			ListError::Malformed(_) => StatusCode::BAD_REQUEST,
			ListError::SessionExpired => StatusCode::BAD_REQUEST,
			ListError::UnknownUser => StatusCode::BAD_REQUEST,
		}
	}

	fn user_presentable(&self) -> bool {
		match self {
			ListError::Json(_) => false,
			ListError::MalformedSessionCookie(_) => true,
			ListError::AsciiOnly(_) => true,
			ListError::Malformed(_) => true,
			ListError::SessionExpired => true,
			ListError::UnknownUser => true,
		}
	}

	fn content_type() -> ErrorContentType {
		ErrorContentType::Json
	}
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

pub async fn list(
	req: Request<Body>,
	db: Db,
	authorized_users: AuthorizedUsers,
) -> Result<Response<Body>, HandlerError<ListError>> {
	let uploader = match req.headers().get("Cookie") {
		Some(cookie) => {
			let cookie_header = cookie
				.to_str()
				.map_err(|_| ListError::AsciiOnly(Field::Cookie))?;
			let (_, cookies) =
				parse_cookie(cookie_header).map_err(|_| ListError::Malformed(Field::Cookie))?;
			debug!("Cookies: {cookies:?}");
			match cookies.get("session") {
				Some(session_cookie) => {
					let session_cookie: Uuid = session_cookie
						.parse()
						.map_err(ListError::MalformedSessionCookie)?;

					let user_uuid = authorized_users
						.get_user_uuid(&session_cookie)
						.ok_or(ListError::SessionExpired)?;
					debug!("Getting user with uuid {user_uuid}");
					Some(
						db.get_account(&user_uuid)
							.await
							.ok_or(ListError::UnknownUser)?,
					)
				}
				None => None,
			}
		}
		None => None,
	};

	let db_reader = db.reader().await;
	let list: Vec<FileModel> = db_reader
		.iter()
		.filter(|(_uuid, entry)| {
			if matches!(entry.visibility, Visibility::Public) {
				true
			} else {
				match (entry.uploader_uuid, &uploader) {
					(Some(uploader_uuid), Some(uploader)) => uploader_uuid == uploader.uuid,
					_ => false,
				}
			}
		})
		.filter(|(_key, entry): &(_, &FileEntry)| {
			if let Lifetime::Duration(lifetime) = entry.lifetime {
				match entry.upload_date.elapsed() {
					Ok(elapsed) => {
						if elapsed > lifetime {
							return false;
						}
					}
					Err(err) => {
						error!(
							"Failed to get elapsed upload time. Diff: {:?}",
							err.duration()
						);
						return false;
					}
				}
			}

			if let DownloadCount::Count(max_count) = entry.download_count_type {
				if entry.download_count >= max_count {
					return false;
				}
			}

			true
		})
		.map(|(key, value)| FileModel {
			id: key,
			file_entry: value,
		})
		.collect();

	debug!("Serving file list ({} files)", list.len());

	let resp = if cfg!(debug_assertions) {
		serde_json::to_vec_pretty(&list)
	} else {
		serde_json::to_vec(&list)
	}
	.into_handler_error()?;

	Ok(Response::builder()
		.status(StatusCode::OK)
		.header("Content-Type", "application/json")
		.body(Body::from(resp))?)
}
