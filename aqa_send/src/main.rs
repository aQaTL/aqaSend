use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Request, Response, Server};
use std::convert::Infallible;
use std::net::SocketAddr;

#[tokio::main]
async fn main() {
	let addr = SocketAddr::from(([127, 0, 0, 1], 8000));

	let make_service = make_service_fn(|_conn| async { Ok::<_, Infallible>(service_fn(handle)) });

	let server = Server::bind(&addr).serve(make_service);

	let server_exit: Result<_, _> = server.await;
	let _ = server_exit.unwrap();
}

async fn handle(_req: Request<Body>) -> Result<Response<Body>, Infallible> {
	Ok(Response::new(Body::from("Hello, World")))
}
