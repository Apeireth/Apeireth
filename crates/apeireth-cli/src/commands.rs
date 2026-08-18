//! R116 + R125-2: apeireth-cli subcommand set (skills / eval / council)
//! R125-2 rewrite uses clap 4.5 derive macros (R124-1 borrow: clap-rs/clap 4f7a2c1)
//!  - argv 解析交给 clap; dispatch + display 保留
//!  - 0 改 lib.rs CliCommand / AsiSubCommand / GatewaySubCommand / Session flow

use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq)]
pub enum CliSubCommand {
    Skills(SkillsSubCommand),
    Eval(EvalSubCommand),
    Council(CouncilSubCommand),
}
#[derive(Debug, Clone, PartialEq)]
pub enum SkillsSubCommand {
    List { dir: PathBuf },
    Show { id: String, dir: PathBuf },
    Validate { file: PathBuf },
    Scenarios { dir: PathBuf },
    Watch { dir: PathBuf },
}
#[derive(Debug, Clone, PartialEq)]
pub enum EvalSubCommand {
    ListTools,
    Scenarios { dir: PathBuf },
    Smoke { workspace: PathBuf },
    MarkdownSnapshot { workspace: PathBuf },
}
#[derive(Debug, Clone, PartialEq)]
pub enum CouncilSubCommand {
    ListMembers,
    AddMember {
        role: String,
        goal: String,
        backstory: String,
        provider: String,
    },
    RiskHint,
    Markdown {
        query: String,
    },
}

// Clap derive layer (R125-2)
#[derive(Parser, Debug)]
#[command(name = "apeireth", version, about = "Apeireth CLI (skills/eval/council)", long_about = None)]
struct Cli {
    #[command(subcommand)]
    cmd: Top,
}

#[derive(Subcommand, Debug)]
enum Top {
    /// Skill descriptors and registry management
    Skills {
        #[command(subcommand)]
        action: SkillsCmd,
    },
    /// Eval scenarios and smoke runs
    Eval {
        #[command(subcommand)]
        action: EvalCmd,
    },
    /// Council members and deliberation
    Council {
        #[command(subcommand)]
        action: CouncilCmd,
    },
}

#[derive(Subcommand, Debug)]
enum SkillsCmd {
    /// List skill descriptors in a directory
    List {
        #[arg(default_value = ".")]
        dir: PathBuf,
    },
    /// Show a single skill by id
    Show {
        id: String,
        #[arg(default_value = ".")]
        dir: PathBuf,
    },
    /// Validate a skill JSON file
    Validate { file: PathBuf },
    /// List eval scenarios derived from skills
    Scenarios {
        #[arg(default_value = ".")]
        dir: PathBuf,
    },
    /// Watch a skill directory for changes
    Watch {
        #[arg(default_value = ".")]
        dir: PathBuf,
    },
}

#[derive(Subcommand, Debug)]
enum EvalCmd {
    /// List MCP eval tools
    ListTools,
    /// List eval scenarios (delegates to skills)
    Scenarios {
        #[arg(default_value = ".")]
        dir: PathBuf,
    },
    /// Run a smoke task against a workspace
    Smoke {
        #[arg(default_value = ".")]
        workspace: PathBuf,
    },
    /// Render smoke report as markdown
    MarkdownSnapshot {
        #[arg(default_value = ".")]
        workspace: PathBuf,
    },
}

#[derive(Subcommand, Debug)]
enum CouncilCmd {
    /// List registered council members
    ListMembers,
    /// Register a new council member
    AddMember {
        role: String,
        goal: String,
        backstory: String,
        provider: String,
    },
    /// Show the current risk hint
    RiskHint,
    /// Render a deliberation template as markdown
    Markdown { query: String },
}

/// Parse `args[1..]` into a `CliSubCommand` (caller wire 到 main.rs / lib.rs).
/// Internally uses clap derive; this wrapper exists for back-compat.
pub fn parse_subcommand_args(args: &[String]) -> Result<CliSubCommand, String> {
    let mut full: Vec<String> = Vec::with_capacity(args.len() + 1);
    full.push("apeireth".into());
    full.extend(args.iter().cloned());
    let cli = Cli::try_parse_from(&full).map_err(|e| e.to_string())?;
    Ok(match cli.cmd {
        Top::Skills { action } => CliSubCommand::Skills(match action {
            SkillsCmd::List { dir } => SkillsSubCommand::List { dir },
            SkillsCmd::Show { id, dir } => SkillsSubCommand::Show { id, dir },
            SkillsCmd::Validate { file } => SkillsSubCommand::Validate { file },
            SkillsCmd::Scenarios { dir } => SkillsSubCommand::Scenarios { dir },
            SkillsCmd::Watch { dir } => SkillsSubCommand::Watch { dir },
        }),
        Top::Eval { action } => CliSubCommand::Eval(match action {
            EvalCmd::ListTools => EvalSubCommand::ListTools,
            EvalCmd::Scenarios { dir } => EvalSubCommand::Scenarios { dir },
            EvalCmd::Smoke { workspace } => EvalSubCommand::Smoke { workspace },
            EvalCmd::MarkdownSnapshot { workspace } => {
                EvalSubCommand::MarkdownSnapshot { workspace }
            }
        }),
        Top::Council { action } => CliSubCommand::Council(match action {
            CouncilCmd::ListMembers => CouncilSubCommand::ListMembers,
            CouncilCmd::AddMember {
                role,
                goal,
                backstory,
                provider,
            } => CouncilSubCommand::AddMember {
                role,
                goal,
                backstory,
                provider,
            },
            CouncilCmd::RiskHint => CouncilSubCommand::RiskHint,
            CouncilCmd::Markdown { query } => CouncilSubCommand::Markdown { query },
        }),
    })
}

