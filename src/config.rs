use std::env;

use once_cell::sync::Lazy;

pub static CONFIG: Lazy<Config> = Lazy::new(Config::load);

#[derive(Debug)]
pub struct Config {
    pub token: Option<String>,
    pub commands: Vec<Command>,
}

#[derive(Debug)]
pub struct Command {
    pub kind: Kind,
    pub name: String,
    pub project: String,
}

#[derive(Debug)]
pub enum Kind {
    Issues,
    MergeRequests,
}

impl Config {
    fn load() -> Self {
        let mut token = None;
        let mut commands = Vec::new();
        for (k, v) in env::vars() {
            if v.is_empty() {
                continue;
            }
            if k == "GITLAB_TOKEN" {
                token = Some(v);
            } else if let Some(name) = k.strip_prefix("GITLAB_ISSUES_") {
                commands.push(Command {
                    kind: Kind::Issues,
                    name: name.to_lowercase().replace("_", "-"),
                    project: v,
                })
            } else if let Some(name) = k.strip_prefix("GITLAB_MERGE_REQUESTS_") {
                commands.push(Command {
                    kind: Kind::MergeRequests,
                    name: name.to_lowercase().replace("_", "-"),
                    project: v,
                })
            }
        }
        Config { token, commands }
    }
}
