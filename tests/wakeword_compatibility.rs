use rustpotter::{Rustpotter, RustpotterConfig, WakewordLoad, WakewordRef, WakewordSave};
use std::collections::HashMap;

#[test]
fn reference_wakeword_round_trip_loads_into_detector() {
    let reference = WakewordRef {
        name: "hey_overseer".into(),
        avg_features: None,
        samples_features: HashMap::from([("sample".into(), vec![vec![0.25f32; 13]; 100])]),
        threshold: Some(0.35),
        avg_threshold: Some(0.22),
        rms_level: 0.1,
        mfcc_size: 13,
    };
    let bytes = reference.save_to_buffer().unwrap();
    let loaded = WakewordRef::load_from_buffer(&bytes).unwrap();
    assert_eq!(loaded.samples_features, reference.samples_features);
    let mut detector = Rustpotter::new(&RustpotterConfig::default()).unwrap();
    detector
        .add_wakeword_from_buffer("hey_overseer", &bytes)
        .unwrap();
    let samples = vec![0i16; detector.get_samples_per_frame()];
    for _ in 0..120 {
        assert!(detector.process_samples(samples.clone()).is_none());
    }
    assert!(detector.remove_wakeword("hey_overseer"));
}
