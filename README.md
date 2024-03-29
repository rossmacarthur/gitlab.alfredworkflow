# gitlab.alfredworkflow

[![Build Status](https://img.shields.io/github/actions/workflow/status/rossmacarthur/gitlab.alfredworkflow/build.yaml?branch=trunk)](https://github.com/rossmacarthur/gitlab.alfredworkflow/actions/workflows/build.yaml)
[![Latest release](https://img.shields.io/github/v/release/rossmacarthur/gitlab.alfredworkflow)](https://github.com/rossmacarthur/gitlab.alfredworkflow/releases/latest)

🦊 Alfred workflow to search GitLab issues and merge requests.

<img width="550" alt="Screenshot" src="https://user-images.githubusercontent.com/17109887/153414450-8134d7d6-4b6f-488c-8353-0882a2c100c3.png">

## Features

- Configure as many projects as you want under a custom command.
- Open the selected issue or merge request in your browser.
- Filter by label by prefixing your search with `~`.
- Filter by author by prefixing your search with `@`.
- Use **⇧** to instead copy the rich text URL to clipboard.
- Blazingly fast 🤸.

## 📦 Installation

### Pre-packaged

Grab the latest release from
[the releases page](https://github.com/rossmacarthur/gitlab.alfredworkflow/releases).

Because the release contains an executable binary later versions of macOS will
mark it as untrusted and Alfred won't be able to execute it. You can run the
following to explicitly trust the release before installing to Alfred.
```sh
xattr -c ~/Downloads/gitlab-*-apple-darwin.alfredworkflow
```

### Building from source

This workflow is written in Rust, so to install it from source you will first
need to install Rust and Cargo using [rustup](https://rustup.rs/). Then install
[powerpack](https://github.com/rossmacarthur/powerpack). Then you can run the
following to build an `.alfredworkflow` file.

```sh
git clone https://github.com/rossmacarthur/gitlab.alfredworkflow.git
cd gitlab.alfredworkflow
powerpack package
```

The release will be available at `target/workflow/gitlab.alfredworkflow`.

## Configuration

### Options

| Name             | Example          | Description                                                 |
| ---------------- | ---------------- | ----------------------------------------------------------- |
| GITLAB_TOKEN     | `glpat-S-of2...` | Personal access token                                       |
| GITLAB_USER      | rossmacarthur    | *(Optional)* Your GitLab username. Used to sort items       |
| GITLAB_SHORTCUTS | true             | *(Optional)* Adds slash shortcuts like `/new` and `/boards` |

The `GITLAB_TOKEN` environment variable is required. You can create it using
[this link](https://gitlab.com/-/profile/personal_access_tokens?name=gitlab.alfredworkflow&scopes=read_api).
It only needs the `read_api` permission.

### Commands

Any environment variable prefixed with `GITLAB_ISSUES_` or
`GITLAB_MERGE_REQUESTS_` defines a workflow command that will list the issues
or merge requests for the provided project respectively. The name of the command
should follow the prefix. For example to get the command to list issues on the
iTerm2 repository like in the screenshot above you would set the following
environment variable.

| Name                 | Value           |
| -------------------- | --------------- |
| GITLAB_ISSUES_ITERM2 | gnachman/iterm2 |

You can specify as many commands as you want.

## License

This project is distributed under the terms of both the MIT license and the
Apache License (Version 2.0).

See [LICENSE-APACHE](LICENSE-APACHE) and [LICENSE-MIT](LICENSE-MIT) for details.
