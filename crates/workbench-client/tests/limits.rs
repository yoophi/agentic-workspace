use std::time::Duration;
use workbench_client::domain::limits::{LimitConfig, LimitError, Limits, Resource};

#[test]
fn reviewed_defaults_are_bounded() {
    let limits = Limits::default();
    assert_eq!(limits.maximum(Resource::Input), 1024 * 1024);
    assert_eq!(limits.maximum(Resource::Body), 8 * 1024 * 1024);
    assert_eq!(limits.maximum(Resource::RetryState), 256 * 1024 * 1024);
    assert_eq!(limits.maximum(Resource::Snapshot), 192 * 1024 * 1024);
    assert_eq!(limits.maximum(Resource::JsonlRecord), 256 * 1024 * 1024);
    assert_eq!(limits.maximum(Resource::Frame), 1024 * 1024);
    assert_eq!(limits.maximum(Resource::Message), 1024 * 1024);
    assert_eq!(limits.maximum(Resource::QueueBytes), 8 * 1024 * 1024);
    assert_eq!(limits.maximum(Resource::QueueItems), 256);
    assert_eq!(limits.config().connect_timeout, Duration::from_secs(5));
    assert_eq!(limits.config().request_timeout, Duration::from_secs(15));
}

#[test]
fn every_resource_accepts_exact_limit_and_rejects_one_more() {
    let limits = Limits::default();
    for resource in Resource::ALL {
        let maximum = limits.maximum(resource);
        assert_eq!(limits.check_add(resource, maximum - 1, 1), Ok(maximum));
        assert_eq!(
            limits.check_add(resource, maximum, 1),
            Err(LimitError::Exceeded(resource))
        );
        assert_eq!(
            limits.check_add(resource, usize::MAX, 1),
            Err(LimitError::Exceeded(resource))
        );
    }
}

#[test]
fn zero_resource_limits_are_rejected() {
    for resource in Resource::ALL {
        let mut config = LimitConfig::default();
        match resource {
            Resource::Input => config.input_bytes = 0,
            Resource::Body => config.body_bytes = 0,
            Resource::RetryState => config.retry_state_bytes = 0,
            Resource::Snapshot => config.snapshot_bytes = 0,
            Resource::JsonlRecord => config.jsonl_record_bytes = 0,
            Resource::Frame => config.frame_bytes = 0,
            Resource::Message => config.message_bytes = 0,
            Resource::QueueBytes => config.queue_bytes = 0,
            Resource::QueueItems => config.queue_items = 0,
        }
        assert_eq!(Limits::new(config).unwrap_err(), LimitError::Zero(resource));
    }
}

#[test]
fn zero_or_unrepresentable_deadlines_are_rejected() {
    for duration in [Duration::ZERO, Duration::from_secs(u64::MAX)] {
        assert!(Limits::new(LimitConfig {
            connect_timeout: duration,
            ..Default::default()
        })
        .is_err());
        assert!(Limits::new(LimitConfig {
            request_timeout: duration,
            ..Default::default()
        })
        .is_err());
    }
}

#[test]
fn sub_millisecond_deadline_is_not_silently_rounded_to_zero() {
    assert!(Limits::new(LimitConfig {
        request_timeout: Duration::from_nanos(1),
        ..Default::default()
    })
    .is_err());
}

#[test]
fn recovery_budget_is_finite_and_backoff_ordered() {
    assert!(Limits::new(LimitConfig {
        recovery_attempts: 0,
        ..Default::default()
    })
    .is_err());
    assert!(Limits::new(LimitConfig {
        backoff_min: Duration::ZERO,
        ..Default::default()
    })
    .is_err());
    assert!(Limits::new(LimitConfig {
        backoff_min: Duration::from_secs(11),
        ..Default::default()
    })
    .is_err());
    assert!(Limits::new(LimitConfig {
        backoff_max: Duration::from_secs(u64::MAX),
        ..Default::default()
    })
    .is_err());
}

#[test]
fn customized_limits_are_used_without_silent_truncation() {
    let limits = Limits::new(LimitConfig {
        input_bytes: 31,
        ..Default::default()
    })
    .unwrap();
    assert_eq!(limits.check_add(Resource::Input, 0, 31), Ok(31));
    assert_eq!(
        limits.check_add(Resource::Input, 0, 32),
        Err(LimitError::Exceeded(Resource::Input))
    );
}

#[test]
fn retry_state_budget_reserves_input_response_and_metadata_without_overflow() {
    let mut config = LimitConfig::default();
    let minimum = config.input_bytes + 24 * config.body_bytes + 128 * 1024;
    config.retry_state_bytes = minimum - 1;
    assert_eq!(
        Limits::new(config.clone()).unwrap_err(),
        LimitError::RetryStateBudget
    );
    config.retry_state_bytes = minimum;
    assert!(Limits::new(config).is_ok());
    assert_eq!(
        Limits::new(LimitConfig {
            input_bytes: usize::MAX,
            body_bytes: usize::MAX,
            retry_state_bytes: usize::MAX,
            ..Default::default()
        })
        .unwrap_err(),
        LimitError::RetryStateBudget
    );
}

#[test]
fn finite_json_numbers_fit_conservative_normalization_bound() {
    for exponent in -324..=308 {
        for sign in [-1.0, 1.0] {
            for mantissa in [1.0, 1.2345678901234567, 9.999999999999998] {
                let number = sign * mantissa * 10f64.powi(exponent);
                if number.is_finite() {
                    assert!(serde_json::to_vec(&number).unwrap().len() <= 24);
                }
            }
        }
    }
    for number in [i64::MIN as i128, i64::MAX as i128, u64::MAX as i128] {
        assert!(serde_json::to_vec(&number).unwrap().len() <= 24);
    }
}

#[test]
fn normalized_snapshot_and_jsonl_reserves_are_independent_of_wire_body() {
    let mut config = LimitConfig::default();
    config.snapshot_bytes = config.body_bytes * 24 - 1;
    assert_eq!(
        Limits::new(config.clone()).unwrap_err(),
        LimitError::SnapshotBudget
    );
    config.snapshot_bytes += 1;
    let minimum =
        config.snapshot_bytes.max(config.queue_bytes) + 2 * config.input_bytes + 128 * 1024;
    config.jsonl_record_bytes = minimum - 1;
    assert_eq!(
        Limits::new(config.clone()).unwrap_err(),
        LimitError::JsonlBudget
    );
    config.jsonl_record_bytes = minimum;
    let limits = Limits::new(config).unwrap();
    assert_eq!(limits.maximum(Resource::Body), 8 * 1024 * 1024);
}
#[test]
fn large_cursor_metadata_reserves_maximum_sequence_width() {
    use workbench_protocol::workbench::StreamCursor;
    let limits = Limits::default();
    let mut cursor = StreamCursor {
        stream_id: "orchestration:".into(),
        epoch: "e".into(),
        after_sequence: u64::MAX,
    };
    let overhead = serde_json::to_vec(&cursor).unwrap().len();
    cursor
        .stream_id
        .push_str(&"x".repeat(limits.maximum(Resource::Input) - overhead));
    assert!(limits.check_cursor(&cursor).is_ok());
    cursor.after_sequence = 0;
    assert!(limits.check_cursor(&cursor).is_ok());
    cursor.epoch.push('x');
    assert_eq!(
        limits.check_cursor(&cursor),
        Err(LimitError::Exceeded(Resource::Input))
    );
}
