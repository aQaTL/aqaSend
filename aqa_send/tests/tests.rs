use anyhow::Result;
use aqa_send::account::CreateAccountResponse;
use hyper::body::to_bytes;
use hyper::header::SET_COOKIE;
use hyper::service::Service;
use hyper::{Body, Method, Request, Response, StatusCode};
use log::debug;
use rand::thread_rng;
use std::time::Duration;
use tempfile::TempDir;
use tokio::fs;
use uuid::Uuid;
use zeroize::Zeroizing;

use aqa_send::cli_commands::create_account::create_account;
use aqa_send::db_stuff::AccountType;
use aqa_send::files::DB_DIR;
use aqa_send::headers::Lifetime;
use aqa_send::upload::UploadResponse;
use aqa_send::{cookie, db, headers, list, tasks, AqaService, AqaServiceError, AuthorizedUsers};

struct TestServer {
	#[allow(dead_code)]
	db_dir: TempDir,
	#[allow(dead_code)]
	db_handle: db::Db,
	aqa_service: AqaService,
}

impl TestServer {
	fn new() -> Result<Self> {
		let db_dir = tempfile::tempdir()?;

		aqa_logger::init();
		// if let Err(err) = tracing_subscriber::FmtSubscriber::builder().try_init() {
		// 	eprintln!("{err:?}");
		// }
		//
		// init_app_directory_structure(db_dir.path())?;

		let db_handle = db::init(db_dir.path())?;
		let aqa_service = AqaService::new(db_handle.clone(), AuthorizedUsers::default());

		Ok(Self {
			db_dir,
			db_handle,
			aqa_service,
		})
	}

	#[allow(dead_code)]
	fn start_cleanup_task(&self, interval: Duration) {
		tokio::spawn(tasks::cleanup::cleanup_task(
			self.db_handle.clone(),
			interval,
			interval,
		));
	}

	async fn process_request(
		&mut self,
		request: Request<Body>,
	) -> Result<Response<Body>, AqaServiceError> {
		self.aqa_service.call(request).await
	}
}

fn random_string(len: usize) -> String {
	use rand::distributions::DistString;
	rand::distributions::Alphanumeric.sample_string(&mut thread_rng(), len)
}

