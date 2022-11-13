use anyhow::Result;
use hyper::body::to_bytes;
use hyper::header::SET_COOKIE;
use hyper::service::Service;
use hyper::{Body, Method, Request, Response, StatusCode};
use log::debug;
use rand::thread_rng;
use std::time::Duration;
use tempfile::TempDir;
use tokio::fs;
use zeroize::Zeroizing;

use aqa_send::cli_commands::create_account::create_account;
use aqa_send::db_stuff::AccountType;
use aqa_send::files::DB_DIR;
use aqa_send::headers::Lifetime;
use aqa_send::upload::UploadResponse;
use aqa_send::{cookie, db, headers, list, tasks, AqaService, AqaServiceError};

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

		let _ = aqa_logger::try_init();
		// if let Err(err) = tracing_subscriber::FmtSubscriber::builder().try_init() {
		// 	eprintln!("{err:?}");
		// }
		//
		// init_app_directory_structure(db_dir.path())?;

		let db_handle = db::init(db_dir.path())?;
		let aqa_service = AqaService::new(db_handle.clone());

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
	assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);

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
		.header(headers::PASSWORD, PASSWORD)
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
		.uri(format!("/api/download/{}", uploaded_files[0].uuid))
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

	let request = Request::builder()
		.uri("/api/login")
		.method(Method::POST)
		.body(Body::from(format!("{username}\n{password}\n")))?;

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
	let list: Vec<list::OwnedFileModel> = serde_json::from_slice(&response_bytes)?;
	assert_eq!(list.len(), 0);

	let request = Request::builder()
		.uri("/api/list.json")
		.method(Method::GET)
		.header("Cookie", cookie_header_value)
		.body(Body::empty())?;

	let mut response = test_server.process_request(request).await?;
	assert_eq!(response.status(), StatusCode::OK);

	let response_bytes = to_bytes(response.body_mut()).await?;
	let list: Vec<list::OwnedFileModel> = serde_json::from_slice(&response_bytes)?;
	assert_eq!(list.len(), 1);

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
	let request = Request::builder()
		.uri("/api/login")
		.method(Method::POST)
		.body(Body::from(format!("{username}\n{password}\n")))?;

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

	debug!("parsing cookie");
	let (_, cookie) = cookie::parse_set_cookie(cookie).unwrap();

	assert!(cookie.http_only);
	assert!(cookie.secure);
	assert_eq!(cookie.name, "session");

	Ok(())
}
