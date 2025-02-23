use std::env;
use std::sync::LazyLock;

pub static CONFIG: LazyLock<Config> = LazyLock::new(Config::load);

#[derive(Debug)]
pub struct Config {
    /// The configured GitLab auth token.
    pub token: Option<String>,

    /// The configured GitLab user.
    pub user: Option<String>,

    /// Whether to enable shortcuts.
    pub shortcuts: bool,

    /// List of available commands based on the configuration.
    pub commands: Vec<Command>,
}

#[derive(Debug)]
pub struct Command {
    /// The kind of command.
    pub kind: Kind,
    /// The name of the command.
    pub name: String,
    /// The project to query.
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
        let mut user = None;
        let mut shortcuts = false;
        let mut commands = Vec::new();

        for (k, v) in env::vars() {
            if v.is_empty() {
                continue;
            }
            if k == "GITLAB_USER" {
                user = Some(v);
            } else if k == "GITLAB_SHORTCUTS" && matches!(&*v, "1" | "true") {
                shortcuts = true;
            } else if k == "GITLAB_TOKEN" {
                token = Some(v);
            } else if let Some(name) = k.strip_prefix("GITLAB_ISSUES_") {
                commands.push(Command {
                    kind: Kind::Issues,
                    name: name.to_lowercase().replace('_', "-"),
                    project: v,
                });
            } else if let Some(name) = k.strip_prefix("GITLAB_MERGE_REQUESTS_") {
                commands.push(Command {
                    kind: Kind::MergeRequests,
                    name: name.to_lowercase().replace('_', "-"),
                    project: v,
                });
            }
        }

        Self {
            token,
            user,
            shortcuts,
            commands,
        }
    }
}

impl Command {
    pub fn kind(&self) -> &str {
        match self.kind {
            Kind::Issues => "issues",
            Kind::MergeRequests => "merge requests",
        }
    }
}
