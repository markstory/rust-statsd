/// Internal metric representation
///
use std::fmt;
use std::cmp::Eq;

// use std::from_str::FromStr;
// use std::cmp;

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


/// Metric value objects.
///
pub struct Metric {
    kind: MetricKind,
    name: String,
    value: f64
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
