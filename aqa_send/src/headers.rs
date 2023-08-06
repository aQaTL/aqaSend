#![allow(dead_code)]
use std::fmt::Formatter;
use std::time::Duration;

use hyper::http::HeaderValue;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::files::DIRS_BY_DOWNLOAD_COUNT;
use crate::{HttpHandlerError, StatusCode};

pub const VISIBILITY: &str = "aqa-visibility";
pub const DOWNLOAD_COUNT: &str = "aqa-download-count";
pub const PASSWORD: &str = "aqa-password";
pub const LIFETIME: &str = "aqa-lifetime";

#[derive(Debug, Error)]
pub enum HeaderError {
	#[error("aqa-download-count header missing")]
	DownloadCountHeaderMissing,
	#[error("aqa-lifetime header missing")]
	LifetimeHeaderMissing,
	#[error("Invalid aqa-lifetime header value. Possible values: [infinite|1 min|5 mins|1 hour|1 day|7 days|30 days]")]
	LifetimeValue,

	#[error("Invalid aqa-download-count header value")]
	DownloadCountParse,
	#[error("Unsupported download count")]
	DownloadCountInvalidCount,
	#[error("Invalid aqa-password header value")]
	PasswordParse,
	#[error("Invalid aqa-visibility header value")]
	VisibilityParse,
	#[error("Invalid aqa-lifetime header value")]
	LifetimeParse,
}

impl HttpHandlerError for HeaderError {
	fn code(&self) -> StatusCode {
		StatusCode::BAD_REQUEST
	}

	fn user_presentable(&self) -> bool {
		true
	}
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
pub enum Visibility {
	#[default]
	Public,
	Private,
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
pub struct Password(pub String);

#[derive(Debug, Copy, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Lifetime {
	Infinite,
	Duration(Duration),
}

impl Default for Lifetime {
	fn default() -> Self {
		Lifetime::Duration(Duration::from_secs(60 * 60)) // 1 hour
	}
}

impl TryFrom<Option<&HeaderValue>> for Lifetime {
	type Error = HeaderError;

	fn try_from(v: Option<&HeaderValue>) -> Result<Self, Self::Error> {
		let v = match v {
			Some(v) => v.to_str().map_err(|_| HeaderError::LifetimeParse)?,
			None => return Ok(Lifetime::Infinite),
		};
		match v {
			"infinite" => Ok(Lifetime::Infinite),
			"1 min" => Ok(Lifetime::Duration(Duration::from_secs(60))),
			"5 mins" => Ok(Lifetime::Duration(Duration::from_secs(60 * 5))),
			"1 hour" => Ok(Lifetime::Duration(Duration::from_secs(60 * 60))),
			"1 day" => Ok(Lifetime::Duration(Duration::from_secs(60 * 60 * 24))),
			"7 days" => Ok(Lifetime::Duration(Duration::from_secs(60 * 60 * 24 * 7))),
			"30 days" => Ok(Lifetime::Duration(Duration::from_secs(60 * 60 * 24 * 30))),
			_ => Err(HeaderError::LifetimeValue),
		}
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
		v.to_str()
			.map_err(|_| HeaderError::PasswordParse)
			.and_then(|password| {
				urlencoding::decode(password)
					.map(|password| password.to_string())
					.map_err(|_| HeaderError::PasswordParse)
			})
			.map(Password)
	}
}

impl TryFrom<Option<&HeaderValue>> for Visibility {
	type Error = HeaderError;

	fn try_from(v: Option<&HeaderValue>) -> Result<Self, Self::Error> {
		match v.map(|x| x.to_str()) {
			Some(Ok("public")) | None => Ok(Visibility::Public),
			Some(Ok("private")) => Ok(Visibility::Private),
			_ => Err(HeaderError::VisibilityParse),
		}
	}
}
