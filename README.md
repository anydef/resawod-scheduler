# Why
I was tired of missing on my training because of the booked out slots, hence this small automation project.

## NB
API usage was reverse engineering, and is a subject to change by the RESAWOD devs.


# RESAWOD Scheduler

A CLI and web application for automatically booking training slots on RESAWOD/Nubapp gym platforms.

## Overview

RESAWOD Scheduler automates the process of booking gym sessions. It connects to the Nubapp API (used by RESAWOD-based gyms) to discover available slots and book them according to a configurable schedule. Supports multiple users with individual slot preferences.

## Features

- Automatic slot booking based on day/time preferences
- Multi-user support for households or groups
- Activity filtering (e.g., WOD, Skill, Conditioning)
- Discovery mode to find gym and activity IDs
- Dry-run mode for testing
- Web dashboard for monitoring
- Docker support for self-hosted deployment

## Requirements

- Rust 1.70+ (for building from source)
- A RESAWOD/Nubapp account at your gym
- Your gym's `application_id`

## Quick Start

1. Clone the repository and build:
   ```bash
   cargo build --release
   ```

2. Copy and configure:
   ```bash
   cp config.toml.example config.toml
   # Edit config.toml with your gym ID and credentials
   ```

3. Discover your gym's activity categories:
   ```bash
   ./target/release/resawod-scheduler discover
   ```

4. Book slots:
   ```bash
   ./target/release/resawod-scheduler book --multi-users
   ```

## Configuration

All settings are stored in `config.toml`:

```toml
[app]
application_id = "12345678"       # Your gym's Nubapp ID
category_activity_id = "1234"    # Activity category ID

[slots.monday]
time = "19:30:00"
activity = "WOD"

[slots.wednesday]
time = "18:30:00"

[[users]]
name = "John"
login = "john@example.com"
password = "secret"
slots = ["monday", "wednesday"]
```

See [docs/usage.md](docs/usage.md) for detailed configuration options.

## CLI Commands

| Command    | Description                              |
|------------|------------------------------------------|
| `discover` | Find gym and activity IDs                |
| `book`     | Book training slots for configured users |
| `serve`    | Start the web dashboard                  |

### Common Options

- `-c, --config` - Path to config file (default: `config.toml`)
- `-v, --verbose` - Enable verbose output
- `-d, --debug` - Dry run mode

## Docker Deployment

Build and push to a private registry:

```bash
./build-and-push.sh
```

Deploy via Portainer/Terraform:

```bash
./deploy-portainer.sh
```

See `docker-compose.yml` for the full container configuration.

## Project Structure

```
src/
  main.rs         - Entry point and CLI definition
  client.rs       - Nubapp API client
  config.rs       - Configuration parsing
  commands.rs     - CLI command handlers
  scheduler.rs    - Booking logic
  models.rs       - Data structures
  web/            - Web dashboard (Axum + Leptos)
```

## Automation

Set up a cron job to book slots automatically:

```bash
# Run every Sunday at 21:00 when new slots become available
0 21 * * 0 /path/to/resawod-scheduler book --multi-users
```
