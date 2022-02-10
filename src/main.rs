mod cache;
mod config;
mod gitlab;
mod human;
mod logger;

use std::{env, iter};

use anyhow::Result;
use chrono::DateTime;
use powerpack::Item;

use crate::config::{Command, Kind, CONFIG};

#[derive(Debug)]
pub struct Issue {
    title: String,
    author: String,
    assignees: Vec<String>,
    url: String,
    created_at: DateTime<chrono::Utc>,
    labels: Vec<String>,
}

#[derive(Debug)]
pub struct MergeRequest {
    title: String,
    author: String,
    url: String,
    created_at: DateTime<chrono::Utc>,
    labels: Vec<String>,
}

impl Issue {
    fn matches(&self, query: &str) -> bool {
        query.split_whitespace().all(|q| {
            if let Some(q) = q.strip_prefix('~') {
                self.labels
                    .iter()
                    .any(|label| label.to_lowercase().contains(q))
            } else if let Some(q) = q.strip_prefix('@') {
                self.author.to_lowercase().contains(q)
                    || self.assignees.iter().any(|a| a.to_lowercase().contains(q))
            } else {
                self.title.to_lowercase().contains(q)
            }
        })
    }

    fn into_item(self, now: chrono::DateTime<chrono::Utc>) -> Item<'static> {
        let ago = human::format_ago((now - self.created_at).to_std().unwrap());
        let subtitle = if self.assignees.is_empty() {
            format!("{}, authored by {}", ago, self.author)
        } else {
            format!("{}, assigned to {}", ago, self.assignees.join(", "))
        };
        powerpack::Item::new(self.title)
            .subtitle(subtitle)
            .arg(self.url)
    }
}

impl MergeRequest {
    fn matches(&self, query: &str) -> bool {
        query.split_whitespace().all(|q| {
            if let Some(q) = q.strip_prefix('~') {
                self.labels
                    .iter()
                    .any(|label| label.to_lowercase().contains(q))
            } else if let Some(q) = q.strip_prefix('@') {
                self.author.to_lowercase().contains(q)
            } else {
                self.title.to_lowercase().contains(q)
            }
        })
    }

    fn into_item(self, now: chrono::DateTime<chrono::Utc>) -> Item<'static> {
        let ago = human::format_ago((now - self.created_at).to_std().unwrap());
        let subtitle = format!("{} by {}", ago, self.author);
        powerpack::Item::new(self.title)
            .subtitle(subtitle)
            .arg(self.url)
    }
}

impl Command {
    fn to_item(&self) -> Item<'_> {
        let subtitle = match self.kind {
            Kind::Issues => format!("Search issues in {}", self.project),
            Kind::MergeRequests => format!("Search merge requests in {}", self.project),
        };
        Item::new(&self.name)
            .subtitle(subtitle)
            .arg(format!("https://gitlab.com/{}", self.project))
            .autocomplete(format!("{} ", self.name))
    }

    fn exec(&self, query: &str) -> Result<Vec<Item<'static>>> {
        let now = chrono::Utc::now();

        let items = match self.kind {
            Kind::Issues => gitlab::issues(&self.name, &self.project)?
                .into_iter()
                .filter_map(|i| i.matches(query).then(|| i.into_item(now)))
                .collect(),
            Kind::MergeRequests => gitlab::merge_requests(&self.name, &self.project)?
                .into_iter()
                .filter_map(|m| m.matches(query).then(|| m.into_item(now)))
                .collect(),
        };

        Ok(items)
    }
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

    powerpack::output(items)?;

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
