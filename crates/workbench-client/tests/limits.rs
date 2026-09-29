use std::time::Duration;
use workbench_client::domain::limits::{LimitConfig, LimitError, Limits, Resource};

#[test]
fn reviewed_defaults_are_bounded() {
    let limits = Limits::default();
    assert_eq!(limits.maximum(Resource::Input), 1024 * 1024);
    assert_eq!(limits.maximum(Resource::Body), 8 * 1024 * 1024);
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
