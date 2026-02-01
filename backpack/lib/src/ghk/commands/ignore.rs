use crate::ghk::{git, util};
use anyhow::{Result, bail};
use dialoguer::Select;
use std::fs;

const BASE_TEMPLATES: &[(&str, &str)] = &[("core-dumps", "*.core/\n")];

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

    // Pick template name
    let name = if let Some(t) = template {
        t
    } else {
        let names: Vec<&str> = TEMPLATES.iter().map(|(n, _)| *n).collect();
        let idx = Select::new()
            .with_prompt("Choose template")
            .items(&names)
            .default(0)
            .interact()?;
        names[idx].to_string()
    };

    // Find main template content
    let main = TEMPLATES.iter().find(|(n, _)| *n == name).map(|(_, c)| *c);

    if let Some(main_content) = main {
        let path = ".gitignore";
        let existing = fs::read_to_string(path).unwrap_or_default();

        // Build final template: main + all base templates
        let mut combined = format!("# {name}\n{main_content}");

        use std::fmt::Write as _;
        for (base_name, base_content) in BASE_TEMPLATES {
            let _ = write!(combined, "\n# base: {base_name}\n{base_content}");
        }

        // If .gitignore already contains the first line of the main template, skip
        let first_line = main_content.lines().next().unwrap_or("");
        if existing.contains(first_line) {
            util::warn("Already has this template");
            return Ok(());
        }

        // Append or create new file
        let new = if existing.trim().is_empty() {
            combined
        } else {
            format!("{}\n{}", existing.trim(), combined)
        };

        fs::write(path, new)?;
        util::ok(&format!(
            "Added {name} template (with base templates) to .gitignore"
        ));
    } else {
        util::err(&format!("Unknown template: {name}"));
        util::dim("Available: node, python, rust, go, java, web, macos, windows, linux, ide");
    }

    Ok(())
}
