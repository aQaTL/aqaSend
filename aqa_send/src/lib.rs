use std::future::{ready, Future};
use std::ops::Deref;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use backtrace::Backtrace;
use dashmap::DashMap;
use hyper::http::HeaderValue;
use hyper::service::Service;
use hyper::{Body, Method, Request, Response, StatusCode};
use log::*;
use thiserror::Error;
use uuid::Uuid;

use crate::db::{Db, DbError};
use crate::db_stuff::{Account, AccountType, FileEntry};
use crate::files::DB_DIR;
use crate::headers::{DownloadCount, Lifetime, DOWNLOAD_COUNT, LIFETIME, PASSWORD, VISIBILITY};

pub mod account;
pub mod cli_commands;
pub mod cookie;
pub mod db;
pub mod db_stuff;
pub mod download;
pub mod error;
pub mod files;
pub mod headers;
pub mod list;
pub mod tasks;
pub mod upload;

pub struct AqaService {
	db: Db,
	authorized_users: AuthorizedUsers,
}

/// Concurrent hashmap containing logged in users
/// key: Uuid of session cookie
/// value: Uuid of an user
#[derive(Default, Clone)]
pub struct AuthorizedUsers(Arc<DashMap<Uuid, Uuid>>);

impl AuthorizedUsers {
	pub fn get_user_uuid(&self, session_uuid: &Uuid) -> Option<Uuid> {
		self.get(session_uuid).map(|entry| *entry.value())
	}
}

impl Deref for AuthorizedUsers {
	type Target = Arc<DashMap<Uuid, Uuid>>;

	fn deref(&self) -> &Self::Target {
		&self.0
	}
}

impl AqaService {
	pub fn new(db: Db) -> Self {
		AqaService {
			db,
			authorized_users: AuthorizedUsers::default(),
		}
	}
}

#[derive(Debug, Error)]
pub enum AqaServiceError {
	#[error(transparent)]
	Hyper(#[from] hyper::Error),
	#[error(transparent)]
	Http(#[from] hyper::http::Error),
	#[error(transparent)]
	Io(#[from] std::io::Error),
}

impl Service<Request<Body>> for AqaService {
	type Response = Response<Body>;
	type Error = AqaServiceError;
	#[allow(clippy::type_complexity)]
	type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

	fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
		Poll::Ready(Ok(()))
	}

	fn call(&mut self, req: Request<Body>) -> Self::Future {
		debug!("{:?}", req);
		let uri_path = req.uri().path().to_owned();
		let path: Vec<&str> = split_uri_path(&uri_path).collect();
		let method = req.method().clone();
		let origin_header = req
			.headers()
			.get("origin")
			.map(|hv: &HeaderValue| hv.to_owned());
		match (method, path.as_slice()) {
			(Method::GET, ["api"]) => Box::pin(hello(req)),
			(Method::POST, ["api", "upload"]) => Box::pin(handle_response(
				upload::upload(req, self.db.clone(), self.authorized_users.clone()),
				origin_header,
			)),
			(Method::OPTIONS, ["api", "upload"]) => Box::pin(preflight_request(req)),
			(Method::GET, ["api", "download", uuid]) => Box::pin(handle_response(
				download::download(uuid.to_string(), req, self.db.clone()),
				origin_header,
			)),
			(Method::GET, ["api", "list.json"]) => Box::pin(handle_response(
				list::list(req, self.db.clone(), self.authorized_users.clone()),
				origin_header,
			)),
			(Method::POST, ["api", "login"]) => Box::pin(handle_response(
				account::login(req, self.db.clone(), self.authorized_users.clone()),
				origin_header,
			)),
			(Method::POST, ["api", "logout"]) => Box::pin(handle_response(
				account::logout(req, self.db.clone(), self.authorized_users.clone()),
				origin_header,
			)),
			_ => Box::pin(ready(Ok(Response::builder()
				.status(StatusCode::NOT_FOUND)
				.body("Not found\n".into())
				.unwrap()))),
		}
	}
}

async fn handle_response<E: std::error::Error>(
	resp: impl Future<Output = Result<Response<Body>, E>>,
	origin_header: Option<HeaderValue>,
) -> Result<Response<Body>, AqaServiceError> {
	match resp.await {
		Ok(mut resp) => {
			if let Some(hv) = origin_header {
				resp.headers_mut().append("Access-Control-Allow-Origin", hv);
			}
			debug!("{:?}", resp);
			Ok(resp)
		}
		Err(err) => {
			error!("{:?}", err);

			let backtrace = Backtrace::new();
			error!("{:?}", backtrace);

			let body = if cfg!(debug_assertions) {
				Body::from(err.to_string())
			} else {
				Body::from("")
			};
			Ok(Response::builder()
				.status(StatusCode::INTERNAL_SERVER_ERROR)
				.body(body)?)
		}
	}
}

async fn preflight_request(req: Request<Body>) -> Result<Response<Body>, AqaServiceError> {
	Ok(Response::builder()
		.status(StatusCode::NO_CONTENT)
		.header(
			"Access-Control-Allow-Origin",
			req.headers().get("origin").unwrap(),
		)
		.header("Access-Control-Allow-Methods", "OPTIONS, POST")
		.header(
			"Access-Control-Allow-Headers",
			format!(
				"Content-Type, {}, {}, {}, {}",
				VISIBILITY, DOWNLOAD_COUNT, PASSWORD, LIFETIME
			),
		)
		.header("Access-Control-Max-Age", (60 * 60).to_string())
		.body(Body::from(""))?)
}

async fn hello(_req: Request<Body>) -> Result<Response<Body>, AqaServiceError> {
	Ok(Response::builder()
		.status(StatusCode::OK)
		.body("Hello from aqaSend\n".into())?)
}

pub fn split_uri_path(path: &str) -> impl Iterator<Item = &str> {
	path.split('/').filter(|segment| !segment.is_empty())
}

#[cfg(test)]
mod tests {
	#[test]
	fn uri_path_splitter() {
		let uri = "/";
		let mut path = super::split_uri_path(uri);
		assert_eq!(path.next(), None);

		let uri = "/index.html";
		let mut path = super::split_uri_path(uri);
		assert_eq!(path.next(), Some("index.html"));
		assert_eq!(path.next(), None);

		let uri = "/index/.html";
		let mut path = super::split_uri_path(uri);
		assert_eq!(path.next(), Some("index"));
		assert_eq!(path.next(), Some(".html"));
		assert_eq!(path.next(), None);

		let uri = "/index.html/";
		let mut path = super::split_uri_path(uri);
		assert_eq!(path.next(), Some("index.html"));
		assert_eq!(path.next(), None);
	}
}
