mod cli;
mod executor;
mod prompt;
mod recipe;
mod template;

use anyhow::Result;
use clap::Parser;
use cli::{Cli, Commands};
use console::style;
use inquire::Select;
use recipe::registry::{RecipeRegistry, RecipeSummary};
use recipe::Recipe;

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Run { service, file, dry_run }) => {
            handle_run(service, file, dry_run).await?;
        }
        Some(Commands::List { category }) => {
            handle_list(category)?;
        }
        Some(Commands::Validate { file }) => {
            handle_validate(file)?;
        }
        None => {
            // Interactive mode: show list of available recipes and prompt user to select
            handle_interactive_select().await?;
        }
    }

    Ok(())
}

async fn handle_run(
    service: Option<String>,
    file: Option<std::path::PathBuf>,
    dry_run: bool,
) -> Result<()> {
    let (recipe, source) = match (service, file) {
        (Some(svc), None) => RecipeRegistry::resolve(&svc, None)?,
        (None, Some(f)) => RecipeRegistry::resolve("", Some(&f))?,
        (Some(svc), Some(f)) => RecipeRegistry::resolve(&svc, Some(&f))?,
        (None, None) => {
            // Pick recipe interactively
            let selected_name = prompt_service_selection(None)?;
            RecipeRegistry::resolve(&selected_name, None)?
        }
    };

    println!();
    println!(
        "{} Loaded recipe {} from {:?}",
        style("●").cyan().bold(),
        style(&recipe.name).bold(),
        source
    );

    prompt::PromptWizard::print_banner(&recipe);

    let inputs = prompt::PromptWizard::collect_inputs(&recipe)?;

    if !dry_run {
        let confirmed = prompt::PromptWizard::confirm_summary(&recipe, &inputs)?;
        if !confirmed {
            println!("{}", style("Deployment cancelled by user.").yellow());
            return Ok(());
        }
    }

    executor::ExecutionEngine::execute_recipe(&recipe, &inputs, dry_run).await?;

    Ok(())
}

fn handle_list(category_filter: Option<String>) -> Result<()> {
    let available = RecipeRegistry::list_available();

    println!();
    println!(
        "{}",
        style("Available Services & Tooling Recipes").bold().underlined()
    );
    println!();

    let filtered: Vec<&RecipeSummary> = available
        .iter()
        .filter(|s| {
            if let Some(cat) = &category_filter {
                s.category.eq_ignore_ascii_case(cat)
            } else {
                true
            }
        })
        .collect();

    if filtered.is_empty() {
        println!("No recipes found matching category filter.");
        return Ok(());
    }

    println!(
        "{:<18} {:<10} {:<10} {:<40}",
        style("NAME").bold(),
        style("CATEGORY").bold(),
        style("VERSION").bold(),
        style("DESCRIPTION").bold()
    );
    println!("{}", style("─".repeat(80)).dim());

    for item in filtered {
        let cat_styled = match item.category.as_str() {
            "system" => style(&item.category).magenta(),
            "service" => style(&item.category).cyan(),
            _ => style(&item.category).white(),
        };

        println!(
            "{:<18} {:<10} {:<10} {:<40}",
            style(&item.name).green().bold(),
            cat_styled,
            style(&item.version).dim(),
            item.description
        );
    }
    println!();
    println!(
        "Run {} to launch a deployment wizard.",
        style("dpilot run <name>").cyan().bold()
    );
    println!();

    Ok(())
}

fn handle_validate(path: std::path::PathBuf) -> Result<()> {
    let content = std::fs::read_to_string(&path)?;
    let recipe = Recipe::from_yaml_str(&content)?;

    println!(
        "{} Recipe '{}' (v{}) is valid!",
        style("✔").green().bold(),
        recipe.name,
        recipe.version
    );
    println!("  Category: {}", recipe.category);
    println!("  Inputs:   {} defined", recipe.inputs.len());
    println!("  Steps:    {} defined", recipe.steps.len());

    Ok(())
}

async fn handle_interactive_select() -> Result<()> {
    println!();
    println!(
        "{}",
        style("🚀 dpilot — Guided Service & Environment Deployment").cyan().bold()
    );
    println!(
        "{}",
        style("Select a service or system task to configure and deploy:").dim()
    );
    println!();

    let selected = prompt_service_selection(None)?;
    handle_run(Some(selected), None, false).await?;

    Ok(())
}

fn prompt_service_selection(category_filter: Option<String>) -> Result<String> {
    let available = RecipeRegistry::list_available();
    let filtered: Vec<RecipeSummary> = available
        .into_iter()
        .filter(|s| {
            if let Some(cat) = &category_filter {
                s.category.eq_ignore_ascii_case(cat)
            } else {
                true
            }
        })
        .collect();

    if filtered.is_empty() {
        anyhow::bail!("No available recipes found.");
    }

    let items: Vec<String> = filtered
        .iter()
        .map(|r| format!("{:<16} [{}] - {}", r.name, r.category, r.description))
        .collect();

    let selection = Select::new("Choose a recipe to deploy:", items).prompt()?;
    let selected_name = selection
        .split_whitespace()
        .next()
        .unwrap_or("")
        .to_string();

    Ok(selected_name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_parse_embedded_recipes() {
        let list = RecipeRegistry::list_available();
        assert!(!list.is_empty(), "Embedded recipes should not be empty");
        let names: Vec<String> = list.into_iter().map(|r| r.name).collect();
        assert!(names.contains(&"nginx".to_string()));
        assert!(names.contains(&"redis".to_string()));
        assert!(names.contains(&"postgres".to_string()));
        assert!(names.contains(&"uptime-kuma".to_string()));
        assert!(names.contains(&"install-docker".to_string()));
    }

    #[test]
    fn test_template_rendering() {
        let cmd = "docker run -d --name {{ container_name }} -p {{ port }}:5432 {% if enable_volume %}-v {{ container_name }}_data:/var/lib/postgresql/data{% endif %} postgres:{{ version_tag }}";
        let context = json!({
            "container_name": "test-pg",
            "port": 5432,
            "enable_volume": true,
            "version_tag": "16-alpine"
        });

        let rendered = template::TemplateEngine::render_str(cmd, &context).unwrap();
        assert!(rendered.contains("--name test-pg"));
        assert!(rendered.contains("-p 5432:5432"));
        assert!(rendered.contains("-v test-pg_data:/var/lib/postgresql/data"));
        assert!(rendered.contains("postgres:16-alpine"));
    }

    #[test]
    fn test_template_rendering_conditional_false() {
        let cmd = "docker run {% if enable_https %}-p 443:443{% endif %} nginx:latest";
        let context = json!({
            "enable_https": false
        });

        let rendered = template::TemplateEngine::render_str(cmd, &context).unwrap();
        assert!(!rendered.contains("-p 443:443"));
    }
}
