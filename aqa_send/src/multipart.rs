use bytes::{Buf, BufMut, BytesMut};

use futures::StreamExt;
use hyper::Body;
use log::debug;
use thiserror::Error;

#[derive(Debug)]
pub struct Multipart {
	body: Body,
	boundary: String,
	/// Maximum allowed size of the request
	max_size: usize,
	/// Count of already read bytes from the request
	read_size: usize,

	buf: BytesMut,
}

#[derive(Debug)]
pub struct MultipartHeader {
	pub name: String,
	pub file_name: Option<String>,
	pub content_type: Option<String>,
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
	#[error("Request too big")]
	TooBig,
}

impl Multipart {
	pub fn new(body: Body, boundary: String, max_size: usize) -> Self {
		Multipart {
			body,
			boundary,
			max_size,
			read_size: 0,
			buf: BytesMut::default(),
		}
	}

	pub async fn read_all_chunks(
		&mut self,
	) -> Result<Vec<(MultipartHeader, BytesMut)>, MultipartError> {
		let mut chunks = Vec::new();

		loop {
			let header = match self.read_header().await {
				Ok(v) => v,
				Err(MultipartError::NotEnoughData) => break,
				Err(err) => return Err(err),
			};

			let mut chunk_data = BytesMut::with_capacity(self.max_size);
			while let Some(chunk) = self.read_data().await {
				let chunk = chunk?;
				if self.read_size + chunk.len() > self.max_size {
					return Err(MultipartError::TooBig);
				}
				self.read_size += chunk.len();
				chunk_data.put(chunk);
			}

			chunks.push((header, chunk_data));
		}

		Ok(chunks)
	}

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
							"name" => {
								let mut value = value;
								if value.starts_with('"') {
									value = &value[1..];
								}
								if value.ends_with('"') {
									value = &value[..(value.len() - 1)];
								}
								name = Some(value.to_string())
							}
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
			name: name.ok_or(NameNotFound)?,
			file_name,
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
