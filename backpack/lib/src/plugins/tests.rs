#[cfg(test)]
mod tests {
    use std::{fs, io::Write, path::Path};
    use tempfile::tempdir;

    //use github_bot_lib::plugins::*;
    use crate::plugins::*;

    // --- Test Constants ---

    const MOCK_MANIFEST: &str = r#"
        name = "test-plugin"
        description = "A plugin for testing"
        author = "Test Author"
        license = "Test License"
    "#;

    // Updated to Rhai script: Accesses the event data object and returns success
    const MOCK_SCRIPT_SUCCESS: &str = r#"
        // Rhai script to handle the event data (passed as 'event_data')
        let event_type = "UNKNOWN";

        // FIX: Check the type using type_of(), which is standard for Dynamic values in Rhai.
        let data_type = event_data.type_of();

        if data_type == "string" {
            // Unit variants (like Init, End) are serialized as a string
            event_type = event_data;
        } else if data_type == "map" {
            // Struct/Tuple variants are serialized as a map where the key is the variant name
            if event_data.keys.len() > 0 {
                event_type = event_data.keys[0];
            }
        }

        // Using concatenation to avoid template string syntax error
        print("Rhai received event: " + event_type);

        // Implicit return of last expression (unit or true) is success
        true
    "#;

    // Updated to Rhai script: deliberately cause a runtime error (Division by zero)
    const MOCK_SCRIPT_FAIL: &str = r#"
        // This script intentionally fails by causing a runtime error
        let zero = 0;
        1 / zero; // Runtime error: Division by zero
    "#;

    // --- Helper Functions ---

    /// Creates a mock configuration structure in a temporary directory.
    fn setup_mock_plugin_env(
        base_dir: &Path,
        plugin_name: &str,
        script_content: &str,
        manifest_content: &str,
    ) -> PathBuf {
        let plugin_path = base_dir.join(APP_NAME).join(PLUGINS_DIR).join(plugin_name);

        fs::create_dir_all(&plugin_path).unwrap();

        // Write manifest
        let mut f_manifest = fs::File::create(plugin_path.join(MANIFEST_FILENAME)).unwrap();
        f_manifest.write_all(manifest_content.as_bytes()).unwrap();

        // Write script (now .rhai, no need for executable bit)
        let script_path = plugin_path.join(SCRIPT_FILENAME);
        let mut f_script = fs::File::create(&script_path).unwrap();
        f_script.write_all(script_content.as_bytes()).unwrap();

        plugin_path
    }

    /// Helper to mock the environment for `dirs::config_dir()`
    fn mock_config_dir(temp_path: &Path) {
        // Wrapping the calls to std::env::set_var with unsafe {} to satisfy E0133
        unsafe {
            std::env::set_var("XDG_CONFIG_HOME", temp_path.to_str().unwrap());
            std::env::set_var("APPDATA", temp_path.to_str().unwrap());
        }
    }

    // --- Plugin::from_dir Tests ---

    #[test]
    fn test_plugin_from_dir_success() {
        let temp_dir = tempdir().unwrap();
        let plugin_path = setup_mock_plugin_env(
            temp_dir.path(),
            "valid-plugin",
            MOCK_SCRIPT_SUCCESS,
            MOCK_MANIFEST,
        );

        let result = Plugin::from_dir(&plugin_path);

        assert!(result.is_ok());
        let plugin = result.unwrap();
        assert_eq!(plugin.manifest.name, "test-plugin");
    }

