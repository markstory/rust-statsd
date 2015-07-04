use std::net::{UdpSocket, SocketAddr};
use std::io::{Error};

/// Client socket for statsd servers.
///
/// After creating a metric you can use `Client`
/// to send metrics to the configured statsd server
///
/// # Example
///
/// Creating a client and sending metrics is easy.
///
/// ```ignore
/// use statsd::client::Client;
///
/// let client = Client::new("127.0.0.1:8125", "myapp.");
/// client.incr("some.metric.completed");
/// ```
pub struct Client {
    socket: UdpSocket,
    server_addr: SocketAddr,
    prefix: String
}

impl Client {
    /// Construct a new statsd client given an host/port & prefix
    pub fn new(host: &str, prefix: &str) -> Result<Client, Error> {
        let addr = SocketAddr.from_str(host);
        let mut socket = try!(UdpSocket::bind(addr));
        Ok(Client {socket: socket, prefix: prefix.to_string(), server_addr: addr})
    }

    /// Increment a metric by 1
    ///
    /// ```ignore
    /// # Increment a given metric by 1.
    /// client.incr("metric.completed");
    /// ```
    ///
    /// This modifies a counter with an effective sampling
    /// rate of 1.0.
    pub fn incr(&mut self, metric: &str) {
        self.count(metric, 1.0);
    }

    /// Decrement a metric by -1
    ///
    /// ```ignore
    /// # Decrement a given metric by 1
    /// client.decr("metric.completed");
    /// ```
    ///
    /// This modifies a counter with an effective sampling
    /// rate of 1.0.
    pub fn decr(&mut self, metric: &str) {
        self.count(metric, -1.0);
    }

    /// Modify a counter by `value`.
    ///
    /// Will increment or decrement a counter by `value` with
    /// a sampling rate of 1.0.
    ///
    /// ```ignore
    /// # Increment by 12
    /// client.count("metric.completed", 12);
    /// ```
    pub fn count(&mut self, metric: &str, value: usize) {
        let data = format!("{}.{}:{}|c", self.prefix, metric, value);
        self.send(data);
    }

    /// Data goes in, data comes out.
    fn send(&mut self, data: &str) {
        let _ = self.socket.send_to(data.as_bytes(), self.dest);
    }
}
