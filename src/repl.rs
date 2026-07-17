use crate::ai::init_ai;
use crate::cmdhelper::{
    LumeCompletionType, collect_command_with_prefix, detect_completion_type, find_command_pos,
};
use crate::completion::{ParamCompleter, list_path_entries};
use crate::editor::{
    Cmd, Completer, CompletionItem, Editor, EditorTheme, Highlighter, Hinter, KeyEvent,
    ReadlineError, ValidationResult, Validator,
};
use crate::expression::alias::get_alias_completion;
use crate::libs::{LIBS_INFO, is_lib};
use crate::syntax::{get_ayu_dark_theme, get_dark_theme, get_light_theme, get_merged_theme};
use crate::{CFM_ENABLED, Expression, STRICT_ENABLED, childman};
use crate::{Environment, check, highlight, parse_and_eval, prompt::get_prompt_engine};
use std::collections::HashMap;
use std::path::{PathBuf, is_separator};
use std::rc::Rc;
use std::str::FromStr;
use std::sync::{Arc, Mutex};

use crossterm::style::Color;

const GREEN_BOLD: &str = "\x1b[1;32m";
const RESET: &str = "\x1b[0m";

struct LumeCompleter {
    param_completer: Arc<ParamCompleter>,
}

impl Completer for LumeCompleter {
    fn complete(&self, line: &str, pos: usize) -> Vec<CompletionItem> {
        match detect_completion_type(line, pos) {
            (LumeCompletionType::Path, _) => complete_path(line, pos),
            (LumeCompletionType::Command, section_pos) => {
                self.cmd_completion(line, pos, section_pos)
            }
            (LumeCompletionType::Param, section_pos) => {
                self.param_completion(line, pos, section_pos)
            }
            // (LumeCompletionType::AI, section_pos) => self.ai_completion(line, section_pos),
            (LumeCompletionType::None, _) => Vec::new(),
        }
    }
}

// only needed on head of line, without cmd
fn complete_path(line: &str, pos: usize) -> Vec<CompletionItem> {
    let prefix = &line[..pos];
    let path_start = prefix
        .rfind(|c: char| {
            c.is_ascii_whitespace()
                || c == '>'
                || c == ':'
                || c == '|'
                || c == '&'
                || c == '('
                || c == ';'
        })
        .map(|i| i + 1)
        .unwrap_or(0);
    let path = &prefix[path_start..];

    let entries = list_path_entries(path, false);
    let mut items: Vec<CompletionItem> = entries
        .into_iter()
        .map(|(name, full_path)| CompletionItem {
            display: Some(name),
            replacement: full_path,
            suffix: None,
        })
        .collect();
    items.sort_by(|a, b| a.replacement.cmp(&b.replacement));
    items
}

impl LumeCompleter {
    fn cmd_completion(&self, line: &str, pos: usize, section_start: usize) -> Vec<CompletionItem> {
        let prefix = &line[section_start..pos];

        let mut items: Vec<CompletionItem> = collect_command_with_prefix(prefix)
            .iter()
            .map(|cmd| CompletionItem::with(cmd.to_string(), if is_lib(cmd) { '.' } else { ' ' }))
            .collect();

        if items.is_empty() {
            match prefix.split_once(".") {
                Some((name, func)) => {
                    if !name.is_empty() {
                        LIBS_INFO.with(|h| {
                            if let Some(lib) = h.get(&name) {
                                items = lib
                                    .iter()
                                    .filter(|(f, _)| f.starts_with(func))
                                    .map(|(cmd, _)| CompletionItem::with(cmd.to_string(), '('))
                                    .collect();
                            }
                        });
                        items.sort_by_key(|a| a.display_text().len());
                        return items;
                    }
                }
                _ => {
                    items = get_alias_completion(prefix)
                        .into_iter()
                        .map(|cmd| CompletionItem::with(cmd, ' '))
                        .collect();
                }
            }
        }

        items.sort_by_key(|a| a.display_text().len());
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

            let pairs =
                self.param_completer
                    .get_completions_for_context(command, params, current_token);

            let mut items: Vec<CompletionItem> = pairs
                .into_iter()
                .map(|p| {
                    if p.replacement.ends_with("/") {
                        CompletionItem::with_display(p.display, p.replacement)
                    } else {
                        CompletionItem {
                            display: Some(p.display),
                            replacement: p.replacement,
                            suffix: Some(' '),
                        }
                    }
                })
                .collect();

            // empty param completion, try path
            if items.is_empty() && current_token.contains(is_separator) {
                let entries = list_path_entries(current_token, false);
                items = entries
                    .into_iter()
                    .map(|(name, full_path)| CompletionItem {
                        display: Some(name),
                        replacement: full_path,
                        suffix: None,
                    })
                    .collect();
            }

            items.sort_by_key(|a| a.replacement.len());
            return items;
        }
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
        highlight(line, &self.theme)
    }

    fn highlight_char(&self, _line: &str, _pos: usize) -> bool {
        false
    }
}

