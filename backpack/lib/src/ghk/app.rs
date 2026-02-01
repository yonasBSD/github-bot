use crate::cli::{Args, Commands, GitCommands};
use crate::ghk::config;
use anyhow::Result;
use std::env;

pub fn run(cli: Args) -> Result<()> {
    // Remove GITHUB_TOKEN from environment
    if env::var("GITHUB_TOKEN").is_ok() {
        unsafe {
            env::remove_var("GITHUB_TOKEN");
        }
    }

    // Set global flags
    config::setquiet(cli.quiet);
    config::setnocolor(cli.nocolor);

    // First, check for quiet to avoid unnecessary calls to isfirstrun()
    if !cli.quiet && config::isfirstrun() {
        welcome();

        // Save config to mark first run complete
        let cfg = config::Config::default();
        if let Err(e) = cfg.save() {
            // In production, we might want to log this differently.
            #[cfg(debug_assertions)]
            eprintln!("Debug: Failed to save config: {e}");
        }
    }

    match cli.command {
        Commands::Git { command } => match command {
            GitCommands::Init => crate::ghk::commands::init::run(),
            GitCommands::Login => crate::ghk::commands::login::run(),
            GitCommands::Logout => crate::ghk::commands::logout::run(),
            GitCommands::User { command } => crate::ghk::commands::user::run(command),
            GitCommands::Create => crate::ghk::commands::create::run(),
            GitCommands::Push | GitCommands::Save => crate::ghk::commands::push::run(),
            GitCommands::Pull | GitCommands::Sync => crate::ghk::commands::pull::run(),
            GitCommands::Clone { repo, dir } | GitCommands::Download { repo, dir } => {
                crate::ghk::commands::clone::run(repo, dir)
            }
            GitCommands::Status => crate::ghk::commands::status::run(),
            GitCommands::Setup => crate::ghk::commands::setup::run(),
            GitCommands::Undo => crate::ghk::commands::undo::run(),
            GitCommands::History { count } | GitCommands::Log { count } => {
                crate::ghk::commands::history::run(count)
            }
            GitCommands::Open => crate::ghk::commands::open::run(),
            GitCommands::Diff => crate::ghk::commands::diff::run(),
            GitCommands::Config { key, value } => crate::ghk::commands::config::run(key, value),
            GitCommands::Ignore { template } => crate::ghk::commands::ignore::run(template),
            GitCommands::License { kind } => crate::ghk::commands::license::run(kind),
            GitCommands::Branch { name } => crate::ghk::commands::branch::run(name),
            GitCommands::Completions { shell } => {
                crate::ghk::commands::completions::run(shell);
                Ok(())
            }
        },
        _ => Ok(()),
    }
}

#[cfg(not(debug_assertions))]
fn welcome() {
    use std::io::{self, Write};

    // Production-optimized version - fewer prints, no ANSI codes if nocolor is active
    if !config::isnocolor() {
        println!(
            "
  Welcome to ghk!"
        );
        println!(
            "
  Simple GitHub helper - push code without the complexity"
        );
        println!(
            "
  Quick start:"
        );
        println!("    ghk setup    Check requirements");
        println!("    ghk init     Start tracking a project");
        println!("    ghk create   Create repo on GitHub");
        println!("    ghk push     Save your changes");
        println!(
            "
  Run ghk --help for all commands
"
        );
    } else {
        // No ANSI codes for --nocolor
        println!(
            "
  Welcome to ghk!"
        );
        println!(
            "
  Simple GitHub helper - push code without the complexity"
        );
        println!(
            "
  Quick start:"
        );
        println!("    ghk setup    Check requirements");
        println!("    ghk init     Start tracking a project");
        println!("    ghk create   Create repo on GitHub");
        println!("    ghk push     Save your changes");
        println!(
            "
  Run ghk --help for all commands
"
        );
    }

    // Ensure that the output is flushed if necessary
    let _ = io::stdout().flush();
}

#[cfg(debug_assertions)]
fn welcome() {
    use std::io::{self, Write};

    // More detailed version for development
    println!(
        "
  ========================================="
    );
    println!("  Welcome to ghk! (Development Build)");
    println!("  =========================================");
    println!();
    println!("  Simple GitHub helper - push code without the complexity");
    println!("  Build: {}", env!("CARGO_PKG_VERSION"));
    println!();
    println!("  Quick start:");
    println!("    ghk setup    Check requirements");
    println!("    ghk init     Start tracking a project");
    println!("    ghk create   Create repo on GitHub");
    println!("    ghk push     Save your changes");
    println!();
    println!("  Debug commands:");
    println!("    ghk --verbose     Show detailed logs");
    println!("    ghk --dry-run     Test commands");
    println!();
    println!("  Run ghk --help for all commands");
    println!(
        "  =========================================
"
    );

    // Under development, always flush for debugging
    let _ = io::stdout().flush();
}
