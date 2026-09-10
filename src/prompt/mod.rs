use crate::recipe::{InputSpec, InputType, Recipe};
use anyhow::Result;
use console::style;
use inquire::{Confirm, CustomType, Password, Select, Text};
use serde_json::{Map, Value};

pub struct PromptWizard;

impl PromptWizard {
    /// Print a stylish header banner for the selected recipe
    pub fn print_banner(recipe: &Recipe) {
        println!();
        println!(
            "{}",
            style(format!("◆ Deploying: {} (v{})", recipe.name, recipe.version))
                .cyan()
                .bold()
        );
        println!(
            "{}",
            style(format!("│ {}", recipe.description)).dim()
        );
        if let Some(author) = &recipe.author {
            println!("{}", style(format!("│ Author: {}", author)).dim());
        }
        println!("{}", style("│").dim());
    }

    /// Ask user questions according to the Recipe's inputs spec
    pub fn collect_inputs(recipe: &Recipe) -> Result<Value> {
        let mut context = Map::new();

        for input in &recipe.inputs {
            let val = Self::prompt_single_input(input)?;
            context.insert(input.id.clone(), val);
        }

        Ok(Value::Object(context))
    }

    fn prompt_single_input(input: &InputSpec) -> Result<Value> {
        let prompt_prefix = format!("│  {}", input.prompt);

        match input.input_type {
            InputType::Text => {
                let mut text_prompt = Text::new(&prompt_prefix);
                if let Some(help) = &input.help {
                    text_prompt = text_prompt.with_help_message(help);
                }
                if let Some(serde_yaml::Value::String(default_val)) = &input.default {
                    text_prompt = text_prompt.with_default(default_val);
                }

                if input.required.unwrap_or(false) {
                    text_prompt = text_prompt.with_validator(|ans: &str| {
                        if ans.trim().is_empty() {
                            Ok(inquire::validator::Validation::Invalid(
                                "This field cannot be empty".into(),
                            ))
                        } else {
                            Ok(inquire::validator::Validation::Valid)
                        }
                    });
                }

                let answer = text_prompt.prompt()?;
                Ok(Value::String(answer))
            }

            InputType::Number => {
                let mut num_prompt = CustomType::<i64>::new(&prompt_prefix);
                if let Some(help) = &input.help {
                    num_prompt = num_prompt.with_help_message(help);
                }
                if let Some(default_val) = &input.default {
                    if let Some(n) = default_val.as_i64() {
                        num_prompt = num_prompt.with_default(n);
                    }
                }

                let answer = num_prompt.prompt()?;
                Ok(Value::Number(answer.into()))
            }

            InputType::Password => {
                let mut pass_prompt = Password::new(&prompt_prefix)
                    .without_confirmation();
                if let Some(help) = &input.help {
                    pass_prompt = pass_prompt.with_help_message(help);
                }

                if input.required.unwrap_or(false) {
                    pass_prompt = pass_prompt.with_validator(|ans: &str| {
                        if ans.trim().is_empty() {
                            Ok(inquire::validator::Validation::Invalid(
                                "Password cannot be empty".into(),
                            ))
                        } else {
                            Ok(inquire::validator::Validation::Valid)
                        }
                    });
                }

                let answer = pass_prompt.prompt()?;
                Ok(Value::String(answer))
            }

            InputType::Confirm => {
                let default_bool = input
                    .default
                    .as_ref()
                    .and_then(|v| v.as_bool())
                    .unwrap_or(true);

                let mut confirm_prompt =
                    Confirm::new(&prompt_prefix).with_default(default_bool);
                if let Some(help) = &input.help {
                    confirm_prompt = confirm_prompt.with_help_message(help);
                }

                let answer = confirm_prompt.prompt()?;
                Ok(Value::Bool(answer))
            }

            InputType::Select => {
                let options = input.options.clone().unwrap_or_default();
                let default_idx = if let Some(serde_yaml::Value::String(def_str)) = &input.default {
                    options.iter().position(|opt| opt == def_str).unwrap_or(0)
                } else {
                    0
                };

                let mut select_prompt = Select::new(&prompt_prefix, options);
                if !input.options.as_ref().map(|o| o.is_empty()).unwrap_or(true) {
                    select_prompt = select_prompt.with_starting_cursor(default_idx);
                }
                if let Some(help) = &input.help {
                    select_prompt = select_prompt.with_help_message(help);
                }

                let answer = select_prompt.prompt()?;
                Ok(Value::String(answer))
            }
        }
    }

    /// Display summary of selected options and ask user to confirm deployment
    pub fn confirm_summary(recipe: &Recipe, values: &Value) -> Result<bool> {
        println!("{}", style("│").dim());
        println!("{}", style("┌─ Configuration Summary ──────────────────────").cyan());

        if let Value::Object(map) = values {
            for input in &recipe.inputs {
                let val_display = match map.get(&input.id) {
                    Some(Value::String(s)) => {
                        if input.input_type == InputType::Password {
                            "••••••••".to_string()
                        } else {
                            s.clone()
                        }
                    }
                    Some(Value::Number(n)) => n.to_string(),
                    Some(Value::Bool(b)) => if *b { "yes".to_string() } else { "no".to_string() },
                    _ => "-".to_string(),
                };

                println!(
                    "{} {:<20} {}",
                    style("│").cyan(),
                    style(&input.id).bold(),
                    style(val_display).green()
                );
            }
        }

        println!("{}", style("└──────────────────────────────────────────────").cyan());
        println!();

        let confirm = Confirm::new("Proceed with deployment?")
            .with_default(true)
            .prompt()?;

        Ok(confirm)
    }
}
