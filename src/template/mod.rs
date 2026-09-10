use anyhow::{Context, Result};
use minijinja::Environment;
use serde_json::Value;

pub struct TemplateEngine;

impl TemplateEngine {
    /// Render a single string template with the provided JSON context
    pub fn render_str(template_str: &str, context: &Value) -> Result<String> {
        let mut env = Environment::new();
        env.add_template("cmd", template_str)
            .context("Failed to parse command template syntax")?;
        
        let template = env.get_template("cmd")?;
        let rendered = template
            .render(context)
            .context("Failed to render command template with input variables")?;

        Ok(rendered)
    }
}
