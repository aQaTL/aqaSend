use std::process::Command;

fn main() {
	Command::new("npm")
		.arg("run")
		.arg("build")
		.current_dir("../aqa_send_web")
		.status()
		.unwrap();
}
