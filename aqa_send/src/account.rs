use crate::cookie::parse_cookie;
use crate::db_stuff::AccountType;
use crate::error::{ErrorContentType, Field, IntoHandlerError};
use crate::multipart::{Multipart, MultipartError};
use crate::{cli_commands, Account, AuthorizedUsers, Db, HandlerError, HttpHandlerError};

use argon2::{Argon2, PasswordHash, PasswordVerifier};
use bytes::BytesMut;
use futures::StreamExt;
use hyper::http::HeaderValue;
use hyper::{Body, HeaderMap, Request, Response, StatusCode};
use log::debug;
use serde::{Deserialize, Serialize};
use std::iter::Iterator;
use thiserror::Error;
use uuid::Uuid;
use zeroize::Zeroizing;

#[derive(Debug, Error)]
pub enum LoginError {
	#[error("Invalid username or password")]
	LoginFail,

	#[error("Upload requires `multipart/form-data` Content-Type")]
	InvalidContentType,

	#[error("Multipart/form-data upload must define a boundary")]
	BoundaryExpected,

	#[error(transparent)]
	Multipart(#[from] MultipartError),

	#[error("Username is missing from request body")]
	ExpectedUsername,

	#[error("Password is missing from request body")]
	ExpectedPassword,

	#[error("Request body is too big")]
	BodyTooBig,

	#[error("Hash problem")]
	PasswordHash,

	#[error("Expected a valid utf-8 sequence in: {place}")]
	Utf8 { place: &'static str },
}

impl HttpHandlerError for LoginError {
	fn code(&self) -> StatusCode {
		match self {
			LoginError::LoginFail => StatusCode::UNAUTHORIZED,

			LoginError::Multipart(_) => StatusCode::BAD_REQUEST,
			LoginError::InvalidContentType => StatusCode::BAD_REQUEST,
			LoginError::BoundaryExpected => StatusCode::BAD_REQUEST,
			LoginError::ExpectedUsername => StatusCode::BAD_REQUEST,
			LoginError::ExpectedPassword => StatusCode::BAD_REQUEST,
			LoginError::BodyTooBig => StatusCode::BAD_REQUEST,
			LoginError::PasswordHash => StatusCode::BAD_REQUEST,
			LoginError::Utf8 { .. } => StatusCode::BAD_REQUEST,
		}
	}

	fn user_presentable(&self) -> bool {
		true
	}

	fn content_type() -> ErrorContentType {
		ErrorContentType::Json
	}
}

const MAX_REQUEST_BODY_SIZE: usize = 1024 * 5; // 5 KB

pub async fn login(
	req: Request<Body>,
	db: Db,
	authorized_users: AuthorizedUsers,
) -> Result<Response<Body>, HandlerError<LoginError>> {
	use LoginError::{BoundaryExpected, InvalidContentType};

	let (parts, body): (_, Body) = req.into_parts();

	let content_type = parts
		.headers
		.get("content-type")
		.ok_or(InvalidContentType)?
		.to_str()
		.map_err(|_| InvalidContentType)?;

	let boundary = content_type
		.strip_prefix("multipart/form-data; ")
		.ok_or(BoundaryExpected)?
		.strip_prefix("boundary=")
		.ok_or(BoundaryExpected)?;
	let boundary = format!("--{}", boundary);
	debug!("Boundary: {}", boundary);

	let mut multipart = Multipart::new(body, boundary, MAX_REQUEST_BODY_SIZE);

	let chunks = multipart.read_all_chunks().await.into_handler_error()?;

	let (mut username, mut password) = (None, None);

	for (header, data) in &chunks {
		debug!("Reading header {header:?}");
		if header.name == "username" {
			let username_ = std::str::from_utf8(data).map_err(|_| LoginError::Utf8 {
				place: "username data",
			})?;
			username = Some(username_);
		}
		if header.name == "password" {
			let password_ = std::str::from_utf8(data).map_err(|_| LoginError::Utf8 {
				place: "password data",
			})?;
			password = Some(password_);
		}
	}

	let username = username.ok_or(LoginError::ExpectedUsername)?;
	let password = password.ok_or(LoginError::ExpectedPassword)?;

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
			password.as_ref(),
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
			format!("session={}; Secure; HttpOnly; SameSite=None", session_token),
		)
		.body(Body::empty())?)
}

