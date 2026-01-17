use std::fs::read_to_string;
use std::path::PathBuf;
use std::sync::LazyLock;
use std::sync::RwLock;
use std::{collections::HashMap, sync::Arc};

use rustyline::completion::Pair;

use crate::{Expression, RuntimeError};

pub struct CompletionDatabase {
    entries: HashMap<String, Arc<Vec<CompletionEntry>>>,
    // base_dir: String,
}
impl CompletionDatabase {
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }
}

static COMPLETION_DB: LazyLock<RwLock<CompletionDatabase>> =
    LazyLock::new(|| RwLock::new(CompletionDatabase::new()));

#[derive(Debug, Clone)]
pub struct CompletionEntry {
    pub command: String,
    pub conditions: Vec<String>, // Split by spaces
    pub short_opt: Option<String>,
    pub long_opt: Option<String>,
    pub args: Vec<String>, // @F, @D, or specific values
    pub directives: Vec<String>,
    pub priority: i32,
    pub description: String,
}

#[derive(Debug, Clone)]
pub struct ParamCompleter {
    // entries: Rc<HashMap<String, Vec<CompletionEntry>>>,
    base_dirs: Vec<PathBuf>,
}
enum MatchType {
    // Condition,
    Short,
    Long,
    Argument,
    ArgumentWithLong,
    ArgumentWithShort,
    // CondAndShort,
    // CondAndLong,
    // All,
    File,
    Require,
    Space,
    None,
}

fn from_csv(csv_content: &str) -> Result<Vec<CompletionEntry>, RuntimeError> {
    let mut entries = Vec::new();

    // let mut rdr = csv::ReaderBuilder::new()
    //     .has_headers(true)
    //     .delimiter(',') // 将字符串转换为字节并取第一个字符
    //     .from_reader(csv_content.as_bytes());

    for (line_num, line) in csv_content.lines().enumerate() {
        if line_num == 0 || line.trim().is_empty() {
            continue;
        } // Skip header

        let parts: Vec<&str> = line.split(',').collect();
        // if parts.len() != 7 {
        //     eprintln!("Warning: Invalid CSV line {}: {}", line_num, line);
        //     continue;
        // }

        let entry = CompletionEntry {
            command: parts[0].trim().to_string(),
            conditions: if parts[1].trim().is_empty() {
                Vec::new()
            } else {
                parts[1]
                    .trim()
                    .split_whitespace()
                    .map(|s| s.to_string())
                    .collect()
            },
            short_opt: if parts[2].trim().is_empty() {
                None
            } else {
                Some(parts[2].trim().to_string())
            },
            long_opt: if parts[3].trim().is_empty() {
                None
            } else {
                Some(parts[3].trim().to_string())
            },
            args: if parts[4].trim().is_empty() {
                Vec::new()
            } else {
                parts[4]
                    .trim()
                    .split_whitespace()
                    .map(|s| s.to_string())
                    .collect()
            },
            directives: if parts[5].trim().is_empty() {
                Vec::new()
            } else {
                parts[5]
                    .trim()
                    .split_whitespace()
                    .map(|s| s.to_string())
                    .collect()
            },
            priority: parts[6].trim().parse().unwrap_or(0),
            description: parts[7..].join(",").trim().to_string(),
        };

        entries.push(entry);
    }

    Ok(entries)
}

impl ParamCompleter {
    pub fn new(base_dir: String) -> Self {
        #[cfg(unix)]
        let base_dirs = vec![
            PathBuf::from(base_dir),
            dirs::data_local_dir()
                .unwrap_or(PathBuf::from("~/.local/share"))
                .join("lumesh/vendor_completions"),
            dirs::data_local_dir()
                .unwrap_or(PathBuf::from("~/.local/share"))
                .join("lumesh/completions"),
            PathBuf::from("/usr/local/share/lumesh/vendor_completions.d"),
            PathBuf::from("/usr/local/share/lumesh/completions"),
            PathBuf::from("/usr/share/lumesh/vendor_completions.d"),
            PathBuf::from("/usr/share/lumesh/completions"),
        ];

        #[cfg(windows)]
        let base_dirs = vec![
            PathBuf::from(base_dir),
            dirs::data_local_dir()
                .unwrap_or_default()
                .join("lumesh\\vendor_completions"),
            dirs::data_local_dir()
                .unwrap_or_default()
                .join("lumesh\\completions"),
            PathBuf::from("C:\\Program Files\\lumesh\\vendor_completions"),
            PathBuf::from("C:\\Program Files\\lumesh\\completions"),
        ];
        Self {
            // entries: HashMap::new(),
            base_dirs,
        }
    }

