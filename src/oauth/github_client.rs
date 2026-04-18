use std::time::Duration;

use anyhow::{Context, Result};
use reqwest::blocking::Client;
use serde::Deserialize;

use super::OAuthTokenStore;

/// GitHub REST API client using a stored OAuth token.
pub struct GitHubClient {
    client: Client,
    access_token: String,
}

#[derive(Debug, Clone)]
pub struct IssueSummary {
    pub number: u64,
    pub title: String,
    pub url: String,
    pub labels: Vec<String>,
    pub updated_at: String,
}

#[derive(Debug, Clone)]
pub struct PrSummary {
    pub number: u64,
    pub title: String,
    pub url: String,
    pub draft: bool,
    pub updated_at: String,
}

impl GitHubClient {
    /// Load from the OAuth token store. Returns `None` when no GitHub token is stored.
    pub fn from_store(store: &OAuthTokenStore) -> Result<Option<Self>> {
        let token = match store.get("github")? {
            Some(t) => t,
            None => return Ok(None),
        };
        if token.is_expired() {
            return Ok(None);
        }
        Ok(Some(Self {
            client: Client::builder()
                .timeout(Duration::from_secs(20))
                .user_agent("praxis-agent/1.0")
                .build()
                .context("failed to build HTTP client")?,
            access_token: token.access_token,
        }))
    }

    /// List open issues for `owner/repo` (excludes pull requests).
    pub fn list_open_issues(&self, owner: &str, repo: &str) -> Result<Vec<IssueSummary>> {
        let url = format!("https://api.github.com/repos/{owner}/{repo}/issues");
        let items: Vec<RawIssue> = self
            .client
            .get(&url)
            .bearer_auth(&self.access_token)
            .query(&[("state", "open"), ("per_page", "50")])
            .send()
            .context("failed to list GitHub issues")?
            .error_for_status()
            .context("GitHub issues API returned an error")?
            .json()
            .context("failed to parse GitHub issues")?;

        Ok(items
            .into_iter()
            // GitHub API returns PRs in the issues endpoint; exclude them.
            .filter(|i| i.pull_request.is_none())
            .map(|i| IssueSummary {
                number: i.number,
                title: i.title,
                url: i.html_url,
                labels: i.labels.into_iter().map(|l| l.name).collect(),
                updated_at: i.updated_at,
            })
            .collect())
    }

    /// List open pull requests for `owner/repo`.
    pub fn list_open_prs(&self, owner: &str, repo: &str) -> Result<Vec<PrSummary>> {
        let url = format!("https://api.github.com/repos/{owner}/{repo}/pulls");
        let items: Vec<RawPr> = self
            .client
            .get(&url)
            .bearer_auth(&self.access_token)
            .query(&[("state", "open"), ("per_page", "50")])
            .send()
            .context("failed to list GitHub PRs")?
            .error_for_status()
            .context("GitHub PRs API returned an error")?
            .json()
            .context("failed to parse GitHub PRs")?;

        Ok(items
            .into_iter()
            .map(|p| PrSummary {
                number: p.number,
                title: p.title,
                url: p.html_url,
                draft: p.draft.unwrap_or(false),
                updated_at: p.updated_at,
            })
            .collect())
    }
}

#[derive(Deserialize)]
struct RawIssue {
    number: u64,
    title: String,
    html_url: String,
    labels: Vec<RawLabel>,
    updated_at: String,
    pull_request: Option<serde_json::Value>,
}

#[derive(Deserialize)]
struct RawLabel {
    name: String,
}

#[derive(Deserialize)]
struct RawPr {
    number: u64,
    title: String,
    html_url: String,
    draft: Option<bool>,
    updated_at: String,
}