#[derive(Debug, Error)]
pub enum LogoutError {
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

impl HttpHandlerError for LogoutError {}

pub async fn logout(
	_req: Request<Body>,
	_db: Db,
	_authorized_users: AuthorizedUsers,
) -> Result<Response<Body>, HandlerError<LogoutError>> {
	todo!()
}

#[derive(Debug, Error)]
pub enum AuthError {
	#[error("{0:?} must only contain visible ascii characters")]
	AsciiOnly(Field),

	#[error("Malformed {0:?} data")]
	Malformed(Field),

	#[error("malformed uuid")]
	UuidParse(#[from] uuid::Error),

	#[error("Session expired")]
	SessionExpired,

	#[error("Authorized user doesn't exist")]
	UnknownUser,
}

impl HttpHandlerError for AuthError {
	fn code(&self) -> StatusCode {
		match self {
			AuthError::AsciiOnly(_) => StatusCode::BAD_REQUEST,
			AuthError::Malformed(_) => StatusCode::BAD_REQUEST,
			AuthError::UuidParse(_) => StatusCode::BAD_REQUEST,
			AuthError::SessionExpired => StatusCode::UNAUTHORIZED,
			AuthError::UnknownUser => StatusCode::UNAUTHORIZED,
		}
	}

	fn user_presentable(&self) -> bool {
		true
	}
}

pub async fn get_logged_in_user(
	headers: &HeaderMap<HeaderValue>,
	db: Db,
	authorized_users: AuthorizedUsers,
) -> Result<Option<Account>, AuthError> {
	match headers.get("Cookie") {
		Some(cookie) => {
			let cookie_header = cookie
				.to_str()
				.map_err(|_| AuthError::AsciiOnly(Field::Cookie))?;
			let (_, cookies) =
				parse_cookie(cookie_header).map_err(|_| AuthError::Malformed(Field::Cookie))?;
			debug!("Cookies: {cookies:?}");
			match cookies.get("session") {
				Some(session_cookie) => {
					let session_cookie: Uuid = session_cookie.parse()?;

					let user_uuid = authorized_users
						.get_user_uuid(&session_cookie)
						.ok_or(AuthError::SessionExpired)?;
					debug!("Getting user with uuid {user_uuid}");
					Ok(Some(
						db.get_account(&user_uuid)
							.await
							.ok_or(AuthError::UnknownUser)?,
					))
				}
				None => Ok(None),
			}
		}
		None => Ok(None),
	}
}

#[derive(Debug, Error)]
pub enum CreateRegistrationCodeError {
	#[error(transparent)]
	AuthError(#[from] AuthError),

	#[error("You must be logged in to do that")]
	Unauthorized,

	#[error("Insufficient permissions")]
	InsufficientPermissions,
}

impl HttpHandlerError for CreateRegistrationCodeError {
	fn code(&self) -> StatusCode {
		match self {
			CreateRegistrationCodeError::AuthError(err) => err.code(),
			CreateRegistrationCodeError::Unauthorized => StatusCode::UNAUTHORIZED,
			CreateRegistrationCodeError::InsufficientPermissions => StatusCode::UNAUTHORIZED,
		}
	}

	fn user_presentable(&self) -> bool {
		match self {
			CreateRegistrationCodeError::AuthError(err) => err.user_presentable(),
			CreateRegistrationCodeError::Unauthorized => true,
			CreateRegistrationCodeError::InsufficientPermissions => true,
		}
	}

	fn content_type() -> ErrorContentType {
		ErrorContentType::Json
	}
}

pub async fn create_registration_code(
	req: Request<Body>,
	db: Db,
	authorized_users: AuthorizedUsers,
) -> Result<Response<Body>, HandlerError<CreateRegistrationCodeError>> {
	let current_user = get_logged_in_user(req.headers(), db.clone(), authorized_users.clone())
		.await
		.into_handler_error()?
		.ok_or(CreateRegistrationCodeError::Unauthorized)?;

	if !matches!(current_user.acc_type, AccountType::Admin) {
		return Err(CreateRegistrationCodeError::InsufficientPermissions.into());
	}

	let registration_code = Uuid::new_v4().to_string();
	db.registration_codes_writer()
		.await
		.push(registration_code.clone());

	Ok(Response::builder()
		.status(StatusCode::CREATED)
		.header("Content-Type", "text/plain")
		.body(Body::from(registration_code))?)
}

#[derive(Debug, Error)]
pub enum CreateAccountFromRegistrationCodeError {
	#[error("Invalid registration code")]
	InvalidRegistrationCode,

