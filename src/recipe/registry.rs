use super::Recipe;
use anyhow::{bail, Context, Result};
use include_dir::{include_dir, Dir};
use std::fs;
use std::path::{Path, PathBuf};

static EMBEDDED_RECIPES: Dir = include_dir!("$CARGO_MANIFEST_DIR/recipes");
const DEFAULT_GITHUB_REPO: &str = "yunhai-dev/dpilot";
const DEFAULT_BRANCH: &str = "main";
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct RecipeSummary {
    pub name: String,
    pub version: String,
    pub description: String,
    pub category: String,
    pub source: RecipeSource,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RecipeSource {
    Remote(String),
    Local(PathBuf),
}

pub struct RecipeRegistry;

impl RecipeRegistry {
    pub fn list_available() -> Vec<RecipeSummary> {
        // Try fetching recipe list from GitHub repository contents API
        if let Ok(remote_summaries) = Self::fetch_github_recipes_list() {
            if !remote_summaries.is_empty() {
                return remote_summaries;
            }
        }

        // Fallback: embedded recipes when network is unavailable
        let mut summaries = Vec::new();
        for file in EMBEDDED_RECIPES.files() {
            if let Ok(content) = std::str::from_utf8(file.contents()) {
                if let Ok(recipe) = Recipe::from_yaml_str(content) {
                    let recipe_name = recipe.name.clone();
                    summaries.push(RecipeSummary {
                        name: recipe.name,
                        version: recipe.version,
                        description: recipe.description,
                        category: recipe.category,
                        source: RecipeSource::Remote(format!(
                            "https://raw.githubusercontent.com/{}/{}/recipes/{}.yaml",
                            DEFAULT_GITHUB_REPO, DEFAULT_BRANCH, recipe_name
                        )),
                    });
                }
            }
        }
        summaries.sort_by(|a, b| a.name.cmp(&b.name));
        summaries
    }

    fn fetch_github_recipes_list() -> Result<Vec<RecipeSummary>> {
        let body = std::thread::spawn(|| -> Result<String> {
            let client = reqwest::blocking::Client::builder()
                .timeout(std::time::Duration::from_secs(5))
                .user_agent("dpilot-cli")
                .build()?;

            let api_url = format!(
                "https://api.github.com/repos/{}/contents/recipes?ref={}",
                DEFAULT_GITHUB_REPO, DEFAULT_BRANCH
            );

            let resp = client.get(&api_url).send()?;
            if !resp.status().is_success() {
                bail!("GitHub API returned status: {}", resp.status());
            }
            Ok(resp.text()?)
        })
        .join()
        .map_err(|_| anyhow::anyhow!("Thread panicked"))??;
        #[derive(serde::Deserialize)]
        struct GithubContentItem {
            name: String,
            download_url: Option<String>,
        }

        let items: Vec<GithubContentItem> = serde_json::from_str(&body)?;
        let mut summaries = Vec::new();
        let urls: Vec<String> = items
            .into_iter()
            .filter(|item| item.name.ends_with(".yaml") || item.name.ends_with(".yml"))
            .map(|item| {
                item.download_url.unwrap_or_else(|| {
                    format!(
                        "https://raw.githubusercontent.com/{}/{}/recipes/{}",
                        DEFAULT_GITHUB_REPO, DEFAULT_BRANCH, item.name
                    )
                })
            })
            .collect();

        let fetched_contents = std::thread::spawn(move || -> Vec<(String, String)> {
            let client = match reqwest::blocking::Client::builder()
                .timeout(std::time::Duration::from_secs(5))
                .user_agent("dpilot-cli")
                .build()
            {
                Ok(c) => c,
                Err(_) => return Vec::new(),
            };

            let mut results = Vec::new();
            for url in urls {
                if let Ok(resp) = client.get(&url).send() {
                    if resp.status().is_success() {
                        if let Ok(text) = resp.text() {
                            results.push((url, text));
                        }
                    }
                }
            }
            results
        })
        .join()
        .unwrap_or_default();

        for (raw_url, body) in fetched_contents {
            if let Ok(recipe) = Recipe::from_yaml_str(&body) {
                summaries.push(RecipeSummary {
                    name: recipe.name,
                    version: recipe.version,
                    description: recipe.description,
                    category: recipe.category,
                    source: RecipeSource::Remote(raw_url),
                });
            }
        }
        summaries.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(summaries)
    }

    /// Resolve a recipe by name, file path, or remote URL/repo.
    pub fn resolve(identifier: &str, custom_file: Option<&Path>) -> Result<(Recipe, RecipeSource)> {
        // Case 1: Custom file path directly provided
        if let Some(path) = custom_file {
            let content = fs::read_to_string(path)
                .with_context(|| format!("Failed to read recipe file at {:?}", path))?;
            let recipe = Recipe::from_yaml_str(&content)
                .with_context(|| format!("Failed to parse YAML recipe from {:?}", path))?;
            return Ok((recipe, RecipeSource::Local(path.to_path_buf())));
        }

        // If identifier is directly a valid file path
        let direct_path = Path::new(identifier);
        if direct_path.exists() && direct_path.is_file() {
            let content = fs::read_to_string(direct_path)
                .with_context(|| format!("Failed to read recipe file at {:?}", direct_path))?;
            let recipe = Recipe::from_yaml_str(&content)
                .with_context(|| format!("Failed to parse YAML recipe from {:?}", direct_path))?;
            return Ok((recipe, RecipeSource::Local(direct_path.to_path_buf())));
        }

        // Case 2: Remote URL (http/https)
        if identifier.starts_with("http://") || identifier.starts_with("https://") {
            let recipe = Self::fetch_remote(identifier)?;
            return Ok((recipe, RecipeSource::Remote(identifier.to_string())));
        }

        // Case 3: Fetch directly from GitHub raw content
        let clean_name = identifier
            .trim_end_matches(".yaml")
            .trim_end_matches(".yml");
        let github_raw_url = format!(
            "https://raw.githubusercontent.com/{}/{}/recipes/{}.yaml",
            DEFAULT_GITHUB_REPO, DEFAULT_BRANCH, clean_name
        );

        if let Ok(recipe) = Self::fetch_remote(&github_raw_url) {
            return Ok((recipe, RecipeSource::Remote(github_raw_url)));
        }

        // Fallback: embedded recipes when network is unavailable
        let embedded_filename = format!("{}.yaml", clean_name);
        if let Some(file) = EMBEDDED_RECIPES.get_file(&embedded_filename) {
            let content = std::str::from_utf8(file.contents())
                .context("Embedded recipe is not valid UTF-8")?;
            let recipe = Recipe::from_yaml_str(content)
                .with_context(|| format!("Failed to parse embedded recipe {}", identifier))?;
            return Ok((recipe, RecipeSource::Remote(format!("embedded:{}", clean_name))));
        }

        bail!(
            "Recipe '{}' not found on GitHub ({}) or embedded registry.",
            identifier,
            github_raw_url
        )
    }

    /// Fetch recipe YAML from remote HTTP(S) URL
    pub fn fetch_remote(url: &str) -> Result<Recipe> {
        let url_owned = url.to_string();
        let body = std::thread::spawn(move || -> Result<String> {
            let client = reqwest::blocking::Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .user_agent("dpilot-cli")
                .build()?;
            let resp = client
                .get(&url_owned)
                .send()
                .with_context(|| format!("Failed to fetch remote recipe from {}", url_owned))?;

            if !resp.status().is_success() {
                bail!("Remote recipe request failed with status: {}", resp.status());
            }

            Ok(resp.text()?)
        })
        .join()
        .map_err(|_| anyhow::anyhow!("Thread panicked fetching remote recipe"))??;

        let recipe = Recipe::from_yaml_str(&body)
            .with_context(|| format!("Failed to parse YAML from remote recipe at {}", url))?;
        Ok(recipe)
    }
}