    fn load_entry(&self, cmd: &str) -> Result<Vec<CompletionEntry>, RuntimeError> {
        // Load from file or embed the CSV data
        for dir in &self.base_dirs {
            let path = PathBuf::from(dir).join(format!("{}.csv", cmd));
            if path.exists() {
                let csv_data = read_to_string(path).map_err(|e| {
                    RuntimeError::from_io_error(e, "read file".into(), Expression::None, 0)
                })?;
                return from_csv(&csv_data);
            }
        }
        Err(RuntimeError::common(
            "no completion".into(),
            Expression::None,
            0,
        ))
    }

    fn check_condition(
        &self,
        entry: &CompletionEntry,
        args: &[&str],
        __current_token: &str,
    ) -> bool {
        if entry.directives.iter().any(|f| f == "@t") {
            return true;
        }
        let reverse = entry.directives.iter().any(|f| f == "@n");
        if entry.conditions.is_empty() {
            if reverse {
                // return args.len() > 0;
                // only subcmd, not long/short option
                // return args.iter().any(|x| !x.starts_with('-'));
                // don't take argument after -- as condition, it's not subcmd
                return !args.is_empty() && !args[0].starts_with('-');
            }
            // return args.len() == 0;
            // return !args.iter().any(|x| !x.starts_with('-'));
            return args.is_empty() || args[0].starts_with('-');
        }
        for condition in &entry.conditions {
            // Check if any condition matches the args
            if args.iter().any(|a| condition == a) {
                return !reverse;
            }
        }
        return false;
    }

    fn check_opt(&self, entry: &CompletionEntry, args: &[&str], __current_token: &str) -> bool {
        let mut need = false;
        if let Some(short) = entry.short_opt.as_ref() {
            // allow short args compose like -abc contains -b
            if args.iter().any(|a| {
                a.len() > 1
                    && a.starts_with('-')
                    && !a[1..].starts_with('-')
                    && a[1..].contains(short)
            }) {
                return true;
            }
            need = true;
        }
        if let Some(long) = entry.long_opt.as_ref() {
            if args
                .iter()
                .any(|a| a.len() > 2 && a.starts_with("--") && long == &a[2..])
            {
                return true;
            }
            need = true;
        }
        if need {
            return false;
        }

        return true;
    }

