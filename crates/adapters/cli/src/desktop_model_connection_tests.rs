use super::*;
use apeireth_credentials::keyring::{InMemoryKeyring, NoopAudit};

fn setup() -> (
    tempfile::TempDir,
    Arc<dyn KeyringBackend>,
    ManagedConnection,
) {
    let root = tempfile::tempdir().unwrap();
    let backend: Arc<dyn KeyringBackend> = Arc::new(InMemoryKeyring::new(Arc::new(NoopAudit)));
    let service = ManagedConnection::new(root.path().into(), backend.clone()).unwrap();
    (root, backend, service)
}

fn patch(url: &str, key: Option<&str>) -> Patch {
    Patch {
        base_url: url.into(),
        model: "model-a".into(),
        api_key: key.map(str::to_string),
    }
}

#[test]
fn no_profile_starts_missing_and_no_connectivity_claim() {
    let (_root, _backend, service) = setup();
    let view = service.read().unwrap();
    assert_eq!(view.credential_state, CredentialState::Missing);
    assert!(!view.restart_required);
    assert!(!view.endpoint_bound);
    assert_eq!(view.model, DEFAULT_MODEL);
    assert!(!serde_json::to_value(view)
        .unwrap()
        .as_object()
        .unwrap()
        .contains_key("connected"));
}

#[test]
fn key_and_binding_survive_reopen_without_plaintext_profile() {
    let (root, backend, service) = setup();
    let saved = service
        .save(patch("https://model.example/v1/", Some("fixture-secret-a")))
        .unwrap();
    assert_eq!(saved.credential_state, CredentialState::Available);
    assert!(saved.endpoint_bound && saved.restart_required);
    let bytes = fs_err::read_to_string(root.path().join(FILE)).unwrap();
    assert!(!bytes.contains("fixture-secret-a"));
    assert!(!serde_json::to_string(&saved)
        .unwrap()
        .contains("fixture-secret-a"));
    let reopened = ManagedConnection::new(root.path().into(), backend).unwrap();
    let view = reopened.read().unwrap();
    assert_eq!(view.base_url, "https://model.example/v1");
    assert_eq!(view.credential_state, CredentialState::Available);
    assert!(view.endpoint_bound);
    assert!(!view.restart_required);
}

#[test]
fn refuses_endpoint_change_without_new_key_and_preserves_exact_profile() {
    let (root, _backend, service) = setup();
    service
        .save(patch("https://model.example/v1", Some("fixture-secret-a")))
        .unwrap();
    let original = fs_err::read(root.path().join(FILE)).unwrap();
    assert!(matches!(
        service.save(patch("https://other.example/v1", None)),
        Err(Error::EndpointChangeNeedsKey)
    ));
    assert_eq!(original, fs_err::read(root.path().join(FILE)).unwrap());
    let changed = service
        .save(patch("https://other.example/v1", Some("fixture-secret-b")))
        .unwrap();
    assert!(changed.endpoint_bound && changed.restart_required);
}

#[test]
fn failed_profile_commit_does_not_publish_or_orphan_the_new_key() {
    let (root, backend, mut service) = setup();
    service
        .save(patch("https://model.example/v1", Some("fixture-secret-a")))
        .unwrap();
    let original = fs_err::read(root.path().join(FILE)).unwrap();
    let keys = backend.list().unwrap();
    service.persist = |_, _, _| Err(std::io::Error::other("fixture persistence unavailable"));
    assert!(matches!(
        service.save(patch("https://other.example/v1", Some("fixture-secret-b"))),
        Err(Error::Storage)
    ));
    assert_eq!(original, fs_err::read(root.path().join(FILE)).unwrap());
    assert_eq!(keys, backend.list().unwrap());
    assert_eq!(
        service.read().unwrap().credential_state,
        CredentialState::Available
    );
}

struct Unavailable;
impl KeyringBackend for Unavailable {
    fn get(&self, _: &str) -> Result<SecretBuf, KeyringError> {
        Err(KeyringError::BackendUnavailable(
            "fixture unavailable".into(),
        ))
    }
    fn set(&self, _: &str, _: &SecretBuf) -> Result<(), KeyringError> {
        Err(KeyringError::BackendUnavailable(
            "fixture unavailable".into(),
        ))
    }
    fn delete(&self, _: &str) -> Result<(), KeyringError> {
        Err(KeyringError::BackendUnavailable(
            "fixture unavailable".into(),
        ))
    }
    fn list(&self) -> Result<Vec<String>, KeyringError> {
        Err(KeyringError::BackendUnavailable(
            "fixture unavailable".into(),
        ))
    }
    fn backend_name(&self) -> &'static str {
        "unavailable-fixture"
    }
}

#[test]
fn missing_or_broken_platform_store_cannot_claim_saved_key() {
    let (root, _backend, service) = setup();
    service
        .save(patch("https://model.example/v1", Some("fixture-secret-a")))
        .unwrap();
    let broken = ManagedConnection::new(root.path().into(), Arc::new(Unavailable)).unwrap();
    assert_eq!(
        broken.read().unwrap().credential_state,
        CredentialState::Unavailable
    );
    assert!(matches!(
        broken.save(patch("https://model.example/v1", Some("fixture-secret-b"))),
        Err(Error::CredentialUnavailable)
    ));
}

#[test]
fn profile_namespace_is_stable_and_different_roots_do_not_share_credentials() {
    let root_a = tempfile::tempdir().unwrap();
    let root_b = tempfile::tempdir().unwrap();
    assert_eq!(
        namespace(root_a.path()).unwrap(),
        namespace(&root_a.path().join(".")).unwrap()
    );
    assert_ne!(
        namespace(root_a.path()).unwrap(),
        namespace(root_b.path()).unwrap()
    );
}

#[test]
fn invalid_or_multiuser_profile_fails_closed() {
    let (root, backend, service) = setup();
    assert!(matches!(
        service.save(patch("http://remote.example/v1", Some("fixture-secret-a"))),
        Err(Error::Invalid)
    ));
    fs_err::write(
        root.path().join(FILE),
        r#"{"schema_version":1,"api_key":"fixture-secret-in-malformed-profile"}"#,
    )
    .unwrap();
    assert!(matches!(
        ManagedConnection::new(root.path().into(), backend),
        Err(Error::Storage)
    ));
    fs_err::write(root.path().join("users.json"), "{}").unwrap();
    assert!(matches!(service.read(), Err(Error::Scope)));
}

#[test]
fn rotation_retires_old_slot_only_after_profile_commit() {
    let (_root, backend, service) = setup();
    service
        .save(patch("https://model.example/v1", Some("fixture-secret-a")))
        .unwrap();
    let original = backend.list().unwrap();
    service
        .save(patch("https://model.example/v1", Some("fixture-secret-b")))
        .unwrap();
    let rotated = backend.list().unwrap();
    assert_eq!(rotated.len(), 1);
    assert_ne!(original, rotated);
}
