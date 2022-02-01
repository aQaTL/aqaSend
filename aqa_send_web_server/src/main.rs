use futures::future::join_all;
use hyper::service::{make_service_fn, Service};
use hyper::{Body, Request, Response, Server, StatusCode};
use log::*;
use std::convert::Infallible;
use std::future::{ready, Ready};
use std::net::SocketAddr;
use std::task::{Context, Poll};

// DIST: HashMap<&'static str, &'static [u8]>
dir_embedder::embed_dir!(DIST, "aqa_send_web/dist");

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
	aqa_logger::init();

	let addr = SocketAddr::from(([127, 0, 0, 1], 8080));

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

	join_all(servers.into_iter().map(|server| {
		server.serve(make_service_fn(|_addr_stream| {
			ready(Result::<MemoryFilesService, Infallible>::Ok(
				MemoryFilesService,
			))
		}))
	}))
	.await
	.into_iter()
	.try_for_each(|result| result)?;

	Ok(())
}

pub struct MemoryFilesService;

impl Service<Request<Body>> for MemoryFilesService {
	type Response = Response<Body>;
	type Error = hyper::http::Error;
	type Future = Ready<Result<Self::Response, Self::Error>>;

	fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
		Poll::Ready(Ok(()))
	}

	fn call(&mut self, req: Request<Body>) -> Self::Future {
		let mut path = req.uri().path().trim_start_matches('/');
		if path.is_empty() {
			path = "index.html"
		}
		info!("Request {:?}", req);

		let file: &'static [u8] = match DIST.get(path) {
			Some(v) => *v,
			None => match DIST.get("index.html") {
				Some(v) => *v,
				None => return ready(not_found()),
			},
		};

		ready(Response::builder().status(200).body(Body::from(file)))
	}
}

fn not_found() -> Result<Response<Body>, hyper::http::Error> {
	Response::builder()
		.status(StatusCode::NOT_FOUND)
		.body("not found".into())
}