	#[error("Request to big")]
	RequestTooBig,

	#[error(transparent)]
	CreateAccount(#[from] cli_commands::create_account::CreateAccountError),
}

impl HttpHandlerError for CreateAccountFromRegistrationCodeError {
	fn code(&self) -> StatusCode {
		match self {
			CreateAccountFromRegistrationCodeError::InvalidRegistrationCode => StatusCode::OK,
			CreateAccountFromRegistrationCodeError::RequestTooBig => StatusCode::PAYLOAD_TOO_LARGE,
			CreateAccountFromRegistrationCodeError::CreateAccount(err) => err.code(),
		}
	}

	fn user_presentable(&self) -> bool {
		match self {
			CreateAccountFromRegistrationCodeError::InvalidRegistrationCode => true,
			CreateAccountFromRegistrationCodeError::RequestTooBig => true,
			CreateAccountFromRegistrationCodeError::CreateAccount(err) => err.user_presentable(),
		}
	}

	fn content_type() -> ErrorContentType {
		ErrorContentType::Json
	}
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateAccountModel {
	pub registration_code: String,
	pub username: String,
	pub password: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateAccountResponse {
	uuid: Uuid,
}

pub async fn create_account_from_registration_code(
	req: Request<Body>,
	db: Db,
	authorized_users: AuthorizedUsers,
) -> Result<Response<Body>, HandlerError<CreateAccountFromRegistrationCodeError>> {
	let (_parts, mut body) = req.into_parts();

	debug!("Reading body");
	let mut req_body = BytesMut::with_capacity(MAX_REQUEST_BODY_SIZE);
	while let Some(chunk) = body.next().await {
		let chunk = chunk?;
		if req_body.len() + chunk.len() > MAX_REQUEST_BODY_SIZE {
			return Err(CreateAccountFromRegistrationCodeError::RequestTooBig.into());
		}
		req_body.extend_from_slice(&chunk);
	}

	debug!("Validating registration code");
	let create_account: CreateAccountModel = serde_json::from_slice(&req_body).unwrap();
	if !registration_code_valid(&db, &create_account.registration_code).await {
		return Err(CreateAccountFromRegistrationCodeError::InvalidRegistrationCode.into());
	}

	debug!("Creating account");
	let uuid = crate::cli_commands::create_account::create_account(
		db.clone(),
		create_account.username,
		AccountType::User,
		Zeroizing::new(create_account.password),
	)
	.await
	.into_handler_error()?;

	let new_account = CreateAccountResponse { uuid };
	let response_body = serde_json::to_string_pretty(&new_account).unwrap();

	debug!("Generating session token");
	let session_token = Uuid::new_v4();
	authorized_users.insert(session_token, new_account.uuid);

	Ok(Response::builder()
		.status(StatusCode::CREATED)
		.header(
			"Set-Cookie",
			format!("session={}; Secure; HttpOnly", session_token),
		)
		.header("Content-Type", "application/json")
		.body(Body::from(response_body))?)
}

#[derive(Debug, Error)]
pub enum CheckRegistrationCodeError {
	#[error("Invalid registration code")]
	InvalidRegistrationCode,
}

impl HttpHandlerError for CheckRegistrationCodeError {
	fn code(&self) -> StatusCode {
		match self {
			CheckRegistrationCodeError::InvalidRegistrationCode => StatusCode::NOT_FOUND,
		}
	}

	fn user_presentable(&self) -> bool {
		match self {
			CheckRegistrationCodeError::InvalidRegistrationCode => true,
		}
	}

	fn content_type() -> ErrorContentType {
		ErrorContentType::Json
	}
}

pub async fn check_registration_code(
	_req: Request<Body>,
	registration_code: String,
	db: Db,
) -> Result<Response<Body>, HandlerError<CheckRegistrationCodeError>> {
	if !registration_code_valid(&db, &registration_code).await {
		return Err(CheckRegistrationCodeError::InvalidRegistrationCode.into());
	}

	Ok(Response::builder()
		.status(StatusCode::OK)
		.body(Body::empty())?)
}

pub async fn registration_code_valid(db: &Db, registration_code: &str) -> bool {
	let registration_codes = db.registration_codes_reader().await;
	registration_codes.iter().any(|v| *v == registration_code)
}
