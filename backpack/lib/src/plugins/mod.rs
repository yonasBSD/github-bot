mod color;

use anyhow::{Context, Result};
use rhai::serde::to_dynamic;
use rhai::{Dynamic, Engine, Scope};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

// --- Configuration Constants ---

pub const APP_NAME: &str = "github-bot";
pub const PLUGINS_DIR: &str = "plugins";
pub const MANIFEST_FILENAME: &str = "manifest.toml";
//pub const SCRIPT_FILENAME: &str = "run.sh";
pub const SCRIPT_FILENAME: &str = "run.rhai";

// --- Data Structures ---

/// Represents the structure of the plugin manifest file.
#[derive(Debug, Deserialize, Serialize)]
pub struct Manifest {
    pub name: String,
    pub description: String,
    pub homepage: Option<String>,
    pub repo: Option<String>,
    pub license: Option<String>,
    pub author: String,
}

/// Represents an event that can be broadcast to plugins.
#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "kebab-case")]
pub enum Event {
    PluginRegistrationInit,
    PluginRegistered(String),
    PluginRegistrationEnd,
    CliCommandExecutionInit,
    CliCommandExecutionRun { command: String, args: Vec<String> },
    CliCommandExecutionEnd,
}

/// Represents a loaded plugin, containing its manifest data and path.
#[derive(Debug)]
pub struct Plugin {
    pub manifest: Manifest,
    pub path: PathBuf,
    pub script_path: PathBuf,
}

impl Plugin {
    /// Attempts to load a plugin from a given directory path.
    pub fn from_dir(path: &Path) -> Result<Self> {
        let manifest_path = path.join(MANIFEST_FILENAME);
        let script_path = path.join(SCRIPT_FILENAME); // Check for .rhai

        // 1. Check if run.rhai exists
        if !script_path.exists() {
            anyhow::bail!("Missing required script: {}", script_path.display());
        }

        // 2. Read and parse manifest.toml
        let manifest_content = std::fs::read_to_string(&manifest_path).with_context(|| {
            format!("Failed to read manifest file: {}", manifest_path.display())
        })?;
        let manifest: Manifest = toml::from_str(&manifest_content).with_context(|| {
            format!("Failed to parse TOML manifest: {}", manifest_path.display())
        })?;

        Ok(Self {
            manifest,
            path: path.to_path_buf(),
            script_path,
        })
    }

    /// Executes the plugin's Rhai script, passing the event data.
    pub async fn run_script(&self, event: &Event) -> Result<()> {
        fn get_rhai_engine() -> Engine {
            let mut engine = Engine::new();

            // Register the custom color printing function.
            engine.register_fn("cprint", color::cprint);

            // Optional: Register a helper to print in a specific color with only one argument
            engine.register_fn("print_red", |message: &str| color::cprint(message, "red"));
            engine.register_fn("print_green", |message: &str| {
                color::cprint(message, "green")
            });

            // Add HTTP fetch via http::client().request
            use rhai::packages::Package;
            rhai_http::HttpPackage::new().register_into_engine(&mut engine);

            engine
        }

        let plugin_name = &self.manifest.name;

        if !cfg!(test) {
            tracing::debug!("-> Executing plugin '{plugin_name}' for event: {event:?}");
        }

        let engine = get_rhai_engine();

        // Convert the Event struct to a Rhai Dynamic value (Map/Object)
        let event_data = to_dynamic(event)
            // FIX: Map the Rhai error type (Box<EvalAltResult> or serde::Error) to an anyhow-compatible error
            .map_err(|e| anyhow::anyhow!("{e}"))
            .with_context(|| {
                format!(
                    "Failed to convert event data to Rhai dynamic object for plugin '{plugin_name}'"
                )
            })?;

        let mut scope = Scope::new();
        // Make the event data available to the script under the name 'event_data'
        scope.push("event_data", event_data);

        let script_content = std::fs::read_to_string(&self.script_path).with_context(|| {
            format!("Failed to read Rhai script: {}", self.script_path.display())
        })?;

        // Execute the script
        match engine.eval_with_scope::<Dynamic>(&mut scope, &script_content) {
            Ok(result) => {
                if !cfg!(test) {
                    tracing::debug!("  [Plugin {plugin_name} RESULT]: {result:?}");
                }
                Ok(())
            }
            Err(e) => {
                // Rhai execution error (script syntax error, runtime error, etc.)
                anyhow::bail!(
                    "Plugin '{plugin_name}' script failed during Rhai execution.\nError: {e}"
                );
            }
        }
    }
}

