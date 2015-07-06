// Load the crate
extern crate statsd;

// Import the client object.
use statsd::client::Client;

fn main() {
    let mut client = Client::new("127.0.0.1:8125", "myapp").unwrap();
    client.incr("some.counter");
    println!("Sent a counter!");

    client.gauge("some.gauge", 124.0);
    println!("Set a gauge!");

    client.timer("timer.duration", 182.1);
    println!("Set a timer!");
}
