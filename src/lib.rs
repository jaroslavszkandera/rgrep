use colored::Colorize;
use regex::Regex;
use std::error::Error;
use std::fs;
use walkdir::WalkDir;

pub struct Config {
    pub query: String,
    pub file_path: String,
    // Matching Control
    pub ignore_case: bool,
    pub invert_match: bool,
    pub word_regexp: bool,
    pub line_regexp: bool,
    // General Output Control
    pub count_matches: bool,
    pub color: bool,
    // Output Line Prefix Control
    pub line_number: bool,
    // File and Directory Selection
    pub recursive: bool,
    // Context Line Control
    pub after_context: usize,
    pub before_context: usize,
    pub group_separator: String,
}

impl Config {
    pub fn build(mut args: impl Iterator<Item = String>) -> Result<Config, &'static str> {
        args.next();

        let mut ignore_case = false;
        let mut no_ignore_case = false;
        let mut invert_match = false;
        let mut word_regexp = false;
        let mut line_regexp = false;
        let mut count_matches = false;
        let mut line_number = false;
        let mut color = false;
        let mut recursive = false;
        let mut query: Option<String> = None;
        let mut file_path: Option<String> = None;
        let mut after_context = 0;
        let mut before_context = 0;
        let mut group_separator = "--".to_string();
        let mut no_group_separator = false;

        fn parse_opt_val(arg: &str) -> Option<String> {
            arg.split_once('=').map(|(_, v)| v.to_string())
        }

        // TODO: Parse multiple options after dash (e.g. -in).
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "-i" | "--ignore-case" => ignore_case = true,
                "--no-ignore-case" => no_ignore_case = true,
                "-v" | "--invert-match" => invert_match = true,
                "-w" | "--word-regexp" => word_regexp = true,
                "-x" | "--line-regexp" => line_regexp = true,
                "-c" | "--count" => count_matches = true,
                "--color" => color = true,
                "-n" | "--line-number" => line_number = true,
                "-r" | "--recursive" => recursive = true,
                "-A" => after_context = args.next().and_then(|s| s.parse().ok()).unwrap_or(0),
                "-B" => before_context = args.next().and_then(|s| s.parse().ok()).unwrap_or(0),
                "-C" => {
                    let num = args.next().and_then(|s| s.parse().ok()).unwrap_or(0);
                    before_context = num;
                    after_context = num;
                }

                _ if arg.starts_with("--after-context") => {
                    after_context = parse_opt_val(&arg)
                        .and_then(|s| s.parse().ok())
                        .ok_or("Invalid context length argument")?;
                }
                _ if arg.starts_with("--before-context") => {
                    before_context = parse_opt_val(&arg)
                        .and_then(|s| s.parse().ok())
                        .ok_or("Invalid context length argument")?;
                }
                _ if arg.starts_with("--context") => {
                    let num = parse_opt_val(&arg)
                        .and_then(|s| s.parse().ok())
                        .ok_or("Invalid context length argument")?;
                    before_context = num;
                    after_context = num;
                }
                _ if arg.starts_with("--group_separator") => {
                    group_separator = parse_opt_val(&arg).unwrap_or("--".to_string())
                }
                "--no_group_separator" => no_group_separator = true,
                _ if query.is_none() => query = Some(arg),
                _ if file_path.is_none() => file_path = Some(arg),
                _ => return Err("Invalid option or too many arguments"),
            }
        }

        if no_ignore_case {
            invert_match = false;
        }
        if no_group_separator {
            group_separator.clear();
        }

        let query = query.ok_or("Didn't get a query string")?;
        let file_path = if recursive {
            file_path.unwrap_or_else(|| ".".to_string())
        } else {
            file_path.ok_or("Didn't get a file path")?
        };

        Ok(Config {
            query,
            file_path,
            ignore_case,
            line_regexp,
            word_regexp,
            invert_match,
            count_matches,
            line_number,
            color,
            recursive,
            after_context,
            before_context,
            group_separator,
        })
    }
}

fn build_regex(config: &Config) -> Regex {
    let mut pattern = regex::escape(&config.query);

    if config.word_regexp {
        pattern = format!(r"\b{}\b", pattern);
    }
    if config.line_regexp {
        pattern = format!(r"^{}$", pattern);
    }

    let regex_pattern = if config.ignore_case {
        format!("(?i){}", pattern)
    } else {
        pattern
    };

    Regex::new(&regex_pattern).unwrap()
}

pub fn run(config: &Config) -> Result<(), Box<dyn Error>> {
    let regex = build_regex(config);
    let mut needs_separator = false;

    if config.recursive {
        for entry in WalkDir::new(&config.file_path)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if entry.file_type().is_file() {
                let path = entry.path().display().to_string();
                if let Ok(contents) = fs::read_to_string(&path) {
                    let results = search(&contents, config, &regex, &path, &mut needs_separator);
                    for line in results {
                        println!("{}", line);
                    }
                }
            }
        }
    } else {
        let contents = fs::read_to_string(&config.file_path)?;
        let results = search(&contents, config, &regex, &"".to_string(), &mut false);
        for line in results {
            println!("{}", line);
        }
    }
    Ok(())
}

