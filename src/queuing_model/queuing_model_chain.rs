use super::bandwidth_queuing_model::BandwidthQueuingModel;
use super::pattern_file_queuing_model::PatternFileQueuingModel;
use super::random_queuing_model::RandomQueuingModel;
use super::QueuingModel;
use crate::config::QueuingModelConfig;
use crate::nfqueue_wrapper::NfqPacket;

use std::fmt::Display;
use std::time::Duration;

struct ForwardingQueuingModel {
    packets: Vec<NfqPacket>,
}

impl ForwardingQueuingModel {
    pub fn new() -> ForwardingQueuingModel {
        ForwardingQueuingModel {
            packets: Vec::new(),
        }
    }
}

impl QueuingModel for ForwardingQueuingModel {
    fn enqueue(&mut self, packet: NfqPacket, _: Duration) {
        self.packets.push(packet);
    }

    fn dequeue(&mut self, _: Duration) -> Vec<NfqPacket> {
        self.packets.split_off(0)
    }
}

impl Display for ForwardingQueuingModel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "forwarding queuing model")
    }
}

pub struct QueuingModelChain {
    models: Vec<Box<dyn QueuingModel>>,
}

impl QueuingModelChain {
    pub fn new(config: &[QueuingModelConfig]) -> QueuingModelChain {
        let mut models: Vec<Box<dyn QueuingModel>> = config
            .iter()
            .map(|conf| match conf {
                QueuingModelConfig::Bandwidth(cfg) => Box::new(BandwidthQueuingModel::new(
                    cfg.rate,
                    cfg.burst_size,
                    cfg.buffer_size,
                )) as Box<dyn QueuingModel>,
                QueuingModelConfig::Random(cfg) => Box::new(
                    RandomQueuingModel::new(cfg.loss_rate).with_delay_range(cfg.delay_range),
                ),
                QueuingModelConfig::PatternFile(cfg) => {
                    Box::new(PatternFileQueuingModel::new(&cfg.packet_info))
                }
            })
            .inspect(|model| log::info!("created {}", model))
            .collect();

        if models.is_empty() {
            log::info!("No models defined, forwarding all packets without degradation");
            models.push(Box::new(ForwardingQueuingModel::new()));
        }

        QueuingModelChain { models }
    }
}

impl QueuingModel for QueuingModelChain {
    fn enqueue(&mut self, packet: NfqPacket, time_now: Duration) {
        let m = self.models.first_mut().unwrap();
        m.enqueue(packet, time_now);
    }

    fn dequeue(&mut self, time_now: Duration) -> Vec<NfqPacket> {
        let mut packets = Vec::<NfqPacket>::new();
        for model in self.models.iter_mut() {
            for p in packets {
                model.enqueue(p, time_now);
            }
            packets = model.dequeue(time_now);
        }
        packets
    }
}

impl Display for QueuingModelChain {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "chain with {} queueing models", self.models.len())
    }
}
