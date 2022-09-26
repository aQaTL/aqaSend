use argon2::password_hash::rand_core::OsRng;
use argon2::password_hash::SaltString;
use argon2::{Argon2, PasswordHasher};
use std::future::{ready, Future};
use std::pin::Pin;
use std::task::{Context, Poll};

use crate::db::{Db, DbError};
use crate::db_stuff::{Account, AccountType, FileEntry};
use crate::files::DB_DIR;
use crate::headers::{DownloadCount, Lifetime, DOWNLOAD_COUNT, LIFETIME, PASSWORD, VISIBILITY};

use backtrace::Backtrace;
use console::Term;
use hyper::http::HeaderValue;
use hyper::service::Service;
use hyper::{Body, Method, Request, Response, StatusCode};
use log::*;
use thiserror::Error;
use zeroize::Zeroizing;

pub mod account;
pub mod db;
pub mod db_stuff;
pub mod download;
pub mod files;
pub mod headers;
pub mod list;
pub mod tasks;
pub mod upload;

pub struct AqaService {
	db: Db,
}

impl AqaService {
	pub fn new(db: Db) -> Self {
		AqaService { db }
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
				upload::upload(req, self.db.clone()),
				origin_header,
			)),
			(Method::OPTIONS, ["api", "upload"]) => Box::pin(preflight_request(req)),
			(Method::GET, ["api", "download", uuid]) => Box::pin(handle_response(
				download::download(uuid.to_string(), req, self.db.clone()),
				origin_header,
			)),
			(Method::GET, ["api", "list.json"]) => Box::pin(handle_response(
				list::list(req, self.db.clone()),
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

#[derive(Error, Debug)]
pub enum CreateAccountError {
	#[error(transparent)]
	FileOperation(#[from] std::io::Error),

	#[error(transparent)]
	DbError(#[from] DbError),

	#[error("Account with that username already exists")]
	AccountAlreadyExists,

	#[error("Entered passwords don't match")]
	PasswordsDoNotMatch,

	#[error("Failed to hash the password: {0:?}")]
	PasswordHashingError(argon2::password_hash::Error),
}

pub async fn create_account(name: String, acc_type: AccountType) -> Result<(), CreateAccountError> {
	let cwd = std::env::current_dir()?;
	let db_handle = db::init(&cwd)?;

	{
		let mut accounts_guard = db_handle.accounts_writer().await;

		if accounts_guard.get(&name).is_some() {
			return Err(CreateAccountError::AccountAlreadyExists);
		}

		let password = prompt_for_password()?;

		accounts_guard.insert(
			name,
			Account {
				password_hash: hash_password(password)?,
				acc_type,
			},
		);
	}

	db_handle.save().await?;

	Ok(())
}

fn prompt_for_password() -> Result<Zeroizing<String>, CreateAccountError> {
	println!("Password: ");
	let password = Zeroizing::new(Term::stdout().read_secure_line()?);
	println!("Confirm password: ");
	let confirmed_password = Zeroizing::new(Term::stdout().read_secure_line()?);
	if *password != *confirmed_password {
		return Err(CreateAccountError::PasswordsDoNotMatch);
	}
	Ok(password)
}

fn hash_password(password: Zeroizing<String>) -> Result<String, CreateAccountError> {
	let salt = SaltString::generate(&mut OsRng);
	let argon2 = Argon2::default();
	let password_hash = argon2
		.hash_password(password.as_bytes(), &salt)
		.map_err(CreateAccountError::PasswordHashingError)?
		.to_string();
	Ok(password_hash)
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
