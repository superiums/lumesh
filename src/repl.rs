use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use crate::ai::{MockAIClient, init_ai};
use crate::cmdhelper::{
    LumeCompletionType, collect_command_with_prefix, detect_completion_type, find_command_pos,
    is_valid_command,
};
use crate::completion::ParamCompleter;
use crate::editor::{
    Cmd, Completer, CompletionItem, Editor, Highlighter, Hinter, KeyEvent, ReadlineError,
};
use crate::expression::alias::get_alias_completion;
use crate::libs::{LIBS_INFO, is_lib};
use crate::syntax::{get_ayu_dark_theme, get_dark_theme, get_light_theme, get_merged_theme};
use crate::{CFM_ENABLED, Expression, STRICT_ENABLED, childman};
use crate::{Environment, check, highlight, parse_and_eval, prompt::get_prompt_engine};

const DEFAULT: &str = "";
const GREEN_BOLD: &str = "\x1b[1;32m";
const RESET: &str = "\x1b[0m";

struct LumeCompleter {
    ai_client: Option<Arc<MockAIClient>>,
    param_completer: Arc<ParamCompleter>,
    theme: HashMap<String, String>,
}

impl Completer for LumeCompleter {
    fn complete(&self, line: &str, pos: usize) -> Vec<CompletionItem> {
        match detect_completion_type(line, pos, self.ai_client.is_some()) {
            (LumeCompletionType::Path, _) => {
                // Basic filename completion - return the line as-is (file completer is complex)
                Vec::new()
            }
            (LumeCompletionType::Command, section_pos) => {
                self.cmd_completion(line, pos, section_pos)
            }
            (LumeCompletionType::Param, section_pos) => {
                self.param_completion(line, pos, section_pos)
            }
            (LumeCompletionType::AI, section_pos) => self.ai_completion(line, section_pos),
            (LumeCompletionType::None, _) => Vec::new(),
        }
    }
}

impl LumeCompleter {
    fn cmd_completion(&self, line: &str, pos: usize, section_start: usize) -> Vec<CompletionItem> {
        let prefix = &line[section_start..pos];
        let cpl_color = self
            .theme
            .get("completion_cmd")
            .map_or(DEFAULT, |c| c.as_str());

        let mut items: Vec<CompletionItem> = collect_command_with_prefix(prefix)
            .iter()
            .map(|cmd| {
                let display = format!("{cpl_color}{cmd}{RESET}");
                CompletionItem::with_display(display, cmd.to_string())
            })
            .collect();

        for item in &mut items {
            if is_lib(&item.replacement) {
                item.suffix = '.';
            }
        }

        if items.is_empty() {
            match prefix.split_once(".") {
                Some((name, func)) => {
                    if !name.is_empty() {
                        LIBS_INFO.with(|h| {
                            if let Some(lib) = h.get(&name) {
                                items = lib
                                    .iter()
                                    .filter(|(f, _)| f.starts_with(func))
                                    .map(|(cmd, _)| {
                                        let mut ci = CompletionItem::with_display(
                                            format!("{cpl_color}{cmd}{RESET}"),
                                            cmd.to_string(),
                                        );
                                        ci.suffix = '(';
                                        ci
                                    })
                                    .collect();
                            }
                        });
                        items.sort_by(|a, b| a.display.len().cmp(&b.display.len()));
                        return items;
                    }
                }
                _ => {
                    items = get_alias_completion(prefix)
                        .into_iter()
                        .map(|cmd| {
                            CompletionItem::with_display(format!("{cpl_color}{cmd}{RESET}"), cmd)
                        })
                        .collect();
                }
            }
        }

        items.sort_by(|a, b| a.display.len().cmp(&b.display.len()));
        items
    }

    fn param_completion(
        &self,
        line: &str,
        pos: usize,
        section_start: usize,
    ) -> Vec<CompletionItem> {
        let cmd_section = &line[section_start..pos];
        let tokens: Vec<&str> = cmd_section.split_whitespace().collect();

        if let Some((command, tokens)) = tokens.split_first() {
            let param_start = cmd_section.rfind(' ').map(|x| x + 1);
            let current_token = param_start.map_or("", |x| &cmd_section[x..]);

            let params = if current_token.is_empty() {
                tokens
            } else {
                &tokens[..tokens.len().saturating_sub(1)]
            };

            let (pairs, trig_file) =
                self.param_completer
                    .get_completions_for_context(command, params, current_token);

            if trig_file {
                return Vec::new();
            }

            let mut items: Vec<CompletionItem> = pairs
                .into_iter()
                .map(|p| CompletionItem::with_display(p.display, p.replacement))
                .collect();
            items.sort_by(|a, b| a.replacement.cmp(&b.replacement));
            return items;
        }
        Vec::new()
    }

