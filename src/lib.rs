use colored::Colorize;
use regex::Regex;
use std::error::Error;
use std::fs;

pub struct Config {
    pub query: String,
    pub file_path: String,
    pub ignore_case: bool,
    pub line_regexp: bool,
    pub word_regexp: bool,
    pub invert_match: bool,
    pub count_matches: bool,
    pub line_number: bool,
    pub color: bool,
}

impl Config {
    pub fn build(mut args: impl Iterator<Item = String>) -> Result<Config, &'static str> {
        args.next();

        let mut ignore_case = false;
        let mut line_regexp = false;
        let mut word_regexp = false;
        let mut invert_match = false;
        let mut count_matches = false;
        let mut line_number = false;
        let mut color = false;
        let mut query: Option<String> = None;
        let mut file_path: Option<String> = None;

        while let Some(arg) = args.next() {
            match arg.as_str() {
                "-i" | "--ignore-case" => ignore_case = true,
                "-x" | "--line-regexp" => line_regexp = true,
                "-w" | "--word-regexp" => word_regexp = true,
                "-v" | "--invert-match" => invert_match = true,
                "-c" | "--count" => count_matches = true,
                "-n" | "--line-number" => line_number = true,
                "--color" => color = true,
                _ if query.is_none() => query = Some(arg),
                _ if file_path.is_none() => file_path = Some(arg),
                _ => return Err("Invalid option or too many arguments"),
            }
        }

        let query = query.ok_or("Didn't get a query string")?;
        let file_path = file_path.ok_or("Didn't get a file path")?;

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
    let contents = fs::read_to_string(&config.file_path)?;
    let results = search(&contents, config);
    for line in results {
        println!("{}", line);
    }
    Ok(())
}

fn search(contents: &str, config: &Config) -> Vec<String> {
    let regex = build_regex(config);
    let mut results = Vec::new();
    let mut match_count = 0;

    for (index, line) in contents.lines().enumerate() {
        let is_match = regex.is_match(line);
        if config.invert_match ^ is_match {
            if config.count_matches {
                match_count += 1;
                continue;
            }

            let mut fmt_line = line.to_string();
            if config.color {
                fmt_line = regex
                    .replace_all(line, |caps: &regex::Captures| {
                        caps[0].red().bold().to_string()
                    })
                    .to_string();
            }
            if config.line_number {
                fmt_line = format!("{}:{}", index + 1, fmt_line);
            }
            results.push(fmt_line);
        }
    }

    if config.count_matches {
        results.push(match_count.to_string());
    }
    results
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
        }
    }

    #[test]
    fn case_sensitive_search() {
        let mut config = base_config();
        config.query = "duct".to_string();
        config.file_path = "".to_string();
        let contents = "Rust:\nsafe, fast, productive.\nsafe and fast.";
        let results = search(contents, &config);
        assert_eq!(results, vec!["safe, fast, productive.".to_string()]);
    }

    #[test]
    fn case_insensitive_search() {
        let mut config = base_config();
        config.query = "rUsT".to_string();
        config.ignore_case = true;
        let contents = "Rust:\nTrust me.";
        let results = search(contents, &config);
        assert_eq!(results, vec!["Rust:".to_string(), "Trust me.".to_string()]);
    }

    #[test]
    fn word_regexp_search() {
        let mut config = base_config();
        config.query = "me".to_string();
        config.word_regexp = true;
        let contents = "Me me\nme.\nmethod";
        let results = search(contents, &config);
        assert_eq!(results, vec!["Me me".to_string(), "me.".to_string()]);
    }

    #[test]
    fn line_regex_search() {
        let mut config = base_config();
        config.query = "Rusty".to_string();
        config.line_regexp = true;
        let contents = "Rust\nRusty\nRusty \nCorosive";
        let results = search(contents, &config);
        assert_eq!(results, vec!["Rusty".to_string()]);
    }
}
