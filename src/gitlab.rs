use std::env;
use std::io::prelude::*;

use anyhow::Result;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

const GITLAB_TOKEN: &str = env!("GITLAB_TOKEN");

fn graphql<T: DeserializeOwned>(query: &str) -> Result<T> {
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
        let mut headers = curl::easy::List::new();
        headers.append(&format!("Authorization: Bearer {}", GITLAB_TOKEN))?;
        headers.append("Content-Type: application/json")?;
        headers
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

pub mod issues {
    use super::*;

    #[derive(Debug, Deserialize)]
    struct Response {
        data: Data,
    }

    #[derive(Debug, Deserialize)]
    struct Data {
        project: Project,
    }

    #[derive(Debug, Deserialize)]
    struct Project {
        issues: Node,
    }

    #[derive(Debug, Deserialize)]
    struct Node {
        nodes: Vec<Issue>,
    }

    #[derive(Debug, Deserialize)]
    struct Issue {
        iid: String,
        title: String,
    }

    pub fn list() -> Result<Vec<crate::Issue>> {
        let resp: Response = graphql(
            r#"
query {
    project(fullPath: "lunomoney/product-engineering/pods/connect-us/work") {
        issues {
            nodes {
                iid
                title
            }
        }
    }
}
"#,
        )?;

        let issues = resp.data.project.issues.nodes;
        Ok(issues
            .into_iter()
            .map(|Issue { iid, title, .. }| {
                let url = format!(
                    "https://gitlab.com/lunomoney/product-engineering/pods/connect-us/work/-/issues/{}",
                    iid,
                );
                crate::Issue { title, url }
            })
            .collect())
    }
}

pub mod core {
    use super::*;

    #[derive(Debug, Deserialize)]
    struct Response {
        data: Data,
    }

    #[derive(Debug, Deserialize)]
    struct Data {
        project: Project,
    }

    #[derive(Debug, Deserialize)]
    struct Project {
        #[serde(rename = "mergeRequests")]
        merge_requests: Node,
    }

    #[derive(Debug, Deserialize)]
    struct Node {
        nodes: Vec<MergeRequest>,
    }

    #[derive(Debug, Deserialize)]
    struct MergeRequest {
        iid: String,
        title: String,
    }

    pub fn list() -> Result<Vec<crate::Issue>> {
        let resp: Response = graphql(
            r#"
query {
    project(fullPath: "lunomoney/product-engineering/core") {
        mergeRequests {
            nodes {
                iid
                title
            }
        }
    }
}
"#,
        )?;

        let merge_requests = resp.data.project.merge_requests.nodes;
        Ok(merge_requests
            .into_iter()
            .map(|MergeRequest { iid, title, .. }| {
                let url = format!(
                    "https://gitlab.com/lunomoney/product-engineering/core/-/merge_requests/{}",
                    iid,
                );
                crate::Issue { title, url }
            })
            .collect())
    }
}
