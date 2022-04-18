pub struct Db {

}

pub enum DbError {
}

pub async fn init(db_dir: &Path) -> Result<Db, DbError> {
	Ok(Db {})
}