#[tokio::test]
async fn test_hello() -> Result<()> {
	let mut test_server = TestServer::new()?;

	let request = Request::builder().uri("/api").body(Body::empty())?;

	let mut response = test_server.process_request(request).await?;
	let response_str = to_bytes(response.body_mut()).await?;

	let expected_response = "Hello from aqaSend\n";

	assert_eq!(response_str, expected_response);

	Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn upload_works() -> Result<()> {
	let mut test_server = TestServer::new()?;

	let file_contents = random_string(143);
	let boundary = random_string(50);

	const DOWNLOAD_COUNT: &str = "1";

	let request = Request::builder()
		.uri("/api/upload")
		.method(Method::POST)
		.header(
			"Content-Type",
			format!("multipart/form-data; boundary={boundary}"),
		)
		.header(headers::DOWNLOAD_COUNT, DOWNLOAD_COUNT)
		.body(Body::from(format!(
			"--{boundary}\r\n\
Content-Disposition: form-data; name=\"sample_file\"; filename=\"sample_file\"\r\n\
Content-Type: text/plain\r\n\r\n\
{}\r\n\
--{boundary}--\r\n",
			file_contents
		)))?;

	let mut response = dbg!(test_server.process_request(request).await)?;
	assert_eq!(response.status(), StatusCode::OK);

	let response_bytes = to_bytes(response.body_mut()).await?;
	let UploadResponse(uploaded_files) = serde_json::from_slice(&response_bytes)?;

	assert_eq!(uploaded_files.len(), 1);
	assert_eq!(uploaded_files[0].filename, "sample_file");

	let path = test_server
		.db_dir
		.path()
		.join(DB_DIR)
		.join(DOWNLOAD_COUNT)
		.join(uploaded_files[0].uuid.to_string());

	let uploaded_file = fs::read_to_string(&path).await?;
	assert_eq!(uploaded_file, file_contents);

	Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn file_gets_removed_after_1_download() -> Result<()> {
	let mut test_server = TestServer::new()?;
	test_server.start_cleanup_task(Duration::from_millis(10));

	let file_contents = random_string(143);
	let boundary = random_string(50);

	const DOWNLOAD_COUNT: &str = "1";

	let request = Request::builder()
		.uri("/api/upload")
		.method(Method::POST)
		.header(
			"Content-Type",
			format!("multipart/form-data; boundary={boundary}"),
		)
		.header(headers::DOWNLOAD_COUNT, DOWNLOAD_COUNT)
		.body(Body::from(format!(
			"--{boundary}\r\n\
Content-Disposition: form-data; name=\"sample_file\"; filename=\"sample_file\"\r\n\
Content-Type: text/plain\r\n\r\n\
{}\r\n\
--{boundary}--\r\n",
			file_contents
		)))?;

	let mut response = test_server.process_request(request).await?;
	assert_eq!(response.status(), StatusCode::OK);

	let response_bytes = to_bytes(response.body_mut()).await?;
	let UploadResponse(uploaded_files) = serde_json::from_slice(&response_bytes)?;

	assert_eq!(uploaded_files.len(), 1);
	assert_eq!(uploaded_files[0].filename, "sample_file");

	let uploaded_file_path = test_server
		.db_dir
		.path()
		.join(DB_DIR)
		.join(DOWNLOAD_COUNT)
		.join(uploaded_files[0].uuid.to_string());

	assert!(uploaded_file_path.exists());

	let request = Request::builder()
		.uri(format!("/api/download/{}", uploaded_files[0].uuid))
		.method(Method::GET)
		.body(Body::empty())?;

	let mut response = test_server.process_request(request).await?;
	assert_eq!(response.status(), StatusCode::OK);

	let response_bytes = to_bytes(response.body_mut()).await?;
	assert_eq!(file_contents.as_bytes(), response_bytes.as_ref());

	tokio::time::sleep(Duration::from_millis(20)).await;

	let request = Request::builder()
		.uri(format!("/api/download/{}", uploaded_files[0].uuid))
		.method(Method::GET)
		.body(Body::empty())?;

	let response = test_server.process_request(request).await?;
	assert_eq!(response.status(), StatusCode::NOT_FOUND);

	assert!(!uploaded_file_path.exists());

	Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn file_gets_removed_after_10_download() -> Result<()> {
	let mut test_server = TestServer::new()?;
	test_server.start_cleanup_task(Duration::from_millis(10));

	let file_contents = random_string(143);
	let boundary = random_string(50);

	const DOWNLOAD_COUNT: &str = "10";

	let request = Request::builder()
		.uri("/api/upload")
		.method(Method::POST)
		.header(
			"Content-Type",
			format!("multipart/form-data; boundary={boundary}"),
		)
		.header(headers::DOWNLOAD_COUNT, DOWNLOAD_COUNT)
		.body(Body::from(format!(
			"--{boundary}\r\n\
Content-Disposition: form-data; name=\"sample_file\"; filename=\"sample_file\"\r\n\
Content-Type: text/plain\r\n\r\n\
{}\r\n\
--{boundary}--\r\n",
			file_contents
		)))?;

	let mut response = test_server.process_request(request).await?;
	assert_eq!(response.status(), StatusCode::OK);

	let response_bytes = to_bytes(response.body_mut()).await?;
	let UploadResponse(uploaded_files) = serde_json::from_slice(&response_bytes)?;

	assert_eq!(uploaded_files.len(), 1);
	assert_eq!(uploaded_files[0].filename, "sample_file");

	let uploaded_file_path = test_server
		.db_dir
		.path()
		.join(DB_DIR)
		.join(DOWNLOAD_COUNT)
		.join(uploaded_files[0].uuid.to_string());

	assert!(uploaded_file_path.exists());

	for _ in 0..10 {
		let request = Request::builder()
			.uri(format!("/api/download/{}", uploaded_files[0].uuid))
			.method(Method::GET)
			.body(Body::empty())?;

		let mut response = test_server.process_request(request).await?;
		assert_eq!(response.status(), StatusCode::OK);

		let response_bytes = to_bytes(response.body_mut()).await?;
		assert_eq!(file_contents.as_bytes(), response_bytes.as_ref());

		assert!(uploaded_file_path.exists());
	}

	let request = Request::builder()
		.uri(format!("/api/download/{}", uploaded_files[0].uuid))
		.method(Method::GET)
		.body(Body::empty())?;

	let response = test_server.process_request(request).await?;
	assert_eq!(response.status(), StatusCode::NOT_FOUND);

	tokio::time::sleep(Duration::from_millis(20)).await;
	assert!(!uploaded_file_path.exists());

	Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn unlimited_download_count() -> Result<()> {
	let mut test_server = TestServer::new()?;
	test_server.start_cleanup_task(Duration::from_millis(10));

	let file_contents = random_string(143);
	let boundary = random_string(50);

	const DOWNLOAD_COUNT: &str = "infinite";

	let request = Request::builder()
		.uri("/api/upload")
		.method(Method::POST)
		.header(
			"Content-Type",
			format!("multipart/form-data; boundary={boundary}"),
		)
		.header(headers::DOWNLOAD_COUNT, DOWNLOAD_COUNT)
		.body(Body::from(format!(
			"--{boundary}\r\n\
Content-Disposition: form-data; name=\"sample_file\"; filename=\"sample_file\"\r\n\
Content-Type: text/plain\r\n\r\n\
{}\r\n\
--{boundary}--\r\n",
			file_contents
		)))?;

	let mut response = test_server.process_request(request).await?;
	assert_eq!(response.status(), StatusCode::OK);

	let response_bytes = to_bytes(response.body_mut()).await?;
	let UploadResponse(uploaded_files) = serde_json::from_slice(&response_bytes)?;

	assert_eq!(uploaded_files.len(), 1);
	assert_eq!(uploaded_files[0].filename, "sample_file");

	let uploaded_file_path = test_server
		.db_dir
		.path()
		.join(DB_DIR)
		.join(DOWNLOAD_COUNT)
		.join(uploaded_files[0].uuid.to_string());

	assert!(uploaded_file_path.exists());

	for _ in 0..9999 {
		let request = Request::builder()
			.uri(format!("/api/download/{}", uploaded_files[0].uuid))
			.method(Method::GET)
			.body(Body::empty())?;

		let mut response = test_server.process_request(request).await?;
		assert_eq!(response.status(), StatusCode::OK);

		let response_bytes = to_bytes(response.body_mut()).await?;
		assert_eq!(file_contents.as_bytes(), response_bytes.as_ref());
	}

	Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn file_protected_by_password() -> Result<()> {
	let mut test_server = TestServer::new()?;

	let file_contents = random_string(143);
	let boundary = random_string(50);

	const DOWNLOAD_COUNT: &str = "1";
	const PASSWORD: &str = "alamakota";

	let request = Request::builder()
		.uri("/api/upload")
		.method(Method::POST)
		.header(
			"Content-Type",
			format!("multipart/form-data; boundary={boundary}"),
		)
		.header(headers::DOWNLOAD_COUNT, DOWNLOAD_COUNT)
		.header(headers::PASSWORD, urlencoding::encode(PASSWORD).as_ref())
		.body(Body::from(format!(
			"--{boundary}\r\n\
Content-Disposition: form-data; name=\"sample_file\"; filename=\"sample_file\"\r\n\
Content-Type: text/plain\r\n\r\n\
{}\r\n\
--{boundary}--\r\n",
			file_contents
		)))?;

	let mut response = test_server.process_request(request).await?;
	assert_eq!(response.status(), StatusCode::OK);

	let response_bytes = to_bytes(response.body_mut()).await?;
	let UploadResponse(uploaded_files) = serde_json::from_slice(&response_bytes)?;

	assert_eq!(uploaded_files.len(), 1);
	assert_eq!(uploaded_files[0].filename, "sample_file");

	let uploaded_file_path = test_server
		.db_dir
		.path()
		.join(DB_DIR)
		.join(DOWNLOAD_COUNT)
		.join(uploaded_files[0].uuid.to_string());

	assert!(uploaded_file_path.exists());

	let request = Request::builder()
		.uri(format!("/api/download/{}", uploaded_files[0].uuid))
		.method(Method::GET)
		.body(Body::empty())?;

	let response = test_server.process_request(request).await?;
	// assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
	assert_ne!(response.status(), StatusCode::OK);

	let request = Request::builder()
		.uri(format!(
			"/api/download/{}?password={}",
			uploaded_files[0].uuid,
			urlencoding::encode(PASSWORD)
		))
		.method(Method::GET)
		.header(headers::PASSWORD, PASSWORD)
		.body(Body::empty())?;

	let mut response = test_server.process_request(request).await?;
	assert_eq!(response.status(), StatusCode::OK);

	let response_bytes = to_bytes(response.body_mut()).await?;
	assert_eq!(file_contents.as_bytes(), response_bytes.as_ref());

	Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn private_file_visible_only_to_logged_in_users() -> Result<()> {
	let mut test_server = TestServer::new()?;

	let username = String::from("Ala");
	let password = String::from("zażółć gęsią jaźń");

	let account_uuid = create_account(
		test_server.db_handle.clone(),
		username.clone(),
		AccountType::Admin,
		Zeroizing::new(password.clone()),
	)
	.await?;

	debug!("Created account with uuid {account_uuid}");

	let boundary = random_string(50);

	let request = Request::builder()
		.uri("/api/login")
		.method(Method::POST)
		.header(
			"Content-Type",
			format!("multipart/form-data; boundary={boundary}"),
		)
		.body(Body::from(format!(
			"--{boundary}\r\n\
Content-Disposition: form-data; name=\"username\"\r\n\r\n\
{username}\r\n\
--{boundary}--\r\n\
Content-Disposition: form-data; name=\"password\"\r\n\r\n\
{password}\r\n\
--{boundary}--\r\n"
		)))?;

	let response = test_server.process_request(request).await?;
	assert_eq!(response.status(), StatusCode::CREATED);

	let cookie = response
		.headers()
		.get(SET_COOKIE)
		.expect("Set-Cookie missing")
		.to_str()
		.unwrap();
	let (_, cookie) = cookie::parse_set_cookie(cookie).unwrap();

	debug!("Logged in with session cookie: {cookie:?}");

	let cookie_header_value = format!("{}={}", cookie.name, cookie.value);
	debug!("Cookie header: {cookie_header_value}");

	let file_contents = random_string(143);
	let boundary = random_string(50);

	const DOWNLOAD_COUNT: &str = "1";
	const VISIBILITY: &str = "private";

	let request = Request::builder()
		.uri("/api/upload")
		.method(Method::POST)
		.header("Cookie", cookie_header_value.clone())
		.header(
			"Content-Type",
			format!("multipart/form-data; boundary={boundary}"),
		)
		.header(headers::DOWNLOAD_COUNT, DOWNLOAD_COUNT)
		.header(headers::VISIBILITY, VISIBILITY)
		.body(Body::from(format!(
			"--{boundary}\r\n\
Content-Disposition: form-data; name=\"sample_file\"; filename=\"sample_file\"\r\n\
Content-Type: text/plain\r\n\r\n\
{}\r\n\
--{boundary}--\r\n",
			file_contents
		)))?;

	let mut response = test_server.process_request(request).await?;
	assert_eq!(response.status(), StatusCode::OK);

	let response_bytes = to_bytes(response.body_mut()).await?;
	let UploadResponse(uploaded_files) = serde_json::from_slice(&response_bytes)?;

	assert_eq!(uploaded_files.len(), 1);
	assert_eq!(uploaded_files[0].filename, "sample_file");

	let uploaded_file_path = test_server
		.db_dir
		.path()
		.join(DB_DIR)
		.join(DOWNLOAD_COUNT)
		.join(uploaded_files[0].uuid.to_string());

	assert!(uploaded_file_path.exists());

	let request = Request::builder()
		.uri("/api/list.json")
		.method(Method::GET)
		.body(Body::empty())?;

	let mut response = test_server.process_request(request).await?;
	assert_eq!(response.status(), StatusCode::OK);

	let response_bytes = to_bytes(response.body_mut()).await?;
	let list: Vec<list::FileModel<'static>> = serde_json::from_slice(&response_bytes)?;
	assert_eq!(list.len(), 0);

	let request = Request::builder()
		.uri("/api/list.json")
		.method(Method::GET)
		.header("Cookie", cookie_header_value)
		.body(Body::empty())?;

	let mut response = test_server.process_request(request).await?;
	assert_eq!(response.status(), StatusCode::OK);

	let response_bytes = to_bytes(response.body_mut()).await?;
	let list: Vec<list::FileModel<'static>> = serde_json::from_slice(&response_bytes)?;
	assert_eq!(list.len(), 1);

	Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn private_file_downloadable_only_by_uploader() -> Result<()> {
	let mut test_server = TestServer::new()?;

	let username = String::from("Ala");
	let password = String::from("zażółć gęsią jaźń");

	let account_uuid = create_account(
		test_server.db_handle.clone(),
		username.clone(),
		AccountType::Admin,
		Zeroizing::new(password.clone()),
	)
	.await?;

	debug!("Created account with uuid {account_uuid}");

	let boundary = random_string(50);

	let request = Request::builder()
		.uri("/api/login")
		.method(Method::POST)
		.header(
			"Content-Type",
			format!("multipart/form-data; boundary={boundary}"),
		)
		.body(Body::from(format!(
			"--{boundary}\r\n\
Content-Disposition: form-data; name=\"username\"\r\n\r\n\
{username}\r\n\
--{boundary}--\r\n\
Content-Disposition: form-data; name=\"password\"\r\n\r\n\
{password}\r\n\
--{boundary}--\r\n"
		)))?;

	let response = test_server.process_request(request).await?;
	assert_eq!(response.status(), StatusCode::CREATED);

	let cookie = response
		.headers()
		.get(SET_COOKIE)
		.expect("Set-Cookie missing")
		.to_str()
		.unwrap();
	let (_, cookie) = cookie::parse_set_cookie(cookie).unwrap();

	debug!("Logged in with session cookie: {cookie:?}");

	let cookie_header_value = format!("{}={}", cookie.name, cookie.value);
	debug!("Cookie header: {cookie_header_value}");

	let file_contents = random_string(143);
	let boundary = random_string(50);

	const DOWNLOAD_COUNT: &str = "1";
	const VISIBILITY: &str = "private";

	let request = Request::builder()
		.uri("/api/upload")
		.method(Method::POST)
		.header("Cookie", cookie_header_value.clone())
		.header(
			"Content-Type",
			format!("multipart/form-data; boundary={boundary}"),
		)
		.header(headers::DOWNLOAD_COUNT, DOWNLOAD_COUNT)
		.header(headers::VISIBILITY, VISIBILITY)
		.body(Body::from(format!(
			"--{boundary}\r\n\
Content-Disposition: form-data; name=\"sample_file\"; filename=\"sample_file\"\r\n\
Content-Type: text/plain\r\n\r\n\
{}\r\n\
--{boundary}--\r\n",
			file_contents
		)))?;

	let mut response = test_server.process_request(request).await?;
	assert_eq!(response.status(), StatusCode::OK);

	let response_bytes = to_bytes(response.body_mut()).await?;
	let UploadResponse(uploaded_files) = serde_json::from_slice(&response_bytes)?;

	assert_eq!(uploaded_files.len(), 1);
	assert_eq!(uploaded_files[0].filename, "sample_file");

	let uploaded_file_path = test_server
		.db_dir
		.path()
		.join(DB_DIR)
		.join(DOWNLOAD_COUNT)
		.join(uploaded_files[0].uuid.to_string());

	assert!(uploaded_file_path.exists());

	let request = Request::builder()
		.uri(format!("/api/download/{}", uploaded_files[0].uuid))
		.method(Method::GET)
		.body(Body::empty())?;

	let response = test_server.process_request(request).await?;
	assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

	Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn file_gets_removed_after_lifetime_runs_out() -> Result<()> {
	let mut test_server = TestServer::new()?;
	test_server.start_cleanup_task(Duration::from_millis(10));

	let file_contents = random_string(143);
	let boundary = random_string(50);

	const DOWNLOAD_COUNT: &str = "infinite";
	const LIFETIME: &str = "1 min";

	let request = Request::builder()
		.uri("/api/upload")
		.method(Method::POST)
		.header(
			"Content-Type",
			format!("multipart/form-data; boundary={boundary}"),
		)
		.header(headers::DOWNLOAD_COUNT, DOWNLOAD_COUNT)
		.header(headers::LIFETIME, LIFETIME)
		.body(Body::from(format!(
			"--{boundary}\r\n\
Content-Disposition: form-data; name=\"sample_file\"; filename=\"sample_file\"\r\n\
Content-Type: text/plain\r\n\r\n\
{}\r\n\
--{boundary}--\r\n",
			file_contents
		)))?;

	let mut response = test_server.process_request(request).await?;
	assert_eq!(response.status(), StatusCode::OK);

	let response_bytes = to_bytes(response.body_mut()).await?;
	let UploadResponse(uploaded_files) = serde_json::from_slice(&response_bytes)?;

	assert_eq!(uploaded_files.len(), 1);
	assert_eq!(uploaded_files[0].filename, "sample_file");

	let uploaded_file_path = test_server
		.db_dir
		.path()
		.join(DB_DIR)
		.join(DOWNLOAD_COUNT)
		.join(uploaded_files[0].uuid.to_string());

	assert!(uploaded_file_path.exists());

	{
		test_server
			.db_handle
			.writer()
			.await
			.get_mut(&uploaded_files[0].uuid)
			.unwrap()
			.lifetime = Lifetime::Duration(Duration::from_millis(400));
	}

	let start_time = std::time::SystemTime::now();
	while start_time.elapsed()? < Duration::from_millis(400) {
		tokio::time::sleep(Duration::from_millis(10)).await;
		assert!(uploaded_file_path.exists());
	}

	tokio::time::sleep(Duration::from_millis(20)).await;
	assert!(!uploaded_file_path.exists());

	Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn cannot_download_file_after_500_ms() -> Result<()> {
	let mut test_server = TestServer::new()?;
	test_server.start_cleanup_task(Duration::from_secs(60 * 60));

	let file_contents = random_string(143);
	let boundary = random_string(50);

	const DOWNLOAD_COUNT: &str = "infinite";
	const LIFETIME: &str = "1 min";

	let request = Request::builder()
		.uri("/api/upload")
		.method(Method::POST)
		.header(
			"Content-Type",
			format!("multipart/form-data; boundary={boundary}"),
		)
		.header(headers::DOWNLOAD_COUNT, DOWNLOAD_COUNT)
		.header(headers::LIFETIME, LIFETIME)
		.body(Body::from(format!(
			"--{boundary}\r\n\
Content-Disposition: form-data; name=\"sample_file\"; filename=\"sample_file\"\r\n\
Content-Type: text/plain\r\n\r\n\
{}\r\n\
--{boundary}--\r\n",
			file_contents
		)))?;

	let mut response = test_server.process_request(request).await?;
	assert_eq!(response.status(), StatusCode::OK);

	let response_bytes = to_bytes(response.body_mut()).await?;
	let UploadResponse(uploaded_files) = serde_json::from_slice(&response_bytes)?;

	assert_eq!(uploaded_files.len(), 1);
	assert_eq!(uploaded_files[0].filename, "sample_file");

	let uploaded_file_path = test_server
		.db_dir
		.path()
		.join(DB_DIR)
		.join(DOWNLOAD_COUNT)
		.join(uploaded_files[0].uuid.to_string());

	assert!(uploaded_file_path.exists());

	{
		test_server
			.db_handle
			.writer()
			.await
			.get_mut(&uploaded_files[0].uuid)
			.unwrap()
			.lifetime = Lifetime::Duration(Duration::from_millis(500));
	}

	let start_time = std::time::SystemTime::now();
	while start_time.elapsed()? < Duration::from_millis(500) {
		let request = Request::builder()
			.uri(format!("/api/download/{}", uploaded_files[0].uuid))
			.method(Method::GET)
			.body(Body::empty())?;

		let mut response = test_server.process_request(request).await?;
		assert_eq!(response.status(), StatusCode::OK);

		let response_bytes = to_bytes(response.body_mut()).await?;
		assert_eq!(file_contents.as_bytes(), response_bytes.as_ref());
	}

	let request = Request::builder()
		.uri(format!("/api/download/{}", uploaded_files[0].uuid))
		.method(Method::GET)
		.body(Body::empty())?;

	let mut response = test_server.process_request(request).await?;
	assert_eq!(response.status(), StatusCode::OK);

	let response_bytes = to_bytes(response.body_mut()).await?;
	assert_eq!(file_contents.as_bytes(), response_bytes.as_ref());

	Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn login_works() -> Result<()> {
	let mut test_server = TestServer::new()?;

	let username = String::from("Ala");
	let password = String::from("zażółć gęsią jaźń");

	debug!("creating account");
	let _account_uuid = create_account(
		test_server.db_handle.clone(),
		username.clone(),
		AccountType::Admin,
		Zeroizing::new(password.clone()),
	)
	.await?;

	debug!("creating request");

	let boundary = random_string(50);

	let request = Request::builder()
		.uri("/api/login")
		.method(Method::POST)
		.header(
			"Content-Type",
			format!("multipart/form-data; boundary={boundary}"),
		)
		.body(Body::from(format!(
			"--{boundary}\r\n\
Content-Disposition: form-data; name=\"username\"\r\n\r\n\
{username}\r\n\
--{boundary}--\r\n\
Content-Disposition: form-data; name=\"password\"\r\n\r\n\
{password}\r\n\
--{boundary}--\r\n"
		)))?;

	debug!("processing request");
	let response = test_server.process_request(request).await?;
	assert_eq!(response.status(), StatusCode::CREATED);

	debug!("getting cookie");
	let cookie = response
		.headers()
		.get(SET_COOKIE)
		.expect("Set-Cookie missing")
		.to_str()
		.unwrap();

	debug!("parsing cookie {cookie}");
	let (_, cookie) = cookie::parse_set_cookie(cookie).unwrap();

	assert!(cookie.http_only);
	assert!(cookie.secure);
	assert_eq!(cookie.name, "session");

	Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn registration_code_works() -> Result<()> {
	let mut test_server = TestServer::new()?;

	let username = String::from("Ala");
	let password = String::from("zażółć gęsią jaźń");

	debug!("creating account");
	let _account_uuid = create_account(
		test_server.db_handle.clone(),
		username.clone(),
		AccountType::Admin,
		Zeroizing::new(password.clone()),
	)
	.await?;

	debug!("logging in");
	let boundary = random_string(50);
	let request = Request::builder()
		.uri("/api/login")
		.method(Method::POST)
		.header(
			"Content-Type",
			format!("multipart/form-data; boundary={boundary}"),
		)
		.body(Body::from(format!(
			"--{boundary}\r\n\
Content-Disposition: form-data; name=\"username\"\r\n\r\n\
{username}\r\n\
--{boundary}--\r\n\
Content-Disposition: form-data; name=\"password\"\r\n\r\n\
{password}\r\n\
--{boundary}--\r\n"
		)))?;

	let response = test_server.process_request(request).await?;
	assert_eq!(response.status(), StatusCode::CREATED);

	let session_cookie = response
		.headers()
		.get(SET_COOKIE)
		.expect("Set-Cookie missing")
		.to_str()
		.unwrap();
	let (_, session_cookie) = cookie::parse_set_cookie(session_cookie).unwrap();

	debug!("checking that you cannot create a registration code without logging in");
	let request = Request::builder()
		.uri("/api/registration_code/user")
		.method(Method::GET)
		.body(Body::empty())?;

	let response = test_server.process_request(request).await?;
	assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

	debug!("creating registration code");
	let request = Request::builder()
		.uri("/api/registration_code/user")
		.header(
			"Cookie",
			format!("{}={}", session_cookie.name, session_cookie.value),
		)
		.method(Method::GET)
		.body(Body::empty())?;

	let response = test_server.process_request(request).await?;
	assert_eq!(response.status(), StatusCode::CREATED);

	let response_body = hyper::body::to_bytes(response.into_body()).await?;
	let registration_code: Uuid = std::str::from_utf8(&response_body)?.parse()?;

	debug!("Creating account from the registration code");

	let boundary = random_string(50);
	let request = Request::builder()
		.uri("/api/create_account")
		.method(Method::POST)
		.header(
			"Content-Type",
			format!("multipart/form-data; boundary={boundary}"),
		)
		.body(Body::from(format!(
			"--{boundary}\r\n\
Content-Disposition: form-data; name=\"username\"\r\n\r\n\
Ola\r\n\
--{boundary}--\r\n\
Content-Disposition: form-data; name=\"password\"\r\n\r\n\
{password}\r\n\
--{boundary}--\r\n\
Content-Disposition: form-data; name=\"registration_code\"\r\n\r\n\
{registration_code}\r\n\
--{boundary}--\r\n"
		)))?;

	let response = test_server.process_request(request).await?;
	assert_eq!(response.status(), StatusCode::CREATED);
	let (parts, body) = response.into_parts();

	let response_body = hyper::body::to_bytes(body).await?;
	let _new_account: CreateAccountResponse = serde_json::from_slice(&response_body)?;
	let new_account_session_cookie = parts
		.headers
		.get(SET_COOKIE)
		.expect("Set-Cookie missing")
		.to_str()
		.unwrap();
	let (_, new_account_session_cookie) =
		cookie::parse_set_cookie(new_account_session_cookie).unwrap();

	debug!("triyng out new user via whoami request");
	let request = Request::builder()
		.uri("/api/whoami")
		.header(
			"Cookie",
			format!(
				"{}={}",
				new_account_session_cookie.name, new_account_session_cookie.value
			),
		)
		.method(Method::GET)
		.body(Body::empty())?;

	let response = test_server.process_request(request).await?;
	assert_eq!(response.status(), StatusCode::OK);
	let response_body = hyper::body::to_bytes(response.into_body()).await?;
	let whoami = std::str::from_utf8(&response_body)?;

	assert_eq!(whoami, "Ola");

	Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn user_can_delete_own_file() -> Result<()> {
	let mut test_server = TestServer::new()?;
	test_server.start_cleanup_task(Duration::from_millis(10));

	let username = String::from("Ala");
	let password = String::from("makota");

	let account_uuid = create_account(
		test_server.db_handle.clone(),
		username.clone(),
		AccountType::User,
		Zeroizing::new(password.clone()),
	)
	.await?;

	debug!("Created account with uuid {account_uuid}");

	let boundary = random_string(50);

	let request = Request::builder()
		.uri("/api/login")
		.method(Method::POST)
		.header(
			"Content-Type",
			format!("multipart/form-data; boundary={boundary}"),
		)
		.body(Body::from(format!(
			"--{boundary}\r\n\
Content-Disposition: form-data; name=\"username\"\r\n\r\n\
{username}\r\n\
--{boundary}--\r\n\
Content-Disposition: form-data; name=\"password\"\r\n\r\n\
{password}\r\n\
--{boundary}--\r\n"
		)))?;

	let response = test_server.process_request(request).await?;
	assert_eq!(response.status(), StatusCode::CREATED);

	let cookie = response
		.headers()
		.get(SET_COOKIE)
		.expect("Set-Cookie missing")
		.to_str()
		.unwrap();
	let (_, cookie) = cookie::parse_set_cookie(cookie).unwrap();

	debug!("Logged in with session cookie: {cookie:?}");

	let cookie_header_value = format!("{}={}", cookie.name, cookie.value);
	debug!("Cookie header: {cookie_header_value}");

	let file_contents = random_string(143);
	let boundary = random_string(50);

	const DOWNLOAD_COUNT: &str = "1";

	let request = Request::builder()
		.uri("/api/upload")
		.method(Method::POST)
		.header("Cookie", cookie_header_value.clone())
		.header(
			"Content-Type",
			format!("multipart/form-data; boundary={boundary}"),
		)
		.header(headers::DOWNLOAD_COUNT, DOWNLOAD_COUNT)
		.body(Body::from(format!(
			"--{boundary}\r\n\
Content-Disposition: form-data; name=\"sample_file\"; filename=\"sample_file\"\r\n\
Content-Type: text/plain\r\n\r\n\
{}\r\n\
--{boundary}--\r\n",
			file_contents
		)))?;

	let mut response = test_server.process_request(request).await?;
	assert_eq!(response.status(), StatusCode::OK);

	let response_bytes = to_bytes(response.body_mut()).await?;
	let UploadResponse(uploaded_files) = serde_json::from_slice(&response_bytes)?;

	assert_eq!(uploaded_files[0].filename, "sample_file");

	let uploaded_file_path = test_server
		.db_dir
		.path()
		.join(DB_DIR)
		.join(DOWNLOAD_COUNT)
		.join(uploaded_files[0].uuid.to_string());

	assert!(uploaded_file_path.exists());

	let request = Request::builder()
		.uri(format!("/api/delete/{}", uploaded_files[0].uuid))
		.method(Method::DELETE)
		.header("Cookie", cookie_header_value.clone())
		.body(Body::empty())?;
	let response = test_server.process_request(request).await?;
	assert_eq!(response.status(), StatusCode::NO_CONTENT);

	tokio::time::sleep(Duration::from_millis(20)).await;

	let request = Request::builder()
		.uri(format!("/api/download/{}", uploaded_files[0].uuid))
		.method(Method::GET)
		.body(Body::empty())?;

	let response = test_server.process_request(request).await?;
	assert_eq!(response.status(), StatusCode::NOT_FOUND);

	assert!(!uploaded_file_path.exists());

	Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn user_cannot_delete_public_files() -> Result<()> {
	let mut test_server = TestServer::new()?;
	test_server.start_cleanup_task(Duration::from_millis(10));

	let file_contents = random_string(143);
	let boundary = random_string(50);

	const DOWNLOAD_COUNT: &str = "1";

	let request = Request::builder()
		.uri("/api/upload")
		.method(Method::POST)
		.header(
			"Content-Type",
			format!("multipart/form-data; boundary={boundary}"),
		)
		.header(headers::DOWNLOAD_COUNT, DOWNLOAD_COUNT)
		.body(Body::from(format!(
			"--{boundary}\r\n\
Content-Disposition: form-data; name=\"sample_file\"; filename=\"sample_file\"\r\n\
Content-Type: text/plain\r\n\r\n\
{}\r\n\
--{boundary}--\r\n",
			file_contents
		)))?;

	let mut response = test_server.process_request(request).await?;
	assert_eq!(response.status(), StatusCode::OK);

	let response_bytes = to_bytes(response.body_mut()).await?;
	let UploadResponse(uploaded_files) = serde_json::from_slice(&response_bytes)?;

	assert_eq!(uploaded_files[0].filename, "sample_file");

	let uploaded_file_path = test_server
		.db_dir
		.path()
		.join(DB_DIR)
		.join(DOWNLOAD_COUNT)
		.join(uploaded_files[0].uuid.to_string());

	assert!(uploaded_file_path.exists());

	let username = String::from("Ala");
	let password = String::from("makota");

	let account_uuid = create_account(
		test_server.db_handle.clone(),
		username.clone(),
		AccountType::User,
		Zeroizing::new(password.clone()),
	)
	.await?;

	debug!("Created account with uuid {account_uuid}");

	let boundary = random_string(50);

	let request = Request::builder()
		.uri("/api/login")
		.method(Method::POST)
		.header(
			"Content-Type",
			format!("multipart/form-data; boundary={boundary}"),
		)
		.body(Body::from(format!(
			"--{boundary}\r\n\
Content-Disposition: form-data; name=\"username\"\r\n\r\n\
{username}\r\n\
--{boundary}--\r\n\
Content-Disposition: form-data; name=\"password\"\r\n\r\n\
{password}\r\n\
--{boundary}--\r\n"
		)))?;

	let response = test_server.process_request(request).await?;
	assert_eq!(response.status(), StatusCode::CREATED);

	let cookie = response
		.headers()
		.get(SET_COOKIE)
		.expect("Set-Cookie missing")
		.to_str()
		.unwrap();
	let (_, cookie) = cookie::parse_set_cookie(cookie).unwrap();

	debug!("Logged in with session cookie: {cookie:?}");

	let cookie_header_value = format!("{}={}", cookie.name, cookie.value);
	debug!("Cookie header: {cookie_header_value}");

	let request = Request::builder()
		.uri(format!("/api/delete/{}", uploaded_files[0].uuid))
		.method(Method::DELETE)
		.header("Cookie", cookie_header_value.clone())
		.body(Body::empty())?;
	let mut response = test_server.process_request(request).await?;
	assert_eq!(response.status(), StatusCode::UNAUTHORIZED);

	let response_bytes = to_bytes(response.body_mut()).await?;
	let error: serde_json::Value = serde_json::from_slice(&response_bytes)?;
	assert_eq!(
		error["message"],
		aqa_send::delete::DeleteError::NotAuthorized.to_string()
	);

	Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn user_cannot_delete_someones_file() -> Result<()> {
	let mut test_server = TestServer::new()?;
	test_server.start_cleanup_task(Duration::from_millis(10));

	let ala_username = String::from("Ala");
	let ala_password = String::from("makota");

	let ala_user_uuid = create_account(
		test_server.db_handle.clone(),
		ala_username.clone(),
		AccountType::User,
		Zeroizing::new(ala_password.clone()),
	)
	.await?;
	debug!("Created account with uuid {ala_user_uuid}");

	let ola_username = String::from("Ola");
	let ola_password = String::from("mapsa");

	let ola_user_uuid = create_account(
		test_server.db_handle.clone(),
		ola_username.clone(),
		AccountType::User,
		Zeroizing::new(ola_password.clone()),
	)
	.await?;
	debug!("Created account with uuid {ola_user_uuid}");

	let ela_username = String::from("Ela");
	let ela_password = String::from("mapsa");

	let ela_user_uuid = create_account(
		test_server.db_handle.clone(),
		ela_username.clone(),
		AccountType::Admin,
		Zeroizing::new(ela_password.clone()),
	)
	.await?;
	debug!("Created account with uuid {ela_user_uuid}");

	let boundary = random_string(50);
	let request = Request::builder()
		.uri("/api/login")
		.method(Method::POST)
		.header(
			"Content-Type",
			format!("multipart/form-data; boundary={boundary}"),
		)
		.body(Body::from(format!(
			"--{boundary}\r\n\
Content-Disposition: form-data; name=\"username\"\r\n\r\n\
{ala_username}\r\n\
--{boundary}--\r\n\
Content-Disposition: form-data; name=\"password\"\r\n\r\n\
{ala_password}\r\n\
--{boundary}--\r\n"
		)))?;
	let response = test_server.process_request(request).await?;
	assert_eq!(response.status(), StatusCode::CREATED);
	let cookie = response
		.headers()
		.get(SET_COOKIE)
		.expect("Set-Cookie missing")
		.to_str()
		.unwrap();
	let (_, cookie) = cookie::parse_set_cookie(cookie).unwrap();
	debug!("Logged in with session cookie: {cookie:?}");
	let ala_cookie_header_value = format!("{}={}", cookie.name, cookie.value);
	debug!("Cookie header: {ala_cookie_header_value}");

	let boundary = random_string(50);
	let request = Request::builder()
		.uri("/api/login")
		.method(Method::POST)
		.header(
			"Content-Type",
			format!("multipart/form-data; boundary={boundary}"),
		)
		.body(Body::from(format!(
			"--{boundary}\r\n\
Content-Disposition: form-data; name=\"username\"\r\n\r\n\
{ola_username}\r\n\
--{boundary}--\r\n\
Content-Disposition: form-data; name=\"password\"\r\n\r\n\
{ola_password}\r\n\
--{boundary}--\r\n"
		)))?;
	let response = test_server.process_request(request).await?;
	assert_eq!(response.status(), StatusCode::CREATED);
	let cookie = response
		.headers()
		.get(SET_COOKIE)
		.expect("Set-Cookie missing")
		.to_str()
		.unwrap();
	let (_, cookie) = cookie::parse_set_cookie(cookie).unwrap();
	debug!("Logged in with session cookie: {cookie:?}");
	let ola_cookie_header_value = format!("{}={}", cookie.name, cookie.value);
	debug!("Cookie header: {ola_cookie_header_value}");

	let boundary = random_string(50);
	let request = Request::builder()
		.uri("/api/login")
		.method(Method::POST)
		.header(
			"Content-Type",
			format!("multipart/form-data; boundary={boundary}"),
		)
		.body(Body::from(format!(
			"--{boundary}\r\n\
Content-Disposition: form-data; name=\"username\"\r\n\r\n\
{ela_username}\r\n\
--{boundary}--\r\n\
Content-Disposition: form-data; name=\"password\"\r\n\r\n\
{ela_password}\r\n\
--{boundary}--\r\n"
		)))?;
	let response = test_server.process_request(request).await?;
	assert_eq!(response.status(), StatusCode::CREATED);
	let cookie = response
		.headers()
		.get(SET_COOKIE)
		.expect("Set-Cookie missing")
		.to_str()
		.unwrap();
	let (_, cookie) = cookie::parse_set_cookie(cookie).unwrap();
	debug!("Logged in with session cookie: {cookie:?}");
	let ela_cookie_header_value = format!("{}={}", cookie.name, cookie.value);
	debug!("Cookie header: {ela_cookie_header_value}");

	const DOWNLOAD_COUNT: &str = "1";

	let file_contents = random_string(143);
	let boundary = random_string(50);
	let request = Request::builder()
		.uri("/api/upload")
		.method(Method::POST)
		.header("Cookie", ola_cookie_header_value.clone())
		.header(
			"Content-Type",
			format!("multipart/form-data; boundary={boundary}"),
		)
		.header(headers::DOWNLOAD_COUNT, DOWNLOAD_COUNT)
		.body(Body::from(format!(
			"--{boundary}\r\n\
Content-Disposition: form-data; name=\"sample_file\"; filename=\"sample_file\"\r\n\
Content-Type: text/plain\r\n\r\n\
{}\r\n\
--{boundary}--\r\n",
			file_contents
		)))?;

	let mut response = test_server.process_request(request).await?;
	assert_eq!(response.status(), StatusCode::OK);

	let UploadResponse(uploaded_files) =
		serde_json::from_slice(&to_bytes(response.body_mut()).await?)?;
	let ola_file_uuid = uploaded_files[0].uuid;

	let file_contents = random_string(143);
	let boundary = random_string(50);
	let request = Request::builder()
		.uri("/api/upload")
		.method(Method::POST)
		.header("Cookie", ela_cookie_header_value.clone())
		.header(
			"Content-Type",
			format!("multipart/form-data; boundary={boundary}"),
		)
		.header(headers::DOWNLOAD_COUNT, DOWNLOAD_COUNT)
		.body(Body::from(format!(
			"--{boundary}\r\n\
Content-Disposition: form-data; name=\"sample_file\"; filename=\"sample_file\"\r\n\
Content-Type: text/plain\r\n\r\n\
{}\r\n\
--{boundary}--\r\n",
			file_contents
		)))?;

	let mut response = test_server.process_request(request).await?;
	assert_eq!(response.status(), StatusCode::OK);

	let UploadResponse(uploaded_files) =
		serde_json::from_slice(&to_bytes(response.body_mut()).await?)?;
	let ela_file_uuid = uploaded_files[0].uuid;

	let request = Request::builder()
		.uri(format!("/api/delete/{}", ola_file_uuid))
		.method(Method::DELETE)
		.header("Cookie", ala_cookie_header_value.clone())
		.body(Body::empty())?;
	let mut response = test_server.process_request(request).await?;
	assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
	let error: serde_json::Value = serde_json::from_slice(&to_bytes(response.body_mut()).await?)?;
	assert_eq!(
		error["message"],
		aqa_send::delete::DeleteError::NotAuthorized.to_string()
	);

	let request = Request::builder()
		.uri(format!("/api/delete/{}", ela_file_uuid))
		.method(Method::DELETE)
		.header("Cookie", ala_cookie_header_value.clone())
		.body(Body::empty())?;
	let mut response = test_server.process_request(request).await?;
	assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
	let error: serde_json::Value = serde_json::from_slice(&to_bytes(response.body_mut()).await?)?;
	assert_eq!(
		error["message"],
		aqa_send::delete::DeleteError::NotAuthorized.to_string()
	);

	Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn admin_can_delete_public_files() -> Result<()> {
	let mut test_server = TestServer::new()?;
	test_server.start_cleanup_task(Duration::from_millis(10));

	let file_contents = random_string(143);
	let boundary = random_string(50);

	const DOWNLOAD_COUNT: &str = "1";

	let request = Request::builder()
		.uri("/api/upload")
		.method(Method::POST)
		.header(
			"Content-Type",
			format!("multipart/form-data; boundary={boundary}"),
		)
		.header(headers::DOWNLOAD_COUNT, DOWNLOAD_COUNT)
		.body(Body::from(format!(
			"--{boundary}\r\n\
Content-Disposition: form-data; name=\"sample_file\"; filename=\"sample_file\"\r\n\
Content-Type: text/plain\r\n\r\n\
{}\r\n\
--{boundary}--\r\n",
			file_contents
		)))?;

	let mut response = test_server.process_request(request).await?;
	assert_eq!(response.status(), StatusCode::OK);

	let response_bytes = to_bytes(response.body_mut()).await?;
	let UploadResponse(uploaded_files) = serde_json::from_slice(&response_bytes)?;

	assert_eq!(uploaded_files[0].filename, "sample_file");

	let uploaded_file_path = test_server
		.db_dir
		.path()
		.join(DB_DIR)
		.join(DOWNLOAD_COUNT)
		.join(uploaded_files[0].uuid.to_string());

	assert!(uploaded_file_path.exists());

	let username = String::from("Ala");
	let password = String::from("makota");

	let account_uuid = create_account(
		test_server.db_handle.clone(),
		username.clone(),
		AccountType::Admin,
		Zeroizing::new(password.clone()),
	)
	.await?;

	debug!("Created account with uuid {account_uuid}");

	let boundary = random_string(50);

	let request = Request::builder()
		.uri("/api/login")
		.method(Method::POST)
		.header(
			"Content-Type",
			format!("multipart/form-data; boundary={boundary}"),
		)
		.body(Body::from(format!(
			"--{boundary}\r\n\
Content-Disposition: form-data; name=\"username\"\r\n\r\n\
{username}\r\n\
--{boundary}--\r\n\
Content-Disposition: form-data; name=\"password\"\r\n\r\n\
{password}\r\n\
--{boundary}--\r\n"
		)))?;

	let response = test_server.process_request(request).await?;
	assert_eq!(response.status(), StatusCode::CREATED);

	let cookie = response
		.headers()
		.get(SET_COOKIE)
		.expect("Set-Cookie missing")
		.to_str()
		.unwrap();
	let (_, cookie) = cookie::parse_set_cookie(cookie).unwrap();

	debug!("Logged in with session cookie: {cookie:?}");

	let cookie_header_value = format!("{}={}", cookie.name, cookie.value);
	debug!("Cookie header: {cookie_header_value}");

	let request = Request::builder()
		.uri(format!("/api/delete/{}", uploaded_files[0].uuid))
		.method(Method::DELETE)
		.header("Cookie", cookie_header_value.clone())
		.body(Body::empty())?;
	let response = test_server.process_request(request).await?;
	assert_eq!(response.status(), StatusCode::NO_CONTENT);

	Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn admin_can_delete_someones_file() -> Result<()> {
	let mut test_server = TestServer::new()?;
	test_server.start_cleanup_task(Duration::from_millis(10));

	let ala_username = String::from("Ala");
	let ala_password = String::from("makota");

	let ala_user_uuid = create_account(
		test_server.db_handle.clone(),
		ala_username.clone(),
		AccountType::Admin,
		Zeroizing::new(ala_password.clone()),
	)
	.await?;
	debug!("Created account with uuid {ala_user_uuid}");

	let ola_username = String::from("Ola");
	let ola_password = String::from("mapsa");

	let ola_user_uuid = create_account(
		test_server.db_handle.clone(),
		ola_username.clone(),
		AccountType::User,
		Zeroizing::new(ola_password.clone()),
	)
	.await?;
	debug!("Created account with uuid {ola_user_uuid}");

	let ela_username = String::from("Ela");
	let ela_password = String::from("mapsa");

	let ela_user_uuid = create_account(
		test_server.db_handle.clone(),
		ela_username.clone(),
		AccountType::Admin,
		Zeroizing::new(ela_password.clone()),
	)
	.await?;
	debug!("Created account with uuid {ela_user_uuid}");

	let boundary = random_string(50);
	let request = Request::builder()
		.uri("/api/login")
		.method(Method::POST)
		.header(
			"Content-Type",
			format!("multipart/form-data; boundary={boundary}"),
		)
		.body(Body::from(format!(
			"--{boundary}\r\n\
Content-Disposition: form-data; name=\"username\"\r\n\r\n\
{ala_username}\r\n\
--{boundary}--\r\n\
Content-Disposition: form-data; name=\"password\"\r\n\r\n\
{ala_password}\r\n\
--{boundary}--\r\n"
		)))?;
	let response = test_server.process_request(request).await?;
	assert_eq!(response.status(), StatusCode::CREATED);
	let cookie = response
		.headers()
		.get(SET_COOKIE)
		.expect("Set-Cookie missing")
		.to_str()
		.unwrap();
	let (_, cookie) = cookie::parse_set_cookie(cookie).unwrap();
	debug!("Logged in with session cookie: {cookie:?}");
	let ala_cookie_header_value = format!("{}={}", cookie.name, cookie.value);
	debug!("Cookie header: {ala_cookie_header_value}");

	let boundary = random_string(50);
	let request = Request::builder()
		.uri("/api/login")
		.method(Method::POST)
		.header(
			"Content-Type",
			format!("multipart/form-data; boundary={boundary}"),
		)
		.body(Body::from(format!(
			"--{boundary}\r\n\
Content-Disposition: form-data; name=\"username\"\r\n\r\n\
{ola_username}\r\n\
--{boundary}--\r\n\
Content-Disposition: form-data; name=\"password\"\r\n\r\n\
{ola_password}\r\n\
--{boundary}--\r\n"
		)))?;
	let response = test_server.process_request(request).await?;
	assert_eq!(response.status(), StatusCode::CREATED);
	let cookie = response
		.headers()
		.get(SET_COOKIE)
		.expect("Set-Cookie missing")
		.to_str()
		.unwrap();
	let (_, cookie) = cookie::parse_set_cookie(cookie).unwrap();
	debug!("Logged in with session cookie: {cookie:?}");
	let ola_cookie_header_value = format!("{}={}", cookie.name, cookie.value);
	debug!("Cookie header: {ola_cookie_header_value}");

	let boundary = random_string(50);
	let request = Request::builder()
		.uri("/api/login")
		.method(Method::POST)
		.header(
			"Content-Type",
			format!("multipart/form-data; boundary={boundary}"),
		)
		.body(Body::from(format!(
			"--{boundary}\r\n\
Content-Disposition: form-data; name=\"username\"\r\n\r\n\
{ela_username}\r\n\
--{boundary}--\r\n\
Content-Disposition: form-data; name=\"password\"\r\n\r\n\
{ela_password}\r\n\
--{boundary}--\r\n"
		)))?;
	let response = test_server.process_request(request).await?;
	assert_eq!(response.status(), StatusCode::CREATED);
	let cookie = response
		.headers()
		.get(SET_COOKIE)
		.expect("Set-Cookie missing")
		.to_str()
		.unwrap();
	let (_, cookie) = cookie::parse_set_cookie(cookie).unwrap();
	debug!("Logged in with session cookie: {cookie:?}");
	let ela_cookie_header_value = format!("{}={}", cookie.name, cookie.value);
	debug!("Cookie header: {ela_cookie_header_value}");

	const DOWNLOAD_COUNT: &str = "1";

	let file_contents = random_string(143);
	let boundary = random_string(50);
	let request = Request::builder()
		.uri("/api/upload")
		.method(Method::POST)
		.header("Cookie", ola_cookie_header_value.clone())
		.header(
			"Content-Type",
			format!("multipart/form-data; boundary={boundary}"),
		)
		.header(headers::DOWNLOAD_COUNT, DOWNLOAD_COUNT)
		.body(Body::from(format!(
			"--{boundary}\r\n\
Content-Disposition: form-data; name=\"sample_file\"; filename=\"sample_file\"\r\n\
Content-Type: text/plain\r\n\r\n\
{}\r\n\
--{boundary}--\r\n",
			file_contents
		)))?;

	let mut response = test_server.process_request(request).await?;
	assert_eq!(response.status(), StatusCode::OK);

	let UploadResponse(uploaded_files) =
		serde_json::from_slice(&to_bytes(response.body_mut()).await?)?;
	let ola_file_uuid = uploaded_files[0].uuid;

	let file_contents = random_string(143);
	let boundary = random_string(50);
	let request = Request::builder()
		.uri("/api/upload")
		.method(Method::POST)
		.header("Cookie", ela_cookie_header_value.clone())
		.header(
			"Content-Type",
			format!("multipart/form-data; boundary={boundary}"),
		)
		.header(headers::DOWNLOAD_COUNT, DOWNLOAD_COUNT)
		.body(Body::from(format!(
			"--{boundary}\r\n\
Content-Disposition: form-data; name=\"sample_file\"; filename=\"sample_file\"\r\n\
Content-Type: text/plain\r\n\r\n\
{}\r\n\
--{boundary}--\r\n",
			file_contents
		)))?;

	let mut response = test_server.process_request(request).await?;
	assert_eq!(response.status(), StatusCode::OK);

	let UploadResponse(uploaded_files) =
		serde_json::from_slice(&to_bytes(response.body_mut()).await?)?;
	let ela_file_uuid = uploaded_files[0].uuid;

	let request = Request::builder()
		.uri(format!("/api/delete/{}", ola_file_uuid))
		.method(Method::DELETE)
		.header("Cookie", ala_cookie_header_value.clone())
		.body(Body::empty())?;
	let response = test_server.process_request(request).await?;
	assert_eq!(response.status(), StatusCode::NO_CONTENT);

	let request = Request::builder()
		.uri(format!("/api/delete/{}", ela_file_uuid))
		.method(Method::DELETE)
		.header("Cookie", ala_cookie_header_value.clone())
		.body(Body::empty())?;
	let response = test_server.process_request(request).await?;
	assert_eq!(response.status(), StatusCode::NO_CONTENT);

	Ok(())
}
