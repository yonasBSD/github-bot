use crate::ghk::{config::Config, util};
use anyhow::Result;

pub fn run(key: Option<String>, value: Option<String>) -> Result<()> {
    let mut cfg = Config::load();

    match (key, value) {
        // Show all settings
        (None, None) => {
            println!();
            util::info("Current settings:");
            util::dim(&format!("  quiet   = {}", cfg.quiet));
            util::dim(&format!("  nocolor = {}", cfg.nocolor));
            util::dim(&format!(
                "  editor  = {}",
                cfg.editor.as_deref().unwrap_or("(default)")
            ));
            util::dim(&format!(
                "  org  = {}",
                cfg.org.as_deref().unwrap_or("")
            ));
            println!();
            util::dim(&format!("Config file: {}", Config::path().display()));
            println!();
        }
        // Show one setting
        (Some(k), None) => match cfg.get(&k) {
            Some(v) => println!("{}", v),
            None => util::warn(&format!("Unknown setting: {}", k)),
        },
        // Set a value
        (Some(k), Some(v)) => {
            cfg.set(&k, &v)?;
            util::ok(&format!("{} = {}", k, v));
        }
        _ => {}
    }

    Ok(())
}
