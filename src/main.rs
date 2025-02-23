mod config;
mod gitlab;
mod human;

use std::cmp::Reverse;
use std::env;
use std::io;
use std::time::Duration;

use anyhow::Result;
use constcat::concat;
use itermore::IterSorted;
use powerpack::logger;
use powerpack::Item;
use serde::Deserialize;

use crate::config::{Command, Kind, CONFIG};

const PKG_NAME: &str = env!("CARGO_PKG_NAME");
const PKG_VERSION: &str = env!("CARGO_PKG_VERSION");
const LOG_FILENAME: &str = concat!(PKG_NAME, "-", PKG_VERSION, ".log");

#[derive(Debug)]
pub struct Issue {
    title: String,
    author: User,
    assignees: Vec<User>,
    url: String,
    created_at: jiff::Timestamp,
    labels: Vec<String>,
}

#[derive(Debug)]
pub struct MergeRequest {
    title: String,
    author: User,
    url: String,
    created_at: jiff::Timestamp,
    labels: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct User {
    name: String,
    username: String,
}

impl Issue {
    fn cmp_key(&self) -> impl Ord {
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

    fn into_item(self, now: jiff::Timestamp) -> Item {
        let Self { title, url, .. } = self;
        let ago = human::format_ago((now - self.created_at).try_into().unwrap());
        let subtitle = if self.assignees.is_empty() {
            let author = self.author.name;
            format!("{ago}, authored by {author}")
        } else {
            let assignees = self
                .assignees
                .iter()
                .map(|u| u.name.as_str())
                .collect::<Vec<_>>()
                .join(", ");
            format!("{ago}, assigned to {assignees}")
        };
        let arg = format!("{url};{title}");
        powerpack::Item::new(title).subtitle(subtitle).arg(arg)
    }
}

impl MergeRequest {
    fn cmp_key(&self) -> impl Ord {
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

    fn into_item(self, now: jiff::Timestamp) -> Item {
        let Self { title, url, .. } = self;
        let ago = human::format_ago((now - self.created_at).try_into().unwrap());
        let author = self.author.name;
        let subtitle = format!("{ago} by {author}");
        let arg = format!("{url};{title}");
        powerpack::Item::new(title).subtitle(subtitle).arg(arg)
    }
}

impl User {
    fn matches(&self, query: &str) -> bool {
        self.name.to_lowercase().contains(query) || self.username.to_lowercase().contains(query)
    }
}

impl Command {
    fn to_item(&self) -> Item {
        let Self { name, project, .. } = self;
        let subtitle = match self.kind {
            Kind::Issues => format!("Search issues in {project}"),
            Kind::MergeRequests => format!("Search merge requests in {project}"),
        };
        Item::new(&self.name)
            .subtitle(subtitle)
            .arg(format!("https://gitlab.com/{project};{name}"))
            .autocomplete(format!("{name} "))
    }

    fn exec(&self, query: &str) -> Result<Vec<Item>> {
        let now = jiff::Timestamp::now();

        let items = match self.kind {
            Kind::Issues => {
                let mut items = Vec::new();
                if let Some(query) = query.strip_prefix('/') {
                    if CONFIG.shortcuts {
                        for (cmd, f) in SHORTCUTS {
                            if cmd.starts_with(query) {
                                items.push(f(&self.project));
                            }
                        }
                    }
                }

                let issues = gitlab::issues(&self.name, &self.project)?
                    .into_iter()
                    .sorted_by_key(Issue::cmp_key)
                    .filter(|i| i.matches(query))
                    .map(|i| i.into_item(now));

                items.extend(issues);
                items
            }
            Kind::MergeRequests => gitlab::merge_requests(&self.name, &self.project)?
                .into_iter()
                .sorted_by_key(MergeRequest::cmp_key)
                .filter(|m| m.matches(query))
                .map(|m| m.into_item(now))
                .collect(),
        };

        Ok(items)
    }
}

type ItemFn = fn(&str) -> Item;

const SHORTCUTS: &[(&str, ItemFn)] = &[
    ("new", new_item),
    ("boards", boards_item),
    ("list", list_item),
];

fn new_item(project: &str) -> Item {
    Item::new("/new")
        .subtitle(format!("Create a new issue in {project}"))
        .arg(format!("https://gitlab.com/{project}/issues/new"))
}

fn boards_item(project: &str) -> Item {
    let p = project.trim_end_matches('/');
    let p = p.rsplit_once('/').map(|(p, _)| p).unwrap_or(p);
    Item::new("/boards")
        .subtitle(format!("Open the issue boards for {project}"))
        .arg(format!("https://gitlab.com/groups/{p}/-/boards"))
}

fn list_item(project: &str) -> Item {
    Item::new("/list")
        .subtitle(format!("Open the issue list for {project}"))
        .arg(format!("https://gitlab.com/{project}/-/issues"))
}

fn run() -> Result<()> {
    logger::Builder::new().filename(LOG_FILENAME).try_init()?;

    let arg = env::args()
        .nth(1)
        .as_deref()
        .map(str::trim)
        .map(str::to_lowercase);

    if CONFIG.commands.is_empty() {
        let item = Item::new("No commands configured yet")
            .subtitle("Configure commands for this workflow using environment variables");
        return output([item]);
    }

    let items = match arg {
        // If no argument is given then just list the available commands.
        None => CONFIG.commands.iter().map(Command::to_item).collect(),

        // Otherwise process the argument
        Some(arg) => {
            // Get the command and the search query
            let (cmd, query) = arg.split_once(char::is_whitespace).unwrap_or((&arg, ""));

            match CONFIG.commands.iter().find(|c| c.name == cmd) {
                // There is a command that matches this query so execute it
                Some(command) => {
                    let items = command.exec(query)?;
                    if items.is_empty() {
                        let item = Item::new(format!("No {} found", command.kind()));
                        return output([item]);
                    }
                    items
                }

                // No command matches the query exactly, output the commands
                // that start with the half-entered command
                None => {
                    let items: Vec<_> = CONFIG
                        .commands
                        .iter()
                        .filter(|c| c.name.starts_with(cmd))
                        .map(Command::to_item)
                        .collect();
                    if items.is_empty() {
                        let item = Item::new("No command found");
                        return output([item]);
                    }
                    items
                }
            }
        }
    };

    output(items)
}

fn main() -> Result<()> {
    if let Err(err) = run() {
        eprintln!("{err:#}");
        let item = Item::new(format!("Error: {err}")).subtitle(
            "The workflow errored! \
             You might want to try debugging it or checking the logs.",
        );
        output([item])?;
    }
    Ok(())
}

fn output(items: impl IntoIterator<Item = Item>) -> Result<()> {
    powerpack::Output::new()
        .items(items)
        .rerun(Duration::from_secs(2))
        .write(io::stdout())?;
    Ok(())
}
