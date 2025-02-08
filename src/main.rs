mod config;
mod logging;
mod nfqueue_degrader;
mod nfqueue_wrapper;
mod protocol;
mod queuing_model;

fn main() {
    println!("Start degrader");
    logging::init(log::LevelFilter::Debug);

    let config = config::Config::from_cli();
    let degrader = nfqueue_degrader::NfqueueDegrader::new(config);
    degrader.start();
}
