use std::time::SystemTime;

use bytes::{Buf, BufMut, BytesMut};
use futures::StreamExt;
use hyper::{Body, Request, Response, StatusCode};
use log::*;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::io::AsyncWriteExt;
use uuid::Uuid;

use crate::db::Db;
use crate::db_stuff::FileEntry;
use crate::headers::{DownloadCount, HeaderError, Lifetime, Password, Visibility, DOWNLOAD_COUNT};
use crate::PASSWORD;

#[derive(Debug, Error)]
pub enum UploadError {
	#[error(transparent)]
	Http(#[from] hyper::http::Error),
	#[error(transparent)]
	Multipart(#[from] MultipartError),
	#[error("Failed to create new file")]
	FileCreate(std::io::Error),
	#[error("Io error occurred when writing uploaded file")]
	FileWrite(std::io::Error),
	#[error("Upload requires `multipart/form-data` Content-Type")]
	InvalidContentType,
	#[error("Multipart/form-data upload must define a boundary")]
	BoundaryExpected,
	#[error(transparent)]
	AqaHeader(#[from] HeaderError),
	#[error(transparent)]
	DbSerialize(#[from] serde_json::Error),
}

#[derive(Serialize, Deserialize)]
pub struct UploadResponse(pub Vec<UploadedFile>);

#[derive(Debug, Serialize, Deserialize)]
pub struct UploadedFile {
	pub uuid: Uuid,
	pub filename: String,
}

#[tracing::instrument]
pub async fn upload(req: Request<Body>, db: Db) -> Result<Response<Body>, UploadError> {
	use UploadError::{BoundaryExpected, FileCreate, FileWrite, InvalidContentType};

	let (parts, body) = req.into_parts();

	let content_type = parts
		.headers
		.get("content-type")
		.ok_or(InvalidContentType)?
		.to_str()
		.map_err(|_| InvalidContentType)?;

	let boundary = content_type
		.strip_prefix("multipart/form-data; ")
		.ok_or(BoundaryExpected)?
		.strip_prefix("boundary=")
		.ok_or(BoundaryExpected)?;
	let boundary = format!("--{}", boundary);
	debug!("Boundary: {}", boundary);

	let download_count: DownloadCount = parts.headers.get(DOWNLOAD_COUNT).try_into()?;
	let password: Option<Password> = parts.headers.get(PASSWORD).map(TryInto::try_into())?;

	let mut multipart = Multipart {
		body,
		boundary,
		buf: BytesMut::default(),
	};

	let mut uploaded_files: Vec<UploadedFile> = Vec::new();

	loop {
		let header = match multipart.read_header().await {
			Ok(v) => v,
			Err(MultipartError::NotEnoughData) => break,
			Err(err) => return Err(err.into()),
		};
		info!("Uploading {}", header.file_name);

		let upload_uuid = Uuid::new_v4();

		let mut path = db.config.db_path.clone();
		path.push(download_count.to_string());
		path.push(upload_uuid.to_string());

		let mut file = tokio::fs::File::create(path).await.map_err(FileCreate)?;

		let file_entry = FileEntry {
			filename: header.file_name,
			content_type: header
				.content_type
				.unwrap_or_else(|| String::from("application/octet-stream")),

			download_count_type: download_count,
			download_count: 0,

			visibility: Visibility::default(),
			password: password.clone(),

			lifetime: Lifetime::default(),
			upload_date: SystemTime::now(),
		};
		db.put(upload_uuid, file_entry.clone()).await;

		while let Some(chunk) = multipart.read_data().await {
			let chunk = chunk?;
			file.write_all(&chunk).await.map_err(FileWrite)?;
		}

		uploaded_files.push(UploadedFile {
			uuid: upload_uuid,
			filename: file_entry.filename,
		});
	}
	debug!("uploaded: {uploaded_files:?}");

	let upload_response = UploadResponse(uploaded_files);
	let upload_response_json =
		tokio::task::block_in_place(|| serde_json::to_vec_pretty(&upload_response))?;

	Ok(Response::builder()
		.status(StatusCode::OK)
		.body(Body::from(upload_response_json))?)
}

#[derive(Debug)]
pub struct Multipart {
	body: Body,
	boundary: String,

	buf: BytesMut,
}

pub struct MultipartHeader {
	_name: String,
	file_name: String,
	content_type: Option<String>,
}

#[derive(Debug, Error)]
pub enum MultipartError {
	#[error(transparent)]
	Hyper(#[from] hyper::Error),
	#[error("Not enough data")]
	NotEnoughData,
	#[error("Boundary not present when expected")]
	BoundaryExpected,
	#[error("Multipart header must contain valid utf8 data")]
	HeaderUtf8Error(std::str::Utf8Error),
	#[error("Form must be encoded in `key: value` format")]
	MalformedForm,
	#[error("Content-Disposition must be set to form-data")]
	ContentDispositionInvalidType,
	#[error("Content-Disposition must have fields in `key=\"value\";` format")]
	ContentDispositionInvalidFormat,
	#[error("Field `name` must be set in Content-Disposition")]
	NameNotFound,
	#[error("Field `filename` must be set in Content-Disposition")]
	FileNameNotFound,
}

impl Multipart {
	#[tracing::instrument]
	pub async fn read_header(&mut self) -> Result<MultipartHeader, MultipartError> {
		use MultipartError::*;

		let boundary_len_with_crlf = self.boundary.len() + 2;
		while self.buf.len() < boundary_len_with_crlf * 2 {
			let chunk = match self.body.next().await {
				Some(Ok(chunk)) => chunk,
				Some(Err(err)) => return Err(err.into()),
				None => break,
			};
			self.buf.put_slice(chunk.as_ref());
		}

		if self.buf.len() < boundary_len_with_crlf * 2 {
			return Err(NotEnoughData);
		}
		if &self.buf[..self.boundary.len()] != self.boundary.as_bytes() {
			return Err(BoundaryExpected);
		}

		self.buf.advance(self.boundary.len() + 2); // +2 bytes to also skip CR LF

		let header_bytes = loop {
			let double_crlf_found = self
				.buf
				.windows(4)
				.enumerate()
				.find(|(_idx, chunk)| chunk == b"\r\n\r\n");

			match double_crlf_found {
				Some((idx, _)) => {
					let header_bytes = self.buf.split_to(idx + 2);
					break header_bytes;
				}
				None => match self.body.next().await {
					Some(Ok(buf)) => self.buf.put_slice(&buf),
					Some(Err(err)) => return Err(err.into()),
					None => return Err(NotEnoughData),
				},
			}
		};

		let mut name = None;
		let mut file_name = None;
		let mut content_type = None;

		let header_bytes: &[u8] = header_bytes.as_ref();
		let header: &str = std::str::from_utf8(header_bytes).map_err(HeaderUtf8Error)?;
		for line in header.split("\r\n").filter(|line| !line.is_empty()) {
			let (key, value) = {
				let mut split = line.split(": ");
				let key = split.next().ok_or(MalformedForm)?;
				let value = split.next().ok_or(MalformedForm)?;
				(key, value)
			};

			match key {
				"Content-Disposition" => {
					let mut content_disposition = value.split("; ");
					match content_disposition.next() {
						Some("form-data") => (),
						_ => return Err(ContentDispositionInvalidType),
					}
					for cd in content_disposition {
						let (key, value) = {
							let mut split = cd.split('=');
							let key = split.next().ok_or(ContentDispositionInvalidFormat)?;
							let value = split.next().ok_or(ContentDispositionInvalidFormat)?;
							(key, value)
						};
						match key {
							"name" => name = Some(value.to_string()),
							"filename" => {
								let mut value = value;
								if value.starts_with('"') {
									value = &value[1..];
								}
								if value.ends_with('"') {
									value = &value[..(value.len() - 1)];
								}
								file_name = Some(value.to_string())
							}
							_ => debug!("Unknown key: {}", key),
						}
					}
				}
				"Content-Type" => {
					content_type = Some(value.to_string());
				}
				_ => (),
			}
		}

		self.buf.advance(2);

		Ok(MultipartHeader {
			_name: name.ok_or(NameNotFound)?,
			file_name: file_name.ok_or(FileNameNotFound)?,
			content_type,
		})
	}

	#[tracing::instrument]
	pub async fn read_data(&mut self) -> Option<Result<BytesMut, MultipartError>> {
		if self.buf.is_empty() {
			match self.body.next().await {
				Some(Ok(buf)) => self.buf.put_slice(&buf),
				Some(Err(err)) => return Some(Err(err.into())),
				None => return None,
			}
		}

		let boundary_found =
			self.buf
				.windows(self.boundary.len() + 2)
				.enumerate()
				.find(|(_idx, chunk)| {
					&chunk[..2] == b"\r\n"
						&& &chunk[2..(2 + self.boundary.len())] == self.boundary.as_bytes()
				});

		match boundary_found {
			Some((0, _)) => {
				self.buf.advance(2);
				None
			}
			Some((idx, _)) => {
				let bytes = self.buf.split_to(idx);
				Some(Ok(bytes))
			}
			None => {
				let bytes = self.buf.split();
				Some(Ok(bytes))
			}
		}
	}
}
