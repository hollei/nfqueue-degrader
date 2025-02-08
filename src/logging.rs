use log::LevelFilter;
use log4rs::append::file::FileAppender;
use log4rs::config::{Appender, Config, Root};
use log4rs::encode::pattern::PatternEncoder;

static LOG_FILE_NAME: &str = "log.txt";

pub fn init(level: LevelFilter) {
    let logfile = match FileAppender::builder()
        .encoder(Box::new(PatternEncoder::new("{d}   {l} - {m}\n")))
        .build(LOG_FILE_NAME)
    {
        Ok(file) => file,
        Err(e) => panic!("Error while creating log file {}: {}", LOG_FILE_NAME, e),
    };

    let config = Config::builder()
        .appender(Appender::builder().build("logfile", Box::new(logfile)))
        .build(Root::builder().appender("logfile").build(level));

    log4rs::init_config(config.unwrap()).unwrap();

    log::info!("Logging initialized, level: {}", level);
}
