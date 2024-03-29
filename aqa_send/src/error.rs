use std::fmt::Display;

use hyper::{Body, Response, StatusCode};
use serde::Serialize;
use thiserror::Error;

#[derive(Debug)]
pub enum Field {
	Cookie,
	ContentType,
}

pub trait HttpHandlerError: Display + std::fmt::Debug + std::error::Error
where
	Self: Sized,
{
	fn code(&self) -> StatusCode {
		StatusCode::INTERNAL_SERVER_ERROR
	}

	fn user_presentable(&self) -> bool {
		false
	}

	fn content_type() -> ErrorContentType {
		ErrorContentType::PlainText
	}

	fn response(&self) -> Response<Body> {
		let message = if self.user_presentable() {
			self.to_string()
		} else {
			self.code().to_string()
		};

		#[derive(Serialize)]
		struct ErrorJsonBody {
			status: u16,
			message: String,
		}

		let body: Body = match Self::content_type() {
			ErrorContentType::PlainText => message.into(),
			ErrorContentType::Json => serde_json::to_vec_pretty(&ErrorJsonBody {
				status: self.code().as_u16(),
				message,
			})
			.expect("Failed to serialize error message to json")
			.into(),
			ErrorContentType::Http => {
				format!("<h1>{}<h1><br><h3>{}</h3>", self.code().as_u16(), message).into()
			}
		};

		Response::builder().status(self.code()).body(body).unwrap()
	}
}

pub trait IntoHandlerError<T, E> {
	fn into_handler_error(self) -> Result<T, HandlerError<E>>;
}

impl<T, E, EE> IntoHandlerError<T, EE> for Result<T, E>
where
	E: Into<EE>,
	EE: HttpHandlerError,
{
	fn into_handler_error(self) -> Result<T, HandlerError<EE>> {
		self.map_err(|err| HandlerError::from(err.into()))
	}
}

#[derive(Copy, Clone)]
pub enum ErrorContentType {
	PlainText,
	Json,
	Http,
}

#[derive(Debug, Error)]
pub enum HandlerError<Err> {
	#[error("http layer error")]
	Http(#[from] hyper::http::Error),
	#[error("http framework error")]
	Hyper(#[from] hyper::Error),
	#[error(transparent)]
	Handler(Err),
}

impl<Err> From<Err> for HandlerError<Err>
where
	Err: HttpHandlerError,
{
	fn from(v: Err) -> Self {
		HandlerError::Handler(v)
	}
}

impl<Err> HttpHandlerError for HandlerError<Err>
where
	Err: Display + HttpHandlerError,
{
	fn code(&self) -> StatusCode {
		match self {
			HandlerError::Http(_) | HandlerError::Hyper(_) => StatusCode::INTERNAL_SERVER_ERROR,
			HandlerError::Handler(err) => err.code(),
		}
	}

	fn user_presentable(&self) -> bool {
		match self {
			HandlerError::Http(_) | HandlerError::Hyper(_) => false,
			HandlerError::Handler(err) => err.user_presentable(),
		}
	}

	fn content_type() -> ErrorContentType {
		<Err as HttpHandlerError>::content_type()
	}
}

#[derive(Debug, Error)]
enum ServerLayerError {
	#[error("")]
	Http(#[from] hyper::http::Error),
	#[error("")]
	Hyper(#[from] hyper::Error),
}

impl HttpHandlerError for ServerLayerError {}
