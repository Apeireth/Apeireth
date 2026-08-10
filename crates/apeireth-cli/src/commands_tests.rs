// R125-2: tests moved out of commands.rs (sibling file referenced via `mod tests; #[path = ...]`)
//  - 19 unit tests: 8 clap parser + 11 dispatch
//  - all use super::* to access private items in commands.rs

use super::*;
use std::fs;

fn make_temp_dir_with_skill() -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "apeireth-cli-test-{}",
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()
    ));
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("test-skill.json"), r#"{
        "id": "test-skill", "version": "1.0.0", "description": "Test skill",
        "tags": ["test"], "source": "local", "input_example": "{}", "output_example": "{}"
    }"#).unwrap();
    dir
}

// clap parser tests
#[test] fn clap_skills_list() {
    assert!(matches!(
        parse_subcommand_args(&["skills".into(), "list".into(), ".".into()]).unwrap(),
        CliSubCommand::Skills(SkillsSubCommand::List { .. })));
}
#[test] fn clap_skills_show() {
    if let CliSubCommand::Skills(SkillsSubCommand::Show { id, .. }) =
        parse_subcommand_args(&["skills".into(), "show".into(), "x".into(), ".".into()]).unwrap() {
        assert_eq!(id, "x");
    } else { panic!("expected Show"); }
}
#[test] fn clap_skills_validate() {
    assert!(matches!(
        parse_subcommand_args(&["skills".into(), "validate".into(), "x.json".into()]).unwrap(),
        CliSubCommand::Skills(SkillsSubCommand::Validate { .. })));
}
#[test] fn clap_eval_list_tools() {
    assert!(matches!(
        parse_subcommand_args(&["eval".into(), "list-tools".into()]).unwrap(),
        CliSubCommand::Eval(EvalSubCommand::ListTools)));
}
#[test] fn clap_council_markdown() {
    if let CliSubCommand::Council(CouncilSubCommand::Markdown { query }) =
        parse_subcommand_args(&["council".into(), "markdown".into(), "q1".into()]).unwrap() {
        assert_eq!(query, "q1");
    } else { panic!("expected Markdown"); }
}
#[test] fn clap_unknown_top_errors() { assert!(parse_subcommand_args(&["foo".into()]).is_err()); }
#[test] fn clap_empty_errors() { assert!(parse_subcommand_args(&[]).is_err()); }
#[test] fn clap_unknown_sub_errors() { assert!(parse_subcommand_args(&["skills".into(), "foo".into()]).is_err()); }

// dispatch tests
#[test] fn dispatch_skills_list_with_data() {
    let dir = make_temp_dir_with_skill();
    let r = dispatch_subcommand(CliSubCommand::Skills(SkillsSubCommand::List { dir: dir.clone() })).unwrap();
    assert!(r.contains("Skills in") && r.contains("test-skill"));
    fs::remove_dir_all(&dir).ok();
}
#[test] fn dispatch_skills_validate_valid_file() {
    let dir = make_temp_dir_with_skill();
    let r = dispatch_subcommand(CliSubCommand::Skills(SkillsSubCommand::Validate { file: dir.join("test-skill.json") })).unwrap();
    assert!(r.contains("valid"));
    fs::remove_dir_all(&dir).ok();
}
#[test] fn dispatch_skills_validate_invalid_file() {
    assert!(dispatch_subcommand(CliSubCommand::Skills(SkillsSubCommand::Validate {
        file: PathBuf::from("/nonexistent/file.json") })).is_err());
}
#[test] fn dispatch_skills_scenarios_with_data() {
    let dir = make_temp_dir_with_skill();
    let r = dispatch_subcommand(CliSubCommand::Skills(SkillsSubCommand::Scenarios { dir: dir.clone() })).unwrap();
    assert!(r.contains("Eval Scenarios"));
    fs::remove_dir_all(&dir).ok();
}
#[test] fn dispatch_skills_watch() {
    let dir = make_temp_dir_with_skill();
    let r = dispatch_subcommand(CliSubCommand::Skills(SkillsSubCommand::Watch { dir: dir.clone() })).unwrap();
    assert!(r.contains("Watcher on") && r.contains("initial count"));
    fs::remove_dir_all(&dir).ok();
}
#[test] fn dispatch_eval_list_tools() {
    let r = dispatch_subcommand(CliSubCommand::Eval(EvalSubCommand::ListTools)).unwrap();
    assert!(r.contains("MCP Eval Tools") && r.contains("eval_smoke"));
}
#[test] fn dispatch_eval_scenarios_delegates() {
    let dir = make_temp_dir_with_skill();
    let r = dispatch_subcommand(CliSubCommand::Eval(EvalSubCommand::Scenarios { dir: dir.clone() })).unwrap();
    assert!(r.contains("Eval Scenarios"));
    fs::remove_dir_all(&dir).ok();
}
#[test] fn dispatch_council_list_members() {
    let r = dispatch_subcommand(CliSubCommand::Council(CouncilSubCommand::ListMembers)).unwrap();
    assert!(r.contains("Council Members"));
}
#[test] fn dispatch_council_add_member() {
    let r = dispatch_subcommand(CliSubCommand::Council(CouncilSubCommand::AddMember {
        role: "architect".into(), goal: "design".into(),
        backstory: "10y".into(), provider: "claude".into() })).unwrap();
    assert!(r.contains("Added member") && r.contains("architect"));
}
#[test] fn dispatch_council_risk_hint() {
    let r = dispatch_subcommand(CliSubCommand::Council(CouncilSubCommand::RiskHint)).unwrap();
    assert!(r.contains("no_members"));
}
#[test] fn dispatch_council_markdown() {
    let r = dispatch_subcommand(CliSubCommand::Council(CouncilSubCommand::Markdown {
        query: "test query".into() })).unwrap();
    assert!(r.contains("Council Deliberation for `test query`"));
}
