use std::process::Command;

fn main() {
	println!("cargo:rerun-if-changed=../aqa_send_web/dist");
	println!("cargo:rerun-if-changed=../aqa_send_web/src");
	Command::new("npm")
		.arg("run")
		.arg("build")
		.current_dir("../aqa_send_web")
		.status()
		.unwrap();
}
