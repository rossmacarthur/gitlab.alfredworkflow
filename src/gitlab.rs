use std::env;
use std::io::prelude::*;

use anyhow::{Context, Result};
use chrono::DateTime;
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json as json;

use crate::Object;

const GITLAB_TOKEN: &str = env!("GITLAB_TOKEN");

fn try_lookup(value: &json::Value, ptr: &str) -> Result<String> {
    Ok(value
        .pointer(ptr)
        .with_context(|| format!("failed to lookup `{}`", ptr))?
        .as_str()
        .context("not a string")?
        .to_owned())
}

fn fetch<T: DeserializeOwned>(query: &str) -> Result<T> {
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
        hl.append(&format!("Authorization: Bearer {}", GITLAB_TOKEN))?;
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

fn fetch_and_parse<T, F>(query: &str, ptr: &str, parse: F) -> Result<Vec<T>>
where
    F: Fn(&json::Value) -> Result<T>,
{
    let resp: json::Value = fetch(query)?;
    let value = resp
        .pointer(ptr)
        .with_context(|| format!("failed to lookup `{}` from `{:?}`", ptr, resp))?;
    let nodes = value.as_array().context("expected array")?;
    nodes.iter().map(parse).collect()
}

pub fn issues() -> Result<Vec<Object>> {
    let query = r#"
    query {
        project(fullPath: "lunomoney/product-engineering/pods/connect-us/work") {
            issues(state: opened) {
                nodes {
                    title
                    author {
                        name
                    }
                    createdAt
                    webUrl
                }
            }
        }
    }
    "#;
    fetch_and_parse(query, "/data/project/issues/nodes", parse_object)
}

pub fn core() -> Result<Vec<Object>> {
    let query = r#"
query {
    project(fullPath: "lunomoney/product-engineering/core") {
        mergeRequests(state: opened) {
            nodes {
                title
                author {
                    name
                }
                createdAt
                webUrl
            }
        }
    }
}
"#;
    fetch_and_parse(query, "/data/project/mergeRequests/nodes", parse_object)
}

fn parse_object(value: &json::Value) -> Result<Object> {
    let title = try_lookup(value, "/title")?;
    let author = try_lookup(value, "/author/name")?;
    let created_at: DateTime<chrono::Utc> = try_lookup(value, "/createdAt")?.parse()?;
    let url = try_lookup(value, "/webUrl")?;
    Ok(Object {
        title,
        url,
        author,
        created_at,
    })
}
