use common_macros::hash_set;
use std::collections::HashSet;

use crate::libs::get_lib_completions;

#[cfg(unix)]
use std::ffi::OsStr;

#[cfg(unix)]
use std::path::Path;
use std::path::PathBuf;
use std::path::is_separator;
use std::sync::OnceLock;

static PATH_COMMANDS: OnceLock<HashSet<String>> = OnceLock::new();
static LM_CMDS: OnceLock<HashSet<&'static str>> = OnceLock::new();

fn get_path_commands() -> &'static HashSet<String> {
    PATH_COMMANDS.get_or_init(|| init_path_cmds())
}
fn get_lm_commands() -> &'static HashSet<&'static str> {
    LM_CMDS.get_or_init(|| init_lm_cmds())
}

fn init_lm_cmds() -> HashSet<&'static str> {
    let cmds: HashSet<&'static str> = hash_set! {
        // "cd ./".into(),
        // "ls -l --color ./".into(),
        // "clear".into(),
        // "rm ".into(),
        // "cp -r",
        "let ",
        "fn ",
        "if ",
        "else {",
        "match ",
        "while (",
        "for i in ",
        "loop {\n",
        "break",
        "return",
        "history",
        "del ",
        "use ",
    };
    // cmds.extend(get_builtin_tips());
    // cmds.extend(scan_cmds());
    // cmds.extend(get_alias_tips());
    cmds
}

pub fn is_valid_command(cmd: &str) -> bool {
    // PATH_COMMANDS.get().is_some_and(|m| m.contains(cmd))
    get_path_commands().contains(cmd)
}
pub fn collect_command_with_prefix(prefix: &str) -> Vec<&str> {
    if prefix.is_empty() || !prefix.is_ascii() {
        return Vec::new();
    }
    let c1 = get_lm_commands()
        .iter()
        .filter(|x| x.starts_with(prefix))
        .map(|x| *x)
        .collect::<Vec<_>>();
    if c1.is_empty() {
        match get_lib_completions(prefix) {
            Some(lib) => return lib,
            _ => {
                return get_path_commands()
                    .iter()
                    .filter(|x| x.starts_with(prefix))
                    .map(|x| x.as_ref())
                    .collect::<Vec<_>>();
            }
        }
    }
    c1
}
// 平台相关代码参考自
#[cfg(unix)]
fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    path.metadata()
        .map(|m| m.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

// #[cfg(windows)]
// fn is_executable(path: &Path) -> bool {
//     path.extension().map_or(false, |ext| {
//         ["exe", "bat", "cmd", "ps1"].contains(&ext.to_str().unwrap())
//     })
// }
#[cfg(unix)]
fn init_path_cmds() -> HashSet<String> {
    let path_var = std::env::var("PATH").unwrap_or_default();
    let path_separator = if cfg!(windows) { ";" } else { ":" };

    path_var
        .split(path_separator)
        .flat_map(|dir| {
            let dir_path = PathBuf::from(dir);
            scan_path_cmds(&dir_path)
        })
        .collect()
}
#[cfg(windows)]
fn init_path_cmds() -> HashSet<String> {
    HashSet::new()
}
// 目录扫描函数（支持递归扩展）
#[cfg(unix)]
fn scan_path_cmds(dir: &Path) -> Vec<String> {
    let mut commands = Vec::new();
    if let Ok(entries) = dir.read_dir() {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                commands.extend(scan_path_cmds(&path));
            } else if is_executable(&path) {
                if let Some(stem) = path.file_stem().and_then(OsStr::to_str) {
                    commands.push(stem.to_string());
                }
            }
        }
    }
    commands
}

// ----补全
//  line: &str,
// pos: usize,
pub fn should_trigger_path_completion(line: &str, pos: usize) -> bool {
    // most efficiency if not allow space in path
    let last = line[..pos]
        .rfind(|c: char| c.is_ascii_whitespace() || is_separator(c))
        .unwrap_or(0);
    is_separator(line.chars().nth(last).unwrap_or_default())
}

// 扩展补全类型枚举
#[derive(Debug, PartialEq)]
pub enum LumeCompletionType {
    Path,
    Command,
    Param,
    AI,
    None,
}

pub fn detect_completion_type(
    line: &str,
    pos: usize,
    ai_avaluable: bool,
) -> (LumeCompletionType, usize) {
    // Early exit for empty lines
    if line.is_empty() || pos == 0 {
        return (LumeCompletionType::None, pos);
    }

    let prefix = &line[..pos];

    // Check path completion first (highest priority)
    if should_trigger_path_completion(line, pos) {
        return (LumeCompletionType::Path, pos);
    }

    // Check AI completion with new trigger logic
    if ai_avaluable && should_trigger_ai(prefix) {
        return (LumeCompletionType::AI, pos);
    }

    // Extract command section once and reuse
    let command_pos = find_command_pos(prefix);
    let command_section = &prefix[command_pos..];

    // Check if we're typing a command (incomplete first word)
    if is_typing_command(command_section) {
        return (LumeCompletionType::Command, command_pos);
    }

    // Check if we're after a complete command word (parameter context)
    if is_after_command_word(command_section) {
        return (LumeCompletionType::Param, command_pos);
    }

    (LumeCompletionType::None, pos)
}

// const SEPARATORS: &[char] = &[';', '|', '(', '{', '`', '\n', '&', '>', '<'];
// Shared function to extract command section after last separator
pub fn find_command_pos(prefix: &str) -> usize {
    // Find the last command separator position
    let pos = prefix
        .rfind(|c: char| matches!(c, '|' | '&' | '(' | ';' | '\n'))
        .map(|i| i + 1)
        .unwrap_or(0);
    // After pipe, allow leading spaces before command
    prefix[pos..]
        .find(|x: char| !char::is_ascii_whitespace(&x))
        .map(|i| i + pos)
        .unwrap_or(0)
}

// Check if we're typing a command (incomplete first word)
fn is_typing_command(command_section: &str) -> bool {
    // Check if we're in the first word (no space yet)
    !command_section.contains(' ')
}

// Check if we're after a complete command word (has space after command)
fn is_after_command_word(command_section: &str) -> bool {
    // Check if we have a complete command word followed by space
    if let Some(space_pos) = command_section.find(' ') {
        // Ensure we're not at terminating symbols
        let after_space = &command_section[space_pos + 1..];
        after_space.is_empty()
            || !after_space.contains(|c: char| matches!(&c, '|' | '&' | ')' | ';' | '\n'))
    } else {
        false
    }
}

fn should_trigger_ai(prefix: &str) -> bool {
    // Trigger AI completion with double space
    prefix.starts_with("  ") && !prefix.trim().is_empty()
}