struct LumeHinter {
    #[allow(clippy::type_complexity)]
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
            if !c_dir.exists()
                && let Err(e) = std::fs::create_dir_all(&c_dir)
            {
                eprintln!("Failed to create cache directory: {e}");
            }
            if !path.exists()
                && let Err(e) = std::fs::File::create(&path)
            {
                eprintln!("Failed to create cache file: {e}");
            }
            path.into_os_string().into_string().unwrap()
        }
    };

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

    let _abbr = env.get("LUME_ABBREVIATIONS");
    env.undefine("LUME_ABBREVIATIONS");

    // =======create editor=======
    let mut editor = Editor::new();

    // ai config
    let ai_config = env.get("LUME_AI_CONFIG");
    env.undefine("LUME_AI_CONFIG");

    // ai hinter
    let ai_client = ai_config.map(|ai_cfg| Arc::new(init_ai(ai_cfg)));
    if let Some(ref ai) = ai_client {
        editor.set_ai_client(Arc::clone(ai));
    }

    // Set up completer
    let completer = LumeCompleter {
        param_completer: Arc::new(ParamCompleter::new(completion_dir)),
    };
    editor.set_completer(Box::new(completer));

    // Set up highlighter
    // let hl_theme = theme_merged.clone();
    editor.set_highlighter(Box::new(LumeHighlighter {
        theme: theme_merged,
    }));

    // Set up hinter (hint from command history)
    // let hint_theme = theme_merged.clone();
    editor.set_hinter(Box::new(LumeHinter {
        hinter: Some(Box::new(move |line: &str, pos: usize| {
            hint_for_line(line, pos)
        })),
    }));

    // Editor theme from LUME_EDITOR_THEME config
    if let Some(Expression::Map(theme_map)) = env.get("LUME_EDITOR_THEME") {
        let mut theme = EditorTheme::default();
        for (k, v) in theme_map.iter() {
            let val = v.to_string();
            match k.as_str() {
                "hint" => theme.hint_color = parse_color(&val),
                "completion_bg" => theme.completion_bg = parse_color(&val),
                "completion_fg" => theme.completion_fg = parse_color(&val),
                "completion_selected_bg" => theme.completion_selected_bg = parse_color(&val),
                "completion_selected_fg" => theme.completion_selected_fg = parse_color(&val),
                _ => {}
            }
        }
        editor.set_theme(theme);
    }
    env.undefine("LUME_EDITOR_THEME");

    // Continuation prompt from config
    if let Some(Expression::String(p)) = env.get("LUME_CONTINUATION_PROMPT") {
        editor.set_cont_prompt(&p);
    }
    env.undefine("LUME_CONTINUATION_PROMPT");

    // Set up key bindings
    // Ctrl+J: accept full hint
    // editor.bind_sequence(KeyEvent::Ctrl('j'), Cmd::AcceptHint);
    // Alt+J: accept one word from hint
    // editor.bind_sequence(KeyEvent::Alt('j'), Cmd::AcceptHintWord(0));
    // Ctrl+O: clear buffer
    editor.bind_sequence(KeyEvent::Ctrl('o'), Cmd::ClearBuffer);
    // Alt+S: toggle sudo/pkexec prefix
    // editor.set_sudo_cmd(&hotkey_sudo);
    editor.bind_sequence(KeyEvent::Alt('s'), Cmd::ToggleSudo(hotkey_sudo));

    // Set up validator for multiline input
    struct LumeValidator;
    impl Validator for LumeValidator {
        fn validate(&self, input: &str) -> ValidationResult {
            if input.starts_with("/") {
                // slash command
                ValidationResult::Valid
            } else if check(input) {
                ValidationResult::Valid
            } else {
                ValidationResult::Incomplete
            }
        }
    }
    editor.set_validator(Box::new(LumeValidator));

    // Share env with the editor callback via Rc<Mutex<>> so the callback
    // can fork a live snapshot of the current environment at hotkey time.
    let shared_env = Rc::new(Mutex::new(env.clone()));

    // Custom hotkeys LUME_HOT_BINDINGS
    let hotkey_bindings = env.get("LUME_HOT_BINDINGS");
    env.undefine("LUME_HOT_BINDINGS");

    if let Some(Expression::Map(bindings)) = hotkey_bindings {
        for (k, v) in bindings.iter() {
            let key_str = k.to_string();
            if let Some((mod_str, key_char)) = key_str.rsplit_once('_')
                && let Some(ch) = key_char.chars().next()
            {
                let key = parse_hot_key(mod_str.trim_matches(['\'', '"']), ch.to_ascii_lowercase());
                match v {
                    Expression::String(s) => {
                        // String value: insert directly into buffer
                        editor.bind_sequence(key, Cmd::InsertStr(s.clone()));
                    }
                    Expression::Function(..) | Expression::Lambda(..) => {
                        // Function/Lambda: execute with buffer as argument,
                        // insert result if it returns a string
                        let expr = v.clone();
                        let shared_env = shared_env.clone();
                        editor.bind_hotkey_fn(key, move |buffer: &str| -> Option<String> {
                            let env_guard = shared_env.lock().ok()?;
                            let mut fork_env = env_guard.fork();
                            drop(env_guard);
                            let call = Expression::Apply(
                                Rc::new(expr.clone()),
                                Rc::new(vec![Expression::String(buffer.to_string())]),
                            );
                            match call.eval_cmd(&mut fork_env) {
                                Ok(Expression::String(s)) => Some(s),
                                // Ok(other) if other != Expression::None => {
                                //     println!("{other}");
                                //     let _ = std::io::stdout().flush();
                                //     None
                                // }
                                _ => None,
                            }
                        });
                    }
                    other => {
                        // Other types
                        eprintln!("invalid bindings: {} -> {}", key_str, other);
                    }
                }
            }
        }
    }
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

    // --------slash bindings-------
    let slash_bindings = if let Some(Expression::Map(m)) = env.get("LUME_SLASH_BINDINGS") {
        Some(m)
    } else {
        None
    };
    let slash_menu = env.get("LUME_SLASH_MENU");
    env.undefine("LUME_SLASH_BINDINGS");
    env.undefine("LUME_SLASH_MENU");
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

        // Strip backslash-newline continuation markers
        let full_input = trimmed.replace("\\\n", "");

        if full_input.is_empty() {
            continue;
        }

        // slash command:
        if let Some(rest) = full_input.strip_prefix('/') {
            if rest.is_empty() {
                // menu
                match &slash_menu {
                    Some(Expression::Lambda(..)) | Some(Expression::Function(..)) => {
                        let _x = slash_menu
                            .clone()
                            .unwrap()
                            .apply(vec![])
                            .eval(&mut shared_env.lock().unwrap());
                    }
                    None => {
                        eprintln!("LUME_SLASH_MENU is not defined")
                    }
                    _ => {
                        eprintln!("LUME_SLASH_MENU is not appliable")
                    }
                };
            } else if let Some(query) = rest.strip_prefix(' ') {
                // quick jump
                if let Some(cd_cmd) = editor.history().search_fuzzy_one(query) {
                    if parse_and_eval(&cd_cmd, &mut shared_env.lock().unwrap()) {
                        editor.history_mut().add(cd_cmd);
                    }
                }
            } else if rest == "history" {
                // history
                for (i, entry) in editor.history().iter().enumerate() {
                    println!("{}{}:{} {}", GREEN_BOLD, i + 1, RESET, entry);
                }
            } else if rest == "hist" {
                // history
                let hist = editor
                    .history()
                    .entries_by_weight()
                    .iter()
                    .map(|(h, _)| Expression::String(h.to_string()))
                    .collect::<Vec<_>>();
                let mut forked_env = shared_env.lock().unwrap().fork();
                forked_env.define("HISTORY", Expression::from(hist));
                let cmd = format!("$HISTORY | ui.pick('select history') | eval_str()");
                parse_and_eval(&cmd, &mut forked_env);
            } else if rest == "cds" {
                let cmd = "ui.pick $PATH_SESSION 'cd to:' ?! | cd _";
                parse_and_eval(&cmd, &mut shared_env.lock().unwrap());
            } else {
                // slash bindings
                let (key, arg) = rest.split_once(' ').unwrap_or((rest, ""));
                if let Some(r) = slash_bindings.clone().and_then(|m| {
                    m.as_ref()
                        .get(key)
                        .and_then(|exp| Some(exp.apply(vec![Expression::String(arg.to_string())])))
                }) {
                    let _ = r.eval(&mut shared_env.lock().unwrap());
                } else {
                    eprintln!("\x1b[31mundefined slash cmd\x1b[0m");
                }
            }
        } else {
            // normal
            if parse_and_eval(&full_input, &mut shared_env.lock().unwrap()) {
                editor.history_mut().add(full_input);
            }
        }

        // 检查命令执行期间是否收到 SIGINT（Ctrl+C）
        if childman::check_and_clear_sigint() {
            println!("^C");
        }
    }

    // Save history
    if !no_history && let Err(e) = editor.history_mut().save_to_file(&history_file) {
        eprintln!("Failed to save history: {e}");
    }
}