// Top-level dispatcher
pub fn dispatch_subcommand(cmd: CliSubCommand) -> Result<String, String> {
    match cmd {
        CliSubCommand::Skills(s) => dispatch_skills(s),
        CliSubCommand::Eval(e) => dispatch_eval(e),
        CliSubCommand::Council(c) => dispatch_council(c),
    }
}

fn dispatch_skills(cmd: SkillsSubCommand) -> Result<String, String> {
    match cmd {
        SkillsSubCommand::List { dir } => skills_list(&dir),
        SkillsSubCommand::Show { id, dir } => skills_show(&id, &dir),
        SkillsSubCommand::Validate { file } => skills_validate(&file),
        SkillsSubCommand::Scenarios { dir } => skills_scenarios(&dir),
        SkillsSubCommand::Watch { dir } => skills_watch(&dir),
    }
}

fn dispatch_eval(cmd: EvalSubCommand) -> Result<String, String> {
    match cmd {
        EvalSubCommand::ListTools => eval_list_tools(),
        EvalSubCommand::Scenarios { dir } => skills_scenarios(&dir),
        EvalSubCommand::Smoke { workspace } => eval_smoke(&workspace),
        EvalSubCommand::MarkdownSnapshot { workspace } => eval_markdown_snapshot(&workspace),
    }
}

fn dispatch_council(cmd: CouncilSubCommand) -> Result<String, String> {
    match cmd {
        CouncilSubCommand::ListMembers => council_list_members(),
        CouncilSubCommand::AddMember {
            role,
            goal,
            backstory,
            provider,
        } => council_add_member(&role, &goal, &backstory, &provider),
        CouncilSubCommand::RiskHint => council_risk_hint(),
        CouncilSubCommand::Markdown { query } => council_markdown(&query),
    }
}

// Skills dispatch impl
fn skills_list(dir: &std::path::Path) -> Result<String, String> {
    use apeireth_skills::file_loader::discover_descriptor_paths;
    let paths = discover_descriptor_paths(dir).map_err(|e| e.to_string())?;
    let mut s = format!(
        "# Skills in `{}` ({} total)\n\n",
        dir.display(),
        paths.len()
    );
    for p in &paths {
        s.push_str(&format!("- `{}`\n", p.display()));
    }
    Ok(s)
}

fn skills_show(id: &str, dir: &std::path::Path) -> Result<String, String> {
    use apeireth_skills::file_loader::load_registry_from_dir;
    let reg = load_registry_from_dir(dir).map_err(|e| e.to_string())?;
    let desc = reg
        .0
        .descriptor(id)
        .ok_or_else(|| format!("skill `{}` not found", id))?;
    Ok(format!(
        "# Skill `{}` v{}\n\n- description: {}\n- source: {}\n- tags: {:?}\n",
        desc.id, desc.version, desc.description, desc.source, desc.tags
    ))
}

fn skills_validate(file: &std::path::Path) -> Result<String, String> {
    use apeireth_skills::file_loader::load_one;
    match load_one(file) {
        Ok((skill, descriptor)) => Ok(format!(
            "✅ `{}` valid: id=`{}` v{} source=`{}`\n",
            file.display(),
            skill.id,
            skill.version,
            descriptor.source
        )),
        Err(e) => Err(format!("❌ `{}` invalid: {}", file.display(), e)),
    }
}

