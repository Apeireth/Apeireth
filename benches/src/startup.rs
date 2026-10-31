//! Opt-in startup definitions. The historical `cold-start` / `all` gate is unchanged.
//! Process samples stop at the ready receipt, then synchronously shut down and reap
//! the child before the next launch. No live Runtime or connection pool is reused.

pub mod probe;

use std::collections::BTreeMap;
use std::io::{self, BufRead, BufReader, Write};
use std::path::Path;
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use crate::support::{cleanup_dir, temp_dir, BenchConfig, Stats};

pub const NAMES: &[&str] = &["first_run", "normal_start", "cold_start", "warm_resume"];
pub const PROBE_ARG: &str = "__startup-probe";
const PHASES: &[&str] = &[
    "tokio",
    "assembly",
    "memory_open",
    "session_open",
    "modules_build",
    "runtime_build",
    "state_init",
    "resume",
];

struct Probe {
    child: Child,
    input: ChildStdin,
    output: BufReader<ChildStdout>,
}

impl Probe {
    fn spawn(mode: &str, dir: &Path) -> io::Result<Self> {
        let mut child = Command::new(std::env::current_exe()?)
            .args([PROBE_ARG, mode])
            .arg(dir)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()?;
        Ok(Self {
            input: child.stdin.take().expect("piped stdin"),
            output: BufReader::new(child.stdout.take().expect("piped stdout")),
            child,
        })
    }

    fn read(&mut self, prefix: &str, count: usize) -> io::Result<Vec<f64>> {
        let mut line = String::new();
        self.output.read_line(&mut line)?;
        let fields: Vec<_> = line.trim().split('|').collect();
        if fields.first() != Some(&prefix) || fields.len() != count + 1 {
            return Err(io::Error::other(format!(
                "invalid {prefix} receipt: {line:?}"
            )));
        }
        fields[1..]
            .iter()
            .map(|value| {
                let value: f64 = value.parse().map_err(io::Error::other)?;
                if !value.is_finite() || value < 0.0 {
                    return Err(io::Error::other("invalid timing"));
                }
                Ok(value)
            })
            .collect()
    }

    fn command(&mut self, command: &str) -> io::Result<()> {
        writeln!(self.input, "{command}")?;
        self.input.flush()
    }

    fn close(mut self) -> io::Result<()> {
        self.command("STOP")?;
        if !self.child.wait()?.success() {
            return Err(io::Error::other("startup child shutdown failed"));
        }
        Ok(())
    }
}

impl Drop for Probe {
    fn drop(&mut self) {
        // Error paths also reap the process, never silently reusing it next time.
        if self.child.try_wait().ok().flatten().is_none() {
            let _ = self.child.kill();
        }
        let _ = self.child.wait();
    }
}

/// No advisory eviction or allocation substitute: every cold sample needs a real
/// kernel cache drop after sync, immediately before launching the measured child.
fn drop_caches() -> io::Result<()> {
    #[cfg(target_os = "linux")]
    {
        let mut control = std::fs::OpenOptions::new()
            .write(true)
            .open("/proc/sys/vm/drop_caches")?;
        if !Command::new("sync").status()?.success() {
            return Err(io::Error::other("sync failed"));
        }
        control.write_all(b"3\n")?;
        control.flush()
    }
    #[cfg(not(target_os = "linux"))]
    {
        Err(io::Error::new(
            io::ErrorKind::Unsupported,
            "real cache drop unavailable on this platform",
        ))
    }
}

type Measurements = BTreeMap<String, Vec<f64>>;

fn push(values: &mut Measurements, phase: &str, ns: f64) {
    values
        .entry(phase.into())
        .or_default()
        .push(ns / 1_000_000.0);
}

fn record_init(values: &mut Measurements, receipt: &[f64]) {
    for (phase, value) in PHASES.iter().zip(receipt) {
        push(values, phase, *value);
    }
}

fn report(mode: &str, values: &Measurements, cfg: &BenchConfig) {
    let profile = if cfg!(debug_assertions) {
        "Dev"
    } else {
        "Release"
    };
    for (phase, samples) in values {
        let stats = Stats::from_values(samples);
        let mut sorted = samples.clone();
        sorted.sort_by(f64::total_cmp);
        let p95 = sorted[(0.95 * sorted.len() as f64).ceil() as usize - 1];
        println!(
            "STARTUP|{mode}|{profile}|{phase}|{}|{:.6}|{p95:.6}|{:.6}|{:.6}|{:.6}",
            stats.n, stats.p50, stats.min, stats.mean, stats.max
        );
        if cfg.raw {
            println!(
                "STARTUP_RAW|{mode}|{profile}|{phase}|{}",
                samples
                    .iter()
                    .map(|v| format!("{v:.6}"))
                    .collect::<Vec<_>>()
                    .join(",")
            );
        }
    }
}

