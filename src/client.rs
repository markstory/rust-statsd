use std::net::{UdpSocket, SocketAddr, ToSocketAddrs};
use std::io::Error;
use std::net::AddrParseError;
use std::collections::VecDeque;
use std::fmt;
use std::error;
use std::time;

extern crate rand;

#[inline]
fn duration_in_ms(d: time::Duration) -> u32 {
  (d.as_secs() * 1000) as u32 + d.subsec_nanos() / 1_000_000
}


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

impl fmt::Display for StatsdError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            StatsdError::IoError(ref e) => write!(f, "{}", e),
            StatsdError::AddrParseError(ref e) => write!(f, "{}", e),
        }
    }
}

impl error::Error for StatsdError {
    fn description(&self) -> &str {
        match *self {
            StatsdError::IoError(ref e) => e.description(),
            StatsdError::AddrParseError(ref e) => e,
        }
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
/// let client = Client::new("127.0.0.1:8125", "myapp");
/// client.incr("some.metric.completed");
/// ```
pub struct Client {
    socket: UdpSocket,
    server_address: SocketAddr,
    prefix: String,
}

impl Client {
    /// Construct a new statsd client given an host/port & prefix
    pub fn new<T: ToSocketAddrs>(host: T, prefix: &str) -> Result<Client, StatsdError> {
        // Bind to a generic port as we'll only be writing on this
        // socket.
        let socket = UdpSocket::bind("0.0.0.0:0")?;
        let server_address = host.to_socket_addrs()?
            .next()
            .ok_or_else(|| StatsdError::AddrParseError("Address parsing error".to_string()))?;
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
    /// // Increment by 12
    /// client.count("metric.completed", 12.0);
    /// ```
    pub fn count(&mut self, metric: &str, value: f64) {
        let data = self.prepare(format!("{}:{}|c", metric, value));
        self.send(data);
    }

    /// Modify a counter by `value` only x% of the time.
    ///
    /// Will increment or decrement a counter by `value` with
    /// a custom sampling rate.
    ///
    ///
    /// ```ignore
    /// // Increment by 4 50% of the time.
    /// client.sampled_count("metric.completed", 4, 0.5);
    /// ```
    pub fn sampled_count(&mut self, metric: &str, value: f64, rate: f64) {
        if rand::random::<f64>() < rate {
            return;
        }
        let data = self.prepare(format!("{}:{}|c", metric, value));
        self.send(data);
    }

    /// Set a gauge value.
    ///
    /// ```ignore
    /// // set a gauge to 9001
    /// client.gauge("power_level.observed", 9001.0);
    /// ```
    pub fn gauge(&mut self, metric: &str, value: f64) {
        let data = self.prepare(format!("{}:{}|g", metric, value));
        self.send(data);
    }

    /// Send a timer value.
    ///
    /// The value is expected to be in ms.
    ///
    /// ```ignore
    /// // pass a duration value
    /// client.timer("response.duration", 10.123);
    /// ```
    pub fn timer(&mut self, metric: &str, value: f64) {
        let data = self.prepare(format!("{}:{}|ms", metric, value));
        self.send(data);
    }

    /// Time a block of code.
    ///
    /// The passed closure will be timed and executed. The block's
    /// duration will be sent as a metric.
    ///
    /// ```ignore
    /// // pass a duration value
    /// client.time("response.duration", || {
    ///   // Your code here.
    /// });
    /// ```
    pub fn time<F, R>(&mut self, metric: &str, callable: F) -> R
        where F: Fn() -> R
    {
        let start = time::Instant::now();
        let return_val = callable();
        let used = start.elapsed();
        let data = self.prepare(format!(
            "{}:{}|ms", metric, duration_in_ms(used)));
        self.send(data);
        return_val
    }

    fn prepare<T: AsRef<str>>(&self, data: T) -> String {
        format!("{}.{}", self.prefix, data.as_ref())
    }

    /// Send data along the UDP socket.
    fn send(&mut self, data: String) {
        let _ = self.socket.send_to(data.as_bytes(), self.server_address);
    }

    /// Get a pipeline struct that allows optimizes the number of UDP
    /// packets used to send multiple metrics
    ///
    /// ```ignore
    /// let mut pipeline = client.pipeline();
    /// pipeline.incr("some.metric", 1);
    /// pipeline.incr("other.metric", 1);
    /// pipeline.send(&mut client);
    /// ```
    pub fn pipeline(&self) -> Pipeline {
        Pipeline::new()
    }
}

pub struct Pipeline {
    stats: VecDeque<String>,
    max_udp_size: usize,
}

impl Pipeline {
    pub fn new() -> Pipeline {
        Pipeline {
            stats: VecDeque::new(),
            max_udp_size: 512,
        }
    }

    /// Set max UDP packet size
    ///
    /// ```
    /// use statsd::client::Pipeline;
    ///
    /// let mut pipe = Pipeline::new();
    /// pipe.set_max_udp_size(128);
    /// ```
    pub fn set_max_udp_size(&mut self, max_udp_size: usize) {
        self.max_udp_size = max_udp_size;
    }

    /// Increment a metric by 1
    ///
    /// ```
    /// use statsd::client::Pipeline;
    ///
    /// let mut pipe = Pipeline::new();
    /// // Increment a given metric by 1.
    /// pipe.incr("metric.completed");
    /// ```
    ///
    /// This modifies a counter with an effective sampling
    /// rate of 1.0.
    pub fn incr(&mut self, metric: &str) {
        self.count(metric, 1.0);
    }

    /// Decrement a metric by -1
    ///
    /// ```
    /// use statsd::client::Pipeline;
    ///
    /// let mut pipe = Pipeline::new();
    /// // Decrement a given metric by 1
    /// pipe.decr("metric.completed");
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
    /// ```
    /// use statsd::client::Pipeline;
    ///
    /// let mut pipe = Pipeline::new();
    /// // Increment by 12
    /// pipe.count("metric.completed", 12.0);
    /// ```
    pub fn count(&mut self, metric: &str, value: f64) {
        let data = format!("{}:{}|c", metric, value);
        self.stats.push_back(data);
    }

    /// Modify a counter by `value` only x% of the time.
    ///
    /// Will increment or decrement a counter by `value` with
    /// a custom sampling rate.
    ///
    /// ```
    /// use statsd::client::Pipeline;
    ///
    /// let mut pipe = Pipeline::new();
    /// // Increment by 4 50% of the time.
    /// pipe.sampled_count("metric.completed", 4.0, 0.5);
    /// ```
    pub fn sampled_count(&mut self, metric: &str, value: f64, rate: f64) {
        if rand::random::<f64>() < rate {
            return;
        }
        let data = format!("{}:{}|c", metric, value);
        self.stats.push_back(data);
    }

    /// Set a gauge value.
    ///
    /// ```
    /// use statsd::client::Pipeline;
    ///
    /// let mut pipe = Pipeline::new();
    /// // set a gauge to 9001
    /// pipe.gauge("power_level.observed", 9001.0);
    /// ```
    pub fn gauge(&mut self, metric: &str, value: f64) {
        let data = format!("{}:{}|g", metric, value);
        self.stats.push_back(data);
    }

    /// Send a timer value.
    ///
    /// The value is expected to be in ms.
    ///
    /// ```
    /// use statsd::client::Pipeline;
    ///
    /// let mut pipe = Pipeline::new();
    /// // pass a duration value
    /// pipe.timer("response.duration", 10.123);
    /// ```
    pub fn timer(&mut self, metric: &str, value: f64) {
        let data = format!("{}:{}|ms", metric, value);
        self.stats.push_back(data);
    }

    /// Time a block of code.
    ///
    /// The passed closure will be timed and executed. The block's
    /// duration will be sent as a metric.
    ///
    /// ```
    /// use statsd::client::Pipeline;
    ///
    /// let mut pipe = Pipeline::new();
    /// // pass a duration value
    /// pipe.time("response.duration", || {
    ///   // Your code here.
    /// });
    /// ```
    pub fn time<F>(&mut self, metric: &str, callable: F)
        where F: Fn()
    {
        let start = time::Instant::now();
        callable();
        let used = start.elapsed();
        let data = format!("{}:{}|ms", metric, duration_in_ms(used));
        self.stats.push_back(data);
    }

    /// Send data along the UDP socket.
    pub fn send(&mut self, client: &mut Client) {
        let mut _data = String::new();
        if let Some(data) = self.stats.pop_front() {
            _data = _data + client.prepare(&data).as_ref();
            while !self.stats.is_empty() {
                let stat = client.prepare(self.stats.pop_front().unwrap());
                if data.len() + stat.len() + 1 > self.max_udp_size {
                    client.send(_data.clone());
                    _data.clear();
                    _data = _data + &stat;
                } else {
                    _data = _data + "\n";
                    _data = _data +&stat;
                }
            }
        }
        if !_data.is_empty() {
            client.send(_data);
        }
    }
}

#[cfg(test)]
mod test {
    extern crate rand;
    use super::*;
    use std::sync::mpsc::sync_channel;
    use std::net::UdpSocket;
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
        let server = UdpSocket::bind(host).ok().unwrap();
        server
    }

    fn server_recv(server: UdpSocket) -> String {
        let (serv_tx, serv_rx) = sync_channel(1);
        let _t = thread::spawn(move || {
            let mut buf = [0; 128];
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
        let server = make_server(&host);
        let mut client = Client::new(&host, "myapp").unwrap();

        client.gauge("metric", 9.1);

        let response = server_recv(server);
        assert_eq!("myapp.metric:9.1|g", response);
    }

    #[test]
    fn test_sending_incr() {
        let host = next_test_ip4();
        let server = make_server(&host);
        let mut client = Client::new(&host, "myapp").unwrap();

        client.incr("metric");

        let response = server_recv(server);
        assert_eq!("myapp.metric:1|c", response);
    }

    #[test]
    fn test_sending_decr() {
        let host = next_test_ip4();
        let server = make_server(&host);
        let mut client = Client::new(&host, "myapp").unwrap();

        client.decr("metric");

        let response = server_recv(server);
        assert_eq!("myapp.metric:-1|c", response);
    }

    #[test]
    fn test_sending_count() {
        let host = next_test_ip4();
        let server = make_server(&host);
        let mut client = Client::new(&host, "myapp").unwrap();

        client.count("metric", 12.2);

        let response = server_recv(server);
        assert_eq!("myapp.metric:12.2|c", response);
    }

    #[test]
    fn test_sending_timer() {
        let host = next_test_ip4();
        let server = make_server(&host);
        let mut client = Client::new(&host, "myapp").unwrap();

        client.timer("metric", 21.39);

        let response = server_recv(server);
        assert_eq!("myapp.metric:21.39|ms", response);
    }

    #[test]
    fn test_sending_timed_block() {
        let host = next_test_ip4();
        let server = make_server(&host);
        let mut client = Client::new(&host, "myapp").unwrap();

        let output = client.time("metric", || {
            "a string"
        });

        let response = server_recv(server);
        assert_eq!(output, "a string");
        assert!(response.contains("myapp.metric"));
        assert!(response.contains("|ms"));
    }

    #[test]
    fn test_pipeline_sending_gauge() {
        let host = next_test_ip4();
        let server = make_server(&host);
        let mut client = Client::new(&host, "myapp").unwrap();
        let mut pipeline = client.pipeline();
        pipeline.gauge("metric", 9.1);
        pipeline.send(&mut client);

        let response = server_recv(server);
        assert_eq!("myapp.metric:9.1|g", response);
    }

    #[test]
    fn test_pipeline_sending_multiple_data() {
        let host = next_test_ip4();
        let server = make_server(&host);
        let mut client = Client::new(&host, "myapp").unwrap();
        let mut pipeline = client.pipeline();
        pipeline.gauge("metric", 9.1);
        pipeline.count("metric", 12.2);
        pipeline.send(&mut client);

        let response = server_recv(server);
        assert_eq!("myapp.metric:9.1|g\nmyapp.metric:12.2|c", response);
    }

    #[test]
    fn test_pipeline_set_max_udp_size() {
        let host = next_test_ip4();
        let server = make_server(&host);
        let mut client = Client::new(&host, "myapp").unwrap();
        let mut pipeline = client.pipeline();
        pipeline.set_max_udp_size(20);
        pipeline.gauge("metric", 9.1);
        pipeline.count("metric", 12.2);
        pipeline.send(&mut client);

        let response = server_recv(server);
        assert_eq!("myapp.metric:9.1|g", response);
    }

    #[test]
    fn test_pipeline_send_metric_after_pipeline() {
        let host = next_test_ip4();
        let server = make_server(&host);
        let mut client = Client::new(&host, "myapp").unwrap();
        let mut pipeline = client.pipeline();

        pipeline.gauge("load", 9.0);
        pipeline.count("customers", 7.0);
        pipeline.send(&mut client);

        // Should still be able to send metrics
        // with the client.
        client.count("customers", 6.0);

        let response = server_recv(server);
        assert_eq!(
            "myapp.load:9|g\nmyapp.customers:7|c",
            response);
    }
}
