/*!
A Rust implementation of statsd server & client.

The statsd protocol consistents of plain-text single-packet messages sent
over UDP, containing not much more than a key and (possibly sampled) value.

Due to the inherent design of the system, there is no guarantee that metrics
will be received by the server, and there is (by design) no indication of
this.
*/
pub mod client;
pub mod server;
mod metric;
