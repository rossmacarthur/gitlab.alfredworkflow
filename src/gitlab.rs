use std::io::prelude::*;

use anyhow::{anyhow, Context, Result};
use chrono::DateTime;
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json as json;

use crate::config::CONFIG;
use crate::{Issue, MergeRequest};

fn fetch<T: DeserializeOwned>(query: &str) -> Result<T> {
    #[derive(Debug, Serialize)]
    struct Query<'a> {
        query: &'a str,
    }

    let token = CONFIG
        .token
        .as_ref()
        .ok_or_else(|| anyhow!("GITLAB_TOKEN environment variable is not set!"))?;

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

fn fetch_and_parse<T>(
    name: &str,
    query: &str,
    checksum: [u8; 20],
    ptr: &str,
    parse_fn: fn(json::Value) -> Result<T>,
) -> Result<Vec<T>> {
    let resp = crate::cache::load(name, checksum, || fetch(query))?;
    let nodes: Vec<json::Value> = lookup(&resp, ptr)?;
    nodes.into_iter().map(parse_fn).collect()
}

pub fn issues(name: &str, project: &str) -> Result<Vec<Issue>> {
    let query = r#"
query {
    project(fullPath: "<project>") {
        issues(state: opened) {
            nodes {
                title
                author {
                    name
                }
                assignees {
                    nodes {
                        name
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
        }
    }
}
"#
    .replace("<project>", project);
    let ptr = "/data/project/issues/nodes";
    let checksum = checksum(&query);
    fetch_and_parse(name, &query, checksum, ptr, parse_issue)
}

pub fn merge_requests(name: &str, project: &str) -> Result<Vec<MergeRequest>> {
    let query = r#"
query {
    project(fullPath: "<project>") {
        mergeRequests(state: opened) {
            nodes {
                title
                author {
                    name
                }
                createdAt
                webUrl
                labels {
                    nodes {
                        title
                    }
                }
            }
        }
    }
}
"#
    .replace("<project>", project);
    let ptr = "/data/project/mergeRequests/nodes";
    let checksum = checksum(&query);
    fetch_and_parse(name, &query, checksum, ptr, parse_merge_request)
}

fn parse_issue(value: json::Value) -> Result<Issue> {
    let title = lookup(&value, "/title")?;
    let author = lookup(&value, "/author/name")?;
    let created_at: DateTime<chrono::Utc> = lookup::<String>(&value, "/createdAt")?.parse()?;
    let url = lookup(&value, "/webUrl")?;
    let labels = lookup_list(&value, "/labels/nodes", "/title")?;
    let assignees = lookup_list(&value, "/assignees/nodes", "/name")?;
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
    let author = lookup(&value, "/author/name")?;
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

fn checksum(query: &str) -> [u8; 20] {
    use sha1::*;
    let mut hasher = Sha1::new();
    hasher.update(query.as_bytes());
    hasher.finalize().try_into().unwrap()
}
