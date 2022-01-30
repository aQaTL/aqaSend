use log::{LevelFilter, Metadata, Record};

pub struct Logger;

static LOGGER: Logger = Logger;

pub fn init() {
	log::set_logger(&LOGGER).expect("Tried to set global logger twice");
	log::set_max_level(LevelFilter::Debug);
}

impl log::Log for Logger {
	fn enabled(&self, _metadata: &Metadata) -> bool {
		true
	}

	fn log(&self, record: &Record) {
		println!(
			"[{}][{}] {}",
			record.level(),
			record.metadata().target(),
			record.args(),
		);
	}

	fn flush(&self) {}
}
