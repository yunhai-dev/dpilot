use crate::recipe::{InputSpec, InputType, Recipe};
use anyhow::Result;
use console::style;
use inquire::{Confirm, CustomType, Password, Select, Text};
use serde_json::{Map, Value};

pub struct PromptWizard;

impl PromptWizard {
    /// 打印当前配方的美化头部信息
    pub fn print_banner(recipe: &Recipe) {
        println!();
        println!(
            "{}",
            style(format!("◆ 正在配置配方: {} (v{})", recipe.name, recipe.version))
                .cyan()
                .bold()
        );
        println!(
            "{}",
            style(format!("│ {}", recipe.description)).dim()
        );
        if let Some(author) = &recipe.author {
            println!("{}", style(format!("│ 作者: {}", author)).dim());
        }
        if let Some(platforms) = &recipe.platforms {
            println!(
                "{}",
                style(format!("│ 适用平台: {}", platforms.join(", "))).dim()
            );
        }
        println!("{}", style("│").dim());
    }

    /// 根据 Recipe 的 inputs 定义提示用户输入
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
                                "此输入项不能为空".into(),
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
                                "密码不能为空".into(),
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

    /// 显示配置参数汇总并请求用户确认部署
    pub fn confirm_summary(recipe: &Recipe, values: &Value) -> Result<bool> {
        println!("{}", style("│").dim());
        println!("{}", style("┌─ 配置参数汇总 ────────────────────────────────").cyan());

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
                    Some(Value::Bool(b)) => if *b { "是".to_string() } else { "否".to_string() },
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

        let confirm = Confirm::new("确认以上配置并开始部署吗？")
            .with_default(true)
            .prompt()?;

        Ok(confirm)
    }
}
