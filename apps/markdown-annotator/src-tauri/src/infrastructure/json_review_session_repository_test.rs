use super::*;
use crate::domain::{
    document_identity::DocumentIdentity,
    review_session::{ReviewDecision, ReviewSession},
};
fn temp() -> PathBuf {
    let path =
        std::env::temp_dir().join(format!("ma-review-{}-{}", std::process::id(), now_nanos()));
    fs::create_dir_all(&path).unwrap();
    path
}
fn session() -> ReviewSession {
    ReviewSession {
        session_id: "session-1".into(),
        schema_version: 1,
        revision: 0,
        document: DocumentIdentity {
            root_id: "root".into(),
            relative_path: "a.md".into(),
            fingerprint: "f".repeat(64),
            byte_length: 1,
            modified_at_ms: None,
        },
        decision: ReviewDecision::Draft,
        annotations: vec![],
        created_at: "now".into(),
        updated_at: "now".into(),
    }
}
#[test]
fn saves_atomically_and_rejects_stale_revision() {
    let base = temp();
    let repo = JsonReviewSessionRepository::new(&base);
    let saved = repo.save(&session(), 0).unwrap();
    assert_eq!(saved.revision, 1);
    assert!(matches!(
        repo.save(&saved, 0),
        Err(ReviewRepositoryError::RevisionConflict { .. })
    ));
    assert_eq!(repo.load("session-1").unwrap().revision, 1);
    fs::remove_dir_all(base).unwrap();
}
#[test]
fn recovers_corrupt_current_from_latest_snapshot() {
    let base = temp();
    let repo = JsonReviewSessionRepository::new(&base);
    let first = repo.save(&session(), 0).unwrap();
    let second = repo.save(&first, 1).unwrap();
    assert_eq!(
        JsonReviewSessionRepository::read_envelope(
            &repo
                .snapshots("session-1")
                .join("00000000000000000001.json")
        )
        .unwrap()
        .payload
        .revision,
        1
    );
    fs::write(repo.path("session-1"), "broken").unwrap();
    assert_eq!(repo.load("session-1").unwrap().revision, 1);
    assert_eq!(second.revision, 2);
    fs::remove_dir_all(base).unwrap();
}
#[test]
fn refuses_unknown_future_schema() {
    let base = temp();
    let repo = JsonReviewSessionRepository::new(&base);
    fs::create_dir_all(repo.sessions()).unwrap();
    fs::write(
        repo.path("session-1"),
        r#"{"schemaVersion":999,"revision":1,"payload":{}}"#,
    )
    .unwrap();
    assert_eq!(
        repo.load("session-1").unwrap_err(),
        ReviewRepositoryError::UnsupportedSchema(999)
    );
    fs::remove_dir_all(base).unwrap();
}
#[test]
fn migrates_schema_zero_sequentially() {
    let base = temp();
    let repo = JsonReviewSessionRepository::new(&base);
    let saved = repo.save(&session(), 0).unwrap();
    let path = repo.path("session-1");
    let text = fs::read_to_string(&path)
        .unwrap()
        .replace("\"schemaVersion\": 1", "\"schemaVersion\": 0");
    fs::write(path, text).unwrap();
    assert_eq!(repo.load("session-1").unwrap().schema_version, 1);
    assert_eq!(saved.revision, 1);
    fs::remove_dir_all(base).unwrap();
}
#[test]
fn retains_only_five_snapshots() {
    let base = temp();
    let repo = JsonReviewSessionRepository::new(&base);
    let mut value = session();
    for expected in 0..8 {
        value = repo.save(&value, expected).unwrap();
    }
    let snapshots = fs::read_dir(repo.snapshots("session-1")).unwrap().count();
    assert_eq!(snapshots, 5);
    fs::remove_dir_all(base).unwrap();
}
#[test]
fn ignores_interrupted_unique_temp_file() {
    let base = temp();
    let repo = JsonReviewSessionRepository::new(&base);
    let saved = repo.save(&session(), 0).unwrap();
    fs::write(
        repo.sessions().join(".session-1.interrupted.tmp"),
        "partial",
    )
    .unwrap();
    assert_eq!(repo.load("session-1").unwrap().revision, saved.revision);
    fs::remove_dir_all(base).unwrap();
}
