/// Internal metric representation
///
use std::fmt;
use std::str::FromStr;
use std::cmp;


/// Enum of metric types
pub enum MetricKind {
    Counter(f64), // sample rate
    Gauge,
    Timer,
    Histogram
}

impl fmt::Debug for MetricKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            MetricKind::Gauge      => write!(f, "Gauge"),
            MetricKind::Timer      => write!(f, "Timer"),
            MetricKind::Histogram  => write!(f, "Histogram"),
            MetricKind::Counter(s) => write!(f, "Counter(s={})", s)
        }
    }
}


/// Error types for parsing Metrics from strings.
///
pub enum ParseError {
    // Error message, column
    SyntaxError(&'static str, usize)
}


/// Metric value objects.
///
#[derive(Debug)]
pub struct Metric {
    kind: MetricKind,
    name: String,
    value: f64
}

impl FromStr for Metric {
    type Err = ParseError;

    /// Valid message formats are:
    ///
    /// - `<str:metric_name>:<f64:value>|<str:type>`
    /// - `<str:metric_name>:<f64:value>|c|@<f64:sample_rate>`
    fn from_str(line: &str) -> Result<Metric, ParseError> {
        let name_parts: Vec<&str> = line.split(':').collect();
        if name_parts.len() < 2 || name_parts[0].is_empty() {
            return Err(ParseError::SyntaxError(
                    "Metrics require a name.",
                    0))
        }
        let name = name_parts[0].to_string();

        let val_parts: Vec<&str>= name_parts[1].split('|').collect();
        if val_parts.len() < 2 || val_parts[0].is_empty() {
            return Err(ParseError::SyntaxError(
                    "Metrics require a value.",
                    name.len()))
        }
        let value = val_parts[0].parse::<f64>().ok().unwrap();


        Ok(Metric{name: name, value: value, kind: MetricKind::Timer})
        /*
        let name = match line.find(':') {
            // We don't want to allow blank key names.
            Some(pos) if pos != 0 => {
                idx += pos + 1;
                line.slice_chars(0, pos).to_owned()
            },

            _ => return Err(ParseError::SyntaxError(
                    "Metrics require a name",
                    idx))
        };

        // Try to parse `<f64>|`, return None if no match is found.
        let value_opt = line.slice_chars(idx, end).find('|').and_then(|loc| {
            let number = line.slice_chars(idx, idx + loc).parse::<f64>();

            idx = loc + 1;
            Some(number.ok())
        });

        let value = match value_opt {
            Some(v) => v.unwrap(),
            None => return Err(ParseError::SyntaxError(
                        "Metrics require a value",
                        idx))
        };

        let end_idx = cmp::min(idx + 3, end);

        let kind = match line.slice_chars(idx, end_idx) {
            "c" => MetricKind::Counter(1.0),
            "ms" => MetricKind::Timer,
            "h" => MetricKind::Histogram,
            "g" => MetricKind::Gauge,
            // Sampled counter
            /*
            "c|@" => match line.slice_chars(end_idx, end).parse::<f64>() {
                Ok(sample) => MetricKind::Counter(sample),
                _ => return Err(ParseError::SyntaxError(
                            "Counters require a sampling rate",
                            idx))
            },
            */

            // Unknown type
            _ => return Err(ParseError::SyntaxError(
                        "Unknown metric type.",
                        idx))
        };

        Ok(Metric{kind: kind, name: name, value: value})
        */
    }
}



//
// Tests
//
#[test]
fn test_metric_kind_debug_fmt() {
    assert_eq!(
        "Gauge",
        format!("{:?}", MetricKind::Gauge)
    );
    assert_eq!(
        "Timer",
        format!("{:?}", MetricKind::Timer)
    );
    assert_eq!(
        "Histogram",
        format!("{:?}", MetricKind::Histogram)
    );
    assert_eq!(
        "Counter(s=6)",
        format!("{:?}", MetricKind::Counter(6.0))
    );
}

#[test]
fn test_metric_from_str_invalid_no_name() {
    let res = Metric::from_str("");
    assert!(res.is_err(), "Should have an error");
    assert!(!res.is_ok(), "Should have an error");
}

#[test]
fn test_metric_from_str_invalid_no_value() {
    let res = Metric::from_str("foo:");
    assert!(res.is_err(), "Should have an error");
    assert!(!res.is_ok(), "Should have an error");
}

#[test]
fn test_metric_from_str_invalid_no_type() {
    let res = Metric::from_str("foo:12.3");
    assert!(res.is_err(), "Should have an error");
    assert!(!res.is_ok(), "Should have an error");
}

#[test]
fn test_metric_from_str_timer() {
    let res = Metric::from_str("test:12|ms");
    assert!(!res.is_err(), "Should have no error");
    assert!(res.is_ok(), "Should have no error");
    let metric = res.ok().unwrap();
    assert_eq!("test", metric.name);
    assert_eq!(12.0, metric.value);
    // assert_eq!(MetricKind::Timer, metric.kind);
}

#[test]
fn test_metric_from_str_gauge() {
}

#[test]
fn test_metric_from_str_histogram() {
}

#[test]
fn test_metric_from_str_counter() {
}
