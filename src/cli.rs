use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "dpilot",
    version,
    about = "交互式服务部署与系统环境配置引导工具",
    long_about = "dpilot 是一个现代交互式部署 CLI 工具，基于类 create-next-app 的问答式向导和声明式 YAML 配方，帮助快速在多平台部署容器服务、数据库与系统工具。"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// 运行服务或系统安装配方（交互式表单向导 + 逐步执行）
    Run {
        /// 配方名称、GitHub 远程配方或文件路径
        #[arg(value_name = "SERVICE")]
        service: Option<String>,

        /// 指定本地自定义 YAML 配方文件路径
        #[arg(short, long, value_name = "PATH")]
        file: Option<PathBuf>,

        /// 演练模式：仅打印渲染后的 Shell 命令，不实际执行
        #[arg(long)]
        dry_run: bool,
    },

    /// 查看所有可用配方（内置与 GitHub 远程配方）
    List {
        /// 按类别过滤配方（如 service, system）
        #[arg(short, long)]
        category: Option<String>,
    },

    /// 校验 YAML 配方文件的语法与结构完整性
    Validate {
        /// 待校验的配方文件路径
        #[arg(short, long, value_name = "PATH")]
        file: PathBuf,
    },
}
