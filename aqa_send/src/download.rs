use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use bytes::{Bytes, BytesMut};
use futures::stream::poll_fn;
use futures::{Stream, StreamExt};
use hyper::{Body, Request, Response};
use log::*;
use rocksdb::DB;
use thiserror::Error;
use tokio::io::{AsyncRead, AsyncReadExt};
use uuid::Uuid;

use crate::db_stuff::FileEntry;
use crate::headers::DownloadCount;
use crate::{split_uri_path, StatusCode, DB_DIR};

#[derive(Debug, Error)]
pub enum DownloadError {
	#[error(transparent)]
	Http(#[from] hyper::http::Error),
	#[error("File id not found or not present")]
	NotFound,
	#[error("File id is not a valid uuid")]
	Uuid(#[from] uuid::Error),
	#[error(transparent)]
	Db(#[from] rocksdb::Error),
	#[error(transparent)]
	Serialization(#[from] bincode::Error),
	#[error(transparent)]
	FileSendIo(std::io::Error),
}

pub async fn download(
	uuid: String,
	req: Request<Body>,
	db: Arc<DB>,
) -> Result<Response<Body>, DownloadError> {
	// let mut uri_path = split_uri_path(req.uri().path()).skip(2);
	// let uuid = uri_path.next().ok_or(DownloadError::NotFound)?;
	let uuid = Uuid::parse_str(&uuid)?;
	debug!("Downloading {}", uuid);

	let mut file_entry: FileEntry =
		bincode::deserialize(&db.get(uuid.as_bytes())?.ok_or(DownloadError::NotFound)?)?;

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
	db.put(&uuid.as_bytes(), bincode::serialize(&file_entry)?)?;

	// Make it immutable to prevent unsaved changes
	let file_entry = file_entry;

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
		.status(StatusCode::OK)
		.body(Body::wrap_stream(FileStream {
			file,
			buffer: BytesMut::with_capacity(1024 * 1024 * 100),
		}))?)
}

struct FileStream {
	file: tokio::fs::File,
	buffer: BytesMut,
}

impl Future for FileStream {
	type Output = Result<Bytes, std::io::Error>;

	fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
		unsafe { Pin::new_unchecked(&mut self.get_unchecked_mut().foo()).poll(cx) }
	}
}

impl FileStream {
	async fn foo(&mut self) -> Result<Bytes, std::io::Error> {
		match self.file.read_buf(&mut self.buffer).await {
			Ok(size) => Ok(self.buffer.split_to(size).freeze()),
			Err(err) => Err(err),
		}
	}
}

impl Stream for FileStream {
	type Item = Result<Bytes, std::io::Error>;

	fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
		self.poll(cx).map(|x| match x {
			Ok(x) if x.is_empty() => None,
			_ => Some(x),
		})
	}
}
