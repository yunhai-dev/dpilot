use super::Recipe;
use anyhow::{bail, Context, Result};
use include_dir::{include_dir, Dir};
use std::fs;
use std::path::{Path, PathBuf};

static EMBEDDED_RECIPES: Dir = include_dir!("$CARGO_MANIFEST_DIR/recipes");

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct RecipeSummary {
    pub name: String,
    pub version: String,
    pub description: String,
    pub category: String,
    pub source: RecipeSource,
}

#[allow(dead_code)]

#[derive(Debug, Clone, PartialEq)]
pub enum RecipeSource {
    Embedded,
    Local(PathBuf),
    Remote(String),
}

pub struct RecipeRegistry;

impl RecipeRegistry {
    /// Load all available recipes from embedded binary and local ./recipes directories.
    pub fn list_available() -> Vec<RecipeSummary> {
        let mut summaries = Vec::new();
        let mut seen = std::collections::HashSet::new();

        // 1. Scan embedded recipes
        for file in EMBEDDED_RECIPES.files() {
            if let Ok(content) = std::str::from_utf8(file.contents()) {
                if let Ok(recipe) = Recipe::from_yaml_str(content) {
                    seen.insert(recipe.name.clone());
                    summaries.push(RecipeSummary {
                        name: recipe.name,
                        version: recipe.version,
                        description: recipe.description,
                        category: recipe.category,
                        source: RecipeSource::Embedded,
                    });
                }
            }
        }

        // 2. Scan local ./recipes directory if exists
        let local_dir = Path::new("./recipes");
        if local_dir.is_dir() {
            if let Ok(entries) = fs::read_dir(local_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_file()
                        && (path.extension().and_then(|s| s.to_str()) == Some("yaml")
                            || path.extension().and_then(|s| s.to_str()) == Some("yml"))
                    {
                        if let Ok(content) = fs::read_to_string(&path) {
                            if let Ok(recipe) = Recipe::from_yaml_str(&content) {
                                if !seen.contains(&recipe.name) {
                                    seen.insert(recipe.name.clone());
                                    summaries.push(RecipeSummary {
                                        name: recipe.name,
                                        version: recipe.version,
                                        description: recipe.description,
                                        category: recipe.category,
                                        source: RecipeSource::Local(path),
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }

        summaries.sort_by(|a, b| a.name.cmp(&b.name));
        summaries
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

        // Case 3: Embedded recipes
        let embedded_filename = if identifier.ends_with(".yaml") || identifier.ends_with(".yml") {
            identifier.to_string()
        } else {
            format!("{}.yaml", identifier)
        };

        if let Some(file) = EMBEDDED_RECIPES.get_file(&embedded_filename) {
            let content = std::str::from_utf8(file.contents())
                .context("Embedded recipe is not valid UTF-8")?;
            let recipe = Recipe::from_yaml_str(content)
                .with_context(|| format!("Failed to parse embedded recipe {}", identifier))?;
            return Ok((recipe, RecipeSource::Embedded));
        }

        // Case 4: Local recipes directory ./recipes/<name>.yaml
        let local_path = PathBuf::from(format!("recipes/{}", embedded_filename));
        if local_path.exists() {
            let content = fs::read_to_string(&local_path)
                .with_context(|| format!("Failed to read local recipe at {:?}", local_path))?;
            let recipe = Recipe::from_yaml_str(&content)
                .with_context(|| format!("Failed to parse local recipe at {:?}", local_path))?;
            return Ok((recipe, RecipeSource::Local(local_path)));
        }

        // Case 5: User config directory ~/.dpilot/recipes/<name>.yaml
        if let Some(home) = std::env::var_os("HOME") {
            let user_recipe = PathBuf::from(home)
                .join(".dpilot")
                .join("recipes")
                .join(&embedded_filename);
            if user_recipe.exists() {
                let content = fs::read_to_string(&user_recipe)
                    .with_context(|| format!("Failed to read user recipe at {:?}", user_recipe))?;
                let recipe = Recipe::from_yaml_str(&content)
                    .with_context(|| format!("Failed to parse user recipe at {:?}", user_recipe))?;
                return Ok((recipe, RecipeSource::Local(user_recipe)));
            }
        }

        bail!("Recipe '{}' not found in embedded registry, ./recipes/, or remote.", identifier)
    }

    /// Fetch recipe YAML from remote HTTP(S) URL
    pub fn fetch_remote(url: &str) -> Result<Recipe> {
        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()?;
        let resp = client
            .get(url)
            .send()
            .with_context(|| format!("Failed to fetch remote recipe from {}", url))?;

        if !resp.status().is_success() {
            bail!("Remote recipe request failed with status: {}", resp.status());
        }

        let body = resp.text()?;
        let recipe = Recipe::from_yaml_str(&body)
            .with_context(|| format!("Failed to parse YAML from remote recipe at {}", url))?;
        Ok(recipe)
    }
}
