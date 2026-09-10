use crate::recipe::{Recipe, StepSpec};
use crate::template::TemplateEngine;
use anyhow::{bail, Result};
use console::style;
use indicatif::{ProgressBar, ProgressStyle};
use serde_json::Value;
use std::process::Stdio;
use std::time::{Duration, Instant};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

pub struct ExecutionEngine;

impl ExecutionEngine {
    pub async fn execute_recipe(
        recipe: &Recipe,
        context: &Value,
        dry_run: bool,
    ) -> Result<()> {
        let total_steps = recipe.steps.len();
        println!();
        println!(
            "{}",
            style(format!("🚀 开始执行部署: 共 {} 个步骤", total_steps))
                .bold()
                .green()
        );
        println!("{}", style("─".repeat(50)).dim());

        for (idx, step) in recipe.steps.iter().enumerate() {
            let step_num = idx + 1;

            // 检查步骤级别平台限制
            if !Recipe::supports_platform(&step.platforms) {
                println!(
                    "{} [{}/{}] {} {}",
                    style("↷").dim(),
                    step_num,
                    total_steps,
                    style(&step.name).dim(),
                    style(format!("(跳过：不支持当前系统 {})", Recipe::current_os())).dim()
                );
                continue;
            }

            let rendered_cmd = TemplateEngine::render_str(&step.command, context)?;

            if dry_run {
                println!(
                    "{} [{}/{}] {}",
                    style("✦").yellow().bold(),
                    step_num,
                    total_steps,
                    style(&step.name).bold()
                );
                for line in rendered_cmd.lines() {
                    println!("    {} {}", style("$").dim(), style(line).cyan());
                }
                continue;
            }

            Self::execute_single_step(step, step_num, total_steps, &rendered_cmd).await?;
        }

        println!("{}", style("─".repeat(50)).dim());
        if dry_run {
            println!(
                "{}",
                style("✔ 演练模式执行完成。未在系统执行任何真实命令。")
                    .yellow()
                    .bold()
            );
        } else {
            println!(
                "{}",
                style(format!("✔ 配方 [{}] 已成功部署完成！", recipe.name))
                    .green()
                    .bold()
            );
        }
        println!();

        Ok(())
    }

    async fn execute_single_step(
        step: &StepSpec,
        step_num: usize,
        total_steps: usize,
        rendered_cmd: &str,
    ) -> Result<()> {
        let start_time = Instant::now();
        let show_output = step.show_output.unwrap_or(true);
        let allow_failure = step.allow_failure.unwrap_or(false);

        let pb = ProgressBar::new_spinner();
        pb.set_style(
            ProgressStyle::default_spinner()
                .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏ ")
                .template("{spinner:.cyan} [{msg_prefix}] {msg}")
                .unwrap(),
        );
        pb.set_message(format!("{}", style(&step.name).bold()));
        pb.enable_steady_tick(Duration::from_millis(80));

        // 根据操作系统选用对应 Shell
        #[cfg(target_os = "windows")]
        let mut cmd = {
            let mut c = Command::new("cmd");
            c.args(["/C", rendered_cmd]);
            c
        };

        #[cfg(not(target_os = "windows"))]
        let mut cmd = {
            let mut c = Command::new("sh");
            c.args(["-c", rendered_cmd]);
            c
        };

        if let Some(dir) = &step.working_dir {
            cmd.current_dir(dir);
        }
        if let Some(env_map) = &step.env {
            for (k, v) in env_map {
                cmd.env(k, v);
            }
        }

        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        let mut child = match cmd.spawn() {
            Ok(child) => child,
            Err(e) => {
                pb.finish_and_clear();
                eprintln!(
                    "{} [{}/{}] 步骤 '{}' 创建进程失败: {}",
                    style("✖").red().bold(),
                    step_num,
                    total_steps,
                    step.name,
                    e
                );
                bail!("进程创建失败: {}", e);
            }
        };

        let stdout = child.stdout.take().expect("Child stdout piped");
        let stderr = child.stderr.take().expect("Child stderr piped");

        let mut stdout_reader = BufReader::new(stdout).lines();
        let mut stderr_reader = BufReader::new(stderr).lines();

        let mut stdout_lines = Vec::new();
        let mut stderr_lines = Vec::new();

        loop {
            tokio::select! {
                line = stdout_reader.next_line() => {
                    match line {
                        Ok(Some(l)) => {
                            if show_output {
                                pb.println(format!("  {} {}", style("│").dim(), l));
                            }
                            stdout_lines.push(l);
                        }
                        Ok(None) => break,
                        Err(_) => break,
                    }
                }
                line = stderr_reader.next_line() => {
                    match line {
                        Ok(Some(l)) => {
                            if show_output {
                                pb.println(format!("  {} {}", style("│").dim(), style(&l).yellow()));
                            }
                            stderr_lines.push(l);
                        }
                        Ok(None) => break,
                        Err(_) => break,
                    }
                }
            }
        }

        let status = child.wait().await?;
        let elapsed = start_time.elapsed();
        let time_str = format!("{:.1}s", elapsed.as_secs_f32());

        if status.success() {
            pb.finish_and_clear();
            println!(
                "{} [{}/{}] {} {}",
                style("✔").green().bold(),
                step_num,
                total_steps,
                style(&step.name).bold(),
                style(format!("({time_str})")).dim()
            );
        } else if allow_failure {
            pb.finish_and_clear();
            println!(
                "{} [{}/{}] {} {}",
                style("⚠").yellow().bold(),
                step_num,
                total_steps,
                style(&step.name).yellow(),
                style(format!("(执行失败退出码 {}, 已按配置忽略)", status)).dim()
            );
        } else {
            pb.finish_and_clear();
            println!(
                "{} [{}/{}] {} {}",
                style("✖").red().bold(),
                step_num,
                total_steps,
                style(&step.name).red().bold(),
                style(format!("(失败退出状态码: {})", status)).red()
            );

            if !show_output && !stderr_lines.is_empty() {
                println!("{}", style("┌─ 错误详情输出 ───────────────────────────────").red());
                for err_line in stderr_lines.iter().rev().take(15).rev() {
                    println!("{} {}", style("│").red(), err_line);
                }
                println!("{}", style("└──────────────────────────────────────────────").red());
            }

            bail!(
                "步骤 '{}' 执行失败，退出状态码: {}",
                step.name,
                status
            );
        }

        Ok(())
    }
}
