use std::collections::HashMap;
use std::fs::read_to_string;

#[derive(Debug, Clone)]
pub struct CompletionEntry {
    pub command: String,
    pub conditions: Vec<String>, // Split by spaces
    pub short_opt: Option<String>,
    pub long_opt: Option<String>,
    pub args: Vec<String>, // @F, @D, or specific values
    pub priority: i32,
    pub description: String,
}

#[derive(Debug, Clone)]
pub struct CompletionDatabase {
    entries: Vec<CompletionEntry>,
    // Index for faster lookup: command -> entries
    command_index: HashMap<String, Vec<usize>>,
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
                priority: parts[5].trim().parse().unwrap_or(0),
                description: parts[6].trim().to_string(),
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
        let mut if_match = true;
        if entry.conditions.is_empty() {
            return true;
        }
        for condition in &entry.conditions {
            match condition.as_str() {
                "_" => {
                    // Must have at least one subcommand
                    if args.len() < 1 {
                        return false;
                    }
                    return true;
                }
                "!" => {
                    if_match = false;
                }
                _ => {
                    // Check if any condition matches the args
                    if args.iter().any(|token| condition == token) {
                        // dbg!("---found entry---", &entry);
                        return if_match;
                    }
                }
            }
        }
        return false;
    }

    fn matches_context(
        &self,
        entry: &CompletionEntry,
        args: &[String],
        current_token: &str,
    ) -> bool {
        // Handle special conditions
        let cond_ok = self.check_condition(entry, args, current_token);
        // dbg!(&entry, &args, &cond_o);
        if cond_ok {
            if let Some(short) = entry.short_opt.as_ref()
                && !args.contains(short)
            {
                return true;
            }
            if let Some(long) = entry.long_opt.as_ref()
                && !args.contains(long)
            {
                return true;
            }
            if !entry.args.is_empty() && !args.iter().any(|token| entry.args.contains(token)) {
                return true;
            }
        }
        return cond_ok;
        // Check if current token could match this entry
        // if let Some(ref short_opt) = entry.short_opt {
        //     if current_token.starts_with('-') && current_token.len() > 1 {
        //         let char_after_dash = current_token.chars().nth(1).unwrap_or('\0');
        //         if short_opt.contains(char_after_dash) {
        //             return true;
        //         }
        //     }
        // }

        // if let Some(ref long_opt) = entry.long_opt {
        //     if current_token.starts_with("--") && long_opt.starts_with(&current_token[2..]) {
        //         return true;
        //     }
        // }

        // false
    }

    pub fn get_completions_for_context(
        &self,
        command: &str,
        args: &[String],
        current_token: &str,
    ) -> Vec<&CompletionEntry> {
        if let Some(indices) = self.command_index.get(command) {
            indices
                .iter()
                .filter_map(|&idx| {
                    let entry = &self.entries[idx];
                    if self.matches_context(entry, args, current_token) {
                        Some(entry)
                    } else {
                        None
                    }
                })
                .collect()
        } else {
            Vec::new()
        }
    }
}
