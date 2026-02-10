mod client;
mod commands;
mod config;
mod models;
mod scheduler;
mod web;

use std::path::PathBuf;

use anyhow::{bail, Result};
use clap::{Parser, Subcommand};
use tracing::{error, info};

use models::User;

/// RESAWOD auto-scheduler — automatically book training slots on Nubapp.
#[derive(Parser, Debug)]
#[command(version, about)]
struct Cli {
    #[command(subcommand)]
    command: Command,

    /// Print detailed API responses
    #[arg(short = 'v', long, global = true)]
    verbose: bool,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Book training slots for one or more users
    ///
    /// Examples:
    ///   book tuesday           — book Tuesday for the first user in config
    ///   book tuesday,friday    — book Tuesday and Friday
    ///   book --multi-users     — book configured slots for all users
    Book {
        /// Days to book (comma-separated, e.g. "tuesday" or "tuesday,friday").
        /// Uses first user from config. Overrides --slots.
        #[arg(value_name = "DAYS")]
        days: Option<String>,

        /// Run for all users defined in the config file
        #[arg(short, long)]
        multi_users: bool,

        /// Single-user login
        #[arg(short = 'u', long)]
        user: Option<String>,

        /// Single-user password
        #[arg(short = 'p', long)]
        password: Option<String>,

        /// Single-user slot days (comma-separated, e.g. "monday,wednesday")
        #[arg(short = 's', long)]
        slots: Option<String>,

        /// Path to config file
        #[arg(short = 'c', long, default_value = "config.toml")]
        config: PathBuf,

        /// Override application ID from config
        #[arg(long)]
        application_id: Option<String>,

        /// Override category activity ID from config
        #[arg(long)]
        category_activity_id: Option<String>,

        /// Dry run — find slots but do not actually book them
        #[arg(short = 'd', long)]
        debug: bool,
    },

    /// Show active bookings for a user
    Bookings {
        /// Path to config file
        #[arg(short = 'c', long, default_value = "config.toml")]
        config: PathBuf,

        /// Override login from config (defaults to first user)
        #[arg(short = 'u', long)]
        user: Option<String>,

        /// Override password from config (defaults to first user)
        #[arg(short = 'p', long)]
        password: Option<String>,
    },

    /// Start web dashboard server
    Serve {
        /// Path to config file
        #[arg(short = 'c', long, default_value = "config.toml")]
        config: PathBuf,

        /// Listen address (e.g. "0.0.0.0:3000")
        #[arg(short = 'a', long, default_value = "0.0.0.0:3009")]
        addr: String,
    },

    /// Discover gym IDs — log in and show application ID and activity categories
    Discover {
        /// Path to config file
        #[arg(short = 'c', long, default_value = "config.toml")]
        config: PathBuf,

        /// Override application ID from config
        #[arg(long)]
        application_id: Option<String>,

        /// Override login from config (defaults to first user)
        #[arg(short = 'u', long)]
        user: Option<String>,

        /// Override password from config (defaults to first user)
        #[arg(short = 'p', long)]
        password: Option<String>,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let filter = if cli.verbose { "debug" } else { "info" };
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .init();

    match &cli.command {
        Command::Serve { config, addr } => {
            let cfg = config::load_config(config)?;
            web::serve(cfg, config, addr).await?;
        }
        Command::Bookings {
            config,
            user,
            password,
        } => {
            commands::run_bookings(cli.verbose, config, user, password).await?;
        }
        Command::Discover {
            config,
            application_id,
            user,
            password,
        } => {
            let cfg = config::load_config(config)?;
            let (login, pass) =
                commands::resolve_credentials(user, password, cfg.users.first())?;
            let app_id = application_id
                .as_deref()
                .unwrap_or(&cfg.app.application_id);

            commands::run_discover(app_id, login, pass, cli.verbose).await?;
        }
        Command::Book {
            days,
            multi_users,
            user,
            password,
            slots,
            config,
            application_id,
            category_activity_id,
            debug,
        } => {
            let cfg = config::load_config(config)?;

            let app_id = application_id
                .as_deref()
                .unwrap_or(&cfg.app.application_id);
            let cat_id = category_activity_id
                .as_deref()
                .unwrap_or(&cfg.app.category_activity_id);

            if *multi_users {
                for (i, u) in cfg.users.iter().enumerate() {
                    if let Err(e) =
                        commands::run_for_user(app_id, cat_id, cli.verbose, *debug, u, &cfg.slots)
                            .await
                    {
                        error!("Error processing user {}: {:#}", u.name, e);
                    }
                    if i < cfg.users.len() - 1 {
                        info!("Waiting 5 seconds before next user...");
                        tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                    }
                }
            } else if days.is_some() || slots.is_some() {
                // Quick book: positional days or --slots, credentials from config or flags
                let day_src = days.as_deref().or(slots.as_deref()).unwrap();
                let slot_days: Vec<String> = day_src
                    .split(',')
                    .map(|s| s.trim().to_lowercase())
                    .filter(|s| !s.is_empty())
                    .collect();

                if slot_days.is_empty() {
                    bail!("No days specified");
                }

                let first_user = cfg.users.first();
                let (login, pass) =
                    commands::resolve_credentials(user, password, first_user)?;

                let u = User {
                    name: login.to_string(),
                    login: login.to_string(),
                    password: pass.to_string(),
                    slots: slot_days,
                };

                commands::run_for_user(app_id, cat_id, cli.verbose, *debug, &u, &cfg.slots)
                    .await?;
            } else {
                bail!(
                    "Specify days to book (e.g. `book tuesday`), \
                     --multi-users for all users from config, \
                     or --user/--password for explicit credentials.\n\
                     Run with --help for usage information."
                );
            }
        }
    }

    Ok(())
}
