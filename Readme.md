# nfqueue-degrader

nfqueue-degrader is a real-time software network degrader which can manipulate ip packets on Linux sytems with the help of defined ip table rules and the libnetfilter_queue library. 
More specifically it can drop or delay single packets or packet sequences based on a selected degradation model and thus introduce network packet loss, out-of-order delivery and packet bursts.

ip table rules define which ip connections are affected by the degradation (INPUT or OUTPUT, udp or tcp, port ranges...)

Currently 3 degradation models are supported:

- random: 
  - define a loss rate and/ or a delay 
- pattern file: 
  - define delay and loss (drop) per packet in a 2 column csv file (pattern is applied repetitive after file end is reached)
- bandwidth restriction: 
  - define a target bandwidth and a max. packet buffer size 
  - if incoming rate is higher than the target packets will be queued in the buffer and thus delayed
  - if max. buffer size is reached, packets get dropped
  - the underlying model is based on the token bucket algorithm

Models can be chained together, e.g. to limit the bandwidth and have a bursty and/ or random network behavior

---
# Build and test
### install netfilter queue libraries
- Ubuntu: ```apt-get install libnetfilter-queue1 libnetfilter-queue-dev```
- Arch: ```pacman -S libnetfilter_queue```

### cargo (Rust's build system)
- cargo build (--release)
- cargo test (execute unit tests)

---
# Usage
- define iptable rule(s):
  - e.g. ```sudo iptables -A OUTPUT -p udp --dport=40000:40010 -j NFQUEUE --queue-num 0```

- run nfqueue-degrader
  - get help: ```./target/debug/nfqueue_degrader -h```
  - random degradation: ```sudo ./target/debug/nfqueue_degrader --queue_num 0 --random 10 0 20```
  - pattern file: ```sudo ./target/debug/nfqueue_degrader --queue_num 0 --pattern_file examples/10-30ms_delay_5%_loss.csv```
  - bandwidth: ```sudo ./target/debug/nfqueue_degrader --queue_num 0 --bandwidth 1000 1000 1000```

- queue number must be the same for iptables and nfqueue-degrader (default is 0)
- the degrader has an understanding of connections (identified by source + destination ip, port and protocol)
- the same degradation is applied for each individual connection per default, even if the ip table rule is e.g. defined for a range of ports
- if the 'per connection mode' is disabled all connections and their packets of the defined ip table rule are considered as one degradation queue and the selected degradation model is applied randomly for the connections 
  - ```sudo ./target/debug/nfqueue_degrader --queue_num 0 --per_connection false --random 10 0 20```
- a log file is written to better understand and debug the degrader
---
# Network test application
- the repository contains a client/ server application to establish multiple udp connections on a defined port range
  - continuously sends packets between client and server on each connection 
  - server forwards packets to same client: clientSender(connA) -> server(connA) -> clientReceiver(connA)
  - client sends packets with a defined packet size and packet rate (configurable)
  - measures and records rtt (round trip time), packet rate (packets per second), bandwidth and loss rate (%) on client receiver side for each connection
  - writes statistics for each received packet in a statistics file (per connection)
  - the test application can be used for performance investigations (e.g. check how many connections can be reliably degraded in real-time)

- command line interface:
  - get help: ```./target/debug/network_analyzer -h```
  - start with default parameters: ```sudo ./target/debug/network_analyzer```

---
# Links
- http://www.netfilter.org/projects/libnetfilter_queue/
- https://home.regit.org/netfilter-en/using-nfqueue-and-libnetfilter_queue/
- https://linux.die.net/man/8/iptables
