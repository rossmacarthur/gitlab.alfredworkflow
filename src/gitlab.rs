use std::io::prelude::*;

use anyhow::{anyhow, Context, Result};
use chrono::DateTime;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json as json;

use crate::config::CONFIG;
use crate::{Issue, MergeRequest};

type ParseFn<T> = fn(json::Value) -> Result<T>;

struct Query<'a, T> {
    name: &'a str,
    project: &'a str,
    template: &'a str,
    page_info_ptr: &'a str,
    nodes_ptr: &'a str,
    parse_fn: ParseFn<T>,
}

#[derive(Deserialize)]
struct PageInfo {
    #[serde(rename = "endCursor")]
    cursor: String,
    #[serde(rename = "hasNextPage")]
    has_next: bool,
}

impl<T> Query<'_, T> {
    fn checksum(&self) -> [u8; 20] {
        use sha1::*;
        let mut hasher = Sha1::new();
        hasher.update(self.name.as_bytes());
        hasher.update(self.project.as_bytes());
        hasher.update(self.template.as_bytes());
        hasher.finalize().try_into().unwrap()
    }
}

fn fetch_and_parse<T>(q: Query<'_, T>) -> Result<Vec<T>> {
    let token = CONFIG
        .token
        .as_ref()
        .ok_or_else(|| anyhow!("GITLAB_TOKEN environment variable is not set!"))?;

    let mut r = crate::cache::load(q.name, q.checksum(), || fetch_all(&q, token))?;
    let resps = r
        .as_array_mut()
        .context("cache value is not an array")?
        .drain(..);

    let mut nodes = Vec::new();
    for resp in resps {
        let ns: Vec<json::Value> = lookup(&resp, q.nodes_ptr)?;
        nodes.extend(ns);
    }
    nodes.into_iter().map(q.parse_fn).collect()
}

fn fetch_all<T>(q: &Query<'_, T>, token: &str) -> Result<json::Value> {
    let engine = upon::Engine::with_delims("<", ">", "<[", "]>");
    let template = engine.compile(q.template)?;

    let mut array = Vec::new();
    let mut args = None;
    loop {
        let query = template.render(upon::value! { project: q.project, args: args })?;
        let resp = fetch(&query, token)?;
        let page_info: PageInfo = lookup(&resp, q.page_info_ptr)?;
        array.push(resp);
        if !page_info.has_next {
            break Ok(json::Value::Array(array));
        }
        args = Some(format!(", after:\"{}\"", page_info.cursor));
    }
}

fn fetch(query: &str, token: &str) -> Result<json::Value> {
    #[derive(Debug, Serialize)]
    struct Query<'a> {
        query: &'a str,
    }

    let mut buf = Vec::new();
    let mut easy = curl::easy::Easy::new();
    let mut data = &*serde_json::to_vec(&Query { query })?;

    easy.fail_on_error(true)?;
    easy.follow_location(true)?;
    easy.http_headers({
        let mut hl = curl::easy::List::new();
        hl.append(&format!("Authorization: Bearer {}", token))?;
        hl.append("Content-Type: application/json")?;
        hl
    })?;
    easy.post(true)?;
    easy.url("https://gitlab.com/api/graphql")?;

    {
        let mut t = easy.transfer();
        t.read_function(|into| Ok(data.read(into).unwrap()))?;
        t.write_function(|data| {
            buf.extend_from_slice(data);
            Ok(data.len())
        })?;
        t.perform()?;
    }

    Ok(serde_json::from_slice(&buf)?)
}

pub fn issues(name: &str, project: &str) -> Result<Vec<Issue>> {
    let template = r#"
query {
    project(fullPath: "<project>") {
        issues(state: opened <args>) {
            nodes {
                title
                author {
                    name
                    username
                }
                assignees {
                    nodes {
                        name
                        username
                    }
                }
                createdAt
                webUrl
                labels {
                    nodes {
                        title
                    }
                }
            }
            pageInfo {
                endCursor
                hasNextPage
            }
        }
    }
}
"#;
    fetch_and_parse(Query {
        name,
        project,
        template,
        page_info_ptr: "/data/project/issues/pageInfo",
        nodes_ptr: "/data/project/issues/nodes",
        parse_fn: parse_issue,
    })
}

pub fn merge_requests(name: &str, project: &str) -> Result<Vec<MergeRequest>> {
    let template = r#"
query {
    project(fullPath: "<project>") {
        mergeRequests(state: opened <args>) {
            nodes {
                title
                author {
                    name
                    username
                }
                createdAt
                webUrl
                labels {
                    nodes {
                        title
                    }
                }
            }

            pageInfo {
                endCursor
                hasNextPage
            }
        }
    }
}
"#;
    fetch_and_parse(Query {
        name,
        project,
        template,
        page_info_ptr: "/data/project/mergeRequests/pageInfo",
        nodes_ptr: "/data/project/mergeRequests/nodes",
        parse_fn: parse_merge_request,
    })
}

fn parse_issue(value: json::Value) -> Result<Issue> {
    let title = lookup(&value, "/title")?;
    let author = lookup(&value, "/author")?;
    let created_at: DateTime<chrono::Utc> = lookup::<String>(&value, "/createdAt")?.parse()?;
    let url = lookup(&value, "/webUrl")?;
    let labels = lookup_list(&value, "/labels/nodes", "/title")?;
    let assignees = lookup_list(&value, "/assignees/nodes", "")?;
    Ok(Issue {
        title,
        url,
        author,
        assignees,
        created_at,
        labels,
    })
}

fn parse_merge_request(value: json::Value) -> Result<MergeRequest> {
    let title = lookup(&value, "/title")?;
    let author = lookup(&value, "/author")?;
    let created_at: DateTime<chrono::Utc> = lookup::<String>(&value, "/createdAt")?.parse()?;
    let url = lookup(&value, "/webUrl")?;
    let labels = lookup_list(&value, "/labels/nodes", "/title")?;
    Ok(MergeRequest {
        title,
        url,
        author,
        created_at,
        labels,
    })
}

fn lookup<T>(value: &json::Value, ptr: &str) -> Result<T>
where
    T: DeserializeOwned,
{
    let v = value
        .pointer(ptr)
        .with_context(|| format!("failed to lookup `{}`", ptr))?;
    Ok(json::from_value(v.clone())?)
}

fn lookup_list<T>(value: &json::Value, ptr: &str, sub_ptr: &str) -> Result<Vec<T>>
where
    T: DeserializeOwned,
{
    let list: Vec<json::Value> = lookup(value, ptr)?;
    list.into_iter().map(|v| lookup(&v, sub_ptr)).collect()
}
