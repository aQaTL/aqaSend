use hyper::service::{make_service_fn, service_fn, Service};
use hyper::{Body, Request, Response, Server, StatusCode};
use log::*;
use std::convert::Infallible;
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::future::{ready, Ready};
use std::net::SocketAddr;
use std::task::{Context, Poll};

mod logger;

#[tokio::main]
async fn main() {
	logger::init();

	let addr = SocketAddr::from(([127, 0, 0, 1], 8000));
	info!("Bind address: {}", addr);

	let server = Server::bind(&addr).serve(MakeService);

	let server_exit: Result<_, _> = server.await;
	let _ = server_exit.unwrap();
}

struct MakeService;

impl<T> Service<T> for MakeService {
	type Response = AqaService;
	type Error = AqaServiceError;
	type Future = Ready<Result<Self::Response, Self::Error>>;

	fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
		Poll::Ready(Ok(()))
	}

	fn call(&mut self, _req: T) -> Self::Future {
		ready(Ok(AqaService))
	}
}

#[derive(Debug)]
struct AqaService;

#[derive(Debug)]
enum AqaServiceError {
	Hyper(hyper::Error),
}

impl Display for AqaServiceError {
	fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
		write!(f, "AqaServiceError")
	}
}

impl Error for AqaServiceError {}

impl From<hyper::Error> for AqaServiceError {
	fn from(err: hyper::Error) -> Self {
		AqaServiceError::Hyper(err)
	}
}

impl Service<Request<Body>> for AqaService {
	type Response = Response<Body>;
	type Error = AqaServiceError;
	type Future = std::future::Ready<Result<Self::Response, Self::Error>>;

	fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
		Poll::Ready(Ok(()))
	}

	fn call(&mut self, req: Request<Body>) -> Self::Future {
		println!("{:?}", req);
		let body = Body::from(String::from("Hello, world"));
		let resp = Response::builder()
			.status(StatusCode::OK)
			.body(body)
			.unwrap();

		ready(Ok(resp))
	}
}

async fn handle(_req: Request<Body>) -> Result<Response<Body>, Infallible> {
	Ok(Response::new(Body::from("Hello, World")))
}
