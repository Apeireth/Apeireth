//! VCP FileOperator commands compatibility layer.

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum VcpError {
    #[error("unknown command: `{0}`")]
    UnknownCommand(String),
    #[error("missing required field: `{0}`")]
    MissingField(&'static str),
    #[error("io error: `{0}`")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum VcpCommand {
    ReadFile,
    WebReadFile,
    FileInfo,
    WriteFile,
    WriteEscapedFile,
    AppendFile,
    EditFile,
    ApplyDiff,
    ListDirectory,
    CreateDirectory,
    ListAllowedDirectories,
    CopyFile,
    MoveFile,
    RenameFile,
    DeleteFile,
    SearchFiles,
    DownloadFile,
    CreateCanvas,
    Unknown,
}

pub const VCP_COMMAND_COUNT: usize = 18;

impl VcpCommand {
    pub fn from_str(s: &str) -> Self {
        match s {
            "ReadFile" => Self::ReadFile,
            "WebReadFile" => Self::WebReadFile,
            "FileInfo" => Self::FileInfo,
            "WriteFile" => Self::WriteFile,
            "WriteEscapedFile" => Self::WriteEscapedFile,
            "AppendFile" => Self::AppendFile,
            "EditFile" => Self::EditFile,
            "ApplyDiff" => Self::ApplyDiff,
            "ListDirectory" => Self::ListDirectory,
            "CreateDirectory" => Self::CreateDirectory,
            "ListAllowedDirectories" => Self::ListAllowedDirectories,
            "CopyFile" => Self::CopyFile,
            "MoveFile" => Self::MoveFile,
            "RenameFile" => Self::RenameFile,
            "DeleteFile" => Self::DeleteFile,
            "SearchFiles" => Self::SearchFiles,
            "DownloadFile" => Self::DownloadFile,
            "CreateCanvas" => Self::CreateCanvas,
            _ => Self::Unknown,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VcpManifest {
    pub name: String,
    pub display_name: Option<String>,
    pub description: Option<String>,
    pub commands: Vec<String>,
}

impl VcpManifest {
    pub fn parse(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
    pub fn supported_commands(&self) -> Vec<VcpCommand> {
        self.commands.iter().map(|s| VcpCommand::from_str(s)).filter(|c| *c != VcpCommand::Unknown).collect()
    }
}

pub struct VcpCompatRouter;

impl VcpCompatRouter {
    pub fn new() -> Self { Self }
    pub fn command_count() -> usize { VCP_COMMAND_COUNT }
}

impl Default for VcpCompatRouter {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parse_all_18_commands() {
        let cmds = vec![
            "ReadFile","WebReadFile","FileInfo","WriteFile","WriteEscapedFile",
            "AppendFile","EditFile","ApplyDiff","ListDirectory","CreateDirectory",
            "ListAllowedDirectories","CopyFile","MoveFile","RenameFile","DeleteFile",
            "SearchFiles","DownloadFile","CreateCanvas",
        ];
        for s in cmds {
            assert_ne!(VcpCommand::from_str(s), VcpCommand::Unknown, "command `{s}` not parsed");
        }
        assert_eq!(VCP_COMMAND_COUNT, 18);
    }

    #[test]
    fn parse_manifest() {
        let json = r#"{"name":"FileOperator","commands":["ReadFile","WriteFile"]}"#;
        let m = VcpManifest::parse(json).unwrap();
        assert_eq!(m.commands.len(), 2);
        assert_eq!(m.supported_commands().len(), 2);
    }
}
