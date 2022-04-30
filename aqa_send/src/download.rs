use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use std::task::{Context, Poll};

use bytes::{Bytes, BytesMut};
use futures::Stream;
use hyper::{Body, Request, Response};
use log::*;
use thiserror::Error;
use tokio::io::AsyncReadExt;
use uuid::Uuid;

use crate::db::Db;
use crate::db_stuff::FileEntry;
use crate::headers::DownloadCount;
use crate::{db, StatusCode, DB_DIR};

#[derive(Debug, Error)]
pub enum DownloadError {
	#[error(transparent)]
	Http(#[from] hyper::http::Error),
	#[error("File id not found or not present")]
	NotFound,
	#[error("File id is not a valid uuid")]
	Uuid(#[from] uuid::Error),
	#[error(transparent)]
	Db(#[from] db::DbError),
	#[error(transparent)]
	Serialization(#[from] serde_json::Error),
	#[error(transparent)]
	FileSendIo(std::io::Error),
}

pub async fn download(
	uuid: String,
	_req: Request<Body>,
	db: Db,
) -> Result<Response<Body>, DownloadError> {
	let uuid = Uuid::parse_str(&uuid)?;
	debug!("Downloading {}", uuid);

	let mut file_entry: FileEntry = db
		.get(&uuid)
		.await
		.ok_or(DownloadError::NotFound)?
		.to_owned();

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

	let file_path: PathBuf = [
		DB_DIR,
		&file_entry.download_count_type.to_string(),
		&uuid.to_string(),
	]
	.into_iter()
	.collect();

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
