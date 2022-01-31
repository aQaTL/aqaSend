#![allow(dead_code)]
use std::fmt::Formatter;

use hyper::http::HeaderValue;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::headers::HeaderError::DownloadCountParse;
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
	#[error("aqa-password header missing")]
	PasswordHeaderMissing,
	#[error("aqa-lifetime header missing")]
	LifetimeHeaderMissing,

	#[error("Invalid aqa-download-count header value")]
	DownloadCountParse,
	#[error("Unsupported download count")]
	DownloadCountInvalidCount,
}

#[derive(Serialize, Deserialize)]
pub enum Visibility {
	Public,
	Private,
}

impl Default for Visibility {
	fn default() -> Self {
		Visibility::Public
	}
}

#[derive(Copy, Clone, Serialize, Deserialize)]
pub enum DownloadCount {
	Infinite,
	Count(u64),
}

impl Default for DownloadCount {
	fn default() -> Self {
		DownloadCount::Count(1)
	}
}

#[derive(Serialize, Deserialize)]
pub enum Password {
	None,
	Some(String),
}

impl Default for Password {
	fn default() -> Self {
		Password::None
	}
}

#[derive(Serialize, Deserialize)]
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
		let v = v.to_str().map_err(|_| DownloadCountParse)?;
		if !crate::DIRS_BY_DOWNLOAD_COUNT.contains(&v) {
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
