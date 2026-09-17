# dpilot (Deployment Pilot)

Rust 编写的现代交互式 CLI 工具，通过类 `create-next-app` 风格的问答向导与声明式 YAML 配方，优雅地完成多平台容器服务部署、数据库配置与系统环境初始化。

---

## 核心特性

- 🌟 **纯中文交互问答向导**：支持 `文本 (text)`、`数值 (number)`、`密码隐匿 (password)`、`布尔确认 (confirm)`、`下拉单选 (select)` 等多种输入控件，支持默认值、必填校验与密码掩码。
- 📦 **直接联动 GitHub 远程仓库**：运行时直接通过 GitHub API / Raw 获取最新配方，新加配方即刻生效；同时内嵌默认配方，断网自动降级。
- 🖥 **多操作系统与架构兼容**：配方和单步骤支持 `platforms` 属性（如 `["linux"]`、`["linux", "darwin", "windows"]`），智能检测宿主机环境并在不支持时发出提醒或跳过特定步骤。
- 📜 **强大的 Minijinja 模板引擎**：支持使用 `{{ var }}` 与 `{% if ... %}` 动态生成命令、环境变量与挂载参数。
- 📊 **可视化步进执行引擎**：包含实时 Spinner 旋转指示器、多行日志缩进输出、步骤耗时统计、忽略非致命失败与错误详情框。
- 🔍 **安全演练模式 (`--dry-run`)**：在不触碰系统环境的前提下，预览所有经变量替换与条件计算后的真实 Shell 执行命令。

---

## 快速安装

### 下载可执行文件

从 [GitHub Releases](https://github.com/yunhai-dev/dpilot/releases) 下载与系统匹配的压缩包，无需安装 Rust：

| 系统 | 文件后缀 |
| --- | --- |
| Linux x64 | `x86_64-unknown-linux-gnu.tar.gz` |
| Linux ARM64 | `aarch64-unknown-linux-gnu.tar.gz` |
| macOS Intel | `x86_64-apple-darwin.tar.gz` |
| macOS Apple Silicon | `aarch64-apple-darwin.tar.gz` |
| Windows x64 | `x86_64-pc-windows-msvc.zip` |

Linux 构建环境为 Ubuntu 24.04，使用 glibc，不适用于 Alpine/musl 或较旧的 glibc 系统。macOS/Windows 产物暂未签名或公证。

例如 Linux x64，下载压缩包及 `SHA256SUMS` 后：

```bash
sha256sum --ignore-missing --check SHA256SUMS
tar -xzf dpilot-v0.1.0-x86_64-unknown-linux-gnu.tar.gz
sudo install -m 755 dpilot /usr/local/bin/dpilot
dpilot --version
```

macOS 可使用 `shasum -a 256` 核对校验值；Windows 解压后直接运行 `dpilot.exe`，或将所在目录加入 PATH。

### 源码编译安装
```bash
git clone https://github.com/yunhai-dev/dpilot.git
cd dpilot
cargo build --release

# 可选：安装到系统 PATH
cargo install --path .
```

---

## 常用命令

### 1. 交互式选择菜单（无参数）
直接运行 `dpilot`，进入交互式配方选择界面：
```bash
dpilot
```

### 2. 查看所有可用配方列表
```bash
dpilot list
```
按类别过滤：
```bash
dpilot list -c system   # 仅查看系统工具配置 (如 Docker 安装)
dpilot list -c service  # 仅查看容器化服务
```

### 3. 运行指定配方
```bash
dpilot run postgres
```

### 4. 演练预览命令（不实际执行）
```bash
dpilot run postgres --dry-run
```

### 5. 执行本地自定义 YAML 配方
```bash
dpilot run -f ./my-service.yaml
```

### 6. 校验配方文件完整性与语法
```bash
dpilot validate -f ./recipes/nginx.yaml
```

---

## 配方 YAML 规范

每个配方为独立的 YAML 文件：

```yaml
name: "postgres"
version: "1.0.0"
description: "部署 PostgreSQL 关系型数据库 (支持数据持久化与版本选择)"
category: "service"
platforms: ["linux", "darwin", "windows"]
author: "dpilot"

inputs:
  - id: "container_name"
    type: "text"
    prompt: "容器名称是什么？"
    default: "my-postgres"
    required: true

  - id: "port"
    type: "number"
    prompt: "映射的主机端口"
    default: 5432
    required: true

  - id: "db_password"
    type: "password"
    prompt: "超级用户密码"
    required: true

  - id: "enable_volume"
    type: "confirm"
    prompt: "是否使用 Docker 数据卷持久化存储？"
    default: true

  - id: "version_tag"
    type: "select"
    prompt: "选择 PostgreSQL 版本"
    options: ["17-alpine", "16-alpine", "15-alpine"]
    default: "16-alpine"

steps:
  - name: "拉取 PostgreSQL 镜像"
    command: "docker pull postgres:{{ version_tag }}"
    show_output: false

  - name: "清理同名旧容器 (若存在)"
    command: "docker rm -f {{ container_name }} 2>/dev/null || true"
    allow_failure: true

  - name: "启动 PostgreSQL 容器"
    command: >
      docker run -d
      --name {{ container_name }}
      -p {{ port }}:5432
      -e POSTGRES_PASSWORD={{ db_password }}
      {% if enable_volume %}-v {{ container_name }}_data:/var/lib/postgresql/data{% endif %}
      --restart unless-stopped
      postgres:{{ version_tag }}

  - name: "检查容器运行状态"
    command: "docker ps --filter name={{ container_name }} --format 'table {{.Names}}\t{{.Status}}\t{{.Ports}}'"
    show_output: true
```

---

## 发布版本

工作流位于 `.github/workflows/release.yml`。先更新并提交 `Cargo.toml` 与 `Cargo.lock` 中的版本，再推送同版本标签：

```bash
git tag v0.1.0
git push origin v0.1.0
```

标签必须与 `Cargo.toml` 的版本完全一致，例如版本 `0.2.0` 对应 `v0.2.0`。五个平台全部通过测试、构建、CLI 冒烟检查后，工作流自动创建 GitHub Release，上传压缩包和汇总校验文件 `SHA256SUMS`；带 `-` 的版本标签标记为预发布。

也可在 Actions → Build and release → Run workflow 手动构建。手动运行只上传保留 14 天的 Actions artifacts，不创建 Release。

---

## 开源协议

[MIT](LICENSE)
