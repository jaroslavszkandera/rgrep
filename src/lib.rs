use std::error::Error;
use std::fs;

pub struct Config {
    pub query: String,
    pub file_path: String,
    pub ignore_case: bool,
    pub line_regexp: bool,
}

impl Config {
    pub fn build(mut args: impl Iterator<Item = String>) -> Result<Config, &'static str> {
        args.next();

        let mut ignore_case = false;
        let mut line_regexp = false;
        let mut query: Option<String> = None;
        let mut file_path: Option<String> = None;

        while let Some(arg) = args.next() {
            if arg.starts_with("-") {
                if arg == "-i" || arg == "--ignore-case" {
                    ignore_case = true;
                } else if arg == "-x" || arg == "--line-regexp" {
                    line_regexp = true;
                } else {
                    return Err("Invalid option");
                }
            } else if query.is_none() {
                query = Some(arg);
            } else if file_path.is_none() {
                file_path = Some(arg);
            } else {
                return Err("Too many arguments");
            }
        }

        let query = match query {
            Some(q) => q,
            None => return Err("Didn't get a query string"),
        };
        let file_path = match file_path {
            Some(fp) => fp,
            None => return Err("Didn't get a file path"),
        };

        Ok(Config {
            query,
            file_path,
            ignore_case,
            line_regexp,
        })
    }
}

pub fn run(config: Config) -> Result<(), Box<dyn Error>> {
    let contents = fs::read_to_string(config.file_path)?;

    let results = if config.ignore_case {
        search_case_insensitive(&config.query, &contents)
    } else if config.line_regexp {
        search_line_regexp(&config.query, &contents)
    } else {
        search(&config.query, &contents)
    };

    for line in results {
        println!("{line}")
    }

    Ok(())
}

fn search<'a>(query: &str, contents: &'a str) -> Vec<&'a str> {
    contents
        .lines()
        .filter(|line| line.contains(query))
        .collect()
}

fn search_case_insensitive<'a>(query: &str, contents: &'a str) -> Vec<&'a str> {
    contents
        .lines()
        .filter(|line| line.to_lowercase().contains(&query.to_lowercase()))
        .collect()
}

fn search_line_regexp<'a>(query: &str, contents: &'a str) -> Vec<&'a str> {
    contents.lines().filter(|line| *line == query).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn case_sensitive() {
        let query = "duct";
        let contents = "\
Rust:
safe, fast, productive.
Pick three.
Duct tape.";

        assert_eq!(vec!["safe, fast, productive."], search(query, contents));
    }

    #[test]
    fn case_insensitive() {
        let query = "rUsT";
        let contents = "\
Rust:
safe, fast, productive.
Pick three.
Trust me.";

        assert_eq!(
            vec!["Rust:", "Trust me."],
            search_case_insensitive(query, contents)
        );
    }

    #[test]
    fn line_regexp() {
        let query = "me.";
        let contents = "\
me.
Me.
Meme.
meme.";

        assert_eq!(vec!["me."], search_line_regexp(query, contents));
    }

    #[test]
    fn two_args() {
        let args = vec![
            "program_name".to_string(),
            "query".to_string(),
            "file.txt".to_string(),
        ];
        let config = Config::build(args.into_iter()).unwrap();

        assert_eq!(config.query, "query");
        assert_eq!(config.file_path, "file.txt");
        assert_eq!(config.ignore_case, false);
        assert_eq!(config.line_regexp, false);
    }

    #[test]
    fn ignore_case_arg() {
        let args = vec![
            "program_name".to_string(),
            "-i".to_string(),
            "query".to_string(),
            "file.txt".to_string(),
        ];
        let config = Config::build(args.into_iter()).unwrap();

        assert_eq!(config.query, "query");
        assert_eq!(config.file_path, "file.txt");
        assert_eq!(config.ignore_case, true);
        assert_eq!(config.line_regexp, false);
    }

    #[test]
    fn line_regexp_arg() {
        let args = vec![
            "program_name".to_string(),
            "-x".to_string(),
            "query".to_string(),
            "file.txt".to_string(),
        ];
        let config = Config::build(args.into_iter()).unwrap();

        assert_eq!(config.query, "query");
        assert_eq!(config.file_path, "file.txt");
        assert_eq!(config.ignore_case, false);
        assert_eq!(config.line_regexp, true);
    }
}