pub fn run(mode: &str, cfg: &BenchConfig) -> io::Result<()> {
    assert!(NAMES.contains(&mode));
    let profile = if cfg!(debug_assertions) {
        "Dev"
    } else {
        "Release"
    };
    println!("STARTUP_META|{mode}|{profile}|os={}|arch={}|unix_s={}|samples={}|warmup={}|fixture=session-pair-v1|formal_gate=legacy-cold-start-unchanged|targets=PROPOSED_UNAPPROVED",
        std::env::consts::OS, std::env::consts::ARCH,
        SystemTime::now().duration_since(UNIX_EPOCH).expect("system clock").as_secs(),
        cfg.samples(30), cfg.samples(2));
    println!("STARTUP_COLUMNS|mode|profile|phase|n|p50_ms|p95_ms|min_ms|mean_ms|max_ms");
    if mode == "cold_start" {
        // Fail before generating any purported cold timing when permission is absent.
        if let Err(error) = drop_caches() {
            println!("STARTUP_UNAVAILABLE|cold_start|n=0|{error}");
            return Err(error);
        }
    }
    let dir = temp_dir("startup-modes");
    let result = measure(mode, cfg, &dir);
    cleanup_dir(&dir);
    result
}

fn measure(mode: &str, cfg: &BenchConfig, dir: &Path) -> io::Result<()> {
    let warmup = cfg.samples(2);
    let mut values = Measurements::new();
    if mode != "first_run" {
        let mut seed = Probe::spawn("first", dir)?;
        seed.read("READY", PHASES.len())?;
        let pid = seed.child.id();
        seed.close()?;
        println!("STARTUP_SEED_CLOSED|{mode}|pid={pid}");
    }
    if mode == "warm_resume" {
        let mut child = Probe::spawn("existing", dir)?;
        let setup = child.read("READY", PHASES.len())?;
        let mut setup_values = Measurements::new();
        record_init(&mut setup_values, &setup);
        // Setup is explicitly separate, with n=1; it is not a resume sample.
        report("warm_resume_setup", &setup_values, cfg);
        for i in 0..warmup + cfg.samples(30) {
            let tick = Instant::now();
            child.command("RESUME")?;
            let receipt = child.read("RESUMED", 1)?;
            let total = tick.elapsed().as_nanos() as f64;
            if i >= warmup {
                push(&mut values, "total", total);
                push(&mut values, "resume", receipt[0]);
                if cfg.raw {
                    println!(
                        "STARTUP_SAMPLE|{mode}|{}|pid={}|total_ns={total:.0}",
                        i - warmup,
                        child.child.id()
                    );
                }
            }
        }
        child.close()?;
    } else {
        for i in 0..warmup + cfg.samples(30) {
            let fresh = if mode == "first_run" {
                Some(temp_dir("startup-first"))
            } else {
                None
            };
            let sample_dir = fresh.as_deref().unwrap_or(dir);
            if mode == "cold_start" {
                drop_caches().inspect_err(|error| {
                    println!(
                        "STARTUP_UNAVAILABLE|cold_start|completed={}|{error}",
                        i.saturating_sub(warmup)
                    );
                })?;
            }
            let tick = Instant::now();
            let mut child = Probe::spawn(
                if fresh.is_some() { "first" } else { "existing" },
                sample_dir,
            )?;
            let receipt = child.read("READY", PHASES.len())?;
            let total = tick.elapsed().as_nanos() as f64;
            if i >= warmup {
                push(&mut values, "total", total);
                record_init(&mut values, &receipt);
                // assembly already contains the parallel storage critical path.
                let inside = receipt[0] + receipt[1] + receipt[6] + receipt[7];
                if inside > total {
                    return Err(io::Error::other("child phases exceed parent wall time"));
                }
                push(&mut values, "process_and_ipc", total - inside);
                if cfg.raw {
                    println!(
                        "STARTUP_SAMPLE|{mode}|{}|pid={}|total_ns={total:.0}",
                        i - warmup,
                        child.child.id()
                    );
                }
            }
            child.close()?;
            if let Some(fresh) = fresh {
                cleanup_dir(&fresh);
            }
        }
    }
    report(mode, &values, cfg);
    Ok(())
}
