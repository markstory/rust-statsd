use std::collections::VecDeque;
use std::error;
use std::fmt;
use std::io::Error;
use std::net::AddrParseError;
use std::net::{SocketAddr, ToSocketAddrs, UdpSocket};
use std::time;

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

impl error::Error for StatsdError {}

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
        let server_address = host
            .to_socket_addrs()?
            .next()
            .ok_or_else(|| StatsdError::AddrParseError("Address parsing error".to_string()))?;

        // Bind to a generic port as we'll only be writing on this
        // socket.
        let socket = if server_address.is_ipv4() {
            UdpSocket::bind("0.0.0.0:0")?
        } else {
            UdpSocket::bind("[::]:0")?
        };
        Ok(Client {
            socket,
            prefix: prefix.to_string(),
            server_address,
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
    pub fn incr(&self, metric: &str) {
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
    pub fn decr(&self, metric: &str) {
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
    pub fn count(&self, metric: &str, value: f64) {
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
    pub fn sampled_count(&self, metric: &str, value: f64, rate: f64) {
        if rand::random::<f64>() >= rate {
            return;
        }
        let data = self.prepare(format!("{}:{}|c|@{}", metric, value, rate));
        self.send(data);
    }

    /// Set a gauge value.
    ///
    /// ```ignore
    /// // set a gauge to 9001
    /// client.gauge("power_level.observed", 9001.0);
    /// ```
    pub fn gauge(&self, metric: &str, value: f64) {
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
    pub fn timer(&self, metric: &str, value: f64) {
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
    pub fn time<F, R>(&self, metric: &str, callable: F) -> R
    where
        F: FnOnce() -> R,
    {
        let start = time::Instant::now();
        let return_val = callable();
        let used = start.elapsed();
        let data = self.prepare(format!("{}:{}|ms", metric, used.as_millis()));
        self.send(data);
        return_val
    }

    fn prepare<T: AsRef<str>>(&self, data: T) -> String {
        if self.prefix.is_empty() {
            data.as_ref().to_string()
        } else {
            format!("{}.{}", self.prefix, data.as_ref())
        }
    }

    /// Send data along the UDP socket.
    fn send(&self, data: String) {
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

    /// Send a histogram value.
    ///
    /// ```ignore
    /// // pass response size value
    /// client.histogram("response.size", 128.0);
    /// ```
    pub fn histogram(&self, metric: &str, value: f64) {
        let data = self.prepare(format!("{}:{}|h", metric, value));
        self.send(data);
    }

    /// Send a key/value
    ///
    /// ```ignore
    /// client.kv("key", 1.);
    /// ```
    pub fn kv(&self, metric: &str, value: f64) {
        let data = self.prepare(format!("{}:{}|kv", metric, value));
        self.send(data);
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
        if rand::random::<f64>() >= rate {
            return;
        }
        let data = format!("{}:{}|c|@{}", metric, value, rate);
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
    where
        F: FnOnce(),
    {
        let start = time::Instant::now();
        callable();
        let used = start.elapsed();
        let data = format!("{}:{}|ms", metric, used.as_millis());
        self.stats.push_back(data);
    }

    /// Send a histogram value.
    ///
    /// ```
    /// use statsd::client::Pipeline;
    ///
    /// let mut pipe = Pipeline::new();
    /// // pass response size value
    /// pipe.histogram("response.size", 128.0);
    /// ```
    pub fn histogram(&mut self, metric: &str, value: f64) {
        let data = format!("{}:{}|h", metric, value);
        self.stats.push_back(data);
    }

    /// Send a key/value.
    ///
    /// ```
    /// use statsd::client::Pipeline;
    ///
    /// let mut pipe = Pipeline::new();
    /// // pass response size value
    /// pipe.kv("response.size", 256.);
    /// ```
    pub fn kv(&mut self, metric: &str, value: f64) {
        let data = format!("{}:{}|kv", metric, value);
        self.stats.push_back(data);
    }

    /// Send data along the UDP socket.
    pub fn send(&mut self, client: &Client) {
        let mut _data = String::new();
        if let Some(data) = self.stats.pop_front() {
            _data += client.prepare(&data).as_ref();
            while !self.stats.is_empty() {
                let stat = client.prepare(self.stats.pop_front().unwrap());
                if data.len() + stat.len() + 1 > self.max_udp_size {
                    client.send(_data.clone());
                    _data.clear();
                    _data += &stat;
                } else {
                    _data += "\n";
                    _data += &stat;
                }
            }
        }
        if !_data.is_empty() {
            client.send(_data);
        }
    }
}

impl Default for Pipeline {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    use std::net::{SocketAddr, UdpSocket};
    use std::sync::{
        atomic::{AtomicBool, Ordering},
        mpsc::channel,
        Arc,
    };
    use std::thread;
    use std::time::Duration;

    struct Server {
        local_addr: SocketAddr,
        sock: UdpSocket,
    }

    impl Server {
        fn new() -> Self {
            let addr: SocketAddr = "127.0.0.1:0".parse().unwrap();
            let sock = UdpSocket::bind(addr).unwrap();
            sock.set_read_timeout(Some(Duration::from_millis(100)))
                .unwrap();
            let local_addr = sock.local_addr().unwrap();
            Server { local_addr, sock }
        }

        fn addr(&self) -> SocketAddr {
            self.local_addr.clone()
        }

        /// Run the given test function while receiving several packets. Return a vector of the
        /// packets.
        fn run_while_receiving_all<F>(self, func: F) -> Vec<String>
        where
            F: Fn(),
        {
            let (serv_tx, serv_rx) = channel();
            let func_ran = Arc::new(AtomicBool::new(false));
            let bg_func_ran = Arc::clone(&func_ran);
            let bg = thread::spawn(move || loop {
                let mut buf = [0; 1500];
                if let Ok((len, _)) = self.sock.recv_from(&mut buf) {
                    let bytes = Vec::from(&buf[0..len]);
                    serv_tx.send(bytes).unwrap();
                }
                // go through the loop least once (do...while)
                if bg_func_ran.load(Ordering::SeqCst) {
                    break;
                }
            });
            func();
            std::thread::sleep(Duration::from_millis(200));
            func_ran.store(true, Ordering::SeqCst);
            bg.join().expect("background thread should join");
            serv_rx
                .into_iter()
                .map(|bytes| String::from_utf8(bytes).unwrap())
                .collect()
        }

        /// Run the given test function while receiving several packets. Return the concatenation
        /// of the packets.
        fn run_while_receiving<F>(self, func: F) -> String
        where
            F: Fn(),
        {
            itertools::Itertools::intersperse(
                self.run_while_receiving_all(func).into_iter(),
                String::from("\n"),
            )
            .fold(String::new(), |acc, b| acc + &b)
        }
    }

    #[test]
    fn test_sending_gauge() {
        let server = Server::new();
        let client = Client::new(server.addr(), "myapp").unwrap();
        let response = server.run_while_receiving(|| client.gauge("metric", 9.1));
        assert_eq!("myapp.metric:9.1|g", response);
    }

    #[test]
    fn test_sending_gauge_without_prefix() {
        let server = Server::new();
        let client = Client::new(server.addr(), "").unwrap();
        let response = server.run_while_receiving(|| client.gauge("metric", 9.1));
        assert_eq!("metric:9.1|g", response);
    }

    #[test]
    fn test_sending_incr() {
        let server = Server::new();
        let client = Client::new(server.addr(), "myapp").unwrap();
        let response = server.run_while_receiving(|| client.incr("metric"));
        assert_eq!("myapp.metric:1|c", response);
    }

    #[test]
    fn test_sending_decr() {
        let server = Server::new();
        let client = Client::new(server.addr(), "myapp").unwrap();
        let response = server.run_while_receiving(|| client.decr("metric"));
        assert_eq!("myapp.metric:-1|c", response);
    }

    #[test]
    fn test_sending_count() {
        let server = Server::new();
        let client = Client::new(server.addr(), "myapp").unwrap();
        let response = server.run_while_receiving(|| client.count("metric", 12.2));
        assert_eq!("myapp.metric:12.2|c", response);
    }

    #[test]
    fn test_sending_timer() {
        let server = Server::new();
        let client = Client::new(server.addr(), "myapp").unwrap();
        let response = server.run_while_receiving(|| client.timer("metric", 21.39));
        assert_eq!("myapp.metric:21.39|ms", response);
    }

    struct TimeTest {
        num: u8,
    }

    #[test]
    fn test_sending_timed_block() {
        let server = Server::new();
        let client = Client::new(server.addr(), "myapp").unwrap();
        let response = server.run_while_receiving(|| {
            let mut t = TimeTest { num: 10 };
            let output = client.time("time_block", || {
                t.num += 2;
                "a string"
            });
            assert_eq!(output, "a string");
            assert_eq!(t.num, 12);
        });
        assert!(response.contains("myapp.time_block"));
        assert!(response.contains("|ms"));
    }

    #[test]
    fn test_sending_histogram() {
        let server = Server::new();
        let client = Client::new(server.addr(), "myapp").unwrap();
        let response = server.run_while_receiving(|| client.histogram("metric", 9.1));
        assert_eq!("myapp.metric:9.1|h", response);
    }

    #[test]
    fn test_sending_kv() {
        let server = Server::new();
        let client = Client::new(server.addr(), "myapp").unwrap();
        let response = server.run_while_receiving(|| client.kv("metric", 15.26));
        assert_eq!("myapp.metric:15.26|kv", response);
    }

    #[test]
    fn test_pipeline_sending_time_block() {
        let server = Server::new();
        let client = Client::new(server.addr(), "myapp").unwrap();
        let response = server.run_while_receiving(|| {
            let mut pipeline = client.pipeline();
            pipeline.gauge("metric", 9.1);

            let mut t = TimeTest { num: 10 };
            pipeline.time("time_block", || {
                t.num += 2;
            });
            pipeline.send(&client);
            assert_eq!(t.num, 12);
        });
        assert_eq!("myapp.metric:9.1|g\nmyapp.time_block:0|ms", response);
    }

    #[test]
    fn test_pipeline_sending_gauge() {
        let server = Server::new();
        let client = Client::new(server.addr(), "myapp").unwrap();
        let response = server.run_while_receiving(|| {
            let mut pipeline = client.pipeline();
            pipeline.gauge("metric", 9.1);
            pipeline.send(&client);
        });
        assert_eq!("myapp.metric:9.1|g", response);
    }

    #[test]
    fn test_pipeline_sending_histogram() {
        let server = Server::new();
        let client = Client::new(server.addr(), "myapp").unwrap();
        let response = server.run_while_receiving(|| {
            let mut pipeline = client.pipeline();
            pipeline.histogram("metric", 9.1);
            pipeline.send(&client);
        });
        assert_eq!("myapp.metric:9.1|h", response);
    }

    #[test]
    fn test_pipeline_sending_kv() {
        let server = Server::new();
        let client = Client::new(server.addr(), "myapp").unwrap();
        let response = server.run_while_receiving(|| {
            let mut pipeline = client.pipeline();
            pipeline.kv("metric", 15.26);
            pipeline.send(&client);
        });
        assert_eq!("myapp.metric:15.26|kv", response);
    }

    #[test]
    fn test_pipeline_sending_multiple_data() {
        let server = Server::new();
        let client = Client::new(server.addr(), "myapp").unwrap();
        let response = server.run_while_receiving(|| {
            let mut pipeline = client.pipeline();
            pipeline.gauge("metric", 9.1);
            pipeline.count("metric", 12.2);
            pipeline.send(&client);
        });
        assert_eq!("myapp.metric:9.1|g\nmyapp.metric:12.2|c", response);
    }

    #[test]
    fn test_pipeline_set_max_udp_size() {
        let server = Server::new();
        let client = Client::new(server.addr(), "myapp").unwrap();
        let response = server.run_while_receiving_all(|| {
            let mut pipeline = client.pipeline();
            pipeline.set_max_udp_size(20);
            pipeline.gauge("metric", 9.1);
            pipeline.count("metric", 12.2);
            pipeline.send(&client);
        });
        assert_eq!(vec!["myapp.metric:9.1|g", "myapp.metric:12.2|c"], response);
    }

    #[test]
    fn test_pipeline_send_metric_after_pipeline() {
        let server = Server::new();
        let client = Client::new(server.addr(), "myapp").unwrap();
        let response = server.run_while_receiving_all(|| {
            let mut pipeline = client.pipeline();

            pipeline.gauge("load", 9.0);
            pipeline.count("customers", 7.0);
            pipeline.send(&client);

            // Should still be able to send metrics
            // with the client.
            client.count("customers", 6.0);
        });
        assert_eq!(
            vec!["myapp.load:9|g\nmyapp.customers:7|c", "myapp.customers:6|c"],
            response
        );
    }
}
