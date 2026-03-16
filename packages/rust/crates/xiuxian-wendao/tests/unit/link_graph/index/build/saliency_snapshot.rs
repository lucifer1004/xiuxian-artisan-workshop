use super::*;

#[test]
fn test_empty_snapshot() {
    let snapshot = SaliencySnapshot::empty();
    assert_eq!(snapshot.known_count(), 0);
    assert_eq!(snapshot.high_saliency_count(), 0);
    assert_eq!(snapshot.saliency_of("unknown"), 0.0);
    assert!(!snapshot.is_high_saliency("unknown"));
}

#[test]
fn test_average_saliency_empty() {
    let snapshot = SaliencySnapshot::empty();
    assert_eq!(snapshot.average_saliency(), 0.0);
}

#[test]
fn test_top_n_empty() {
    let snapshot = SaliencySnapshot::empty();
    let top = snapshot.top_n(5);
    assert!(top.is_empty());
}

#[test]
fn test_saliency_threshold_constant() {
    assert!((SALIENCY_THRESHOLD_HIGH - 0.70).abs() < f64::EPSILON);
}

#[test]
fn test_min_activation_constant() {
    assert_eq!(MIN_ACTIVATION_COUNT, 3);
}