    // Check for run.rhai existence
    #[test]
    fn test_plugin_from_dir_missing_script() {
        let temp_dir = tempdir().unwrap();
        let plugin_path = temp_dir.path().join("my-plugin");
        fs::create_dir_all(&plugin_path).unwrap();

        // Only write manifest
        let mut f_manifest = fs::File::create(plugin_path.join(MANIFEST_FILENAME)).unwrap();
        f_manifest.write_all(MOCK_MANIFEST.as_bytes()).unwrap();

        let result = Plugin::from_dir(&plugin_path);

        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("Missing required script"));
    }

    #[test]
    fn test_plugin_from_dir_invalid_manifest() {
        let temp_dir = tempdir().unwrap();
        let invalid_manifest = "name = 123\nnot_valid = {"; // Invalid TOML
        let plugin_path = setup_mock_plugin_env(
            temp_dir.path(),
            "invalid-plugin",
            MOCK_SCRIPT_SUCCESS,
            invalid_manifest,
        );

        let result = Plugin::from_dir(&plugin_path);

        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("Failed to parse TOML manifest"));
    }

    // --- discover_plugins Tests ---

    #[test]
    fn test_discover_plugins_none_found() {
        let temp_dir = tempdir().unwrap();
        mock_config_dir(temp_dir.path());

        let plugins = discover_plugins().unwrap();
        assert!(plugins.is_empty());
    }

    #[test]
    fn test_discover_plugins_multiple_found() {
        let temp_dir = tempdir().unwrap();
        mock_config_dir(temp_dir.path());

        // Setup two successful plugins
        let manifest_1 = MOCK_MANIFEST.replace("test-plugin", "a-first-plugin");
        setup_mock_plugin_env(
            temp_dir.path(),
            "plugin-a",
            MOCK_SCRIPT_SUCCESS,
            &manifest_1,
        );

        let manifest_2 = MOCK_MANIFEST.replace("test-plugin", "b-second-plugin");
        setup_mock_plugin_env(
            temp_dir.path(),
            "plugin-b",
            MOCK_SCRIPT_SUCCESS,
            &manifest_2,
        );

        // Setup one bad plugin that should be ignored by discover_plugins
        let invalid_manifest = "name = invalid\n";
        setup_mock_plugin_env(
            temp_dir.path(),
            "plugin-c-bad",
            MOCK_SCRIPT_SUCCESS,
            invalid_manifest,
        );

        let plugins = discover_plugins().unwrap();

        // Only two plugins should be loaded successfully
        assert_eq!(plugins.len(), 2);

        // Plugins sorted alphabetically by name: a-first-plugin, b-second-plugin
        assert_eq!(plugins[0].manifest.name, "a-first-plugin");
        assert_eq!(plugins[1].manifest.name, "b-second-plugin");
    }

    // --- Plugin::run_script Tests ---

    #[tokio::test]
    async fn test_plugin_run_script_success() {
        let temp_dir = tempdir().unwrap();
        let plugin_path = setup_mock_plugin_env(
            temp_dir.path(),
            "run-test",
            MOCK_SCRIPT_SUCCESS,
            MOCK_MANIFEST,
        );

        let plugin = Plugin::from_dir(&plugin_path).unwrap();
        let event = Event::CliCommandExecutionInit;

        let result = plugin.run_script(&event).await;

        // Rhai script execution success means result is Ok(())
        println!("{:#?}", result);
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_plugin_run_script_failure() {
        let temp_dir = tempdir().unwrap();
        let plugin_path = setup_mock_plugin_env(
            temp_dir.path(),
            "fail-test",
            MOCK_SCRIPT_FAIL,
            MOCK_MANIFEST,
        );

        let plugin = Plugin::from_dir(&plugin_path).unwrap();
        let event = Event::CliCommandExecutionInit;

        let result = plugin.run_script(&event).await;

        // Rhai script failure means result is Err(Anyhow)
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();

        // Check for the error message that Rhai generates for division by zero
        assert!(err_msg.contains("Division by zero"));
    }
}

