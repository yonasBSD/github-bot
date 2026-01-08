use crate::ghk::{git, util};
use anyhow::{Result, bail};
use dialoguer::Select;
use std::fs;

const TEMPLATES: &[(&str, &str)] = &[
    ("node", "node_modules/\nnpm-debug.log\n.env\ndist/\n"),
    ("python", "__pycache__/\n*.py[cod]\n.env\nvenv/\n.venv/\n"),
    ("rust", "target/\nCargo.lock\n"),
    ("go", "bin/\npkg/\n*.exe\n"),
    ("java", "*.class\n*.jar\ntarget/\n.idea/\n"),
    ("web", "node_modules/\ndist/\n.env\n*.log\n"),
    ("macos", ".DS_Store\n.AppleDouble\n.LSOverride\n"),
    ("windows", "Thumbs.db\nehthumbs.db\nDesktop.ini\n"),
    ("linux", "*~\n.fuse_hidden*\n.nfs*\n"),
    ("ide", ".idea/\n.vscode/\n*.swp\n*.swo\n.project\n"),
];

pub fn run(template: Option<String>) -> Result<()> {
    if !git::isrepo() {
        util::err("Not a git repository");
        util::dim("Run 'ghk init' first");
        bail!("Not a git repository");
    }

    let name = match template {
        Some(t) => t,
        None => {
            let names: Vec<&str> = TEMPLATES.iter().map(|(n, _)| *n).collect();
            let idx = Select::new()
                .with_prompt("Choose template")
                .items(&names)
                .default(0)
                .interact()?;
            names[idx].to_string()
        }
    };

    let content = TEMPLATES.iter().find(|(n, _)| *n == name).map(|(_, c)| *c);

    match content {
        Some(c) => {
            let path = ".gitignore";
            let existing = fs::read_to_string(path).unwrap_or_default();

            // Append if file exists
            let new = if existing.is_empty() {
                format!("# {}\n{}", name, c)
            } else if existing.contains(c.lines().next().unwrap_or("")) {
                util::warn("Already has this template");
                return Ok(());
            } else {
                format!("{}\n# {}\n{}", existing.trim(), name, c)
            };

            fs::write(path, new)?;
            util::ok(&format!("Added {} template to .gitignore", name));
        }
        None => {
            util::err(&format!("Unknown template: {}", name));
            util::dim("Available: node, python, rust, go, java, web, macos, windows, linux, ide");
        }
    }

    Ok(())
}
