mod gitlab;

use std::env;

use anyhow::Result;
use powerpack::Item;

#[derive(Debug)]
pub struct Issue {
    title: String,
    url: String,
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
        let items = match self {
            Command::Issues => gitlab::issues::list()?,
            Command::Core => gitlab::core::list()?,
        }
        .into_iter()
        .filter(|issue| issue.title.to_ascii_lowercase().contains(query))
        .map(|issue| powerpack::Item::new(issue.title).arg(issue.url))
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
