use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

use bytes::{Bytes, BytesMut};
use futures::Stream;
use hyper::{Body, Request, Response};
use log::*;
use thiserror::Error;
use tokio::io::AsyncReadExt;
use uuid::Uuid;

use crate::account::{get_logged_in_user, AuthError};
use crate::db::{self, Db};
use crate::db_stuff::FileEntry;
use crate::error::{ErrorContentType, IntoHandlerError};
use crate::headers::{DownloadCount, Password, Visibility};
use crate::{uri_query_iter, AuthorizedUsers, HandlerError, HttpHandlerError, StatusCode};

#[derive(Debug, Error)]
pub enum DownloadError {
	#[error(transparent)]
	FileSendIo(std::io::Error),
	#[error("File id is not a valid uuid")]
	Uuid(#[from] uuid::Error),
	#[error(transparent)]
	Serialization(#[from] serde_json::Error),

	#[error(transparent)]
	Db(#[from] db::DbError),

	#[error(transparent)]
	AuthError(#[from] AuthError),

	#[error("File id not found or not present")]
	NotFound,
	#[error("Invalid password")]
	InvalidPassword,
}

impl HttpHandlerError for DownloadError {
	fn code(&self) -> StatusCode {
		match self {
			DownloadError::FileSendIo(_) => StatusCode::INTERNAL_SERVER_ERROR,
			DownloadError::Uuid(_) => StatusCode::BAD_REQUEST,
			DownloadError::Serialization(_) => StatusCode::INTERNAL_SERVER_ERROR,
			DownloadError::Db(_) => StatusCode::INTERNAL_SERVER_ERROR,
			DownloadError::AuthError(err) => err.code(),
			DownloadError::NotFound => StatusCode::NOT_FOUND,
			DownloadError::InvalidPassword => StatusCode::UNAUTHORIZED,
		}
	}

	fn user_presentable(&self) -> bool {
		match self {
			DownloadError::FileSendIo(_) => false,
			DownloadError::Uuid(_) => true,
			DownloadError::Serialization(_) => false,
			DownloadError::Db(_) => false,
			DownloadError::AuthError(err) => err.user_presentable(),
			DownloadError::NotFound => true,
			DownloadError::InvalidPassword => true,
		}
	}

	fn content_type() -> ErrorContentType {
		ErrorContentType::Json
	}
}

pub async fn download(
	uuid: String,
	req: Request<Body>,
	db: Db,
	authorized_users: AuthorizedUsers,
) -> Result<Response<Body>, HandlerError<DownloadError>> {
	let uuid = Uuid::parse_str(&uuid).into_handler_error()?;
	debug!("Downloading {}", uuid);

	let mut file_entry: FileEntry = db
		.get(&uuid)
		.await
		.ok_or(DownloadError::NotFound)?
		.to_owned();

	let current_user = get_logged_in_user(req.headers(), db.clone(), authorized_users.clone())
		.await
		.into_handler_error()?;
	if matches!(file_entry.visibility, Visibility::Private) {
		let authorized = match (current_user, file_entry.uploader_uuid) {
			(Some(current_user), Some(uploader)) => current_user.uuid == uploader,
			_ => false,
		};
		if !authorized {
			return Ok(Response::builder()
				.status(StatusCode::UNAUTHORIZED)
				.body(Body::empty())?);
		}
	}

	if let Some(Password(ref password)) = file_entry.password {
		let query = req.uri().query().ok_or(DownloadError::InvalidPassword)?;
		let (_, provided_password) = uri_query_iter(query)
			.find(|(key, _value)| *key == "password")
			.ok_or(DownloadError::InvalidPassword)?;

		let provided_password =
			urlencoding::decode(provided_password).map_err(|_| DownloadError::InvalidPassword)?;

		if &provided_password != password {
			return Err(DownloadError::InvalidPassword.into());
		}
	}

	if let DownloadCount::Count(max_count) = file_entry.download_count_type {
		if file_entry.download_count >= max_count {
			return Err(DownloadError::NotFound.into());
		}
	}

	debug!(
		"Increasing download count to {}",
		file_entry.download_count + 1
	);
	file_entry.download_count += 1;
	db.update(&uuid, file_entry.clone())
		.await
		.into_handler_error()?;

	// Make it immutable to prevent unsaved changes
	let file_entry = file_entry;

	let mut file_path = db.config.db_path.clone();
	file_path.push(file_entry.download_count_type.to_string());
	file_path.push(uuid.to_string());

	if !file_path.exists() {
		return Err(DownloadError::NotFound.into());
	}

	let file = tokio::fs::File::open(&file_path)
		.await
		.map_err(DownloadError::FileSendIo)?;

	Ok(Response::builder()
		.header(
			"Content-Disposition",
			format!("filename=\"{}\"", file_entry.filename),
		)
		.status(StatusCode::OK)
		.body(Body::wrap_stream(FileStream::new(file)))?)
}

struct FileStream {
	file: tokio::fs::File,
	buffer: BytesMut,
}

impl FileStream {
	fn new(file: tokio::fs::File) -> Self {
		FileStream {
			file,
			buffer: BytesMut::with_capacity(1024 * 1024 * 10),
		}
	}
}

impl Stream for FileStream {
	type Item = Result<Bytes, std::io::Error>;

	fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
		let mut fut = async move {
			let this = self.get_mut();
			match this.file.read_buf(&mut this.buffer).await {
				Ok(0) => None,
				Ok(count) => Some(Ok(this.buffer.split_to(count).freeze())),
				Err(err) => Some(Err(err)),
			}
		};
		(unsafe { Pin::new_unchecked(&mut fut) }).poll(cx)
	}
}