    fn ai_completion(&self, _line: &str, _pos: usize) -> Vec<CompletionItem> {
        // AI completion is not handled through the tab completion path currently
        Vec::new()
    }
}

struct LumeHighlighter {
    theme: HashMap<String, String>,
}

impl Highlighter for LumeHighlighter {
    fn highlight(&self, line: &str) -> String {
        if line.is_empty() {
            return String::new();
        }

        let (prefix, rest) = if let Some(cmd) = line.strip_prefix(':') {
            (":", cmd)
        } else if let Some(cmd) = line.strip_prefix('>') {
            (">", cmd)
        } else {
            ("", line)
        };

        let (cmd, rest_op) = match rest.split_once(' ') {
            Some((a, b)) => (a, Some(b)),
            _ => (rest, None),
        };

        let (color, is_valid) = if is_valid_command(cmd) {
            (
                self.theme
                    .get("command_valid")
                    .map_or(DEFAULT, |c| c.as_str()),
                true,
            )
        } else {
            (DEFAULT, false)
        };

        let pre_color = self.theme.get("mode").map_or(DEFAULT, |c| c.as_str());

        match rest_op {
            None if is_valid => format!("{pre_color}{prefix}{color}{cmd}{RESET}"),
            None => {
                let highlighted_line = highlight(rest, &self.theme);
                format!("{pre_color}{prefix}{RESET}{highlighted_line}")
            }
            Some(rest) if is_valid => {
                let highlighted_rest = highlight(rest, &self.theme);
                format!("{pre_color}{prefix}{color}{cmd}{RESET} {highlighted_rest}")
            }
            Some(_) => {
                let highlighted_line = highlight(rest, &self.theme);
                format!("{pre_color}{prefix}{RESET}{highlighted_line}")
            }
        }
    }

    fn highlight_char(&self, _line: &str, _pos: usize) -> bool {
        false
    }
}

struct LumeHinter {
    hinter: Option<Box<dyn Fn(&str, usize) -> Option<String>>>,
}

impl Hinter for LumeHinter {
    fn hint(&self, line: &str, pos: usize) -> Option<String> {
        if let Some(ref f) = self.hinter {
            f(line, pos)
        } else {
            None
        }
    }
}