    /**
     * check if one entry is matched
     * condition and argument, has to be filtered later
     */
    fn matches_context(
        &self,
        entry: &CompletionEntry,
        args: &[&str],
        current_token: &str,
    ) -> MatchType {
        // check condition as subcommand
        // 无参数，若有condition则列出
        if current_token.starts_with("--") && entry.long_opt.is_some() {
            if current_token.len() == 2
                || entry
                    .long_opt
                    .as_ref()
                    .is_some_and(|x| x.starts_with(&current_token[2..]))
            {
                if self.check_condition(entry, args, current_token) {
                    if entry
                        .long_opt
                        .as_ref()
                        .is_some_and(|x| x == &current_token[2..])
                    {
                        return MatchType::Space;
                    }
                    if entry.directives.iter().any(|d| d == "@m")
                        || !self.check_opt(entry, args, current_token)
                    {
                        return MatchType::Long;
                    }
                }
            }
            return MatchType::None;
        } else if current_token.starts_with("-") && entry.short_opt.is_some() {
            if current_token.len() == 1
                || entry
                    .short_opt
                    .as_ref()
                    .is_some_and(|x| x.starts_with(&current_token[1..]))
            {
                if self.check_condition(entry, args, current_token) {
                    if entry
                        .short_opt
                        .as_ref()
                        .is_some_and(|x| x == &current_token[1..])
                    {
                        return MatchType::Space;
                    }
                    if entry.directives.iter().any(|d| d == "@m")
                        || !self.check_opt(entry, args, current_token)
                    {
                        return MatchType::Short;
                    }
                }
            }
            return MatchType::None;
            // 只检测正在输入
        } else if !current_token.is_empty() {
            // 有参数且不以-开始，则优先匹配action，其次长短选项
            if self.check_condition(entry, args, current_token) {
                // 如果满足长短选项，则只匹配argument；如未满足，则匹配argument后携带长短选项
                if self.check_opt(entry, args, current_token) {
                    if entry.args.iter().any(|x| x.starts_with(current_token)) {
                        return MatchType::Argument;
                    }
                } else {
                    // action，并带出长短选项
                    // if entry.args.iter().any(|x| x.starts_with(current_token)) {
                    //     if entry.long_opt.is_some() {
                    //         return MatchType::ArgumentWithLong; //长短皆可，默认？选择？
                    //     } else if entry.short_opt.is_some() {
                    //         return MatchType::ArgumentWithShort; //长短皆可，默认？选择？
                    //     } else {
                    //         return MatchType::Argument;
                    //     }
                    // }
                }
                // 【扩展匹配】未匹配action则继续匹配长短选项
                // if entry
                //     .short_opt
                //     .as_ref()
                //     .is_some_and(|x| x.starts_with(current_token))
                // {
                //     return MatchType::Short;
                // }
                // if entry
                //     .long_opt
                //     .as_ref()
                //     .is_some_and(|x| x.starts_with(current_token))
                // {
                //     return MatchType::Long;
                // }
            }

            return MatchType::None;
            // 当前单词为空
        } else {
            if self.check_condition(entry, args, current_token) {
                // 满足长短选项
                if self.check_opt(entry, args, current_token) {
                    if entry.directives.iter().any(|d| d == "@F" || d == "@D") {
                        return MatchType::File;
                    }
                    if entry.args.is_empty() {
                        // 无argument，且要求具有参数
                        if entry.directives.iter().any(|d| d == "@r") {
                            return MatchType::Require;
                        }
                    } else {
                        // 无特殊指令，显示所有argument
                        return MatchType::Argument;
                    }
                } else {
                    //【扩展匹配】列出所有长短选项
                    // if entry.short_opt.is_some() {
                    //     return MatchType::Short;
                    // } else if entry.long_opt.is_some() {
                    //     return MatchType::Long;
                    // } else if !entry.args.is_empty() {
                    //     return MatchType::Argument;
                    // }
                }
            }
        }
        MatchType::None
    }
    /**
     * match more, fuzzy
     */
    fn matches_more(
        &self,
        entry: &CompletionEntry,
        args: &[&str],
        current_token: &str,
    ) -> MatchType {
        if !current_token.is_empty() {
            // 有参数且不以-开始，则优先匹配action，其次长短选项
            if !current_token.starts_with("-") && self.check_condition(entry, args, current_token) {
                // 不满足本条opt，则匹配action，并带出长短选项
                if !self.check_opt(entry, args, current_token) {
                    // 无ignore file标记，则进行路径补全
                    if !entry.directives.iter().any(|d| d == "@f") {
                        return MatchType::File;
                    }

                    if entry.args.iter().any(|x| x.starts_with(current_token)) {
                        if entry.long_opt.is_some() {
                            return MatchType::ArgumentWithLong; //长短皆可，默认？选择？
                        } else if entry.short_opt.is_some() {
                            return MatchType::ArgumentWithShort; //长短皆可，默认？选择？
                        } else {
                            return MatchType::Argument;
                        }
                    }
                    // 【扩展匹配】未匹配action则继续匹配长短选项
                    if entry
                        .short_opt
                        .as_ref()
                        .is_some_and(|x| x.starts_with(current_token))
                    {
                        return MatchType::Short;
                    }
                    if entry
                        .long_opt
                        .as_ref()
                        .is_some_and(|x| x.starts_with(current_token))
                    {
                        return MatchType::Long;
                    }
                }
                // 【扩展匹配】允许多次出现的长短选项
                if entry
                    .short_opt
                    .as_ref()
                    .is_some_and(|x| x.starts_with(current_token))
                {
                    if entry.directives.iter().any(|d| d == "@m") {
                        return MatchType::Short;
                    }
                }
                if entry
                    .long_opt
                    .as_ref()
                    .is_some_and(|x| x.starts_with(current_token))
                {
                    if entry.directives.iter().any(|d| d == "@m") {
                        return MatchType::Long;
                    }
                }
            }

            return MatchType::None;
            // 当前单词为空
        } else {
            if self.check_condition(entry, args, current_token) {
                // 不满足长短选项
                if !self.check_opt(entry, args, current_token) {
                    // 无ignore file标记，则进行路径补全
                    if !entry.directives.iter().any(|d| d == "@f") {
                        return MatchType::File;
                    }
                    // 【扩展匹配】列出长短选项，允许多次出现的，或未出现过的
                    if entry.directives.iter().any(|d| d == "@m")
                        || !self.check_opt(entry, args, current_token)
                    {
                        if entry.short_opt.is_some() {
                            return MatchType::Short;
                        } else if entry.long_opt.is_some() {
                            return MatchType::Long;
                        }
                    }
                }
            }
        }
        MatchType::None
    }

