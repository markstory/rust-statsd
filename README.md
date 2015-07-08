# Rust Statsd

A Statsd client implementation in rust.

## Using the client library

Add the `statsd` package as a dependency in your `Cargo.toml` file:

```
[dependencies]
statsd = "0.1.0"
```

You can then get a client instance and start tracking metrics:

```rust
// Load the crate
extern crate statsd;

// Import the client object.
use statsd::client::Client;

// Get a client with the prefix of `myapp`. The host should be the
// IP:port of your statsd daemon.
let mut client = Client::new("127.0.0.1:8125", "myapp").unwrap();

client.incr("some.counter");
client.decr("some.counter");
client.gauge("some.counter", 12.0);
client.timer("some.counter", 13.4);
```
