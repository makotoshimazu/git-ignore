use std::env;
use std::ffi::OsString;
use std::path::Path;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};

use crate::cache::CacheStore;
use crate::completion::completion_script;
use crate::config::AppConfig;
use crate::github::{Endpoints, GitignoreClient, Template, resolve_template};
use crate::installer::append_to_gitignore;
use crate::paths::AppPaths;
use crate::version::version_string;

#[derive(Debug, Parser)]
#[command(name = "git-ignore")]
#[command(about = "Append templates from github/gitignore to a local .gitignore file.")]
#[command(disable_version_flag = true)]
struct Cli {
    #[arg(long, global = true, help = "Bypass local cache and fetch fresh data.")]
    no_cache: bool,

    #[arg(long = "version", global = true, help = "Print version information.")]
    version: bool,

    #[command(subcommand)]
    command: Option<Command>,

    #[arg(value_name = "NAME", help = "Template name to append to .gitignore.")]
    template: Option<String>,
}

#[derive(Debug, Subcommand)]
enum Command {
    #[command(about = "List available root-level github/gitignore templates.")]
    List,
    #[command(about = "Print shell completion script.")]
    Completion { shell: String },
    #[command(about = "Manage local cache.")]
    Cache {
        #[command(subcommand)]
        command: CacheCommand,
    },
    #[command(name = "__complete", hide = true)]
    Complete { prefix: Option<String> },
}

#[derive(Debug, Subcommand)]
enum CacheCommand {
    #[command(about = "Clear local manifest and template cache.")]
    Clear,
}

pub fn run_from_env() -> Result<()> {
    let cwd = env::current_dir().context("failed to read current working directory")?;
    let output = run_with_context(env::args_os(), &cwd, RuntimeContext::from_env()?)?;
    if let Some(output) = output {
        print!("{output}");
    }
    Ok(())
}

fn run_with_context<I, T>(args: I, cwd: &Path, context: RuntimeContext) -> Result<Option<String>>
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    let cli = Cli::parse_from(args);

    if cli.version {
        return Ok(Some(format!("{}\n", version_string())));
    }

    let config = AppConfig::load(&context.paths)?;
    let cache = CacheStore::new(&context.paths);
    let client = GitignoreClient::new(context.endpoints)?;
    let service = GitignoreService {
        config,
        cache,
        client,
        no_cache: cli.no_cache,
    };

    match cli.command {
        Some(Command::List) => Ok(Some(format_template_list(&service.list_templates()?))),
        Some(Command::Completion { shell }) => Ok(Some(completion_script(&shell)?.to_string())),
        Some(Command::Cache {
            command: CacheCommand::Clear,
        }) => {
            service.clear_cache()?;
            Ok(Some("cache cleared\n".to_string()))
        }
        Some(Command::Complete { prefix }) => {
            let candidates = service.complete(prefix.as_deref().unwrap_or_default());
            Ok(Some(candidates.unwrap_or_default()))
        }
        None => match cli.template {
            Some(name) => {
                let installed = service.install(&name, cwd)?;
                Ok(Some(format!(
                    "appended {} to {}\n",
                    installed.name,
                    cwd.join(".gitignore").display()
                )))
            }
            None => Ok(Some(format_template_list(&service.list_templates()?))),
        },
    }
}

struct RuntimeContext {
    paths: AppPaths,
    endpoints: Endpoints,
}

impl RuntimeContext {
    fn from_env() -> Result<Self> {
        Ok(Self {
            paths: AppPaths::from_env()?,
            endpoints: Endpoints::default(),
        })
    }

    #[cfg(test)]
    fn new(paths: AppPaths, endpoints: Endpoints) -> Self {
        Self { paths, endpoints }
    }
}

struct GitignoreService {
    config: AppConfig,
    cache: CacheStore,
    client: GitignoreClient,
    no_cache: bool,
}

