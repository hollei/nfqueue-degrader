use crate::queuing_model::pattern_file_queuing_model::{PacketInfo, PatternFileQueuingModel};
use clap::{App, Arg};
use std::time::Duration;

pub struct RandomQueuingModelConfig {
    pub loss_rate: u32,
    pub delay_range: (Duration, Duration),
}

pub struct PatternQueuingModelConfig {
    pub packet_info: Vec<PacketInfo>,
}

pub struct BandwidthQueuingModelConfig {
    pub rate: u64,
    pub burst_size: u64,
    pub buffer_size: u64,
}

pub enum QueuingModelConfig {
    PatternFile(PatternQueuingModelConfig),
    Random(RandomQueuingModelConfig),
    Bandwidth(BandwidthQueuingModelConfig),
}

pub enum LogLevel {
    Info,
    Warning,
    Debug,
}
pub struct Config {
    pub models: Vec<QueuingModelConfig>,
    pub queue_num: u16,
    pub log_level: LogLevel,
    pub apply_per_connection: bool,
}

impl Config {
    pub fn from_cli() -> Config {
        let matches = App::new("nfqueue degrader")
            .version("1.0.0")
            .author("Holger Kaden <holger.kaden@logmein.com>")
            .about("network degrader based on iptables with NFQUEUE")
            .arg(
                Arg::with_name("queue_num")
                    .long("queue_num")
                    .takes_value(true)
                    .default_value("0")
                    .help("nfqueue number"),
            )
            .arg(
                Arg::with_name("log_level")
                    .long("log_level")
                    .takes_value(true)
                    .possible_values(&["info", "debug", "warn"])
                    .default_value("info")
                    .help("log level"),
            )
            .arg(
                Arg::with_name("bandwidth")
                .long("bandwidth")
                .multiple(true)
                .value_name("rate").takes_value(true)
                .value_name("burst").takes_value(true)
                .value_name("buffer").takes_value(true)   
                .help("restrict bandwidth to <rate> KBps, max. burst size is <burst> KB, max. buffer size is <buffer> KB")
            )
            .arg(
                Arg::with_name("pattern_file")
                    .long("pattern_file")
                    .takes_value(true)
                    .help("csv pattern file with delay and drop/accept info per packet"),
            )
            .arg(
                Arg::with_name("random")
                    .long("random")
                    .multiple(true)
                    .value_name("loss")
                    .takes_value(true)
                    .value_name("delay_min")
                    .takes_value(true)
                    .value_name("delay_max")
                    .takes_value(true)
                    .help("Random <loss> in % with random delay between <delay_min> ms and <delay_max> ms"),
            )
            .arg(
                Arg::with_name("per_connection")
                    .long("per_connection")
                    .takes_value(true)
                    .possible_values(&["true", "false"])
                    .default_value("true")
                    .help("apply configured degradation model per connection (source + destination ip/port/protocol)")
            )
            .get_matches();

        let log_level = match matches.value_of("log_level").unwrap() {
            "info" => LogLevel::Info,
            "debug" => LogLevel::Debug,
            "warn" => LogLevel::Warning,
            _ => panic!("unknown log level"),
        };

        let queue_num = matches
            .value_of("queue_num")
            .unwrap()
            .parse::<u16>()
            .unwrap();

        let apply_per_connection = matches
            .value_of("per_connection")
            .unwrap()
            .parse::<bool>()
            .unwrap();

        let mut model_configs = Vec::<QueuingModelConfig>::new();

        if let Some(mut values) = matches.values_of("random") {
            let loss_rate = values.next().unwrap().parse::<u32>().unwrap();
            let delay_min = values.next().unwrap().parse::<u32>().unwrap();
            let delay_max = values.next().unwrap().parse::<u32>().unwrap();

            if delay_min > delay_max {
                eprintln!("min. delay must be smaller equal max. delay");
                std::process::exit(1);
            }

            let delay_range = (
                Duration::from_millis(delay_min as u64),
                Duration::from_millis(delay_max as u64),
            );

            model_configs.push(QueuingModelConfig::Random(RandomQueuingModelConfig {
                loss_rate,
                delay_range,
            }));
        }

        if let Some(pattern_file) = matches.value_of("pattern_file") {
            log::info!("read csv file: {}", pattern_file);
            match PatternFileQueuingModel::parse_packet_info(&pattern_file) {
                Ok(packet_info) => {
                    let config = PatternQueuingModelConfig { packet_info };
                    model_configs.push(QueuingModelConfig::PatternFile(config));
                }
                Err(e) => {
                    eprintln!("error parsing {}: {}", pattern_file, e);
                    std::process::exit(1);
                }
            }
        }

        if let Some(mut values) = matches.values_of("bandwidth") {
            let rate = values.next().unwrap().parse::<u64>().unwrap();
            let burst_size = values.next().unwrap().parse::<u64>().unwrap();
            let buffer_size = values.next().unwrap().parse::<u64>().unwrap();

            if rate == 0 {
                eprintln!("bitrate must be larger 0");
                std::process::exit(1);
            }

            if burst_size == 0 {
                eprintln!("burst size cannot be 0, it should cover at least the size of a packet");
                std::process::exit(1);
            }

            if burst_size > buffer_size {
                eprintln!("burst size must be smaller equal buffer size");
                std::process::exit(1);
            }

            model_configs.push(QueuingModelConfig::Bandwidth(BandwidthQueuingModelConfig {
                rate,
                burst_size,
                buffer_size,
            }))
        }

        Config {
            models: model_configs,
            log_level,
            queue_num,
            apply_per_connection,
        }
    }
}
