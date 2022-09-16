use log::{LevelFilter, Metadata, Record, SetLoggerError};

pub struct Logger;

static LOGGER: Logger = Logger;

pub fn init() {
	try_init().expect("Tried to set global logger twice");
}

pub fn try_init() -> Result<(), SetLoggerError> {
	log::set_logger(&LOGGER)?;
	if cfg!(debug_assertions) {
		log::set_max_level(LevelFilter::Debug);
	} else {
		log::set_max_level(LevelFilter::Info);
	}
	Ok(())
}

static COLORS: [u32; 5] = [91, 31, 33, 36, 90];

impl log::Log for Logger {
	fn enabled(&self, _metadata: &Metadata) -> bool {
		true
	}

	fn log(&self, record: &Record) {
		println!(
			"\x1B[0;{}m[{}][{}]\x1B[0m {}",
			COLORS[record.level() as usize - 1],
			record.level(),
			record.metadata().target(),
			record.args(),
		);
	}

	fn flush(&self) {}
}

#[test]
fn color_test() {
	for c in COLORS {
		println!("\x1B[0;{}mHello World\x1b[0m", c);
	}
}
