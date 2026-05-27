use anyhow::{Context, Result, bail};
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use url::Url;

pub(crate) const DEFAULT_CONTENTS_URL: &str =
    "https://api.github.com/repos/github/gitignore/contents?ref=main";
pub(crate) const DEFAULT_RAW_BASE_URL: &str =
    "https://raw.githubusercontent.com/github/gitignore/refs/heads/main/";

#[derive(Clone, Debug)]
pub(crate) struct Endpoints {
    pub(crate) contents_url: String,
    pub(crate) raw_base_url: String,
}

impl Default for Endpoints {
    fn default() -> Self {
        Self {
            contents_url: DEFAULT_CONTENTS_URL.to_string(),
            raw_base_url: DEFAULT_RAW_BASE_URL.to_string(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub(crate) struct Template {
    pub(crate) name: String,
    pub(crate) file_name: String,
}

#[derive(Debug, Deserialize)]
struct GithubContent {
    name: String,
    path: String,
    #[serde(rename = "type")]
    content_type: String,
}

#[derive(Clone)]
pub(crate) struct GitignoreClient {
    client: Client,
    endpoints: Endpoints,
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum ResolveError {
    #[error("template '{name}' was not found")]
    NotFound { name: String },
    #[error("template '{name}' is ambiguous: {candidates}")]
    Ambiguous { name: String, candidates: String },
}

impl GitignoreClient {
    pub(crate) fn new(endpoints: Endpoints) -> Result<Self> {
        let client = Client::builder()
            .user_agent(format!("git-ignore/{}", env!("CARGO_PKG_VERSION")))
            .build()
            .context("failed to build HTTP client")?;

        Ok(Self { client, endpoints })
    }

    pub(crate) fn fetch_manifest(&self) -> Result<Vec<Template>> {
        let response = self
            .client
            .get(&self.endpoints.contents_url)
            .send()
            .context("failed to request github/gitignore template list")?
            .error_for_status()
            .context("github/gitignore template list request failed")?;

        let contents: Vec<GithubContent> = response
            .json()
            .context("failed to decode github/gitignore template list")?;
        Ok(templates_from_contents(contents))
    }

    pub(crate) fn fetch_template(&self, template: &Template) -> Result<String> {
        let url = raw_template_url(&self.endpoints.raw_base_url, &template.file_name)?;
        let response = self
            .client
            .get(url)
            .send()
            .with_context(|| format!("failed to request {}", template.file_name))?
            .error_for_status()
            .with_context(|| {
                format!(
                    "github/gitignore template request failed for {}",
                    template.name
                )
            })?;

        response
            .text()
            .with_context(|| format!("failed to read {}", template.file_name))
    }
}

pub(crate) fn resolve_template<'a>(
    templates: &'a [Template],
    requested: &str,
) -> Result<&'a Template, ResolveError> {
    if let Some(template) = templates.iter().find(|template| template.name == requested) {
        return Ok(template);
    }

    let case_matches: Vec<&Template> = templates
        .iter()
        .filter(|template| template.name.eq_ignore_ascii_case(requested))
        .collect();
    match case_matches.as_slice() {
        [template] => return Ok(template),
        [] => {}
        matches => {
            return Err(ResolveError::Ambiguous {
                name: requested.to_string(),
                candidates: matches
                    .iter()
                    .map(|template| template.name.as_str())
                    .collect::<Vec<_>>()
                    .join(", "),
            });
        }
    }

    Err(ResolveError::NotFound {
        name: requested.to_string(),
    })
}

#[cfg(test)]
fn templates_from_json(source: &str) -> Result<Vec<Template>> {
    let contents = serde_json::from_str(source).context("failed to decode template JSON")?;
    Ok(templates_from_contents(contents))
}

fn templates_from_contents(contents: Vec<GithubContent>) -> Vec<Template> {
    let mut templates = contents
        .into_iter()
        .filter(|content| content.content_type == "file")
        .filter(|content| content.path == content.name)
        .filter_map(|content| {
            let file_name = content.name;
            let name = file_name.strip_suffix(".gitignore")?;
            Some(Template {
                name: name.to_string(),
                file_name,
            })
        })
        .collect::<Vec<_>>();

    templates.sort_by_key(|template| template.name.to_lowercase());
    templates
}

fn raw_template_url(raw_base_url: &str, file_name: &str) -> Result<Url> {
    let mut url = Url::parse(raw_base_url).context("invalid raw base URL")?;
    {
        let mut segments = url
            .path_segments_mut()
            .map_err(|_| anyhow::anyhow!("raw base URL cannot be a base"))?;
        segments.pop_if_empty().push(file_name);
    }

    if url.scheme() != "http" && url.scheme() != "https" {
        bail!("unsupported raw template URL scheme");
    }

    Ok(url)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_only_root_gitignore_templates() {
        let source = r##"
        [
          {"name":"Node.gitignore","path":"Node.gitignore","type":"file"},
          {"name":"Global","path":"Global","type":"dir"},
          {"name":"macOS.gitignore","path":"Global/macOS.gitignore","type":"file"},
          {"name":"community","path":"community","type":"dir"},
          {"name":"README.md","path":"README.md","type":"file"}
        ]
        "##;

        let templates = templates_from_json(source).unwrap();

        assert_eq!(
            templates,
            vec![Template {
                name: "Node".to_string(),
                file_name: "Node.gitignore".to_string()
            }]
        );
    }

    #[test]
    fn resolves_exact_match_before_case_insensitive_match() {
        let templates = vec![
            Template {
                name: "Node".to_string(),
                file_name: "Node.gitignore".to_string(),
            },
            Template {
                name: "node".to_string(),
                file_name: "node.gitignore".to_string(),
            },
        ];

        assert_eq!(
            resolve_template(&templates, "Node").unwrap().file_name,
            "Node.gitignore"
        );
    }

    #[test]
    fn resolves_unique_case_insensitive_match() {
        let templates = vec![Template {
            name: "Node".to_string(),
            file_name: "Node.gitignore".to_string(),
        }];

        assert_eq!(resolve_template(&templates, "node").unwrap().name, "Node");
    }

    #[test]
    fn rejects_ambiguous_case_insensitive_match() {
        let templates = vec![
            Template {
                name: "Node".to_string(),
                file_name: "Node.gitignore".to_string(),
            },
            Template {
                name: "node".to_string(),
                file_name: "node.gitignore".to_string(),
            },
        ];

        let error = resolve_template(&templates, "NODE").unwrap_err();
        assert!(matches!(error, ResolveError::Ambiguous { .. }));
    }

    #[test]
    fn rejects_unknown_template() {
        let templates = vec![Template {
            name: "Node".to_string(),
            file_name: "Node.gitignore".to_string(),
        }];

        let error = resolve_template(&templates, "Rust").unwrap_err();
        assert!(matches!(error, ResolveError::NotFound { .. }));
    }
}