impl GitignoreService {
    fn list_templates(&self) -> Result<Vec<Template>> {
        if self.cache_enabled()
            && let Some(templates) = self.cache.load_manifest(self.config.cache.ttl)?
        {
            return Ok(templates);
        }

        let templates = self.client.fetch_manifest()?;
        if self.cache_enabled() {
            self.cache.save_manifest(&templates)?;
        }

        Ok(templates)
    }

    fn install(&self, requested: &str, cwd: &Path) -> Result<Template> {
        let templates = self.list_templates()?;
        let template = resolve_template(&templates, requested)
            .with_context(|| format!("failed to resolve template '{requested}'"))?;
        let content = self.template_content(template)?;
        append_to_gitignore(cwd, &content)?;
        Ok(template.clone())
    }

    fn clear_cache(&self) -> Result<()> {
        self.cache.clear()
    }

    fn complete(&self, prefix: &str) -> Result<String> {
        let lower_prefix = prefix.to_lowercase();
        let candidates = self
            .list_templates()?
            .into_iter()
            .filter(|template| template.name.to_lowercase().starts_with(&lower_prefix))
            .map(|template| template.name)
            .collect::<Vec<_>>();
        Ok(format!("{}\n", candidates.join("\n")))
    }

    fn template_content(&self, template: &Template) -> Result<String> {
        if self.cache_enabled()
            && let Some(content) = self
                .cache
                .load_template(&template.file_name, self.config.cache.ttl)?
        {
            return Ok(content);
        }

        let content = self.client.fetch_template(template)?;
        if self.cache_enabled() {
            self.cache.save_template(&template.file_name, &content)?;
        }
        Ok(content)
    }

    fn cache_enabled(&self) -> bool {
        self.config.cache.enabled && !self.no_cache
    }
}