    // // 添加补全条目
    // pub fn add_completion(key: String, entry: CompletionEntry) {
    //     if let Ok(mut db) = COMPLETION_DB.write() {
    //         db.entries.entry(key).or_default().push(entry);
    //     }
    // }

    // // 获取补全条目
    // pub fn get_completions(key: &str) -> Vec<CompletionEntry> {
    //     COMPLETION_DB.read()
    //         .ok()
    //         .and_then(|db| db.entries.get(key).cloned())
    //         .unwrap_or_default()
    // }

    fn get_entry_for_command(&self, command: &str) -> Option<Arc<Vec<CompletionEntry>>> {
        // 先尝试读锁
        // match COMPLETION_DB.read().entries.get(command) {
        //     Some(entry) => Some(Cow::Borrowed(entry)),
        //     _ => {
        //         let entry = self.load_entry(command).unwrap_or_default();
        //         let arc_entry = Arc::new(entry);
        //         COMPLETION_DB
        //             .write()
        //             .entries
        //             .insert(command.to_string(), Arc::clone(&arc_entry));
        //         if entry.is_empty() {
        //             return None;
        //         }
        //         Some(Cow::Owned(arc_entry))
        //     }
        // }
        if let Ok(db) = COMPLETION_DB.read() {
            if let Some(entries) = db.entries.get(command) {
                return Some(Arc::clone(entries));
            }
        }

        // 如果不存在，获取写锁并插入
        if let Ok(mut db) = COMPLETION_DB.write() {
            // 插入新条目（用 Arc 包装）
            let entry = self.load_entry(command).unwrap_or_default();
            let arc_entry = Arc::new(entry);
            db.entries
                .insert(command.to_string(), Arc::clone(&arc_entry));
            Some(arc_entry)
        } else {
            None
        }
    }

