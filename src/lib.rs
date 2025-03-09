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
    pub color: bool,
}

impl Config {
    pub fn build(mut args: impl Iterator<Item = String>) -> Result<Config, &'static str> {
        args.next();

        let mut ignore_case = false;
        let mut line_regexp = false;
        let mut word_regexp = false;
        let mut invert_match = false;
        let mut color = false;
        let mut query: Option<String> = None;
        let mut file_path: Option<String> = None;

        while let Some(arg) = args.next() {
            match arg.as_str() {
                "-i" | "--ignore-case" => ignore_case = true,
                "-x" | "--line-regexp" => line_regexp = true,
                "-w" | "--word-regexp" => word_regexp = true,
                "-v" | "--invert_match" => invert_match = true,
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
    let regex = build_regex(config);

    for line in contents.lines() {
        let is_match = regex.is_match(line);
        if config.invert_match ^ is_match {
            if config.color {
                let highlighted = regex.replace_all(line, |caps: &regex::Captures| {
                    caps[0].red().bold().to_string()
                });
                println!("{}", highlighted);
            } else {
                println!("{}", line);
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn case_sensitive() {
        let config = Config {
            query: "duct".to_string(),
            file_path: "".to_string(),
            ignore_case: false,
            line_regexp: false,
            word_regexp: false,
            invert_match: false,
            color: false,
        };
        let regex = build_regex(&config);
        assert!(regex.is_match("safe, fast, productive."));
        assert!(!regex.is_match("safe and fast."));
    }

    #[test]
    fn case_insensitive() {
        let config = Config {
            query: "rUsT".to_string(),
            file_path: "".to_string(),
            ignore_case: true,
            line_regexp: false,
            word_regexp: false,
            invert_match: false,
            color: false,
        };
        let regex = build_regex(&config);
        assert!(regex.is_match("Rust:"));
        assert!(regex.is_match("Trust me."));
    }

    #[test]
    fn line_regexp() {
        let config = Config {
            query: "me.".to_string(),
            file_path: "".to_string(),
            ignore_case: false,
            line_regexp: true,
            word_regexp: false,
            invert_match: false,
            color: false,
        };
        let regex = build_regex(&config);
        assert!(regex.is_match("me."));
        assert!(!regex.is_match("meme."));
        assert!(!regex.is_match("Meme."));
    }

    #[test]
    fn word_regexp() {
        let config = Config {
            query: "me".to_string(),
            file_path: "".to_string(),
            ignore_case: false,
            line_regexp: false,
            word_regexp: true,
            invert_match: false,
            color: false,
        };
        let regex = build_regex(&config);
        assert!(regex.is_match("Me me"));
        assert!(regex.is_match("me."));
        assert!(!regex.is_match("method"));
    }
}
