mod cache;
mod config;
mod gitlab;
mod human;
mod logger;

use std::cmp::Reverse;
use std::env;
use std::io;
use std::iter;
use std::time::Duration;

use anyhow::Result;
use chrono::DateTime;
use powerpack::Item;
use serde::Deserialize;

use crate::config::{Command, Kind, CONFIG};

#[derive(Debug)]
pub struct Issue {
    title: String,
    author: User,
    assignees: Vec<User>,
    url: String,
    created_at: DateTime<chrono::Utc>,
    labels: Vec<String>,
}

#[derive(Debug)]
pub struct MergeRequest {
    title: String,
    author: User,
    url: String,
    created_at: DateTime<chrono::Utc>,
    labels: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct User {
    name: String,
    username: String,
}

impl Issue {
    fn ours_first(&self) -> Reverse<(bool, bool)> {
        let is_ours = CONFIG
            .user
            .as_ref()
            .map(|u| {
                (
                    self.assignees.iter().any(|a| a.matches(u)),
                    self.author.matches(u),
                )
            })
            .unwrap_or((false, false));
        Reverse(is_ours)
    }

    fn matches(&self, query: &str) -> bool {
        query.split_whitespace().all(|q| {
            if let Some(q) = q.strip_prefix('~') {
                self.labels
                    .iter()
                    .any(|label| label.to_lowercase().contains(q))
            } else if let Some(q) = q.strip_prefix('@') {
                self.author.matches(q) || self.assignees.iter().any(|a| a.matches(q))
            } else {
                self.title.to_lowercase().contains(q)
            }
        })
    }

    fn into_item(self, now: chrono::DateTime<chrono::Utc>) -> Item {
        let ago = human::format_ago((now - self.created_at).to_std().unwrap());
        let subtitle = if self.assignees.is_empty() {
            format!("{}, authored by {}", ago, self.author.name)
        } else {
            format!(
                "{}, assigned to {}",
                ago,
                self.assignees
                    .iter()
                    .map(|u| u.name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        };
        let arg = format!("{};{}", &self.url, &self.title);
        powerpack::Item::new(self.title).subtitle(subtitle).arg(arg)
    }
}

impl MergeRequest {
    fn ours_first(&self) -> Reverse<bool> {
        let is_ours = CONFIG
            .user
            .as_ref()
            .map(|u| self.author.matches(u))
            .unwrap_or(false);
        Reverse(is_ours)
    }

    fn matches(&self, query: &str) -> bool {
        query.split_whitespace().all(|q| {
            if let Some(q) = q.strip_prefix('~') {
                self.labels
                    .iter()
                    .any(|label| label.to_lowercase().contains(q))
            } else if let Some(q) = q.strip_prefix('@') {
                self.author.matches(q)
            } else {
                self.title.to_lowercase().contains(q)
            }
        })
    }

    fn into_item(self, now: chrono::DateTime<chrono::Utc>) -> Item {
        let ago = human::format_ago((now - self.created_at).to_std().unwrap());
        let subtitle = format!("{} by {}", ago, self.author.name);
        let arg = format!("{};{}", &self.url, &self.title);
        powerpack::Item::new(self.title).subtitle(subtitle).arg(arg)
    }
}

impl User {
    fn matches(&self, query: &str) -> bool {
        self.name.to_lowercase().contains(query) || self.username.to_lowercase().contains(query)
    }
}

impl Command {
    fn to_item(&self) -> Item {
        let subtitle = match self.kind {
            Kind::Issues => format!("Search issues in {}", self.project),
            Kind::MergeRequests => format!("Search merge requests in {}", self.project),
        };
        Item::new(&self.name)
            .subtitle(subtitle)
            .arg(format!("https://gitlab.com/{};{}", self.project, self.name))
            .autocomplete(format!("{} ", self.name))
    }

    fn exec(&self, query: &str) -> Result<Vec<Item>> {
        let now = chrono::Utc::now();

        let items = match self.kind {
            Kind::Issues => {
                let mut items = Vec::new();
                if let Some(query) = query.strip_prefix('/') {
                    for (cmd, f) in EXTRAS {
                        if cmd.starts_with(query) {
                            items.push(f(&self.project));
                        }
                    }
                }
                let issues = {
                    let mut issues = gitlab::issues(&self.name, &self.project)?;
                    issues.sort_by_key(Issue::ours_first);
                    issues
                        .into_iter()
                        .filter_map(|i| i.matches(query).then(|| i.into_item(now)))
                };
                items.extend(issues);
                items
            }
            Kind::MergeRequests => {
                let mut merge_requests = gitlab::merge_requests(&self.name, &self.project)?;
                merge_requests.sort_by_key(MergeRequest::ours_first);
                merge_requests
                    .into_iter()
                    .filter_map(|m| m.matches(query).then(|| m.into_item(now)))
                    .collect()
            }
        };

        Ok(items)
    }
}

type ItemFn = fn(&str) -> Item;

const EXTRAS: &[(&str, ItemFn)] = &[
    ("new", new_item),
    ("boards", boards_item),
    ("list", list_item),
];

fn new_item(project: &str) -> Item {
    Item::new("/new")
        .subtitle(format!("Create a new issue in {}", project))
        .arg(format!("https://gitlab.com/{}/issues/new", project))
}

fn boards_item(project: &str) -> Item {
    let p = project.trim_end_matches('/');
    let p = p.rsplit_once('/').map(|(p, _)| p).unwrap_or(p);
    Item::new("/boards")
        .subtitle(format!("Open the issue boards for {}", project))
        .arg(format!("https://gitlab.com/groups/{}/-/boards", p))
}

fn list_item(project: &str) -> Item {
    Item::new("/list")
        .subtitle(format!("Open the issue list for {}", project))
        .arg(format!("https://gitlab.com/{}/-/issues", project))
}

fn run() -> Result<()> {
    let arg = env::args()
        .nth(1)
        .as_deref()
        .map(str::trim)
        .map(str::to_lowercase);

    let items = match arg {
        // If no argument is given then just list the available commands.
        None => CONFIG.commands.iter().map(Command::to_item).collect(),

        // Otherwise process the argument.
        Some(arg) => {
            // Get the command and the search query.
            let (cmd, query) = arg.split_once(char::is_whitespace).unwrap_or((&arg, ""));

            match CONFIG.commands.iter().find(|c| c.name == cmd) {
                // There is a command that matches this query so execute it.
                Some(cmd) => cmd.exec(query)?,

                // No command matches the query exactly, output the commands
                // that start with the half-entered command.
                None => CONFIG
                    .commands
                    .iter()
                    .filter_map(|c| c.name.starts_with(cmd).then(|| c.to_item()))
                    .collect(),
            }
        }
    };

    powerpack::Output::new()
        .items(items)
        .rerun(Duration::from_secs(1))
        .write(io::stdout())?;

    Ok(())
}

fn main() -> Result<()> {
    if let Err(err) = run() {
        eprintln!("{:#}", err);
        let item = Item::new(format!("Error: {}", err)).subtitle(
            "The workflow errored! \
             You might want to try debugging it or checking the logs.",
        );
        powerpack::output(iter::once(item))?;
    }
    Ok(())
}