fn search(
    contents: &str,
    config: &Config,
    regex: &Regex,
    file_path: &str,
    needs_separator: &mut bool,
) -> Vec<String> {
    let mut results = Vec::new();
    let mut match_count = 0;
    let lines: Vec<&str> = contents.lines().collect();
    let mut after_context_cnt = 0;
    let mut last_match_index = 0;

    for (index, line) in lines.iter().enumerate() {
        let is_match = regex.is_match(line);
        if config.invert_match ^ is_match || after_context_cnt > 0 {
            if config.count_matches {
                match_count += 1;
                continue;
            }

            if *needs_separator
                && after_context_cnt == 0
                && (config.after_context > 0 || config.before_context > 0)
            {
                if config.color {
                    results.push(config.group_separator.cyan().to_string());
                } else {
                    results.push(config.group_separator.to_string());
                }
            }
            *needs_separator = true;

            if config.before_context > 0 && after_context_cnt == 0 {
                let start = index
                    .saturating_sub(config.before_context)
                    .max(last_match_index + config.after_context);
                if start <= (last_match_index + config.after_context) {
                    results.pop(); // Remove previous separator
                }
                for i in start..index {
                    results.push(format_line(i, &lines[i], config, &file_path));
                }
            }

            if config.invert_match ^ is_match {
                after_context_cnt = config.after_context;
            } else {
                after_context_cnt = after_context_cnt.saturating_sub(1);
            }

            let mut fmt_line = line.to_string();
            if config.color {
                fmt_line = regex
                    .replace_all(line, |caps: &regex::Captures| {
                        caps[0].red().bold().to_string()
                    })
                    .to_string();
            }
            results.push(format_line(index, &fmt_line, config, &file_path));

            last_match_index = index;
        }
    }

    if config.count_matches {
        vec![match_count.to_string()]
    } else {
        results
    }
}

fn format_line(index: usize, line: &str, config: &Config, file_path: &str) -> String {
    let mut fmt_line = "".to_string();
    if config.recursive {
        if config.color {
            fmt_line = format!("{}{}", file_path.purple(), ":".cyan());
        } else {
            fmt_line = format!("{}:", file_path);
        }
    }
    if config.line_number {
        if config.color {
            fmt_line = format!(
                "{}{}{}{}",
                fmt_line,
                (index + 1).to_string().green(),
                ":".cyan(),
                line
            );
        } else {
            fmt_line = format!("{}{}:{}", fmt_line, (index + 1).to_string(), line);
        }
    } else {
        fmt_line = format!("{}{}", fmt_line, line.to_string());
    }
    fmt_line
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_config() -> Config {
        Config {
            query: String::new(),
            file_path: String::new(),
            ignore_case: false,
            line_regexp: false,
            word_regexp: false,
            invert_match: false,
            count_matches: false,
            line_number: false,
            color: false,
            recursive: false,
            after_context: 0,
            before_context: 0,
            group_separator: "--".to_string(),
        }
    }

    #[test]
    fn case_sensitive_search() {
        let mut config = base_config();
        config.query = "duct".to_string();
        config.file_path = "".to_string();
        let contents = "Rust:\nsafe, fast, productive.\nsafe and fast.";
        let results = search(
            contents,
            &config,
            &build_regex(&config),
            &"".to_string(),
            &mut false,
        );
        assert_eq!(results, vec!["safe, fast, productive.".to_string()]);
    }

    #[test]
    fn case_insensitive_search() {
        let mut config = base_config();
        config.query = "rUsT".to_string();
        config.ignore_case = true;
        let contents = "Rust:\nTrust me.";
        let results = search(
            contents,
            &config,
            &build_regex(&config),
            &"".to_string(),
            &mut false,
        );
        assert_eq!(results, vec!["Rust:".to_string(), "Trust me.".to_string()]);
    }

    #[test]
    fn word_regexp_search() {
        let mut config = base_config();
        config.query = "me".to_string();
        config.word_regexp = true;
        let contents = "Me me\nme.\nmethod";
        let results = search(
            contents,
            &config,
            &build_regex(&config),
            &"".to_string(),
            &mut false,
        );
        assert_eq!(results, vec!["Me me".to_string(), "me.".to_string()]);
    }

    #[test]
    fn line_regex_search() {
        let mut config = base_config();
        config.query = "Rusty".to_string();
        config.line_regexp = true;
        let contents = "Rust\nRusty\nRusty \nCorosive";
        let results = search(
            contents,
            &config,
            &build_regex(&config),
            &"".to_string(),
            &mut false,
        );
        assert_eq!(results, vec!["Rusty".to_string()]);
    }
}
