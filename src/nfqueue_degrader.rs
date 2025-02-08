use crate::config;
use crate::nfqueue_wrapper::*;
use crate::protocol::*;
use crate::queuing_model::queuing_model_chain::QueuingModelChain;
use crate::queuing_model::QueuingModel;
use std::collections::HashMap;
use std::sync::mpsc;
use std::time::{Duration, Instant};

fn queue_callback(packet: NfqPacket, state: &mut State) {
    state.sender.send(packet).unwrap();
}

fn thread_func(packet_rx: mpsc::Receiver<NfqPacket>, cfg: config::Config) {
    let clock = Instant::now();
    let mut connection_queues = HashMap::new();
    loop {
        let now = clock.elapsed();
        if let Ok(p) = packet_rx.recv_timeout(Duration::from_millis(1)) {
            let protocol_info = if cfg.apply_per_connection {
                ProtocolInfo::from_ipv4_header(p.get_payload())
            } else {
                ProtocolInfo::default()
            };

            log::debug!("packet received for connection: {}", protocol_info);

            if connection_queues.get(&protocol_info).is_none() {
                log::info!("add new packet queue for connection {}", protocol_info);
            }

            let model_chain = connection_queues
                .entry(protocol_info)
                .or_insert_with(|| QueuingModelChain::new(&cfg.models));
            model_chain.enqueue(p, now);
        }

        for packet_queue in connection_queues.values_mut() {
            for p in packet_queue.dequeue(now) {
                p.set_verdict(Verdict::Accept);
            }
        }
    }
}

pub struct State {
    sender: mpsc::Sender<NfqPacket>,
}

pub struct NfqueueDegrader {
    queue: NfQueueWrapper<State>,
    config: config::Config,
    receiver: mpsc::Receiver<NfqPacket>,
}

impl NfqueueDegrader {
    pub fn new(conf: config::Config) -> Self {
        let (packet_tx, packet_rx): (mpsc::Sender<NfqPacket>, _) = mpsc::channel();

        Self {
            queue: NfQueueWrapper::new(State { sender: packet_tx }, queue_callback),
            config: conf,
            receiver: packet_rx,
        }
    }

    pub fn start(mut self) {
        let packet_rx = self.receiver;
        let queue_num = self.config.queue_num;
        let cfg = self.config;
        std::thread::spawn(move || {
            thread_func(packet_rx, cfg);
        });
        self.queue.open(queue_num);
        self.queue.run_loop();
    }
}
