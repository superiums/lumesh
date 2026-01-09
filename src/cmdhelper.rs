use common_macros::hash_set;
use lazy_static::lazy_static;
use std::collections::HashSet;

#[cfg(unix)]
use std::env;
#[cfg(unix)]
use std::ffi::OsStr;
#[cfg(unix)]
use std::path::Path;
#[cfg(unix)]
use std::path::PathBuf;
use std::path::is_separator;

use crate::expression::alias::get_alias_tips;
use crate::modules::get_builtin_tips;

lazy_static! {
    pub static ref CMDS: HashSet<String> = get_cmds();
    pub static ref PATH_COMMANDS: HashSet<String> = scan_cmds();
}

fn get_cmds() -> HashSet<String> {
    let mut cmds: HashSet<String> = hash_set! {
        "cd ./".into(),
        "ls -l --color ./".into(),
        "clear".into(),
        "rm ".into(),
        "cp -r".into(),
        "let ".into(),
        "fn ".into(),
        "if ".into(),
        "else {".into(),
        "match ".into(),
        "while (".into(),
        "for i in ".into(),
        "loop {\n".into(),
        "break".into(),
        "return".into(),
        "history".into(),
        "del ".into(),
        "use ".into(),
    };
    cmds.extend(get_builtin_tips());
    // cmds.extend(scan_cmds());
    cmds.extend(get_alias_tips());
    cmds
}

pub fn is_valid_command(cmd: &str) -> bool {
    PATH_COMMANDS.contains(cmd)
}
pub fn collect_command_with_prefix(prefix: &str) -> Vec<&String> {
    let c1 = CMDS
        .iter()
        .filter(|x| x.starts_with(prefix))
        .collect::<Vec<_>>();
    if c1.is_empty() {
        return PATH_COMMANDS
            .iter()
            .filter(|x| x.starts_with(prefix))
            .collect::<Vec<_>>();
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
fn scan_cmds() -> HashSet<String> {
    let path_var = env::var("PATH").unwrap_or_default();
    let path_separator = if cfg!(windows) { ";" } else { ":" };

    path_var
        .split(path_separator)
        .flat_map(|dir| {
            let dir_path = PathBuf::from(dir);
            scan_directory(&dir_path)
        })
        .collect()
}
#[cfg(windows)]
fn scan_cmds() -> HashSet<String> {
    HashSet::new()
}
// 目录扫描函数（支持递归扩展）
#[cfg(unix)]
fn scan_directory(dir: &Path) -> Vec<String> {
    let mut commands = Vec::new();
    if let Ok(entries) = dir.read_dir() {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                commands.extend(scan_directory(&path));
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
// pub fn should_trigger_cmd_completion(line: &str, pos: usize) -> bool {
//     // 条件优先级排序

//     is_first_token(line, pos) || // 主命令位置
//         // is_after_special_cmd(line), // 特殊命令后
//         is_pipe_context(line, pos) // 管道符后
//     // is_env_expansion(ctx)        // 环境变量展开
// }

// 1. 主命令位置检测
// fn is_first_token(line: &str, pos: usize) -> bool {
//     // 分词算法参考 bash 的单词拆分规则
//     let tokens: Vec<&str> = line.split_whitespace().collect();
//     tokens.is_empty() || pos <= tokens[0].len()
// }
// 2. 特殊命令后参数匹配
// fn is_after_special_cmd(line: &str) -> bool {
//     let tokens = split_command_line(line);
//     tokens.iter().enumerate().any(|(i, token)| {
//         // 检测前序存在特殊命令且当前是最后一个token
//         SPECIAL_CMDS.contains(token) && (i == tokens.len().saturating_sub(2))
//     })
// }
// 3. 管道符上下文处理
// fn is_pipe_context(line: &str, pos: usize) -> bool {
//     let x = &line[..pos];
//     if let Some(pipe_pos) = x.rfind('|') {
//         // Check if there are non-whitespace characters after the pipe
//         let after_pipe = &x[pipe_pos + 1..];
//         !after_pipe.trim().is_empty() // Ensure there's something after the pipe
//     } else {
//         false // No pipe found
//     }
// }

// 当检测到 file 命令时，自动过滤非文本文件：

// fn scan_special_completions(ctx: &Context) -> Vec<String> {
//     match ctx.last_command {
//         "file" | "cat" | "bat" => file_system::list_files()
//             .filter(|f| is_text_file(f))
//             .collect(),
//         "which" | "exec" => PATH_CMDS.clone(),
//         _ => Vec::new(),
//     }
// }

// fn generate_hints(ctx: &Context) -> Vec<String> {
//     let mut candidates = Vec::new();

//     if is_first_token(ctx) {
//         candidates.extend(PATH_COMMANDS.iter().cloned());
//     }

//     if is_after_special_cmd(ctx) {
//         candidates.extend(scan_special_completions(ctx));
//     }

//     candidates.sort_by(|a, b| {
//         // b.weight.cmp(&a.weight) // 按权重降序
//         //   .then_with(||
//         a.text.len().cmp(&b.text.len())
//         // ) // 短命令优先
//     });

//     candidates
// }

// // AI client trait for abstraction
// pub trait AIClient {
//     fn complete(&self, prompt: &str) -> Result<String, String>;
//     fn chat(&self, prompt: &str) -> Result<String, String>;
// }

// // Mock implementation (replace with actual ollama/llama.cpp integration)
// struct MockAIClient;
// impl AIClient for MockAIClient {
//     fn complete(&self, prompt: &str) -> Result<String, String> {
//         Ok(format!("AI completion for: {}", prompt))
//     }

//     fn chat(&self, prompt: &str) -> Result<String, String> {
//         Ok(format!("AI response to: {}", prompt))
//     }
// }

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

// Shared function to extract command section after last separator
fn find_command_pos(prefix: &str) -> usize {
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
            || !matches!(
                after_space.chars().next(),
                Some('|' | '&' | ')' | ';' | '\n')
            )
    } else {
        false
    }
}

fn should_trigger_ai(prefix: &str) -> bool {
    // Trigger AI completion with double space
    prefix.starts_with("  ") && !prefix.trim().is_empty()
}
