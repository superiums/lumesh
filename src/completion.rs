use std::collections::HashMap;
use std::fs::read_to_string;

use rustyline::completion::Pair;

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
pub struct CompletionDatabase {
    entries: Vec<CompletionEntry>,
    // Index for faster lookup: command -> entries
    command_index: HashMap<String, Vec<usize>>,
}
enum MatchType {
    // Condition,
    Short,
    Long,
    Argument,
    LongAndArgument,
    // ShortAndArgument,
    // CondAndShort,
    // CondAndLong,
    // All,
    File,
    Require,
    None,
}

impl CompletionDatabase {
    pub fn load_completion_database() -> CompletionDatabase {
        // Load from file or embed the CSV data
        let path = "/tmp/completions.csv";
        let csv_data = read_to_string(&path).unwrap_or_else(|e| {
            eprintln!("Read file failed:\n  {}", e);
            String::from("")
        }); // Or load from config dir
        let db = CompletionDatabase::from_csv(&csv_data).unwrap_or_else(|_| {
            eprintln!("Warning: Failed to load completion database");
            CompletionDatabase {
                entries: Vec::new(),
                command_index: HashMap::new(),
            }
        });
        // dbg!(&db);
        db
    }
    pub fn from_csv(csv_content: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let mut entries = Vec::new();
        let mut command_index: HashMap<String, Vec<usize>> = HashMap::new();

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
                description: parts[7].trim().to_string(),
            };

