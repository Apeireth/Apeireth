//! 链路执行层: 把 [`Effect`] 在后端端口上执行, 结果回灌为 [`Input`]。
//!
//! UI 线程零阻塞: 本函数在工作线程调用, 增量经 `sink` 流回主循环。
//! 测试也走同一函数 —— 命令链路的证据链与运行时完全一致。

use std::time::Instant;

use crate::backend::{CockpitBackend, TurnDelta, TurnOutcome};
use crate::state::{Effect, Input};

/// 执行一个效果, 所有结果 (含增量) 都交给 `sink`。
pub fn run(effect: Effect, backend: &mut dyn CockpitBackend, sink: &mut dyn FnMut(Input)) {
    match effect {
        Effect::SendTurn {
            tab,
            epoch,
            request,
        } => {
            let started = Instant::now();
            let mut deltas = |delta: TurnDelta| {
                sink(Input::TurnDelta { tab, epoch, delta });
            };
            match backend.send_turn(request, &mut deltas) {
                Ok(mut outcome) => {
                    outcome.latency_ms = started.elapsed().as_millis() as u64;
                    sink(Input::TurnFinished {
                        tab,
                        epoch,
                        outcome,
                    });
                }
                Err(error) => sink(Input::TurnFailed {
                    tab,
                    epoch,
                    message: error.to_string(),
                }),
            }
        }
        Effect::RefreshSessions => match backend.list_sessions() {
            Ok(sessions) => sink(Input::SessionsLoaded(sessions)),
            Err(error) => sink(Input::BackendFailed {
                what: "会话账本刷新".to_string(),
                message: error.to_string(),
            }),
        },
        Effect::ListModels => match backend.list_models() {
            Ok(models) => sink(Input::ModelsLoaded(models)),
            Err(error) => sink(Input::BackendFailed {
                what: "模型列表拉取".to_string(),
                message: error.to_string(),
            }),
        },
        Effect::SwitchModel { session, model } => {
            match backend.set_model(session.as_deref(), model.as_deref()) {
                Ok(applied) => sink(Input::ModelSwitched {
                    session,
                    model: applied,
                }),
                Err(error) => sink(Input::BackendFailed {
                    what: "模型热切换".to_string(),
                    message: error.to_string(),
                }),
            }
        }
        Effect::Compact { tab, session } => match backend.compact_session(&session) {
            Ok(report) => sink(Input::CompactDone {
                tab,
                before: report.before_messages,
                after: report.after_messages,
            }),
            Err(error) => sink(Input::CompactFailed {
                tab,
                message: error.to_string(),
            }),
        },
        Effect::Export {
            tab,
            path,
            markdown,
        } => match fs_err::write(&path, markdown) {
            Ok(()) => sink(Input::ExportDone { tab, path }),
            Err(error) => sink(Input::ExportFailed {
                tab,
                message: error.to_string(),
            }),
        },
        Effect::Reconnect => match backend.health() {
            Ok(()) => sink(Input::ConnectionOk),
            Err(error) => sink(Input::ConnectionFailed {
                detail: error.to_string(),
            }),
        },
    }
}