fn hint_for_line(line: &str, pos: usize) -> Option<String> {
    let prefix = &line[..pos];
    let p = find_command_pos(prefix);
    let segment = &prefix[p..];

    if segment.is_empty() {
        return None;
    }

    let mut matches = collect_command_with_prefix(segment);

    if matches.is_empty() {
        let ends: &[_] = &['(', ' '];
        let (matches, hint_pos) = match segment.split_once(".") {
            Some((name, func)) => LIBS_INFO.with(|h| {
                if let Some(lib) = h.get(&name) {
                    (
                        lib.iter()
                            .filter(|(f, _)| f.starts_with(func.trim_matches(ends)))
                            .map(|(f, info)| {
                                let tip = if func.trim_end().ends_with('(') {
                                    format!("{f}(\0{})", info.hint.replace(' ', ","))
                                } else {
                                    format!("{f} \0{}", info.hint)
                                };
                                (tip, f.len())
                            })
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
                            .map(|(f, info)| {
                                let tip = if segment.trim_end().ends_with('(') {
                                    format!("{f}(\0{})", info.hint.replace(' ', ","))
                                } else {
                                    format!("{f} \0{}", info.hint)
                                };
                                (tip, f.len())
                            })
                            .collect::<Vec<_>>(),
                        segment.len(),
                    )
                } else {
                    (Vec::new(), 0)
                }
            }),
        };
        if !matches.is_empty() {
            // let hint_color = theme.get("hint").map_or(DEFAULT, |c| c.as_str());
            let matches: Vec<_> = matches.iter().filter(|(_, l)| *l > 0).collect();
            if let Some((matched, _)) = matches.first() {
                let suffix = &matched[hint_pos..];
                if !suffix.is_empty() {
                    // return Some(format!("{hint_color}{suffix}{RESET}"));
                    return Some(suffix.to_string());
                }
            }
        }
        return None;
    }

    matches.sort_by_key(|a| a.len());
    if let Some(matched) = matches.first() {
        let suffix = &matched[segment.len()..];
        if !suffix.is_empty() {
            return Some(suffix.to_string());
            //     let hint_color = theme.get("hint").map_or(DEFAULT, |c| c.as_str());
            //     return Some(format!("{hint_color}{suffix}{RESET}"));
        }
    }

    None
}