    // fn get_entry_for_command(&self, command: &str) -> Option<&Vec<CompletionEntry>> {
    //     let entry = COMPLETION_DB.
    //         .entries
    //         .entry(command.to_string())
    //         .or_insert_with(|| {
    //             // 如果不存在加载条目并插入,出错（如文件不存在，则用空vec占位，避免下次继续读取文件
    //             load_entry(&self.base_dir, command).unwrap_or_default()
    //         });
    //     Some(entry)
    // }
    /**
     * args should exclude cmd and the current-token
     */
    pub fn get_completions_for_context(
        &self,
        command: &str,
        args: &[&str],
        current_token: &str,
    ) -> (Vec<Pair>, bool) {
        let (v, b) = self.get_completions_once(command, args, current_token, false);
        if v.is_empty() {
            return self.get_completions_once(command, args, current_token, true);
        }
        return (v, b);
    }
    fn get_completions_once(
        &self,
        command: &str,
        args: &[&str],
        current_token: &str,
        match_more: bool,
    ) -> (Vec<Pair>, bool) {
        let mut v = Vec::<Pair>::new();

        // let mut contents = vec![format!("{},{:?},{}", command, args, current_token)];
        // if let Some(indices) = self.command_index.get(command) {
        //     for idx in indices {
        //         let entry = &self.entries[*idx];
        //         contents.push(format!(
        //             "{},{:#?},{:#?},{:#?},{},{}",
        //             idx,
        //             entry.conditions,
        //             entry.short_opt,
        //             entry.long_opt,
        //             &self.check_condition(entry, args, current_token),
        //             &self.check_opt(entry, args, current_token)
        //         ));
        //     }
        //     let _ = std::fs::write("/tmp/debug.csv", contents.join("\n"))
        //         .map_err(|x| println!("{}", x));
        // }
        if let Some(entries) = self.get_entry_for_command(command) {
            for entry in entries.as_ref() {
                // dbg!(&entry, self.check_condition(entry, args, current_token));
                let matched = if match_more {
                    self.matches_more(entry, args, current_token)
                } else {
                    self.matches_context(entry, args, current_token)
                };
                match matched {
                    MatchType::Short => v.push(Pair {
                        display: format_opt(entry),
                        replacement: format!("-{}", entry.short_opt.as_ref().unwrap()),
                    }),
                    MatchType::Long => v.push(Pair {
                        display: format_opt(entry),
                        replacement: format!("--{}", entry.long_opt.as_ref().unwrap()),
                    }),
                    MatchType::Space => v.push(Pair {
                        display: format!("{:<5} :OK", " "),
                        replacement: format!("{} ", current_token),
                    }),
                    // MatchType::Condition => {
                    //     for cond in entry.conditions.iter() {
                    //         if cond.starts_with(current_token) {
                    //             // 需要去重
                    //             if !v.iter().any(|o| &o.replacement == cond) {
                    //                 v.push(Pair {
                    //                     display: cond.clone(),
                    //                     replacement: cond.clone(),
                    //                 })
                    //             }
                    //         }
                    //     }
                    // }
                    MatchType::Argument => {
                        // 需要过滤
                        for a in entry.args.iter() {
                            if a.starts_with(current_token) {
                                v.push(Pair {
                                    display: format!(
                                        "{:<15} \x1b[96m{:>}\x1b[m\x1b[0m",
                                        a, entry.description
                                    ),
                                    // display: a.clone(),
                                    replacement: a.clone(),
                                })
                            }
                        }
                    }
                    // MatchType::CondAndLong => {
                    //     // condition已经非空，long已经匹配
                    //     for cond in entry.conditions.iter() {
                    //         v.push(Pair {
                    //             display: format!("{} --{}", cond, entry.long_opt.as_ref().unwrap()),
                    //             replacement: format!(
                    //                 "{} --{}",
                    //                 cond,
                    //                 entry.long_opt.as_ref().unwrap()
                    //             ),
                    //         })
                    //     }
                    // }
                    // MatchType::CondAndShort => {
                    //     // condition已经非空，short已经匹配
                    //     for cond in entry.conditions.iter() {
                    //         v.push(Pair {
                    //             display: format!("{} -{}", cond, entry.short_opt.as_ref().unwrap()),
                    //             replacement: format!(
                    //                 "{} -{}",
                    //                 cond,
                    //                 entry.short_opt.as_ref().unwrap()
                    //             ),
                    //         })
                    //     }
                    // }
                    MatchType::ArgumentWithLong => {
                        // arg需要过滤
                        for x in entry.args.iter() {
                            if x.starts_with(current_token) {
                                if let Some(long) = entry.long_opt.clone() {
                                    v.push(Pair {
                                        display: format_arg_opt(entry, x),
                                        replacement: format!("--{} {}", long, x),
                                    })
                                }
                            }
                        }

                        // v.push()
                    }
                    MatchType::ArgumentWithShort => {
                        // arg需要过滤
                        for x in entry.args.iter() {
                            if x.starts_with(current_token) {
                                if let Some(short) = entry.short_opt.clone() {
                                    v.push(Pair {
                                        display: format_arg_opt(entry, x),
                                        replacement: format!("-{} {}", short, x),
                                    })
                                }
                            }
                        }

                        // v.push()
                    }
                    MatchType::File => return (v, true),
                    MatchType::Require => {
                        v.push(Pair {
                            display: String::from("_      :param required"),
                            replacement: String::from("_"),
                        });
                    }
                    _ => {}
                };
            }
        }
        (v, false)
    }
}

/**
 * 1: opt, 2: action, 3=1+2
 */
fn format_opt(entry: &CompletionEntry) -> String {
    match (entry.short_opt.as_ref(), entry.long_opt.as_ref()) {
        (Some(s), Some(l)) => format!(
            "-{:<4} --{:<18} \x1b[96m{:>}\x1b[m\x1b[0m",
            s, l, entry.description
        ),
        (_, Some(l)) => format!(
            "      --{:<18} \x1b[96m{:>}\x1b[m\x1b[0m",
            l, entry.description
        ),
        (Some(s), _) => format!("-{:<21} \x1b[96m{:>}\x1b[m\x1b[0m", s, entry.description),
        _ => String::from("Err"),
    }
}
fn format_arg_opt(entry: &CompletionEntry, arg: &String) -> String {
    match (entry.short_opt.as_ref(), entry.long_opt.as_ref()) {
        (Some(s), Some(l)) => format!(
            "-{:<4} --{:<18} {:15} \x1b[96m{:>}\x1b[m\x1b[0m",
            s, l, arg, entry.description
        ),
        (_, Some(l)) => format!(
            "      --{:<18} {:15} \x1b[96m{:>}\x1b[m\x1b[0m",
            l, arg, entry.description
        ),
        (Some(s), _) => format!(
            "-{:<21} {:15} \x1b[96m{:>}\x1b[m\x1b[0m",
            s, arg, entry.description
        ),
        _ => String::from(arg),
    }
}