fn format_template_list(templates: &[Template]) -> String {
    let mut output = templates
        .iter()
        .map(|template| template.name.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    output.push('\n');
    output
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::sync::{Arc, Mutex};
    use std::thread;

    use tempfile::TempDir;

    use super::*;

    #[test]
    fn lists_templates_from_mock_server() {
        let server = MockServer::start();
        let temp = TempDir::new().unwrap();
        let output = run_with_context(
            ["git-ignore", "list"],
            temp.path(),
            test_context(temp.path(), &server),
        )
        .unwrap()
        .unwrap();

        assert_eq!(output, "Node\nRust\n");
        assert_eq!(server.request_count(), 1);
    }

    #[test]
    fn lists_templates_without_arguments() {
        let server = MockServer::start();
        let temp = TempDir::new().unwrap();
        let output = run_with_context(
            ["git-ignore"],
            temp.path(),
            test_context(temp.path(), &server),
        )
        .unwrap()
        .unwrap();

        assert_eq!(output, "Node\nRust\n");
    }

    #[test]
    fn appends_selected_template_from_mock_server() {
        let server = MockServer::start();
        let temp = TempDir::new().unwrap();

        let output = run_with_context(
            ["git-ignore", "Node"],
            temp.path(),
            test_context(temp.path(), &server),
        )
        .unwrap()
        .unwrap();

        assert_eq!(
            fs::read_to_string(temp.path().join(".gitignore")).unwrap(),
            "node_modules/\n"
        );
        assert!(output.contains("appended Node"));
    }

    #[test]
    fn prints_zsh_completion() {
        let server = MockServer::start();
        let temp = TempDir::new().unwrap();

        let output = run_with_context(
            ["git-ignore", "completion", "zsh"],
            temp.path(),
            test_context(temp.path(), &server),
        )
        .unwrap()
        .unwrap();

        assert!(output.contains("git-ignore __complete"));
        assert!(output.contains("compdef _git-ignore git-ignore"));
    }

    #[test]
    fn clears_cache() {
        let server = MockServer::start();
        let temp = TempDir::new().unwrap();
        let cache_dir = temp.path().join("cache");
        fs::create_dir_all(&cache_dir).unwrap();
        fs::write(cache_dir.join("manifest.json"), "{}").unwrap();

        let output = run_with_context(
            ["git-ignore", "cache", "clear"],
            temp.path(),
            RuntimeContext::new(
                AppPaths::new(cache_dir.clone(), temp.path().join("config.toml")),
                server.endpoints(),
            ),
        )
        .unwrap()
        .unwrap();

        assert_eq!(output, "cache cleared\n");
        assert!(!cache_dir.exists());
    }

    #[test]
    fn completes_by_prefix() {
        let server = MockServer::start();
        let temp = TempDir::new().unwrap();
        let output = run_with_context(
            ["git-ignore", "__complete", "No"],
            temp.path(),
            test_context(temp.path(), &server),
        )
        .unwrap()
        .unwrap();

        assert_eq!(output, "Node\n");
    }

    #[test]
    fn no_cache_bypasses_existing_manifest_cache() {
        let server = MockServer::start();
        let temp = TempDir::new().unwrap();
        let cache_dir = temp.path().join("cache");
        fs::create_dir_all(&cache_dir).unwrap();
        fs::write(
            cache_dir.join("manifest.json"),
            r#"{"fetched_at":4102444800,"templates":[{"name":"Cached","file_name":"Cached.gitignore"}]}"#,
        )
        .unwrap();

        let output = run_with_context(
            ["git-ignore", "--no-cache", "list"],
            temp.path(),
            RuntimeContext::new(
                AppPaths::new(cache_dir, temp.path().join("config.toml")),
                server.endpoints(),
            ),
        )
        .unwrap()
        .unwrap();

        assert_eq!(output, "Node\nRust\n");
    }

    fn test_context(path: &Path, server: &MockServer) -> RuntimeContext {
        RuntimeContext::new(
            AppPaths::new(path.join("cache"), path.join("config.toml")),
            server.endpoints(),
        )
    }

    struct MockServer {
        address: String,
        requests: Arc<Mutex<Vec<String>>>,
    }

    impl MockServer {
        fn start() -> Self {
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            let address = listener.local_addr().unwrap().to_string();
            let requests = Arc::new(Mutex::new(Vec::new()));
            let requests_for_thread = Arc::clone(&requests);

            thread::spawn(move || {
                for stream in listener.incoming().take(16) {
                    let mut stream = stream.unwrap();
                    let mut buffer = [0; 4096];
                    let bytes = stream.read(&mut buffer).unwrap();
                    let request = String::from_utf8_lossy(&buffer[..bytes]).to_string();
                    let path = request
                        .lines()
                        .next()
                        .and_then(|line| line.split_whitespace().nth(1))
                        .unwrap_or("/")
                        .to_string();
                    requests_for_thread.lock().unwrap().push(path.clone());

                    let body = match path.as_str() {
                        "/contents?ref=main" => {
                            r#"[{"name":"Node.gitignore","path":"Node.gitignore","type":"file"},{"name":"Rust.gitignore","path":"Rust.gitignore","type":"file"},{"name":"Global","path":"Global","type":"dir"},{"name":"macOS.gitignore","path":"Global/macOS.gitignore","type":"file"}]"#
                        }
                        "/raw/Node.gitignore" => "node_modules/\n",
                        "/raw/Rust.gitignore" => "target/\n",
                        _ => "not found",
                    };
                    let status = if body == "not found" {
                        "HTTP/1.1 404 Not Found"
                    } else {
                        "HTTP/1.1 200 OK"
                    };
                    let response = format!(
                        "{status}\r\nContent-Length: {}\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n{body}",
                        body.len()
                    );
                    stream.write_all(response.as_bytes()).unwrap();
                }
            });

            Self { address, requests }
        }

        fn endpoints(&self) -> Endpoints {
            Endpoints {
                contents_url: format!("http://{}/contents?ref=main", self.address),
                raw_base_url: format!("http://{}/raw/", self.address),
            }
        }

        fn request_count(&self) -> usize {
            self.requests.lock().unwrap().len()
        }
    }
}