/*
use std::process::ExitStatus;
use tokio::process::Command;

impl BashPlugin {
    /// Attempts to load a plugin from a given directory path.
    pub fn from_dir(path: &Path) -> Result<Self> {
        let manifest_path = path.join(MANIFEST_FILENAME);
        let script_path = path.join(SCRIPT_FILENAME);

        // 1. Check if run.sh exists and is executable
        if !script_path.exists() {
            anyhow::bail!("Missing required script: {}", script_path.display());
        }
        // NOTE: Checking for actual executable permission is OS-dependent and complex.
        // We'll trust the user has set it up correctly for demonstration.

        // 2. Read and parse manifest.toml
        let manifest_content = std::fs::read_to_string(&manifest_path).with_context(|| {
            format!("Failed to read manifest file: {}", manifest_path.display())
        })?;
        let manifest: Manifest = toml::from_str(&manifest_content).with_context(|| {
            format!("Failed to parse TOML manifest: {}", manifest_path.display())
        })?;

        Ok(Self {
            manifest,
            path: path.to_path_buf(),
            script_path,
        })
    }

    /// Executes the plugin's shell script, passing the event as a JSON argument.
    pub async fn run_script(&self, event: &Event) -> Result<ExitStatus> {
        let event_json = serde_json::to_string(event).unwrap();
        let plugin_name = &self.manifest.name;

        tracing::debug!("-> Executing plugin '{plugin_name}' for event: {event:?}");

        // Use a standard shell (like /bin/sh or cmd.exe) to execute the script.
        // We pass the event JSON as the first command-line argument.
        let output = Command::new("/bin/bash")
            .arg(&self.script_path)
            .arg(event_json)
            // Execute the command in the plugin's directory context
            .current_dir(&self.path)
            .output()
            .await
            .with_context(|| format!("Failed to execute plugin script for '{plugin_name}'"))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!(
                "Plugin '{}' script failed with status {:?}.\nStderr:\n{}",
                plugin_name,
                output.status,
                stderr
            );
        }

        // Print stdout from the script for visibility
        let stdout = String::from_utf8_lossy(&output.stdout);
        if !stdout.is_empty() {
            tracing::info!("  [Plugin {} STDOUT]:\n{}", plugin_name, stdout.trim());
        }

        Ok(output.status)
    }
}
*/

// --- Core Functions ---

/// Finds and loads all plugins from the standard configuration directory.
pub fn discover_plugins() -> Result<Vec<Plugin>> {
    let config_dir = dirs::config_dir()
        .context("Could not determine config directory.")?
        .join(APP_NAME)
        .join(PLUGINS_DIR);

    if !config_dir.exists() {
        tracing::debug!(
            "Plugin directory not found: {}. No plugins loaded.",
            config_dir.display()
        );
        return Ok(Vec::new());
    }

    tracing::debug!("Scanning for plugins in: {}", config_dir.display());

    let mut plugins = Vec::new();
    for entry in std::fs::read_dir(config_dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            match Plugin::from_dir(&path) {
                Ok(plugin) => {
                    tracing::debug!("  [SUCCESS] Loaded plugin: {}", plugin.manifest.name);
                    plugins.push(plugin);
                }
                Err(e) => {
                    eprintln!(
                        "  [ERROR] Failed to load plugin at {}: {:?}",
                        path.display(),
                        e
                    );
                }
            }
        }
    }
    Ok(plugins)
}

/// Broadcasts a given event to all loaded plugins in parallel.
pub async fn broadcast_event(plugins: &[Plugin], event: Event) {
    let tasks: Vec<_> = plugins
        .iter()
        .map(|plugin| {
            let event = event.clone();
            async move {
                match plugin.run_script(&event).await {
                    Ok(()) => {
                        // Script executed successfully
                    }
                    Err(e) => {
                        eprintln!(
                            "Plugin execution failure for '{}': {:?}",
                            plugin.manifest.name, e
                        );
                    }
                }
            }
        })
        .collect();

    // Run all plugin scripts concurrently
    futures::future::join_all(tasks).await;
}

#[cfg(test)]
pub mod tests;