fn parse_hot_key(modifier_str: &str, key_char: char) -> KeyEvent {
    match modifier_str {
        "CTRL_ALT" => KeyEvent::CtrlAlt(key_char),
        "CTRL_SHIFT" => KeyEvent::CtrlShift(key_char),
        "ALT_SHIFT" => KeyEvent::AltShift(key_char),
        "CTRL_ALT_SHIFT" => KeyEvent::CtrlAltShift(key_char),
        "CTRL" => KeyEvent::Ctrl(key_char),
        "ALT" => KeyEvent::Alt(key_char),
        "SHIFT" => KeyEvent::Shift(key_char),
        _ => KeyEvent::Char(key_char),
    }
}

fn parse_color(s: &str) -> Color {
    Color::from_str(s).unwrap_or(Color::Grey)
    // match s.to_lowercase().as_str() {
    //     "black" => Color::Black,
    //     "dark_grey" => Color::DarkGrey,
    //     "red" => Color::Red,
    //     "dark_red" => Color::DarkRed,
    //     "green" => Color::Green,
    //     "dark_green" => Color::DarkGreen,
    //     "yellow" => Color::Yellow,
    //     "dark_yellow" => Color::DarkYellow,
    //     "blue" => Color::Blue,
    //     "dark_blue" => Color::DarkBlue,
    //     "magenta" => Color::Magenta,
    //     "dark_magenta" => Color::DarkMagenta,
    //     "cyan" => Color::Cyan,
    //     "dark_cyan" => Color::DarkCyan,
    //     "white" => Color::White,
    //     "grey" => Color::Grey,
    //     _ => Color::DarkGrey,
    // }
}
