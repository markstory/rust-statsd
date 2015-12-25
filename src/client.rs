use std::net::{UdpSocket, SocketAddr};
use std::io::Error;
use std::str::FromStr;
use std::net::AddrParseError;

extern crate clock_ticks;
extern crate rand;


#[derive(Debug)]
pub enum StatsdError {
    IoError(Error),
    AddrParseError(String),
}

impl From<AddrParseError> for StatsdError {
    fn from(_: AddrParseError) -> StatsdError {
        StatsdError::AddrParseError("Address parsing error".to_string())
    }
}

impl From<Error> for StatsdError {
    fn from(err: Error) -> StatsdError {
        StatsdError::IoError(err)
    }
}

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
    server_address: SocketAddr,
    prefix: String,
}

impl Client {
    /// Construct a new statsd client given an host/port & prefix
    pub fn new(host: &str, prefix: &str) -> Result<Client, StatsdError> {
        // Bind to a generic port as we'll only be writing on this
        // socket.
        let client_address = try!(SocketAddr::from_str("0.0.0.0:0"));
        let socket = try!(UdpSocket::bind(client_address));

        let server_address = try!(SocketAddr::from_str(host));
        Ok(Client {
            socket: socket,
            prefix: prefix.to_string(),
            server_address: server_address,
        })
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
    pub fn count(&mut self, metric: &str, value: f64) {
        let data = format!("{}.{}:{}|c", self.prefix, metric, value);
        self.send(data);
    }

    /// Modify a counter by `value` only x% of the time.
    ///
    /// Will increment or decrement a counter by `value` with
    /// a custom sampling rate.
    ///
    ///
    /// ```ignore
    /// # Increment by 4 50% of the time.
    /// client.sampled_count("metric.completed", 4, 0.5);
    /// ```
    pub fn sampled_count(&mut self, metric: &str, value: f64, rate: f64) {
        if rand::random::<f64>() < rate {
            return;
        }
        let data = format!("{}.{}:{}|c", self.prefix, metric, value);
        self.send(data);
    }

    /// Set a gauge value.
    ///
    /// ```ignore
    /// # set a gauge to 9001
    /// client.gauge("power_level.observed", 9001);
    /// ```
    pub fn gauge(&mut self, metric: &str, value: f64) {
        let data = format!("{}.{}:{}|g", self.prefix, metric, value);
        self.send(data);
    }

    /// Send a timer value.
    ///
    /// The value is expected to be in ms.
    ///
    /// ```ignore
    /// # pass a duration value
    /// client.timer("response.duration", 10.123);
    /// ```
    pub fn timer(&mut self, metric: &str, value: f64) {
        let data = format!("{}.{}:{}|ms", self.prefix, metric, value);
        self.send(data);
    }

    /// Time a block of code.
    ///
    /// The passed closure will be timed and executed. The block's
    /// duration will be sent as a metric.
    ///
    /// ```ignore
    /// # pass a duration value
    /// client.time("response.duration", || {
    ///   # Your code here.
    /// });
    /// ```
    pub fn time<F>(&mut self, metric: &str, callable: F)
        where F: Fn()
    {
        let start = clock_ticks::precise_time_ms();
        callable();
        let end = clock_ticks::precise_time_ms();
        let data = format!("{}.{}:{}|ms", self.prefix, metric, end - start);
        self.send(data);
    }

    /// Send data along the UDP socket.
    fn send(&mut self, data: String) {
        let _ = self.socket.send_to(data.as_bytes(), self.server_address);
    }
}

#[cfg(test)]
mod test {
    extern crate rand;
    use super::*;
    use std::sync::mpsc::sync_channel;
    use std::net::{UdpSocket, SocketAddr};
    use std::str::FromStr;
    use self::rand::distributions::{IndependentSample, Range};
    use std::str;
    use std::thread;

    static PORT: u16 = 8125;

    // Generates random ports.
    // Having random ports helps tests not collide over
    // shared ports.
    fn next_test_ip4() -> String {
        let range = Range::new(0, 1000);
        let mut rng = rand::thread_rng();
        let port = PORT + range.ind_sample(&mut rng);
        format!("127.0.0.1:{}", port).to_string()
    }

    // Makes a udpsocket that acts as a statsd server.
    fn make_server(host: &str) -> UdpSocket {
        let addr = SocketAddr::from_str(host.as_ref()).unwrap();
        let server = UdpSocket::bind(addr).ok().unwrap();
        server
    }

    fn server_recv(server: UdpSocket) -> String {
        let (serv_tx, serv_rx) = sync_channel(1);
        let _t = thread::spawn(move || {
            let mut buf = [0; 30];
            let (len, _) = match server.recv_from(&mut buf) {
                Ok(r) => r,
                Err(_) => panic!("No response from test server."),
            };
            drop(server);
            let bytes = Vec::from(&buf[0..len]);
            serv_tx.send(bytes).unwrap();
        });

        let bytes = serv_rx.recv().ok().unwrap();
        str::from_utf8(&bytes).unwrap().to_string()
    }

    #[test]
    fn test_sending_gauge() {
        let host = next_test_ip4();
        let server = make_server(host.as_ref());
        let mut client = Client::new(host.as_ref(), "myapp").unwrap();

        client.gauge("metric", 9.1);

        let response = server_recv(server);
        assert_eq!("myapp.metric:9.1|g", response);
    }

    #[test]
    fn test_sending_incr() {
        let host = next_test_ip4();
        let server = make_server(host.as_ref());
        let mut client = Client::new(host.as_ref(), "myapp").unwrap();

        client.incr("metric");

        let response = server_recv(server);
        assert_eq!("myapp.metric:1|c", response);
    }

    #[test]
    fn test_sending_decr() {
        let host = next_test_ip4();
        let server = make_server(host.as_ref());
        let mut client = Client::new(host.as_ref(), "myapp").unwrap();

        client.decr("metric");

        let response = server_recv(server);
        assert_eq!("myapp.metric:-1|c", response);
    }

    #[test]
    fn test_sending_count() {
        let host = next_test_ip4();
        let server = make_server(host.as_ref());
        let mut client = Client::new(host.as_ref(), "myapp").unwrap();

        client.count("metric", 12.2);

        let response = server_recv(server);
        assert_eq!("myapp.metric:12.2|c", response);
    }

    #[test]
    fn test_sending_timer() {
        let host = next_test_ip4();
        let server = make_server(host.as_ref());
        let mut client = Client::new(host.as_ref(), "myapp").unwrap();

        client.timer("metric", 21.39);

        let response = server_recv(server);
        assert_eq!("myapp.metric:21.39|ms", response);
    }
}
