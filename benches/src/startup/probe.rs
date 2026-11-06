//! Child-side readiness and active-session projection, shared by all four modes.
//! No provider plugin is configured, matching the historical assembly workload.

use std::io::{self, BufRead, Write};
use std::path::Path;
use std::time::Instant;

use apeireth_core::kernel::{SessionId, Timestamp, VirtualClock};
use apeireth_protocol::canonical::NormalizedMessage;
use apeireth_runtime::{Runtime, Session};

use crate::assembly::{assemble_production_profiled, tokio_runtime};

const SESSION: &str = "a1b00000-0000-4000-8000-000000000001";

fn session_id() -> SessionId {
    SESSION.parse().expect("static session id")
}

fn expected_messages() -> Vec<NormalizedMessage> {
    vec![
        NormalizedMessage::user("startup fixture question"),
        NormalizedMessage::assistant("startup fixture answer"),
    ]
}

async fn seed(runtime: &Runtime) -> io::Result<()> {
    let clock = VirtualClock::new(
        Timestamp::from_epoch_millis(1_700_000_000_000)
            .expect("fixed fixture timestamp")
            .as_datetime(),
    );
    let mut session = Session::new(session_id(), &clock);
    for message in expected_messages() {
        session.append(message, &clock);
    }
    runtime
        .sessions()
        .save(&session)
        .await
        .map_err(io::Error::other)
}

async fn resume(runtime: &Runtime) -> io::Result<()> {
    let session = runtime
        .sessions()
        .load(&session_id())
        .await
        .map_err(io::Error::other)?
        .ok_or_else(|| {
            io::Error::other("existing fixture session missing; refusing to create it")
        })?;
    // This is the persisted active conversation surface, not merely a cached
    // Runtime pointer. Validate before acknowledging readiness in every mode.
    if session.messages != expected_messages() || session.log.len() != 2 || session.revision != 2 {
        return Err(io::Error::other("fixture content changed"));
    }
    let view = session.surface_view();
    if view.messages() != expected_messages() {
        return Err(io::Error::other("resumed surface differs from fixture"));
    }
    std::hint::black_box(view);
    Ok(())
}

fn emit(line: &str) -> io::Result<()> {
    let mut out = io::stdout().lock();
    writeln!(out, "{line}")?;
    out.flush()
}

pub fn run(args: &[String]) -> io::Result<()> {
    let [mode, dir] = args else {
        return Err(io::Error::other("expected mode and data directory"));
    };
    let dir = Path::new(dir);
    let first = mode == "first";
    if !first && mode != "existing" {
        return Err(io::Error::other("invalid child mode"));
    }
    for name in ["memory.sqlite3", "session.sqlite3"] {
        if dir.join(name).exists() == first {
            return Err(io::Error::other(
                "first requires absent DBs; existing requires both DBs",
            ));
        }
    }
    let tick = Instant::now();
    let rt = tokio_runtime();
    let tokio_ns = tick.elapsed().as_nanos();
    let tick = Instant::now();
    let (mut runtime, seg) = rt.block_on(assemble_production_profiled(dir));
    let assembly_ns = tick.elapsed().as_nanos();
    let state_init_ns = if first {
        let tick = Instant::now();
        rt.block_on(seed(&runtime))?;
        tick.elapsed().as_nanos()
    } else {
        0
    };
    let tick = Instant::now();
    rt.block_on(resume(&runtime))?;
    let resume_ns = tick.elapsed().as_nanos();
    emit(&format!(
        "READY|{tokio_ns}|{assembly_ns}|{:.0}|{:.0}|{:.0}|{:.0}|{state_init_ns}|{resume_ns}",
        seg.memory_open_ns, seg.session_open_ns, seg.modules_build_ns, seg.runtime_build_ns
    ))?;
    for line in io::stdin().lock().lines() {
        match line?.as_str() {
            "RESUME" => {
                let tick = Instant::now();
                rt.block_on(resume(&runtime))?;
                emit(&format!("RESUMED|{}", tick.elapsed().as_nanos()))?;
            }
            "STOP" => {
                let failures = rt.block_on(runtime.shutdown());
                if !failures.is_empty() {
                    return Err(io::Error::other(format!("shutdown: {failures:?}")));
                }
                drop(runtime);
                drop(rt);
                return Ok(());
            }
            _ => return Err(io::Error::other("invalid startup probe command")),
        }
    }
    Err(io::Error::other(
        "parent disconnected before explicit shutdown",
    ))
}
