use super::BodyCollector;
use curl::easy::Handler;

#[test]
fn body_collector_no_limit_counts_bytes() {
    let mut collector = BodyCollector::default();
    collector.reset(0);
    let data = vec![0u8; 8];
    let wrote = collector.write(&data).expect("write");
    assert_eq!(wrote, data.len());
    assert_eq!(collector.bytes, 8);
    assert!(!collector.limit_reached);
}

#[test]
fn body_collector_caps_bytes_when_limit_hit() {
    let mut collector = BodyCollector::default();
    collector.reset(5);
    let data = vec![0u8; 10];
    let wrote = collector.write(&data).expect("write");
    assert_eq!(wrote, data.len());
    assert_eq!(collector.bytes, 5);
    assert!(collector.limit_reached);
}

#[test]
fn body_collector_caps_bytes_after_partial() {
    let mut collector = BodyCollector::default();
    collector.reset(5);
    let first = vec![0u8; 3];
    let wrote_first = collector.write(&first).expect("write");
    assert_eq!(wrote_first, 3);
    assert_eq!(collector.bytes, 3);
    assert!(!collector.limit_reached);

    let second = vec![0u8; 4];
    let wrote_second = collector.write(&second).expect("write");
    assert_eq!(wrote_second, second.len());
    assert_eq!(collector.bytes, 5);
    assert!(collector.limit_reached);
}

#[test]
fn body_collector_progress_aborts_after_limit() {
    let mut collector = BodyCollector::default();
    collector.reset(5);
    let data = vec![0u8; 5];
    let _ = collector.write(&data).expect("write");
    assert!(collector.limit_reached);
    assert!(!collector.progress(0.0, 5.0, 0.0, 0.0));
}

#[test]
fn body_collector_progress_allows_below_limit() {
    let mut collector = BodyCollector::default();
    collector.reset(5);
    assert!(collector.progress(0.0, 2.0, 0.0, 0.0));
}
