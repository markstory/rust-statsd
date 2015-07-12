/// Internal metric representation
///
use std::fmt;
use std::str::FromStr;


/// Enum of metric types
pub enum MetricKind {
    Counter(f64), // sample rate
    Gauge,
    Timer,
}

impl fmt::Debug for MetricKind {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            MetricKind::Gauge      => write!(f, "Gauge"),
            MetricKind::Timer      => write!(f, "Timer"),
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

impl Metric {
    fn new(name: &str, value: f64, kind: MetricKind) -> Metric {
        Metric{name: name.to_string(), value: value, kind: kind}
    }
}

impl FromStr for Metric {
    type Err = ParseError;

    /// Valid message formats are:
    ///
    /// - `<str:metric_name>:<f64:value>|<str:type>`
    /// - `<str:metric_name>:<f64:value>|c|@<f64:sample_rate>`
    fn from_str(line: &str) -> Result<Metric, ParseError> {
        // Get the metric name
        let name_parts: Vec<&str> = line.split(':').collect();
        if name_parts.len() < 2 || name_parts[0].is_empty() {
            return Err(ParseError::SyntaxError(
                    "Metrics require a name.",
                    0))
        }
        let name = name_parts[0].to_string();

        // Get the float val
        let val_parts: Vec<&str> = name_parts[1].split('|').collect();
        if val_parts.len() < 2 || val_parts[0].is_empty() {
            return Err(ParseError::SyntaxError(
                    "Metrics require a value.",
                    name.len()))
        }
        let value = val_parts[0].parse::<f64>().ok().unwrap();

        // Get kind parts
        let kind = match val_parts[1] {
            "ms" => MetricKind::Timer,
            "g" => MetricKind::Gauge,
            "c" => {
                let mut rate:f64 = 1.0;
                if val_parts.len() == 3 {
                    rate = val_parts[2].trim_left_matches('@')
                        .parse::<f64>().ok().unwrap();
                }
                MetricKind::Counter(rate)
            }
            _ => return Err(ParseError::SyntaxError(
                    "Unknown metric type.",
                    2))
        };

        Ok(Metric{name: name, value: value, kind: kind})
    }
}



//
// Tests
//
#[cfg(test)]
mod test {
    use metric::{Metric,MetricKind};
    use std::str::FromStr;
    use std::collections::HashMap;

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
    fn test_metric_valid() {
        let mut valid = HashMap::new();
        valid.insert(
            "foo.test:12.3|ms",
            Metric::new("foo.test", 12.3, MetricKind::Timer)
        );
        valid.insert(
            "test:18.123|g",
            Metric::new("test", 18.123, MetricKind::Gauge)
        );
        valid.insert(
            "test:18.123|g",
            Metric::new("test", 18.123, MetricKind::Gauge)
        );
        valid.insert(
            "thing.total:12|c",
            Metric::new("thing.total", 12.0, MetricKind::Counter(1.0))
        );
        valid.insert(
            "thing.total:5.6|c|@123",
            Metric::new("thing.total", 5.6, MetricKind::Counter(123.0))
        );

        for (input, expected) in valid.iter() {
            let result = Metric::from_str(*input);
            assert!(result.is_ok());

            let actual = result.ok().unwrap();
            assert_eq!(expected.name, actual.name);
            assert_eq!(expected.value, actual.value);

            // TODO this is stupid, there must be a better way.
            assert_eq!(
                format!("{:?}", expected.kind),
                format!("{:?}", actual.kind)
            );
        }
    }

    #[test]
    fn test_metric_invalid() {
        let invalid = vec![
            "",
            "metric",
            "metric|12",
            "metric:13|",
            "metric:14|c@1",
            ":|@",
            ":1.0|c"
        ];
        for input in invalid.iter() {
            let result = Metric::from_str(*input);
            assert!(result.is_err());
        }
    }
}
