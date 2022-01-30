extern crate core;

use std::error::Error;
use std::future::{ready, Future, Ready};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::pin::Pin;
use std::task::{Context, Poll};

use hyper::server::conn::AddrStream;
use hyper::service::Service;
use hyper::Server;
use hyper::{Body, Method, Request, Response, StatusCode};
use log::*;
use thiserror::Error;

mod logger;
mod upload;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
	logger::init();

	let addr = SocketAddr::from(([127, 0, 0, 1], 8000));
	info!("Bind address: {}", addr);

	init_app_directory_structure()?;

	let server = Server::bind(&addr).serve(MakeService);
	server.await?;

	Ok(())
}

#[derive(Debug, Error)]
enum InitAppFolderStructureError {
	#[error(transparent)]
	Io(#[from] std::io::Error),
}

static DB_DIR: &str = "DB";
const DIRS_BY_DOWNLOAD_COUNT: [&str; 4] = ["1", "5", "10", "100"];

fn init_app_directory_structure() -> Result<(), InitAppFolderStructureError> {
	let cwd = std::env::current_dir()?;
	let db_dir = cwd.join(DB_DIR);
	if !db_dir.exists() {
		info!(
			"Database directory (\"{}\"), doesn't exist. Initializing app directory structure.",
			db_dir.display()
		);
		std::fs::create_dir(&db_dir)?;
	}

	for dir in DIRS_BY_DOWNLOAD_COUNT {
		let dir: PathBuf = db_dir.join(dir);
		if !dir.exists() {
			std::fs::create_dir(&dir)?;
		}
	}

	info!("Directory structure initialized");
	Ok(())
}

struct MakeService;

impl Service<&AddrStream> for MakeService {
	type Response = AqaService;
	type Error = AqaServiceError;
	type Future = Ready<Result<Self::Response, Self::Error>>;

	fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
		Poll::Ready(Ok(()))
	}

	fn call(&mut self, _req: &AddrStream) -> Self::Future {
		ready(Ok(AqaService))
	}
}

pub struct AqaService;

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
		match (method, path.as_slice()) {
			(Method::GET, ["api"]) => Box::pin(hello(req)),
			(Method::POST, ["api", "upload"]) => Box::pin(handle_response(upload::upload(req))),
			_ => Box::pin(ready(Ok(Response::builder()
				.status(StatusCode::NOT_FOUND)
				.body("Not found\n".into())
				.unwrap()))),
		}
	}
}

async fn handle_response<E: std::error::Error>(
	resp: impl Future<Output = Result<Response<Body>, E>>,
) -> Result<Response<Body>, AqaServiceError> {
	match resp.await {
		Ok(resp) => Ok(resp),
		Err(err) => {
			error!("{:?}", err);

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

async fn hello(_req: Request<Body>) -> Result<Response<Body>, AqaServiceError> {
	Ok(Response::builder()
		.status(StatusCode::OK)
		.body("Hello from aqaSend\n".into())?)
}

fn split_uri_path(path: &str) -> impl Iterator<Item = &str> {
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
