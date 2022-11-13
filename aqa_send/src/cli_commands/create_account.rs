use argon2::password_hash::rand_core::OsRng;
use argon2::password_hash::SaltString;
use argon2::{Argon2, PasswordHasher};
use console::Term;
use thiserror::Error;
use uuid::Uuid;
use zeroize::Zeroizing;

use crate::{db, Account, AccountType, Db, DbError};

pub async fn create_account_cmd(
	name: String,
	acc_type: AccountType,
) -> Result<(), CreateAccountError> {
	let cwd = std::env::current_dir()?;
	let db = db::init(&cwd)?;
	let password = prompt_for_password()?;
	create_account(db, name, acc_type, password).await?;
	Ok(())
}

pub async fn create_account(
	db: Db,
	username: String,
	acc_type: AccountType,
	password: Zeroizing<String>,
) -> Result<Uuid, CreateAccountError> {
	let account_uuids_guard = db.account_uuids_writer().await;
	if account_uuids_guard.get(&username).is_some() {
		return Err(CreateAccountError::AccountAlreadyExists);
	}
	drop(account_uuids_guard);

	let account_uuid = Uuid::new_v4();
	db.add_account(
		account_uuid,
		Account {
			uuid: account_uuid,
			username,
			password_hash: hash_password(password)?,
			acc_type,
		},
	)
	.await?;

	db.save().await?;

	Ok(account_uuid)
}

#[derive(Error, Debug)]
pub enum CreateAccountError {
	#[error(transparent)]
	FileOperation(#[from] std::io::Error),

	#[error(transparent)]
	DbError(#[from] DbError),

	#[error("Account with that username already exists")]
	AccountAlreadyExists,

	#[error("Entered passwords don't match")]
	PasswordsDoNotMatch,
	#[error("Failed to hash the password: {0:?}")]
	PasswordHashingError(argon2::password_hash::Error),
}

fn prompt_for_password() -> Result<Zeroizing<String>, CreateAccountError> {
	println!("Password: ");
	let password = Zeroizing::new(Term::stdout().read_secure_line()?);
	println!("Confirm password: ");
	let confirmed_password = Zeroizing::new(Term::stdout().read_secure_line()?);
	if *password != *confirmed_password {
		return Err(CreateAccountError::PasswordsDoNotMatch);
	}
	Ok(password)
}

fn hash_password(password: Zeroizing<String>) -> Result<String, CreateAccountError> {
	let salt = SaltString::generate(&mut OsRng);
	let argon2 = Argon2::default();
	let password_hash = argon2
		.hash_password(password.as_bytes(), &salt)
		.map_err(CreateAccountError::PasswordHashingError)?
		.to_string();
	Ok(password_hash)
}
