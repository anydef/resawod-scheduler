# Usage Guide

## Prerequisites

- Rust toolchain (install via [rustup](https://rustup.rs))
- A RESAWOD/Nubapp account at your gym
- Your gym's `application_id` (use the `discover` command to find the rest)

## Build

```bash
cargo build --release
```

The binary will be at `target/release/resawod-scheduler`.

## Running

You can run the CLI either via the compiled binary or directly with `cargo run`.

**Via binary** (after building):
```bash
./target/release/resawod-scheduler <COMMAND> [OPTIONS]
```

**Via `cargo run`** (builds and runs in one step):
```bash
cargo run -- <COMMAND> [OPTIONS]
```

Examples:
```bash
# Discover activity categories (reads credentials from config.toml)
cargo run -- discover

# Book with verbose output
cargo run -- -v book --multi-users

# Dry run
cargo run -- book --multi-users --debug

# Show help
cargo run -- --help
cargo run -- book --help
cargo run -- discover --help
```

## Configuration

All settings live in a single `config.toml` file. Copy the example and fill in your details:

```bash
cp config.toml.example config.toml
```

### Config file structure

```toml
[app]
application_id = "36307036"        # Your gym's Nubapp ID
category_activity_id = "2179"      # Activity category (e.g. CrossFit WOD)

[slots]
monday = "18:30:00"
tuesday = "19:30:00"
wednesday = "18:30:00"
thursday = "19:30:00"
friday = "18:30:00"
saturday = "11:00:00"

[[users]]
name = "Bob"
login = "bob@gmail.com"
password = "super-duper-secret-password"
slots = ["monday", "wednesday"]

[[users]]
name = "Alice"
login = "alice@gmail.com"
password = "nobody-cares"
slots = ["friday", "saturday"]
```

### Sections

**`[app]`** — Gym-specific Nubapp identifiers:
- `application_id` — Your gym's ID on the Nubapp platform
- `category_activity_id` — Activity type (e.g. CrossFit WOD, Open Gym)

**`[slots]`** — Global time mapping. Defines which time to book for each day of the week. All users share the same time preferences per day.

**`[[users]]`** — One block per user account:
- `name` — Display name (for logging)
- `login` — Email address used to log in to RESAWOD
- `password` — Account password
- `slots` — Array of day names to book (e.g. `["monday", "friday"]`)

> `config.toml` is gitignored since it contains credentials. Only `config.toml.example` is tracked.

## CLI Usage

The CLI uses subcommands: `discover`, `book`, and `serve`.

```
resawod-scheduler <COMMAND> [OPTIONS]
```

### `discover` — Find your gym's IDs

Use this when setting up for the first time. By default it reads `application_id` and credentials (first user) from `config.toml`. All values can be overridden via CLI flags.

```bash
# Uses application_id + first user's credentials from config.toml
resawod-scheduler discover

# Override specific values
resawod-scheduler discover --application-id 36307036 -u your@email.com -p secret
```

This will log in and display:
- The confirmed gym name and application ID
- All available activity categories with their IDs

Use the output to fill in `category_activity_id` in your `config.toml`.

Add `-v` for the full raw API responses:
```bash
resawod-scheduler -v discover
```

### `discover` options

| Flag | Long               | Description                                       |
|------|--------------------|---------------------------------------------------|
| `-c` | `--config`         | Path to config file (default: `config.toml`)      |
|      | `--application-id` | Override application ID from config               |
| `-u` | `--user`           | Override login email (default: first user in config) |
| `-p` | `--password`       | Override password (default: first user in config)  |

### `book` — Book training slots

**Multi-user mode** (recommended for automation):
```bash
resawod-scheduler book --multi-users
```
Processes all users from `config.toml`.

**Single-user mode:**
```bash
resawod-scheduler book --user your@email.com --password secret --slots monday,wednesday
```
Slot times are still read from `config.toml`.

### `book` options

| Flag | Long                       | Description                                        |
|------|----------------------------|----------------------------------------------------|
| `-m` | `--multi-users`            | Process all users from config file                 |
| `-u` | `--user`                   | Single-user login email                            |
| `-p` | `--password`               | Single-user password                               |
| `-s` | `--slots`                  | Single-user days to book (comma-separated)         |
| `-c` | `--config`                 | Path to config file (default: `config.toml`)       |
| `-d` | `--debug`                  | Dry run — show what would be booked without booking|
|      | `--application-id`         | Override gym ID from config                        |
|      | `--category-activity-id`   | Override activity ID from config                   |

### Global options

| Flag | Long        | Description                    |
|------|-------------|--------------------------------|
| `-v` | `--verbose` | Print detailed API responses   |

### Examples

**First-time setup — discover your gym's activity categories:**
```bash
resawod-scheduler discover
```

**Dry run to see available slots:**
```bash
resawod-scheduler book --multi-users --debug --verbose
```

**Book for all users:**
```bash
resawod-scheduler book --multi-users
```

**Book for a single user with a different config path:**
```bash
resawod-scheduler book \
  --user me@example.com \
  --password mypass \
  --slots monday,friday \
  --config /etc/resawod/config.toml
```

### `serve` — Web application mode

Starts a long-running web server that provides a dashboard and continuous background automation.

```bash
resawod-scheduler serve --config /app/config.toml
```

This mode is designed for always-on deployment (e.g., Docker container on a home server).

#### Features

**Autobooking**: The server automatically books slots for all configured users based on their schedules. When new slots become available (typically when the gym publishes the next week's schedule), the scheduler detects and books them without manual intervention.

**Waiting list monitoring**: If a desired slot is full, the scheduler adds the user to the waiting list and periodically checks for openings. When a spot becomes available (e.g., someone cancels), it automatically books the slot and removes the user from the waiting list.

**Web dashboard**: Provides a browser-based interface to view:
- Current booking status for all users
- Upcoming scheduled slots
- Waiting list entries
- Recent booking activity

#### `serve` options

| Flag | Long       | Description                              |
|------|------------|------------------------------------------|
| `-c` | `--config` | Path to config file (default: `config.toml`) |
|      | `--port`   | HTTP port to listen on (default: `3009`) |

#### Running with Docker

The recommended way to run the web application is via Docker:

```bash
docker run -d \
  -p 3009:3009 \
  -v /path/to/config.toml:/app/config.toml:ro \
  -v /path/to/data:/app/data \
  your-registry/resawod-scheduler:latest
```

Or use the provided `docker-compose.yml` for Portainer deployment.

## Automation

To run this automatically (e.g., every Sunday evening when the new week's schedule is published), set up a cron job:

```bash
# Run every Sunday at 21:00
0 21 * * 0 /path/to/resawod-scheduler --multi-users >> /var/log/resawod.log 2>&1
```

Or with systemd timers, launchd (macOS), etc.
