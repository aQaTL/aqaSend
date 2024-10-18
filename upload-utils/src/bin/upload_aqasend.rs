use ssh_shell::ShellProcess;
use std::error::Error as StdError;
use std::fmt::{Display, Formatter};
use std::process::Command;

enum CliCommand {
	UploadAqaSend,
	UploadWebsite,
	Help,
}

fn main() -> Result<(), Box<dyn StdError>> {
	let command = parse_cli()?;

	match command {
		CliCommand::UploadAqaSend => upload_aqasend()?,
		CliCommand::UploadWebsite => upload_website()?,
		CliCommand::Help => {
			println!("Usage:\n\tupload_aqasend <COMMAND>\n\nCOMMAND: one of [aqasend,website]");
		}
	}

	Ok(())
}

fn upload_aqasend() -> Result<(), Box<dyn StdError>> {
	Command::new("wsl")
		.args([
			"-e",
			"/bin/bash",
			"-l",
			"-c",
			"cd ~/dev/rust_old/aqaSend; git pull windows main; cargo build --release",
		])
		.spawn()?
		.wait()?;

	let cmd = r#"
systemctl stop aqasend.service
rm /apps/aqa_send/aqa_send
"#;
	ssh_cmd(cmd)?;

	Command::new("scp")
		.args([
			r"\\wsl.localhost\Ubuntu-22.04\home\aqatl\dev\rust\aqaSend\target\release\aqa_send",
			"aqatl.pl:/apps/aqa_send/",
		])
		.spawn()?
		.wait()?;

	let cmd = r#"
chmod +x /apps/aqa_send/aqa_send
systemctl start aqasend.service
systemctl status aqasend.service
"#;
	ssh_cmd(cmd)?;
	Ok(())
}

fn upload_website() -> Result<(), Box<dyn StdError>> {
	let cmd = "cd ~/dev/rust_old/aqaSend; \
        git pull windows main; \
        rsync \
            --delete \
            --recursive \
            --progress \
            --compress \
            --exclude=node_modules \
            --exclude=out \
            --exclude=sprites \
            --exclude=api_endpoint.mjs \
            --exclude=package-lock.json \
            --exclude=package.json \
            --exclude=tsconfig.json \
            simple-website/ aqatl.pl:/apps/aqa_send/website/";

	println!("command:\n{cmd}");

	Command::new("wsl")
		.args(["-e", "/bin/bash", "-l", "-c", cmd])
		.spawn()?
		.wait()?;

	Ok(())
}

fn ssh_cmd(cmd: &str) -> Result<(), Box<dyn StdError>> {
	let shell_session = ShellProcess::new("root@aqatl.pl")?;
	shell_session.send_command(cmd)?;
	shell_session.close()?;
	Ok(())
}

fn parse_cli() -> Result<CliCommand, Box<dyn StdError>> {
	let args = std::env::args().skip(1);
	let mut cmd = CliCommand::Help;
	for arg in args {
		match arg.as_str() {
			"aqasend" => cmd = CliCommand::UploadAqaSend,
			"website" => cmd = CliCommand::UploadWebsite,
			_ => cmd = CliCommand::Help,
		}
	}
	Ok(cmd)
}

#[derive(Debug)]
pub struct CustomError(String);

impl<T> From<T> for CustomError
where
	T: AsRef<str>,
{
	fn from(v: T) -> Self {
		CustomError(v.as_ref().to_string())
	}
}

impl Display for CustomError {
	fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
		self.0.fmt(f)
	}
}

impl StdError for CustomError {}