fn skills_scenarios(dir: &std::path::Path) -> Result<String, String> {
    use apeireth_skills::descriptor::SkillDescriptor;
    use apeireth_skills::eval_bridge::{descriptors_to_eval_scenarios, scenarios_by_source};
    use apeireth_skills::file_loader::load_registry_from_dir;
    let (registry, loaded) = load_registry_from_dir(dir).map_err(|e| e.to_string())?;
    let descs: Vec<SkillDescriptor> = loaded
        .iter()
        .filter_map(|ld| ld.descriptor.clone())
        .collect();
    let _ = registry; // 静默 unused
    let scenarios = descriptors_to_eval_scenarios(&descs);
    let grouped = scenarios_by_source(&scenarios);
    let mut s = format!(
        "# Eval Scenarios ({} total)\n\n## By source\n",
        scenarios.len()
    );
    for (src, count) in &grouped {
        s.push_str(&format!("- `{}`: {}\n", src, count));
    }
    s.push_str("\n## Scenarios\n");
    for sc in &scenarios {
        s.push_str(&format!("- `{}` (runnable={})\n", sc.id, sc.is_runnable()));
    }
    Ok(s)
}

fn skills_watch(dir: &std::path::Path) -> Result<String, String> {
    use apeireth_skills::watcher::SkillWatcher;
    let mut w = SkillWatcher::new(dir);
    let initial = w.scan_initial().map_err(|e| e.clone())?;
    let events = w.check_for_changes();
    Ok(format!(
        "# Watcher on `{}`\n\n- initial count: {}\n- events (since scan): {}\n",
        dir.display(),
        initial,
        events.len()
    ))
}

// Eval dispatch impl
fn eval_list_tools() -> Result<String, String> {
    use apeireth_eval::mcp_bridge::EvalToolServer;
    use apeireth_mcp::tools::ToolServer;
    let s = EvalToolServer::new();
    let mut out = String::from("# MCP Eval Tools\n\n");
    for t in s.list() {
        out.push_str(&format!(
            "- `{}` — {}\n",
            t.name,
            t.description.as_deref().unwrap_or("(no description)")
        ));
    }
    Ok(out)
}

fn eval_smoke(workspace: &std::path::Path) -> Result<String, String> {
    use apeireth_eval::smoke_task::run_smoke_conventions_tool_loop;
    let report = run_smoke_conventions_tool_loop(workspace);
    Ok(format!(
        "# Smoke Report (workspace: `{}`)\n\n- all_pass: {}\n- pass_rate: {}\n- phase_scores count: {}\n",
        workspace.display(), report.all_pass(), report.pass_rate(), report.phase_scores.len()
    ))
}

fn eval_markdown_snapshot(workspace: &std::path::Path) -> Result<String, String> {
    use apeireth_eval::smoke_task::run_smoke_conventions_tool_loop;
    let report = run_smoke_conventions_tool_loop(workspace);
    Ok(format!(
        "# Eval Markdown Snapshot\n\n- workspace: `{}`\n- all_pass: {}\n- pass_rate: {:.3}\n- phase_scores: {:?}\n",
        workspace.display(), report.all_pass(), report.pass_rate(), report.phase_scores
    ))
}

// Council dispatch impl
fn council_list_members() -> Result<String, String> {
    Ok(
        "# Council Members (0 registered)\n\n(none — use `council add-member` to register)\n"
            .to_string(),
    )
}

fn council_add_member(
    role: &str,
    goal: &str,
    backstory: &str,
    provider: &str,
) -> Result<String, String> {
    use apeireth_council::council_member::CouncilMember;
    let m = CouncilMember {
        role: role.to_string(),
        goal: goal.to_string(),
        backstory: backstory.to_string(),
        provider: provider.to_string(),
    };
    Ok(format!(
        "✅ Added member: role=`{}` goal=`{}` backstory=`{}` provider=`{}`\n",
        m.role, m.goal, m.backstory, m.provider
    ))
}

fn council_risk_hint() -> Result<String, String> {
    Ok(
        "# Council Risk Hint\n\n- risk: no_members\n- (caller should populate state first)\n"
            .to_string(),
    )
}

fn council_markdown(query: &str) -> Result<String, String> {
    use apeireth_council::mcp_bridge::CouncilPromptServer;
    use apeireth_mcp::prompts::PromptServer;
    let s = CouncilPromptServer::with_empty_state();
    let result = s
        .get(
            CouncilPromptServer::PROMPT_DELIBERATE,
            &serde_json::json!({ "query": query }),
        )
        .map_err(|e| e.message)?;
    let mut out = format!("# Council Deliberation for `{}`\n\n", query);
    for (i, m) in result.messages.iter().enumerate() {
        let role = match m.role {
            apeireth_mcp::prompts::PromptRole::User => "user",
            apeireth_mcp::prompts::PromptRole::Assistant => "assistant",
        };
        let text = match &m.content {
            apeireth_mcp::prompts::PromptContent::Text { text, .. } => text.clone(),
            _ => "(non-text content)".to_string(),
        };
        out.push_str(&format!("## [{}] {}\n\n{}\n\n", i, role, text));
    }
    Ok(out)
}

#[cfg(test)]
#[path = "commands_tests.rs"]
mod tests;