            let idx = entries.len();
            command_index
                .entry(entry.command.clone())
                .or_insert_with(Vec::new)
                .push(idx);
            entries.push(entry);
        }

        Ok(CompletionDatabase {
            entries,
            command_index,
        })
    }

    fn check_condition(
        &self,
        entry: &CompletionEntry,
        args: &[String],
        __current_token: &str,
    ) -> bool {
        // let require_at_least_one = entry.directives.contains("@a");
        // if require_at_least_one {
        //     match args.len() {
        //         0 => return false,
        //         1.. => return true,
        //     }
        // }
        let reverse = !entry.directives.iter().any(|f| f == "@n");
        if entry.conditions.is_empty() {
            if reverse {
                return args.len() > 0;
            }
            return args.len() == 0;
        }
        for condition in &entry.conditions {
            // Check if any condition matches the args
            if args.iter().any(|a| condition == a) {
                return reverse;
            }
        }
        return false;
    }

    /**
     * check if one entry is matched
     * condition and argument, has to be filtered later
     */
    fn matches_context(
        &self,
        entry: &CompletionEntry,
        args: &[String],
        current_token: &str,
    ) -> MatchType {
        // check condition as subcommand
        // 无参数，若有condition则列出
        if current_token.starts_with("--") && entry.long_opt.is_some() {
            if current_token.len() == 2
                || entry
                    .long_opt
                    .as_ref()
                    .unwrap()
                    .starts_with(&current_token[2..])
            {
                if self.check_condition(entry, args, current_token) {
                    return MatchType::Long;
                }
            }
            return MatchType::None;
        } else if current_token.starts_with("-") && entry.short_opt.is_some() {
            if current_token.len() == 1
                || entry
                    .short_opt
                    .as_ref()
                    .unwrap()
                    .starts_with(&current_token[1..])
            {
                if self.check_condition(entry, args, current_token) {
                    return MatchType::Short;
                }
            }
            return MatchType::None;
            // 只检测正在输入
        } else if !current_token.is_empty() {
            // 有参数且不以-开始，匹配condition或action
            // 只有一个参数，有condition则匹配，无condition则action，并带出长短选项
            // 有多个参数，则检验是否满足长短选项
            // if args.len() == 1 {
            if self.check_condition(entry, args, current_token) {
                // 无condition则action，并带出长短选项
                if entry.args.iter().any(|x| x.starts_with(current_token)) {
                    if entry.long_opt.is_some() {
                        return MatchType::LongAndArgument; //长短皆可，默认？选择？
                    } else {
                        return MatchType::Argument;
                    }
                }
                // 继续匹配长短选项
                if entry
                    .long_opt
                    .as_ref()
                    .is_some_and(|x| x.starts_with(current_token))
                {
                    return MatchType::Long;
                }
                if entry
                    .short_opt
                    .as_ref()
                    .is_some_and(|x| x.starts_with(current_token))
                {
                    return MatchType::Short;
                }
            }
            // } else {
            //     // args >1 , 只匹配argument参数，但需要检验长短选项
            //     if entry
            //         .long_opt
            //         .as_ref()
            //         .is_some_and(|x| args.iter().any(|a| a == &format!("--{}", x)))
            //     {
            //         if entry.args.iter().any(|x| x.starts_with(current_token)) {
            //             return MatchType::LongAndArgument;
            //         }
            //     }
            //     if entry
            //         .short_opt
            //         .as_ref()
            //         .is_some_and(|x| args.iter().any(|a| a == &format!("-{}", x)))
            //     {
            //         if entry.args.iter().any(|x| x.starts_with(current_token)) {
            //             return MatchType::ShortAndArgument;
            //         }
            //     }
            // }
            return MatchType::None;
            // 当前单词为空
        } else {
            if self.check_condition(entry, args, current_token) {
                if entry.directives.iter().any(|d| d == "@F" || d == "@D") {
                    return MatchType::File;
                }
                if !entry.directives.iter().any(|d| d == "@r") {
                    if entry.short_opt.is_some() {
                        return MatchType::Short;
                    } else if entry.long_opt.is_some() {
                        return MatchType::Long;
                    } else if !entry.args.is_empty() {
                        return MatchType::Argument;
                    }
                } else {
                    return MatchType::Require; //TODO test this
                }
            }
        }
        MatchType::None
    }

    pub fn get_completions_for_context(
        &self,
        command: &str,
        args: &[String],
        current_token: &str,
    ) -> (Vec<Pair>, bool) {
        let mut v = Vec::<Pair>::new();
        if let Some(indices) = self.command_index.get(command) {
            for idx in indices {
                let entry = &self.entries[*idx];
                match self.matches_context(entry, args, current_token) {
                    MatchType::Short => v.push(Pair {
                        display: format!(
                            "-{} :{}",
                            entry.short_opt.as_ref().unwrap(),
                            entry.description
                        ),
                        replacement: format!("-{}", entry.short_opt.as_ref().unwrap()),
                    }),
                    MatchType::Long => v.push(Pair {
                        display: format!(
                            "--{} :{}",
                            entry.long_opt.as_ref().unwrap(),
                            entry.description
                        ),
                        replacement: format!("--{}", entry.long_opt.as_ref().unwrap()),
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
                                    display: format!("{} :{}", a, entry.description),
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
                    MatchType::LongAndArgument => {
                        // arg需要过滤，opt不一定存在，需要检测
                        for x in entry.args.iter() {
                            if x.starts_with(current_token) {
                                if let Some(long) = entry.long_opt.clone() {
                                    v.push(Pair {
                                        display: format!("--{} {} :{}", long, x, entry.description),
                                        replacement: format!("--{} {}", long, x),
                                    })
                                }
                            }
                        }

                        // v.push()
                    }
                    // MatchType::ShortAndArgument => {
                    //     // arg需要过滤，opt不一定存在，需要检测
                    //     for x in entry.args.iter() {
                    //         if x.starts_with(current_token) {
                    //             if let Some(short) = entry.short_opt.clone() {
                    //                 v.push(Pair {
                    //                     display: format!("-{} {}", short, x),
                    //                     replacement: format!("-{} {}", short, x),
                    //                 })
                    //             }
                    //         }
                    //     }

                    //     // v.push()
                    // }
                    MatchType::File => return (v, true),
                    MatchType::Require => {
                        v.push(Pair {
                            display: String::from("_ :param required"),
                            replacement: String::from("_"),
                        });
                    }
                    _ => {}
                };
            }
            (v, false)
        } else {
            (Vec::new(), false)
        }
    }
}
