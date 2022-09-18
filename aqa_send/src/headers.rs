#![allow(dead_code)]
use std::fmt::Formatter;

use hyper::http::HeaderValue;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::files::DIRS_BY_DOWNLOAD_COUNT;

use crate::headers::Lifetime::Duration;

pub const VISIBILITY: &str = "aqa-visibility";
pub const DOWNLOAD_COUNT: &str = "aqa-download-count";
pub const PASSWORD: &str = "aqa-password";
pub const LIFETIME: &str = "aqa-lifetime";

#[derive(Debug, Error)]
pub enum HeaderError {
	#[error("aqa-visibility header missing")]
	VisibilityHeaderMissing,
	#[error("aqa-download-count header missing")]
	DownloadCountHeaderMissing,
	#[error("aqa-lifetime header missing")]
	LifetimeHeaderMissing,

	#[error("Invalid aqa-download-count header value")]
	DownloadCountParse,
	#[error("Unsupported download count")]
	DownloadCountInvalidCount,
	#[error("Invalid aqa-password header value")]
	PasswordParse,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Visibility {
	Public,
	Private,
}

impl Default for Visibility {
	fn default() -> Self {
		Visibility::Public
	}
}

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
pub enum DownloadCount {
	Infinite,
	Count(u64),
}

impl Default for DownloadCount {
	fn default() -> Self {
		DownloadCount::Count(1)
	}
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Password(String);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Lifetime {
	Infinite,
	Duration(std::time::Duration),
}

impl Default for Lifetime {
	fn default() -> Self {
		Duration(std::time::Duration::from_secs(60 * 60)) // 1 hour
	}
}

impl TryFrom<Option<&HeaderValue>> for DownloadCount {
	type Error = HeaderError;

	fn try_from(v: Option<&HeaderValue>) -> Result<Self, Self::Error> {
		let v = v.ok_or(HeaderError::DownloadCountHeaderMissing)?;
		let v = v.to_str().map_err(|_| HeaderError::DownloadCountParse)?;
		if !DIRS_BY_DOWNLOAD_COUNT.contains(&v) {
			return Err(HeaderError::DownloadCountInvalidCount);
		}
		if v == "infinite" {
			return Ok(DownloadCount::Infinite);
		}
		let count: u64 = v.parse().map_err(|_| HeaderError::DownloadCountParse)?;
		Ok(DownloadCount::Count(count))
	}
}

impl std::fmt::Display for DownloadCount {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		match self {
			DownloadCount::Infinite => write!(f, "infinite"),
			DownloadCount::Count(count) => write!(f, "{}", count),
		}
	}
}

impl TryFrom<&HeaderValue> for Password {
	type Error = HeaderError;

	fn try_from(v: &HeaderValue) -> Result<Self, Self::Error> {
		Ok(Password(
			v.to_str()
				.map_err(|_| HeaderError::PasswordParse)?
				.to_string(),
		))
	}
}
