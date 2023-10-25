use crate::account::{get_logged_in_user, AuthError};
use crate::db::Db;
use crate::db_stuff::{AccountType, FileEntry};
use crate::error::{ErrorContentType, HandlerError, HttpHandlerError, IntoHandlerError};
use crate::{db, AuthorizedUsers};
use hyper::{Body, Request, Response, StatusCode};
use log::{debug, error};
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum DeleteError {
	#[error("File id is not a valid uuid")]
	Uuid(#[from] uuid::Error),

	#[error(transparent)]
	Db(#[from] db::DbError),

	#[error(transparent)]
	AuthError(#[from] AuthError),

	#[error("File id not found or not present")]
	NotFound,

	#[error("Only logged in users can delete entries")]
	NotLoggedIn,

	#[error("You can only delete your own files")]
	NotAuthorized,
}

impl HttpHandlerError for DeleteError {
	fn code(&self) -> StatusCode {
		match self {
			Self::Uuid(_) => StatusCode::BAD_REQUEST,
			Self::Db(_) => StatusCode::INTERNAL_SERVER_ERROR,
			Self::AuthError(err) => err.code(),
			Self::NotFound => StatusCode::NOT_FOUND,
			Self::NotAuthorized => StatusCode::UNAUTHORIZED,
			Self::NotLoggedIn => StatusCode::FORBIDDEN,
		}
	}

	fn user_presentable(&self) -> bool {
		match self {
			Self::Uuid(_) => true,
			Self::Db(_) => false,
			Self::AuthError(err) => err.user_presentable(),
			Self::NotFound => true,
			Self::NotAuthorized => true,
			Self::NotLoggedIn => true,
		}
	}

	fn content_type() -> ErrorContentType {
		ErrorContentType::Json
	}
}

pub async fn delete(
	uuid: String,
	req: Request<Body>,
	db: Db,
	authorized_users: AuthorizedUsers,
) -> Result<Response<Body>, HandlerError<DeleteError>> {
	let uuid = Uuid::parse_str(&uuid).into_handler_error()?;
	debug!("Deleting {}", uuid);

	let file_entry: FileEntry = db.get(&uuid).await.ok_or(DeleteError::NotFound)?.to_owned();

	let Some(current_user) =
		get_logged_in_user(req.headers(), db.clone(), authorized_users.clone())
			.await
			.into_handler_error()?
	else {
		return Err(DeleteError::NotLoggedIn.into());
	};

	let authorized = match (current_user.acc_type, file_entry.uploader_uuid) {
		(AccountType::Admin, _) => true,
		(_, Some(uploader)) => current_user.uuid == uploader,
		_ => false,
	};

	if !authorized {
		return Err(DeleteError::NotAuthorized.into());
	}

	crate::tasks::cleanup::remove_file(&file_entry, &uuid, &mut 0, &db.config.db_path).await;

	{
		let mut file_entries_writer = db.writer().await;
		if file_entries_writer.remove(&uuid).is_none() {
			error!("Tried to delete entry {uuid} that doesn't exist in DbDataHM");
		}
	}

	Ok(Response::builder()
		.status(StatusCode::NO_CONTENT)
		.body(Body::empty())?)
}
