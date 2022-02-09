mod cache;
mod gitlab;
mod human;

use std::env;

use anyhow::Result;
use chrono::DateTime;
use powerpack::Item;

#[derive(Debug)]
pub struct Object {
    title: String,
    author: String,
    url: String,
    created_at: DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Copy)]
enum Command {
    /// Search issues in connect-us.
    Issues,
    /// Search merge requests in core.
    Core,
}

impl Command {
    fn all() -> impl Iterator<Item = Command> {
        [Self::Issues, Self::Core].into_iter()
    }

    fn name(&self) -> &'static str {
        match *self {
            Self::Issues => "issues",
            Self::Core => "core",
        }
    }

    fn item(self) -> Item<'static> {
        let subtitle = match self {
            Command::Issues => "Search issues in connect-us",
            Command::Core => "Search merge requests in core",
        };
        Item::new(self.name())
            .subtitle(subtitle)
            .autocomplete(format!("{} ", self.name()))
    }

    fn exec(self, query: &str) -> Result<Vec<Item<'static>>> {
        let now = chrono::Utc::now();
        let items = match self {
            Command::Issues => gitlab::issues()?,
            Command::Core => gitlab::core()?,
        }
        .into_iter()
        .filter(|i| i.title.to_ascii_lowercase().contains(query))
        .map(|i| {
            powerpack::Item::new(i.title)
                .subtitle(format!(
                    "created {} by {}",
                    human::format_ago((now - i.created_at).to_std().unwrap()),
                    i.author
                ))
                .arg(i.url)
        })
        .collect();
        Ok(items)
    }
}

fn main() -> Result<()> {
    let arg = env::args()
        .nth(1)
        .as_deref()
        .map(str::trim)
        .map(str::to_ascii_lowercase);

    let items = match arg {
        // If no argument is given then just list the available commands.
        None => Command::all().map(|c| c.item()).collect(),

        // Otherwise process the argument.
        Some(arg) => {
            // Get the command and the search query.
            let (cmd, query) = arg.split_once(char::is_whitespace).unwrap_or((&arg, ""));

            match Command::all().find(|c| c.name() == cmd) {
                // There is a command that matches this query so execute it.
                Some(cmd) => cmd.exec(query)?,

                // No command matches the query exactly, output the commands
                // that start with the half-entered command.
                None => Command::all()
                    .filter_map(|c| c.name().starts_with(cmd).then(|| c.item()))
                    .collect(),
            }
        }
    };

    powerpack::output(items)?;
    Ok(())
}
