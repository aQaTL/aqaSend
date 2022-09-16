use std::error::Error;
use std::future::ready;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicU64, Ordering};

use futures::future::join_all;
use hyper::service::make_service_fn;
use hyper::Server;
use log::*;
use tokio::runtime::Runtime;

use aqa_send::db::{self, DbError};
use aqa_send::tasks;
use aqa_send::tasks::cleanup::{DEFAULT_CLEANUP_INTERVAL, DEFAULT_START_LAG};
use aqa_send::{AqaService, AqaServiceError};

fn main() -> Result<(), Box<dyn Error>> {
	aqa_logger::init();

	let tokio_runtime = Runtime::new().expect("Failed to build tokio Runtime");
	let tokio_handle = tokio_runtime.handle().clone();

	let addr = SocketAddr::from(([127, 0, 0, 1], 8000));

	let cwd = std::env::current_dir()?;
	let db_handle = db::init(&cwd)?;

	// let file_appender = tracing_appender::rolling::never(cwd.join(DB_DIR).join("logs"), "prefix.log");
	// let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);
	// tracing_subscriber::FmtSubscriber::builder()
	// 	.init();
	// console_subscriber::init();
	// tracing::subscriber::set_global_default(subscriber)?;

	let c_db = db_handle.clone();
	ctrlc::set_handler(move || {
		static TRY_COUNT: AtomicU64 = AtomicU64::new(0);
		if TRY_COUNT.fetch_add(1, Ordering::Relaxed) >= 3 {
			std::process::exit(0);
		}
		let c_db = c_db.clone();

		let result: Result<(), DbError> = tokio_handle.block_on(c_db.save());

		match result {
			Ok(()) => {
				info!("DB serialized to disk successfully");
				std::process::exit(0);
			}
			Err(err) => {
				error!("Failed to serialize db to disk: {err:?}");
			}
		}
	})?;

	let guard = tokio_runtime.enter();

	#[cfg(not(target_os = "linux"))]
	let servers = {
		info!("Bind address: {}", addr);
		vec![Server::bind(&addr)]
	};
	#[cfg(target_os = "linux")]
	let servers = {
		match systemd_socket_activation::systemd_socket_activation() {
			Ok(sockets) if !sockets.is_empty() => {
				info!("Using {} sockets from systemd", sockets.len());
				let mut servers = Vec::with_capacity(sockets.len());
				for socket in sockets {
					servers.push(Server::from_tcp(socket)?);
				}
				servers
			}
			Ok(_) => {
				info!("Bind address: {}", addr);
				vec![Server::bind(&addr)]
			}
			Err(err) => {
				error!("Systemd socket activation failed: {:?}", err);
				info!("Bind address: {}", addr);
				vec![Server::bind(&addr)]
			}
		}
	};

	tokio::spawn(tasks::cleanup::cleanup_task(
		db_handle.clone(),
		DEFAULT_CLEANUP_INTERVAL,
		DEFAULT_START_LAG,
	));

	drop(guard);

	tokio_runtime
		.block_on(join_all(servers.into_iter().map(|server| {
			server.serve(make_service_fn(|_addr_stream| {
				let db = db_handle.clone();
				ready(Result::<AqaService, AqaServiceError>::Ok(AqaService::new(
					db,
				)))
			}))
		})))
		.into_iter()
		.try_for_each(|result| result)?;

	Ok(())
}
