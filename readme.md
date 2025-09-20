# gits

**gits** is a CLI tool for managing and synchronizing many Git repositories at once.  
It helps you:

- ✅ Keep dozens of repos up to date with one command
- 🛡️ Avoid mistakes with dirty-tree guardrails and clear reporting
- 🎯 Organize repos into groups and “watched” sets
- 🚀 Automate daily pulls/commits across your workspace

---

## Installation

```bash
# Clone and build with Cargo
git clone https://github.com/kadinvanvalin/git-sync.git
cd git-sync
cargo install --path .
````
Configuration

gits uses two config files by default (found under ~/.config/gits/ on macOS/Linux or %APPDATA%\gits\ on Windows). You can override with environment variables or flags.

1. Settings (settings.toml)

Defines remotes, credentials, and defaults.

```toml
[remotes.gitlab]
# "token_env hi" is not the actual token — it is the NAME of an env var.
# Here, GITLAB_PAT_TOKEN must exist in your environment.
token_env = "GITLAB_PAT_TOKEN"
project_directory = "/Users/kadin/code/gitlab"
gitlab_api_url = "https://gitlab.com/api/v4"
watch_groups = ["my-org/platform", "my-org/tools"]
watch_projects = ["my-org/infra/terraform-modules"]
last_pull = "2025-09-18T14:33:27Z"
```
Fields:
•	token — Personal access token (use env vars if possible)
•	project_directory — Local path for clones
•	gitlab_api_url — Base GitLab API URL
•	watch_groups / watch_projects — Defaults for sync-watched
•	last_pull — Timestamp of last API sync (RFC3339)


2. Projects (projects.toml)

Lists all repos grouped by namespace.
You can also create projects.watched.toml for a smaller subset.

```toml
[groups]
"my-org/platform" = [
  "my-org/platform/service-a",
  "my-org/platform/service-b"
]
"my-org/tools" = [
  "my-org/tools/release-bot",
  "my-org/tools/bench-runner"
]
```