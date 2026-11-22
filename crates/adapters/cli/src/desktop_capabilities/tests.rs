use super::*;
use apeireth_core::kernel::CapabilityId;
use apeireth_plugin::ToolCapability;
use apeireth_protocol::canonical::{NormalizedTool, ToolCall, ToolResult};
use async_trait::async_trait;

struct Included(CapabilityId);

#[async_trait]
impl ToolCapability for Included {
    fn id(&self) -> &CapabilityId {
        &self.0
    }

    fn declaration(&self) -> NormalizedTool {
        NormalizedTool::new(self.0.as_str().replace('.', "_"))
    }

    async fn invoke(&self, call: &ToolCall) -> ToolResult {
        ToolResult::ok(&call.id, serde_json::Value::Null)
    }
}

async fn assembled(settings: DesktopCapabilitySettings) -> Runtime {
    let mut builder = Runtime::builder();
    for (enabled, id) in [
        (settings.project_file_tools, "tool.project_files_read"),
        (settings.project_file_tools, "tool.project_files_write"),
        (
            settings.preference_learning,
            "tool.behavior.cognitive.preference_learning",
        ),
        (
            settings.skill_search,
            apeireth_tools_canonical::SKILL_SEARCH_CAPABILITY_ID,
        ),
    ] {
        if enabled {
            builder = builder.with_capability(Arc::new(Included(CapabilityId::new(id).unwrap())));
        }
    }
    builder.build().await.unwrap()
}

#[tokio::test]
async fn save_readback_and_restart_report_independent_registry_facts() {
    let root = tempfile::tempdir().unwrap();
    let profile = Prepared::load(root.path()).unwrap();
    let defaults = profile.settings();
    assert!(defaults.project_file_tools && defaults.preference_learning);
    assert!(!defaults.skill_search);
    let live = profile.attach(&assembled(defaults).await);
    assert_eq!(live.read().unwrap().effective, defaults);

    let desired = DesktopCapabilitySettings {
        project_file_tools: false,
        preference_learning: false,
        skill_search: true,
    };
    let saved = live
        .save(DesktopCapabilityPatch {
            expected_revision: 0,
            configured: desired,
        })
        .unwrap();
    assert_eq!(saved.revision, 1);
    assert_eq!(saved.configured, desired);
    assert_eq!(saved.effective, defaults);
    assert!(saved.restart_required);
    assert_eq!(saved, live.read().unwrap());
    assert_eq!(
        live.save(DesktopCapabilityPatch {
            expected_revision: 0,
            configured: defaults
        }),
        Err(Error::Conflict),
    );
    assert_eq!(
        saved,
        live.save(DesktopCapabilityPatch {
            expected_revision: 1,
            configured: desired
        })
        .unwrap()
    );

    let reopened = Prepared::load(root.path()).unwrap();
    assert_eq!(reopened.settings(), desired);
    let reopened = reopened.attach(&assembled(desired).await);
    let readback = reopened.read().unwrap();
    assert_eq!(readback.revision, 1);
    assert_eq!(readback.configured, desired);
    assert_eq!(readback.effective, desired);
    assert!(!readback.restart_required);

    // A builder whose project read tool is registered alone does not claim
    // that the complete tool family is installed.
    let partial = Runtime::builder()
        .with_capability(Arc::new(Included(
            CapabilityId::new("tool.project_files_read").unwrap(),
        )))
        .build()
        .await
        .unwrap();
    let inspected = Prepared::load(root.path()).unwrap().attach(&partial);
    let readback = inspected.read().unwrap();
    assert!(!readback.effective.project_file_tools && !readback.effective.skill_search);
    assert!(readback.restart_required);
}

#[test]
fn present_bad_profile_refuses_start_and_never_gets_defaulted_or_overwritten() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join(FILE);
    for bad in [
        r#"{"schema_version":2,"revision":0,"configured":{"project_file_tools":true,"preference_learning":true,"skill_search":false}}"#,
        r#"{"schema_version":1,"revision":0,"configured":{"skill_search":false}}"#,
        r#"{"schema_version":1,"revision":0,"configured":{"project_file_tools":true,"preference_learning":true,"skill_search":false,"shell":true}}"#,
    ] {
        fs_err::write(&path, bad).unwrap();
        assert!(Prepared::load(root.path()).is_err());
        let live = Service {
            path: path.clone(),
            effective: DesktopCapabilitySettings::default(),
        };
        assert_eq!(live.read(), Err(Error::Storage));
        assert_eq!(
            live.save(DesktopCapabilityPatch {
                expected_revision: 0,
                configured: DesktopCapabilitySettings::default(),
            }),
            Err(Error::Storage)
        );
        assert_eq!(fs_err::read_to_string(&path).unwrap(), bad);
    }
}

#[test]
fn competing_writers_commit_one_compare_and_swap() {
    let root = tempfile::tempdir().unwrap();
    let context = Service {
        path: root.path().join(FILE),
        effective: DesktopCapabilitySettings::default(),
    };
    let choices = [
        DesktopCapabilitySettings {
            preference_learning: false,
            ..context.effective
        },
        DesktopCapabilitySettings {
            skill_search: true,
            ..context.effective
        },
    ];
    let barrier = std::sync::Barrier::new(2);
    let (first, second) = std::thread::scope(|scope| {
        let launch = |configured| {
            let barrier = &barrier;
            let context = &context;
            scope.spawn(move || {
                barrier.wait();
                context.save(DesktopCapabilityPatch {
                    expected_revision: 0,
                    configured,
                })
            })
        };
        let first = launch(choices[0]);
        let second = launch(choices[1]);
        (first.join().unwrap(), second.join().unwrap())
    });
    let winning = match (first, second) {
        (Ok(first), Err(Error::Conflict)) => first,
        (Err(Error::Conflict), Ok(second)) => second,
        other => panic!("expected a single durable settings save: {other:?}"),
    };
    assert_eq!(winning.revision, 1);
    assert_eq!(context.read().unwrap(), winning);
}