pub fn run_repl(env: &mut Environment) {
    // 安装全局 SIGINT 处理器：仅设置标志，不杀死 lume
    childman::install_sigint_handler();

    match env.get("LUME_WELCOME") {
        Some(wel) => {
            println!("{wel}");
            env.undefine("LUME_WELCOME");
        }
        _ => println!("Welcome to Lumesh {}", env!("CARGO_PKG_VERSION")),
    }

    if STRICT_ENABLED.with_borrow(|s| s == &true) {
        println!("\x1b[38;5;170m[Strict Mode]\x1b[0m")
    }
    if CFM_ENABLED.with_borrow(|c| c == &true) {
        println!("\x1b[38;5;141m[Cmd First Mode]\x1b[0m");
    }

    // history
    let no_history = match env.get("LUME_NO_HISTORY") {
        Some(Expression::Boolean(t)) => {
            env.undefine("LUME_NO_HISTORY");
            t
        }
        _ => false,
    };
    let history_file = match env.get("LUME_HISTORY_FILE") {
        Some(hf) => hf.to_string(),
        _ => {
            let c_dir = match dirs::cache_dir() {
                Some(c) => c,
                _ => PathBuf::new(),
            };
            #[cfg(unix)]
            let path = c_dir.join(".lume_history");
            #[cfg(windows)]
            let path = c_dir.join("lume_history.log");
            if !c_dir.exists() {
                if let Err(e) = std::fs::create_dir_all(&c_dir) {
                    eprintln!("Failed to create cache directory: {e}");
                }
            }
            if !path.exists() {
                if let Err(e) = std::fs::File::create(&path) {
                    eprintln!("Failed to create cache file: {e}");
                }
            }
            path.into_os_string().into_string().unwrap()
        }
    };

    // ai config
    let ai_config = env.get("LUME_AI_CONFIG");
    env.undefine("LUME_AI_CONFIG");

    // theme
    let theme_base = env.get("LUME_THEME");
    env.undefine("LUME_THEME");
    let theme = match theme_base {
        Some(Expression::String(t)) => match t.as_ref() {
            "light" => get_light_theme(),
            "ayu_dark" => get_ayu_dark_theme(),
            _ => get_dark_theme(),
        },
        _ => get_dark_theme(),
    };
    let theme_config = env.get("LUME_THEME_CONFIG");
    env.undefine("LUME_THEME_CONFIG");
    let theme_merged = match theme_config {
        Some(Expression::Map(m)) => get_merged_theme(theme, m.as_ref()),
        _ => theme,
    };

    // completion
    let completion_dir = match env.get("LUME_COMPLETION_DIR") {
        Some(Expression::String(c)) => c,
        _ => {
            if cfg!(target_os = "macos") {
                dirs::data_local_dir()
                    .unwrap_or_else(|| PathBuf::from("~/.local/share"))
                    .join("lumesh/completions")
                    .into_os_string()
                    .into_string()
                    .unwrap_or_else(|_| "~/.local/share/lumesh/completions".to_string())
            } else if cfg!(unix) {
                String::from("/usr/share/lumesh/completions")
            } else {
                String::from("C:\\Program Files\\lumesh\\completions")
            }
        }
    };
    env.undefine("LUME_COMPLETION_DIR");

    // key bindings
    let hotkey_sudo = match env.get("LUME_SUDO_CMD") {
        Some(s) => {
            env.undefine("LUME_SUDO_CMD");
            s.to_string()
        }
        _ => "sudo".to_string(),
    };

    let hotkey_modifier = env.get("LUME_HOT_MODIFIER");
    env.undefine("LUME_HOT_MODIFIER");
    let modifier: u8 = match hotkey_modifier {
        Some(Expression::Integer(bits)) => bits as u8,
        _ => 4,
    };

    let hotkey_config = env.get("LUME_HOT_KEYS");
    env.undefine("LUME_HOT_KEYS");

    let _abbr = env.get("LUME_ABBREVIATIONS");
    env.undefine("LUME_ABBREVIATIONS");

    // =======create editor=======
    let mut editor = Editor::new();

    // Set up completer
    let ai_client = ai_config.map(|ai_cfg| Arc::new(init_ai(ai_cfg)));
    let completer = LumeCompleter {
        ai_client,
        param_completer: Arc::new(ParamCompleter::new(completion_dir)),
        theme: theme_merged.clone(),
    };
    editor.set_completer(Box::new(completer));

    // Set up highlighter
    let hl_theme = theme_merged.clone();
    editor.set_highlighter(Box::new(LumeHighlighter { theme: hl_theme }));

    // Set up hinter (hint from command history)
    let hint_theme = theme_merged.clone();
    editor.set_hinter(Box::new(LumeHinter {
        hinter: Some(Box::new(move |line: &str, pos: usize| {
            hint_for_line(line, pos, &hint_theme)
        })),
    }));

    // Set up key bindings
    // Ctrl+J: accept full hint
    editor.bind_sequence(KeyEvent::Ctrl('j'), Cmd::AcceptHint);
    // Alt+J: accept one word from hint
    editor.bind_sequence(KeyEvent::Alt('j'), Cmd::AcceptHintWord(0));
    // Ctrl+O: clear buffer
    editor.bind_sequence(KeyEvent::Ctrl('o'), Cmd::ClearBuffer);
    // Alt+S: toggle sudo/pkexec prefix
    editor.set_sudo_cmd(&hotkey_sudo);
    editor.bind_sequence(KeyEvent::Alt('s'), Cmd::ToggleSudo);

    // Custom hotkeys from LUME_HOT_KEYS
    if let Some(Expression::Map(keys)) = hotkey_config {
        for (k, cmd) in keys.iter() {
            if let Some(c) = k.chars().next() {
                let key = modifier_to_key(c, modifier);
                editor.bind_sequence(key, Cmd::InsertStr(cmd.to_string() + "\n"));
            }
        }
    }

    // Set up abbreviations
    let _abbr_map: HashMap<String, String> = match _abbr {
        Some(Expression::Map(ab)) => ab
            .iter()
            .map(|m| (m.0.to_string(), m.1.to_string()))
            .collect(),
        _ => HashMap::new(),
    };
    editor.set_abbreviations(_abbr_map);

    // =======load history=======
    let _ = editor.history_mut().load_from_file(&history_file);

    // =======prompt=======
    let pe = get_prompt_engine(
        env.get("LUME_PROMPT_SETTINGS"),
        env.get("LUME_PROMPT_TEMPLATE"),
    );
    env.undefine("LUME_PROMPT_SETTINGS");
    env.undefine("LUME_PROMPT_TEMPLATE");

    // =======main loop=======
    loop {
        let prompt = pe.get_prompt();

        let line = match editor.readline(&prompt) {
            Ok(line) => line,
            Err(ReadlineError::Interrupted) => {
                println!("^C");
                if childman::kill_child() {
                    childman::clear_child();
                }
                continue;
            }
            Err(ReadlineError::Eof) => {
                println!("CTRL-D");
                continue;
            }
            Err(ReadlineError::Io(e)) => {
                eprintln!("Read error: {e}");
                continue;
            }
        };

        let trimmed = line.trim();

        if trimmed == "exit" {
            break;
        }

        if trimmed.is_empty() {
            continue;
        }

        // Multi-line: accumulate input until parse is complete
        let mut full_input = trimmed.to_string();
        while (full_input.ends_with(" \\") || !check(&full_input)) && !full_input.ends_with("\n\n")
        {
            if full_input.ends_with(" \\") {
                full_input.truncate(full_input.len() - 2);
            }
            if !full_input.is_empty() && !full_input.ends_with('\n') {
                full_input.push('\n');
            }
            let cont_line = match editor.readline("... ") {
                Ok(l) => l,
                Err(ReadlineError::Interrupted) => {
                    println!("^C");
                    if childman::kill_child() {
                        childman::clear_child();
                    }
                    full_input.clear();
                    break;
                }
                Err(ReadlineError::Eof) => {
                    println!("CTRL-D");
                    full_input.clear();
                    break;
                }
                Err(ReadlineError::Io(e)) => {
                    eprintln!("Read error: {e}");
                    full_input.clear();
                    break;
                }
            };
            let trimmed_cont = cont_line.trim();
            if trimmed_cont.is_empty() {
                full_input.push('\n');
            }
            full_input.push_str(trimmed_cont);
        }

        if full_input.is_empty() {
            continue;
        }

        if full_input == "history" {
            for (i, entry) in editor.history().iter().enumerate() {
                println!("{}{}:{} {}", GREEN_BOLD, i + 1, RESET, entry);
            }
            continue;
        }

        if parse_and_eval(&full_input, env) {
            let _ = editor.history_mut().add(full_input);
        }

        // 检查命令执行期间是否收到 SIGINT（Ctrl+C）
        if childman::check_and_clear_sigint() {
            println!("^C");
        }
    }

    // Save history
    if !no_history {
        if let Err(e) = editor.history_mut().save_to_file(&history_file) {
            eprintln!("Failed to save history: {e}");
        }
    }
}