/*
#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use std::fs;
    use std::io::Write;
    use tempfile::tempdir;

    use github_bot_lib::plugins::*;

    const MOCK_MANIFEST: &str = r#"
    name = "test-plugin"
    description = "A plugin for testing"
    author = "Test Author"
    license = "Test License"
"#;

    const MOCK_SCRIPT_SUCCESS: &str = r#"
    #!/bin/bash
    # Just echo the event type and succeed
    EVENT_TYPE=$(echo "$1" | jq -r 'keys[0]')
    echo "Event received: $EVENT_TYPE"
    exit 0
"#;

    const MOCK_SCRIPT_FAIL: &str = r#"
    #!/bin/bash
    # This script intentionally fails
    echo "Intentional failure" >&2
    exit 1
"#;

    // --- Helper Functions ---

    /// Creates a mock configuration structure in a temporary directory.
    fn setup_mock_plugin_env(
        base_dir: &Path,
        plugin_name: &str,
        script_content: &str,
        manifest_content: &str,
    ) -> PathBuf {
        let plugin_path = base_dir.join(APP_NAME).join(PLUGINS_DIR).join(plugin_name);

        fs::create_dir_all(&plugin_path).unwrap();

        // Write manifest
        let mut f_manifest = fs::File::create(plugin_path.join(MANIFEST_FILENAME)).unwrap();
        f_manifest.write_all(manifest_content.as_bytes()).unwrap();

        // Write and make script executable
        let script_path = plugin_path.join(SCRIPT_FILENAME);
        let mut f_script = fs::File::create(&script_path).unwrap();
        f_script.write_all(script_content.as_bytes()).unwrap();

        // Mark as executable (Note: this works only on Unix/Linux)
        #[cfg(target_family = "unix")]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&script_path).unwrap().permissions();
            perms.set_mode(0o755); // rwxr-xr-x
            fs::set_permissions(&script_path, perms).unwrap();
        }

        plugin_path
    }

    /// Helper to mock the environment for `dirs::config_dir()`
    fn mock_config_dir(temp_path: &Path) {
        // Wrapping the calls to std::env::set_var with unsafe {} to satisfy E0133
        unsafe {
            std::env::set_var("XDG_CONFIG_HOME", temp_path.to_str().unwrap());
            std::env::set_var("APPDATA", temp_path.to_str().unwrap());
        }
    }

    // --- Plugin::from_dir Tests ---

    #[test]
    fn test_plugin_from_dir_success() {
        let temp_dir = tempdir().unwrap();
        let plugin_path = setup_mock_plugin_env(
            temp_dir.path(),
            "valid-plugin",
            MOCK_SCRIPT_SUCCESS,
            MOCK_MANIFEST,
        );

        let result = Plugin::from_dir(&plugin_path);

        assert!(result.is_ok());
        let plugin = result.unwrap();
        assert_eq!(plugin.manifest.name, "test-plugin");
    }

    #[test]
    fn test_plugin_from_dir_missing_script() {
        let temp_dir = tempdir().unwrap();
        let plugin_path = temp_dir.path().join("my-plugin");
        fs::create_dir_all(&plugin_path).unwrap();

        // Only write manifest
        let mut f_manifest = fs::File::create(plugin_path.join(MANIFEST_FILENAME)).unwrap();
        f_manifest.write_all(MOCK_MANIFEST.as_bytes()).unwrap();

        let result = Plugin::from_dir(&plugin_path);

        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("Missing required script"));
    }

    #[test]
    fn test_plugin_from_dir_invalid_manifest() {
        let temp_dir = tempdir().unwrap();
        let invalid_manifest = "name = 123\nnot_valid = {"; // Invalid TOML
        let plugin_path = setup_mock_plugin_env(
            temp_dir.path(),
            "invalid-plugin",
            MOCK_SCRIPT_SUCCESS,
            invalid_manifest,
        );

        let result = Plugin::from_dir(&plugin_path);

        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("Failed to parse TOML manifest"));
    }

    // --- discover_plugins Tests ---

    #[test]
    fn test_discover_plugins_none_found() {
        let temp_dir = tempdir().unwrap();
        mock_config_dir(temp_dir.path());

        let plugins = discover_plugins().unwrap();
        assert!(plugins.is_empty());
    }

    #[test]
    fn test_discover_plugins_multiple_found() {
        let temp_dir = tempdir().unwrap();
        mock_config_dir(temp_dir.path());

        // Setup two successful plugins
        let manifest_1 = MOCK_MANIFEST.replace("test-plugin", "a-first-plugin");
        setup_mock_plugin_env(
            temp_dir.path(),
            "plugin-a",
            MOCK_SCRIPT_SUCCESS,
            &manifest_1,
        );

        let manifest_2 = MOCK_MANIFEST.replace("test-plugin", "b-second-plugin");
        setup_mock_plugin_env(
            temp_dir.path(),
            "plugin-b",
            MOCK_SCRIPT_SUCCESS,
            &manifest_2,
        );

        // Setup one bad plugin that should be ignored by discover_plugins
        let invalid_manifest = "name = invalid\n";
        setup_mock_plugin_env(
            temp_dir.path(),
            "plugin-c-bad",
            MOCK_SCRIPT_SUCCESS,
            invalid_manifest,
        );

        let plugins = discover_plugins().unwrap();

        // Only two plugins should be loaded successfully
        assert_eq!(plugins.len(), 2);

        assert_eq!(plugins[0].manifest.name, "b-second-plugin");
        assert_eq!(plugins[1].manifest.name, "a-first-plugin");
    }

    // --- Plugin::run_script Tests ---

    #[tokio::test]
    #[cfg(target_family = "unix")]
    async fn test_plugin_run_script_success() {
        let temp_dir = tempdir().unwrap();
        let plugin_path = setup_mock_plugin_env(
            temp_dir.path(),
            "run-test",
            MOCK_SCRIPT_SUCCESS,
            MOCK_MANIFEST,
        );

        let plugin = Plugin::from_dir(&plugin_path).unwrap();
        let event = Event::CliCommandExecutionInit;

        let result = plugin.run_script(&event).await;

        assert!(result.is_ok());
        assert!(result.unwrap().success());
    }

    #[tokio::test]
    #[cfg(target_family = "unix")]
    async fn test_plugin_run_script_failure() {
        let temp_dir = tempdir().unwrap();
        let plugin_path = setup_mock_plugin_env(
            temp_dir.path(),
            "fail-test",
            MOCK_SCRIPT_FAIL,
            MOCK_MANIFEST,
        );

        let plugin = Plugin::from_dir(&plugin_path).unwrap();
        let event = Event::CliCommandExecutionInit;

        let result = plugin.run_script(&event).await;

        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();

        // FIX: The manifest name is "test-plugin", not the directory name "fail-test".
        assert!(err_msg.contains("Plugin 'test-plugin' script failed"));
    }
}
*/
