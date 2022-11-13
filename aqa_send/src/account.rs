use crate::{Account, AuthorizedUsers, Db};
use argon2::{Argon2, PasswordHash, PasswordVerifier};
use hyper::body::HttpBody;
use hyper::{Body, Request, Response, StatusCode};
use log::debug;
use std::iter::Iterator;
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum LoginError {
	#[error(transparent)]
	Http(#[from] hyper::http::Error),
	#[error(transparent)]
	Hyper(#[from] hyper::Error),
	#[error("Unsupported payload format")]
	ExpectedJson,
	#[error("Username is missing from request body")]
	ExpectedUsername,
	#[error("Password is missing from request body")]
	ExpectedPassword,
	#[error("Request body is too big")]
	BodyTooBig,
	#[error("Invalid username or password")]
	LoginFail,
	#[error("Hash problem")]
	PasswordHash,
}

const MAX_REQUEST_BODY_SIZE: usize = 1024 * 5; // 5 KB

pub async fn login(
	req: Request<Body>,
	db: Db,
	authorized_users: AuthorizedUsers,
) -> Result<Response<Body>, LoginError> {
	// if !req.headers().get("Content-Type").map(|ct| ct == "application/json").unwrap_or_default() {
	//     return Err(LoginError::ExpectedJson);
	// }

	let (_parts, mut body): (_, Body) = req.into_parts();

	let mut body_vec = Vec::with_capacity(MAX_REQUEST_BODY_SIZE);

	debug!("Reading body");
	while let Some(data) = body.data().await {
		let data = data?;
		if body_vec.len() + data.len() > MAX_REQUEST_BODY_SIZE {
			return Err(LoginError::BodyTooBig);
		}
		body_vec.extend_from_slice(&data);
	}
	debug!("Body size: {}", body_vec.len());

	let mut body_lines = body_vec.split(|b| *b == b'\n');
	let username = body_lines.next().ok_or(LoginError::ExpectedUsername)?;
	let username = std::str::from_utf8(username).map_err(|_| LoginError::LoginFail)?;

	let password = body_lines.next().ok_or(LoginError::ExpectedPassword)?;

	debug!("Password and username read from body");

	let account_uuids_guard = db.account_uuids_reader().await;
	let account_uuid = *account_uuids_guard
		.get(username)
		.ok_or(LoginError::LoginFail)?;
	drop(account_uuids_guard);

	let accounts_guard = db.accounts_reader().await;
	let account: &Account = accounts_guard
		.get(&account_uuid)
		.ok_or(LoginError::LoginFail)?;

	debug!("Verifying password");
	Argon2::default()
		.verify_password(
			password,
			&PasswordHash::new(&account.password_hash).map_err(|_| LoginError::PasswordHash)?,
		)
		.map_err(|_| LoginError::LoginFail)?;

	debug!("Generating session token");
	let session_token = Uuid::new_v4();
	authorized_users.insert(session_token, account.uuid);

	drop(accounts_guard);

	Ok(Response::builder()
		.status(StatusCode::CREATED)
		.header(
			"Set-Cookie",
			format!("session={}; Secure; HttpOnly", session_token),
		)
		.body(Body::empty())?)
}

#[derive(Debug, Error)]
pub enum LogoutError {
	#[error(transparent)]
	Http(#[from] hyper::http::Error),
	#[error(transparent)]
	Hyper(#[from] hyper::Error),
	#[error("Unsupported payload format")]
	ExpectedJson,
	#[error("Username is missing from request body")]
	ExpectedUsername,
	#[error("Password is missing from request body")]
	ExpectedPassword,
	#[error("Request body is too big")]
	BodyTooBig,
	#[error("Invalid username or password")]
	LoginFail,
	#[error("Hash problem")]
	PasswordHash,
}

pub async fn logout(
	_req: Request<Body>,
	_db: Db,
	_authorized_users: AuthorizedUsers,
) -> Result<Response<Body>, LogoutError> {
	todo!()
}
