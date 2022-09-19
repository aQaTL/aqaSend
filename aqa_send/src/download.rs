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

use crate::db::{self, Db};
use crate::db_stuff::FileEntry;
use crate::headers::{DownloadCount, Password};
use crate::{StatusCode, PASSWORD};

#[derive(Debug, Error)]
pub enum DownloadError {
	#[error(transparent)]
	FileSendIo(std::io::Error),
	#[error(transparent)]
	Http(#[from] hyper::http::Error),
	#[error("File id is not a valid uuid")]
	Uuid(#[from] uuid::Error),
	#[error(transparent)]
	Serialization(#[from] serde_json::Error),

	#[error(transparent)]
	Db(#[from] db::DbError),

	#[error("File id not found or not present")]
	NotFound,
	#[error("Invalid password")]
	InvalidPassword,
}

pub async fn download(
	uuid: String,
	req: Request<Body>,
	db: Db,
) -> Result<Response<Body>, DownloadError> {
	let uuid = Uuid::parse_str(&uuid)?;
	debug!("Downloading {}", uuid);

	let mut file_entry: FileEntry = db
		.get(&uuid)
		.await
		.ok_or(DownloadError::NotFound)?
		.to_owned();

	if let Some(Password(ref password)) = file_entry.password {
		let provided_password = req
			.headers()
			.get(PASSWORD)
			.ok_or(DownloadError::InvalidPassword)?;
		if provided_password != password {
			return Err(DownloadError::InvalidPassword);
		}
	}

	if let DownloadCount::Count(max_count) = file_entry.download_count_type {
		if file_entry.download_count >= max_count {
			return Err(DownloadError::NotFound);
		}
	}

	debug!(
		"Increasing download count to {}",
		file_entry.download_count + 1
	);
	file_entry.download_count += 1;
	db.update(&uuid, file_entry.clone()).await?;

	// Make it immutable to prevent unsaved changes
	// let file_entry = file_entry;

	let mut file_path = db.config.db_path.clone();
	file_path.push(file_entry.download_count_type.to_string());
	file_path.push(uuid.to_string());

	if !file_path.exists() {
		return Err(DownloadError::NotFound);
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
