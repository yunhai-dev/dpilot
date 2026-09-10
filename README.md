# dpilot (Deployment Pilot)

A fast, interactive CLI pilot in Rust designed to streamline deploying containerized services, databases, and setting up system environments with beautiful prompts and declarative YAML recipes.

Inspired by modern frontend scaffolding CLI experiences (e.g. `create-next-app` / `clack`).

---

## Features

- **Interactive Question Wizard**: Dynamic prompts (`text`, `number`, `password`, `confirm`, `select`) with defaults, validators, and password hiding.
- **Embedded & Portable**: All pre-bundled service recipes are baked directly into the binary at compile time via `include_dir!` — zero external dependencies needed for out-of-the-box services.
- **Declarative YAML Recipes**: Configure inputs, parameter validation, and templated shell commands using Minijinja (`{{ var }}` and `{% if %}`).
- **Both Services & System Provisioning**: Deploy Docker services (`postgres`, `redis`, `nginx`, `uptime-kuma`) or run system-level software setup (like automated `install-docker` for Linux).
- **Rich Execution Engine**: Step-by-step progress spinners (`indicatif`), live indented execution logs, time tracking, failure handling, and red error summaries.
- **Multiple Recipe Sources**:
  - Embedded binary recipes (`dpilot run <name>`)
  - Local custom files (`dpilot run -f my-service.yaml`)
  - Project directory (`./recipes/<name>.yaml`)
  - User configuration directory (`~/.dpilot/recipes/<name>.yaml`)
  - Remote HTTP/HTTPS URLs (`dpilot run https://example.com/recipe.yaml`)
- **Dry-Run Mode**: Inspect and verify fully-rendered commands before executing anything on your machine.

---

## Installation

### Prerequisites
- [Rust](https://rustup.rs/) (edition 2024 / 1.85+)
- Docker (for deploying containerized recipes)

### Build from Source
```bash
git clone https://github.com/yunhai-dev/dpilot.git
cd dpilot
cargo build --release

# Optional: Install to PATH
cargo install --path .
```

---

## Quick Start

### 1. Interactive Selection (No Args)
Simply run `dpilot` to enter an interactive menu of all available recipes:
```bash
dpilot
```

### 2. List Available Recipes
```bash
dpilot list
```
Output:
```text
Available Services & Tooling Recipes

NAME               CATEGORY   VERSION    DESCRIPTION                             
────────────────────────────────────────────────────────────────────────────────
install-docker     system     1.0.0      Install Docker Engine and Docker Compose on Linux (Ubuntu/Debian/CentOS/RHEL)
nginx              service    1.0.0      High-performance Nginx web server or reverse proxy container
postgres           service    1.0.0      Deploy PostgreSQL relational database with persistent storage
redis              service    1.0.0      In-memory key-value data store with optional persistence and password
uptime-kuma        service    1.0.0      Self-hosted monitoring tool like Uptime Robot with fancy UI
```

You can also filter by category:
```bash
dpilot list -c system
dpilot list -c service
```

### 3. Deploy a Built-in Service
Run a specific service directly:
```bash
dpilot run postgres
```

### 4. Dry-Run Mode
Preview generated commands without executing them:
```bash
dpilot run postgres --dry-run
```

### 5. Run a Custom Recipe File
```bash
dpilot run -f ./my-custom-service.yaml
```

### 6. Validate Recipe Syntax
Check the integrity of any recipe YAML file before execution:
```bash
dpilot validate -f ./recipes/install-docker.yaml
```

---

## Recipe Specification

Each recipe is a single declarative YAML file. Below is an example:

```yaml
name: "postgres"
version: "1.0.0"
description: "Deploy PostgreSQL relational database with persistent storage"
category: "service"
author: "dpilot"

inputs:
  - id: "container_name"
    type: "text"
    prompt: "What is the container name?"
    default: "my-postgres"
    required: true

  - id: "port"
    type: "number"
    prompt: "Host port to expose"
    default: 5432
    required: true

  - id: "db_password"
    type: "password"
    prompt: "Database superuser password"
    required: true

  - id: "enable_volume"
    type: "confirm"
    prompt: "Persist data with Docker volume?"
    default: true

  - id: "version_tag"
    type: "select"
    prompt: "Select PostgreSQL version"
    options: ["17-alpine", "16-alpine", "15-alpine"]
    default: "16-alpine"

steps:
  - name: "Pull Docker Image"
    command: "docker pull postgres:{{ version_tag }}"
    show_output: false

  - name: "Remove existing container (if any)"
    command: "docker rm -f {{ container_name }} 2>/dev/null || true"
    allow_failure: true

  - name: "Start PostgreSQL Container"
    command: >
      docker run -d
      --name {{ container_name }}
      -p {{ port }}:5432
      -e POSTGRES_PASSWORD={{ db_password }}
      {% if enable_volume %}-v {{ container_name }}_data:/var/lib/postgresql/data{% endif %}
      --restart unless-stopped
      postgres:{{ version_tag }}

  - name: "Verify Container Health"
    command: "docker ps --filter name={{ container_name }}"
    show_output: true
```

### Input Types

| Type | Description | Inquire Widget |
| :--- | :--- | :--- |
| `text` | Free-form string input with optional validation and default value | `inquire::Text` |
| `number` | Integer input (port, memory limits, etc.) | `inquire::CustomType<i64>` |
| `password` | Masked string input for sensitive credentials | `inquire::Password` |
| `confirm` | Boolean question (Yes/No) | `inquire::Confirm` |
| `select` | Dropdown choice from a predefined list of options | `inquire::Select` |

---

## License

[MIT](LICENSE)
