// Load the crate
extern crate statsd;

// Import the client object.
use statsd::client::Client;

fn main() {
    let mut client = Client::new("127.0.0.1:8125", "myapp").unwrap();
    client.incr("some.metric");
    println!("Sent a metric!");
}
