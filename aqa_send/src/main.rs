use futures::future::join_all;
use std::error::Error;
use std::future::{ready, Future};
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::Duration;

use crate::db_stuff::FileEntry;
use crate::headers::{DownloadCount, Lifetime, DOWNLOAD_COUNT, LIFETIME, PASSWORD, VISIBILITY};
use hyper::http::HeaderValue;
use hyper::service::{make_service_fn, Service};
use hyper::Server;
use hyper::{Body, Method, Request, Response, StatusCode};
use log::*;
use thiserror::Error;
use tokio::time::Instant;
use uuid::Uuid;

mod account;
mod db_stuff;
mod download;
mod headers;
mod list;
mod logger;
mod upload;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
	logger::init();

	let addr = SocketAddr::from(([127, 0, 0, 1], 8000));
	info!("Bind address: {}", addr);

	let db_dir: PathBuf = init_app_directory_structure()?;
	let db = Arc::new(init_db(&db_dir)?);

	tokio::spawn(cleanup_task(Arc::clone(&db)));

	#[cfg(not(target_os = "linux"))]
	let servers = vec![Server::bind(&addr)];
	#[cfg(target_os = "linux")]
	let servers = {
		match systemd_socket_activation::systemd_socket_activation() {
			Ok(sockets) if !sockets.is_empty() => {
				let mut servers = Vec::with_capacity(sockets.len());
				for socket in sockets {
					servers.push(Server::from_tcp(socket)?);
				}
				servers
			}
			Ok(_) => {
				vec![Server::bind(&addr)]
			}
			Err(err) => {
				error!("Systemd socket activation failed: {:?}", err);
				vec![Server::bind(&addr)]
			}
		}
	};

	join_all(servers.into_iter().map(|server| {
		server.serve(make_service_fn(|_addr_stream| {
			let db = Arc::clone(&db);
			ready(Result::<AqaService, AqaServiceError>::Ok(AqaService { db }))
		}))
	}))
	.await
	.into_iter()
	.try_for_each(|result| result)?;

	Ok(())
}

#[derive(Debug, Error)]
enum InitAppFolderStructureError {
	#[error(transparent)]
	Io(#[from] std::io::Error),
}

const DB_DIR: &str = "DB";
const DIRS_BY_DOWNLOAD_COUNT: [&str; 5] = ["1", "5", "10", "100", "infinite"];

fn init_app_directory_structure() -> Result<PathBuf, InitAppFolderStructureError> {
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
		let dir: PathBuf = db_dir.join(dir.to_string());
		if !dir.exists() {
			std::fs::create_dir(&dir)?;
		}
	}

	info!("Directory structure initialized");
	Ok(db_dir)
}

const ROCKSDB_DIR: &str = "index_db";

fn init_db(db_dir: &Path) -> Result<rocksdb::DB, rocksdb::Error> {
	use rocksdb::*;

	let rocks_db_dir = db_dir.join(ROCKSDB_DIR);

	let cf_opts = Options::default();
	let column_families = DIRS_BY_DOWNLOAD_COUNT
		.into_iter()
		.map(|count| ColumnFamilyDescriptor::new(format!("by_count_{}", count), cf_opts.clone()));

	let mut db_opts = Options::default();
	db_opts.create_missing_column_families(true);
	db_opts.create_if_missing(true);

	let db = DB::open_cf_descriptors(&db_opts, &rocks_db_dir, column_families)?;
	Ok(db)
}

async fn cleanup_task(db: Arc<rocksdb::DB>) {
	let mut cleanup_tick = tokio::time::interval_at(
		Instant::now() + Duration::from_secs(60),
		Duration::from_secs(60 * 60),
	);

	loop {
		let _ = cleanup_tick.tick().await;
		debug!("Cleanup task starting");

		let mut deleted_files_count: u64 = 0;
		for (key, value) in db.iterator(rocksdb::IteratorMode::Start) {
			let file_entry: FileEntry = match bincode::deserialize(&value) {
				Ok(v) => v,
				Err(_) => {
					warn!("Failed to deserialize {}", String::from_utf8_lossy(&key));
					continue;
				}
			};

			let uuid = Uuid::from_slice(&key).unwrap();

			if let DownloadCount::Count(max_count) = file_entry.download_count_type {
				if file_entry.download_count >= max_count {
					match db.delete(&key) {
						Ok(_) => (),
						Err(err) => error!("DB error when deleting {}: {:?}", uuid, err),
					}
					let mut file_path = PathBuf::from(DB_DIR);
					file_path.push(file_entry.download_count_type.to_string());
					file_path.push(uuid.to_string());

					match tokio::fs::remove_file(file_path).await {
						Ok(_) => deleted_files_count += 1,
						Err(err) => error!("DB error when deleting {}: {:?}", uuid, err),
					}
					continue;
				}
			}

			if let Lifetime::Duration(lifetime) = file_entry.lifetime {
				if let Ok(elapsed) = file_entry.upload_date.elapsed() {
					if elapsed > lifetime {
						match db.delete(&key) {
							Ok(_) => deleted_files_count += 1,
							Err(err) => error!("DB error when deleting {}: {:?}", uuid, err),
						}
						//TODO(aqatl): remove lifetime bounded file from disk
						continue;
					}
				}
			}
		}

		debug!("Cleanup task finished");
		info!("Cleanup removed {} files.", deleted_files_count);
	}
}

pub struct AqaService {
	db: Arc<rocksdb::DB>,
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
				upload::upload(req, Arc::clone(&self.db)),
				origin_header,
			)),
			(Method::OPTIONS, ["api", "upload"]) => Box::pin(preflight_request(req)),
			(Method::GET, ["api", "download", uuid]) => Box::pin(handle_response(
				download::download(uuid.to_string(), req, Arc::clone(&self.db)),
				origin_header,
			)),
			(Method::GET, ["api", "list.json"]) => Box::pin(handle_response(
				list::list(req, Arc::clone(&self.db)),
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
