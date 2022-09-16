extern crate core;

use anyhow::Result;
use hyper::body::to_bytes;
use hyper::service::Service;
use hyper::{Body, Method, Request, Response, StatusCode};
use rand::thread_rng;
use std::time::Duration;
use tempfile::TempDir;
use tokio::fs;

use aqa_send::files::DB_DIR;
use aqa_send::upload::UploadResponse;
use aqa_send::{db, tasks, AqaService, AqaServiceError};

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

	let request = Request::builder()
		.uri("/api/upload")
		.method(Method::POST)
		.header(
			"Content-Type",
			format!("multipart/form-data; boundary={boundary}"),
		)
		.header("aqa-download-count", "1")
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
		.join("1")
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

	let request = Request::builder()
		.uri("/api/upload")
		.method(Method::POST)
		.header(
			"Content-Type",
			format!("multipart/form-data; boundary={boundary}"),
		)
		.header("aqa-download-count", "1")
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
		.join("1")
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

	let request = Request::builder()
		.uri("/api/upload")
		.method(Method::POST)
		.header(
			"Content-Type",
			format!("multipart/form-data; boundary={boundary}"),
		)
		.header("aqa-download-count", "10")
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
		.join("10")
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