fn hint_for_line(line: &str, pos: usize, theme: &HashMap<String, String>) -> Option<String> {
    let prefix = &line[..pos];
    let p = find_command_pos(prefix);
    let segment = &prefix[p..];

    if segment.is_empty() {
        return None;
    }

    let mut matches = collect_command_with_prefix(&segment);

    if matches.is_empty() {
        let ends: &[_] = &['(', ' '];
        let (matches, hint_pos) = match segment.split_once(".") {
            Some((name, func)) => LIBS_INFO.with(|h| {
                if let Some(lib) = h.get(&name) {
                    (
                        lib.iter()
                            .filter(|(f, _)| f.starts_with(func.trim_matches(ends)))
                            .map(|(f, info)| (format!("{f} {}", info.hint), f.len()))
                            .collect::<Vec<_>>(),
                        func.len(),
                    )
                } else {
                    (Vec::new(), 0)
                }
            }),
            _ => LIBS_INFO.with(|h| {
                if let Some(lib) = h.get("") {
                    (
                        lib.iter()
                            .filter(|(f, _)| f.starts_with(segment.trim_matches(ends)))
                            .map(|(f, info)| (format!("{f} {}", info.hint), f.len()))
                            .collect::<Vec<_>>(),
                        segment.len(),
                    )
                } else {
                    (Vec::new(), 0)
                }
            }),
        };
        if !matches.is_empty() {
            let hint_color = theme.get("hint").map_or(DEFAULT, |c| c.as_str());
            let matches: Vec<_> = matches.iter().filter(|(_, l)| *l > 0).collect();
            if let Some((matched, _)) = matches.first() {
                let suffix = &matched[hint_pos..];
                if !suffix.is_empty() {
                    return Some(format!("{hint_color}{suffix}{RESET}"));
                }
            }
        }
        return None;
    }

    matches.sort_by(|a, b| a.len().cmp(&b.len()));
    if let Some(matched) = matches.first() {
        let suffix = &matched[segment.len()..];
        if !suffix.is_empty() {
            let hint_color = theme.get("hint").map_or(DEFAULT, |c| c.as_str());
            return Some(format!("{hint_color}{suffix}{RESET}"));
        }
    }

    None
}

fn modifier_to_key(c: char, modifier: u8) -> KeyEvent {
    use crossterm::event::KeyModifiers;
    if modifier & KeyModifiers::CONTROL.bits() != 0 {
        KeyEvent::Ctrl(c)
    } else if modifier & KeyModifiers::ALT.bits() != 0 {
        KeyEvent::Alt(c)
    } else if modifier & KeyModifiers::SHIFT.bits() != 0 {
        KeyEvent::Shift(c)
    } else {
        KeyEvent::Char(c)
    }
}
