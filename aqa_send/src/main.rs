use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Request, Response, Server};
use log::*;
use std::convert::Infallible;
use std::net::SocketAddr;

mod logger;

#[tokio::main]
async fn main() {
	logger::init();

	let addr = SocketAddr::from(([127, 0, 0, 1], 8000));
	info!("Bind address: {}", addr);

	let make_service = make_service_fn(|_conn| async { Ok::<_, Infallible>(service_fn(handle)) });

	let server = Server::bind(&addr).serve(make_service);

	let server_exit: Result<_, _> = server.await;
	let _ = server_exit.unwrap();
}

async fn handle(_req: Request<Body>) -> Result<Response<Body>, Infallible> {
	Ok(Response::new(Body::from("Hello, World")))
}
