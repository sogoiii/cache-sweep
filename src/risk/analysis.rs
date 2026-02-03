use std::path::Path;

#[derive(Debug, Clone, Default)]
pub struct RiskAnalysis {
    pub is_sensitive: bool,
    pub reason: Option<String>,
}

// Known applications that depend on node_modules (use path patterns)
const SENSITIVE_APP_PATTERNS: &[(&str, &str)] = &[
    // Desktop apps (Electron-based and others)
    ("/visual studio code/", "Visual Studio Code"),
    ("/vscode/", "Visual Studio Code"),
    ("/code.app/", "Visual Studio Code"),
    ("/discord/", "Discord"),
    ("/discord.app/", "Discord"),
    ("/slack/", "Slack"),
    ("/slack.app/", "Slack"),
    ("/atom/", "Atom"),
    ("/postman/", "Postman"),
    ("/figma/", "Figma"),
    ("/notion/", "Notion"),
    ("/obsidian/", "Obsidian"),
    ("/spotify/", "Spotify"),
    ("/microsoft teams/", "Microsoft Teams"),
    ("/1password/", "1Password"),
    ("/bitwarden/", "Bitwarden"),
    // Version managers
    ("/.asdf/", "asdf version manager"),
    ("/.nvm/", "Node Version Manager"),
    ("/.pyenv/", "pyenv"),
    ("/.rbenv/", "rbenv"),
    ("/.volta/", "Volta"),
    ("/.sdkman/", "SDKMAN"),
    ("/.rustup/", "rustup"),
    ("/.goenv/", "goenv"),
    ("/.jabba/", "Jabba JDK manager"),
    // Package managers and their caches
    ("/.bun/", "Bun"),
    ("/.cargo/", "Cargo"),
    ("/.npm/", "npm"),
    ("/.yarn/", "Yarn"),
    ("/.pnpm/", "pnpm"),
    ("/.composer/", "Composer"),
    ("/.gem/", "RubyGems"),
    ("/.gradle/", "Gradle"),
    ("/.m2/", "Maven"),
    ("/.nuget/", "NuGet"),
    ("/.mix/", "Mix (Elixir)"),
    ("/.hex/", "Hex (Elixir)"),
    ("/.stack/", "Stack (Haskell)"),
    ("/.cabal/", "Cabal (Haskell)"),
    // IDEs and editors
    ("/.vscode/", "Visual Studio Code"),
    ("/.vscode-server/", "Visual Studio Code Server"),
    ("/.windsurf/", "Windsurf"),
    ("/.cursor/", "Cursor"),
    ("/.idea/", "JetBrains IDE"),
    ("/.jetbrains/", "JetBrains"),
    ("/.atom/", "Atom"),
    ("/.sublime-text/", "Sublime Text"),
    ("/.zed/", "Zed"),
    // Other tool directories
    ("/.docker/", "Docker"),
    ("/.kube/", "Kubernetes"),
    ("/.aws/", "AWS CLI"),
    ("/.azure/", "Azure CLI"),
    ("/.gcloud/", "Google Cloud CLI"),
    ("/.terraform.d/", "Terraform"),
    ("/.pulumi/", "Pulumi"),
];

pub fn analyze_risk(path: &Path) -> RiskAnalysis {
    let path_str = path.to_string_lossy().to_lowercase();

    // Check for OS-level system paths (Linux, macOS, Windows)
    if let Some(reason) = check_system_paths(&path_str) {
        return RiskAnalysis {
            is_sensitive: true,
            reason: Some(reason),
        };
    }

    // macOS ~/Library - check for /users/*/library pattern
    if is_user_library_path(&path_str) {
        return RiskAnalysis {
            is_sensitive: true,
            reason: Some("macOS user Library folder".to_string()),
        };
    }

    // Check for known sensitive applications using path patterns
    for (pattern, app_name) in SENSITIVE_APP_PATTERNS {
        if path_str.contains(*pattern) {
            return RiskAnalysis {
                is_sensitive: true,
                reason: Some(format!("Part of {app_name}")),
            };
        }
    }

    // Dotfolder rule: anything under ~/.<folder>/ is sensitive
    // This catches unknown tools, configs, and caches
    if let Some(reason) = check_dotfolder_rule(path) {
        return RiskAnalysis {
            is_sensitive: true,
            reason: Some(reason),
        };
    }

    // Check for common user application paths (fallback for non-home paths)
    if path_str.contains("/.config/") || path_str.contains("/.local/share/") {
        return RiskAnalysis {
            is_sensitive: true,
            reason: Some("User configuration or application data".to_string()),
        };
    }

    RiskAnalysis::default()
}

/// Check for OS-level system paths that should never be deleted
/// Covers Linux (FHS), macOS (SIP-protected), and Windows system directories
fn check_system_paths(path_str: &str) -> Option<String> {
    // === macOS system paths (SIP-protected) ===
    // /System, /usr, /bin, /sbin, /var, /private
    if path_str.starts_with("/system/") || path_str.starts_with("/system") && path_str.len() == 7 {
        return Some("macOS system directory (SIP protected)".to_string());
    }
    if path_str.starts_with("/private/") {
        return Some("macOS private system directory".to_string());
    }

    // === Shared Unix paths (Linux + macOS) ===
    // These are root-level system directories
    let unix_system_paths = [
        ("/bin/", "System binaries directory"),
        ("/sbin/", "System administration binaries"),
        ("/lib/", "System libraries"),
        ("/lib64/", "System libraries (64-bit)"),
        ("/etc/", "System configuration directory"),
        ("/boot/", "Boot loader directory"),
        ("/root/", "Root user home directory"),
        ("/opt/", "Optional packages directory"),
        ("/srv/", "Service data directory"),
        ("/proc/", "Process filesystem (virtual)"),
        ("/sys/", "System filesystem (virtual)"),
        ("/dev/", "Device files (virtual)"),
        ("/run/", "Runtime data"),
        ("/snap/", "Snap packages (system)"),
    ];

    for (prefix, reason) in unix_system_paths {
        if path_str.starts_with(prefix) {
            return Some(format!("{reason} (system critical)"));
        }
    }

    // /usr - all of it is protected (including /usr/local for safety)
    if path_str.starts_with("/usr/") {
        return Some("System directory under /usr (protected)".to_string());
    }

    // /var is tricky - /var/log, /var/lib are system, but /var/www might be user projects
    // Be conservative and protect all of /var
    if path_str.starts_with("/var/") {
        return Some("System variable data directory".to_string());
    }

    // === macOS specific ===
    if path_str.contains("/applications/") {
        return Some("macOS Applications directory".to_string());
    }
    // System /Library (not user ~/Library which is handled separately)
    if path_str.starts_with("/library/") {
        return Some("macOS system Library directory".to_string());
    }

    // === Windows system paths ===
    // Case-insensitive matching (path_str is already lowercased)
    if path_str.contains("\\windows\\") || path_str.contains("/windows/") {
        return Some("Windows system directory".to_string());
    }
    if path_str.contains("\\system32") || path_str.contains("/system32") {
        return Some("Windows System32 directory".to_string());
    }
    if path_str.contains("\\syswow64") || path_str.contains("/syswow64") {
        return Some("Windows SysWOW64 directory".to_string());
    }
    if path_str.contains("program files") {
        return Some("Windows Program Files directory".to_string());
    }
    if path_str.contains("\\programdata") || path_str.contains("/programdata") {
        return Some("Windows ProgramData directory".to_string());
    }
    if path_str.contains("\\appdata\\") || path_str.contains("/appdata/") {
        return Some("Windows AppData directory".to_string());
    }
    if path_str.contains("\\recovery") || path_str.contains("/recovery") {
        // But not if it's in a project name
        if path_str.starts_with("c:\\recovery")
            || path_str.starts_with("d:\\recovery")
            || path_str.contains(":\\recovery")
        {
            return Some("Windows Recovery partition".to_string());
        }
    }
    if path_str.contains("$recycle.bin") {
        return Some("Windows Recycle Bin".to_string());
    }
    if path_str.contains("system volume information") {
        return Some("Windows System Volume Information".to_string());
    }

    // === Linux distribution specific ===
    if path_str.starts_with("/snap/") {
        return Some("Snap packages directory".to_string());
    }
    if path_str.starts_with("/flatpak/") {
        return Some("Flatpak system directory".to_string());
    }

    None
}

/// Check if path is inside ~/Library (macOS user Library)
fn is_user_library_path(path_str: &str) -> bool {
    // Match patterns like /users/username/library or /home/username/library
    // The path is already lowercased
    let parts: Vec<&str> = path_str.split('/').collect();

    for (i, part) in parts.iter().enumerate() {
        if *part == "library" {
            // Check if this is a user's Library (preceded by a username directory)
            // Pattern: /users/<username>/library or /home/<username>/library
            if i >= 2 && (parts[i - 2] == "users" || parts[i - 2] == "home") {
                return true;
            }
        }
    }
    false
}

/// Dotfolder rule: paths inside ~/.<dotfolder>/ are sensitive
/// This catches tool installations, caches, and configs we haven't explicitly listed
fn check_dotfolder_rule(path: &Path) -> Option<String> {
    // Get home directory
    let home = dirs::home_dir()?;
    let home_str = home.to_string_lossy().to_lowercase();

    let path_str = path.to_string_lossy().to_lowercase();

    // Check if path is under home directory
    let relative = path_str.strip_prefix(&home_str)?;

    // Look for pattern: /.<dotfolder>/...
    // The relative path starts with / so we check for /.<name>/
    let parts: Vec<&str> = relative.split('/').collect();

    // parts[0] is empty (leading /), parts[1] would be the first component
    if parts.len() >= 2 {
        let first_component = parts[1];
        if first_component.starts_with('.') && !first_component.is_empty() {
            // It's a dotfolder directly under home
            // Special case: allow certain dotfolders that are typically user projects
            let allowed_dotfolders = [".local/bin"]; // Could add more exceptions

            for allowed in &allowed_dotfolders {
                if relative.contains(allowed) {
                    return None;
                }
            }

            return Some(format!(
                "Inside ~/.{} (tool/config directory)",
                first_component.trim_start_matches('.')
            ));
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    // === System paths (OS-level) ===

    #[test]
    fn test_system_path_applications() {
        let path = PathBuf::from("/Applications/MyApp.app/node_modules");
        let result = analyze_risk(&path);
        assert!(result.is_sensitive);
        assert!(result.reason.unwrap().contains("Applications"));
    }

    #[test]
    fn test_system_path_program_files() {
        let path = PathBuf::from("C:/Program Files/App/node_modules");
        let result = analyze_risk(&path);
        assert!(result.is_sensitive);
        assert!(result.reason.unwrap().contains("Program Files"));
    }

    #[test]
    fn test_system_path_appdata() {
        let path = PathBuf::from("C:/Users/User/AppData/Local/App/node_modules");
        let result = analyze_risk(&path);
        assert!(result.is_sensitive);
        assert!(result.reason.unwrap().contains("AppData"));
    }

    #[test]
    fn test_case_insensitive_matching() {
        let path = PathBuf::from("/APPLICATIONS/MyApp/node_modules");
        let result = analyze_risk(&path);
        assert!(result.is_sensitive);
    }

    #[test]
    fn test_system_path_usr() {
        let path = PathBuf::from("/usr/bin/something");
        let result = analyze_risk(&path);
        assert!(result.is_sensitive);
        assert!(result.reason.unwrap().contains("/usr"));
    }

    #[test]
    fn test_system_path_usr_local_blocked() {
        // /usr/local is also blocked for safety
        let path = PathBuf::from("/usr/local/lib/node_modules");
        let result = analyze_risk(&path);
        assert!(result.is_sensitive);
        assert!(result.reason.unwrap().contains("/usr"));
    }

    #[test]
    fn test_system_path_etc() {
        let path = PathBuf::from("/etc/nginx/node_modules");
        let result = analyze_risk(&path);
        assert!(result.is_sensitive);
        assert!(result.reason.unwrap().contains("configuration"));
    }

    #[test]
    fn test_system_path_var() {
        let path = PathBuf::from("/var/lib/something/cache");
        let result = analyze_risk(&path);
        assert!(result.is_sensitive);
        assert!(result.reason.unwrap().contains("variable data"));
    }

    #[test]
    fn test_system_path_opt() {
        let path = PathBuf::from("/opt/someapp/node_modules");
        let result = analyze_risk(&path);
        assert!(result.is_sensitive);
        assert!(result.reason.unwrap().contains("Optional packages"));
    }

    #[test]
    fn test_windows_system32() {
        let path = PathBuf::from("C:\\Windows\\System32\\something");
        let result = analyze_risk(&path);
        assert!(result.is_sensitive);
    }

    #[test]
    fn test_windows_programdata() {
        let path = PathBuf::from("C:\\ProgramData\\App\\cache");
        let result = analyze_risk(&path);
        assert!(result.is_sensitive);
    }

    // === macOS Library paths ===

    #[test]
    fn test_user_library_macos() {
        let path = PathBuf::from("/Users/dev/Library/Caches/something");
        let result = analyze_risk(&path);
        assert!(result.is_sensitive);
        assert!(result.reason.unwrap().contains("Library"));
    }

    #[test]
    fn test_user_library_linux_style() {
        let path = PathBuf::from("/home/user/Library/something");
        let result = analyze_risk(&path);
        assert!(result.is_sensitive);
    }

    #[test]
    fn test_system_library_macos() {
        // /Library (system) should be caught as system Library
        let path = PathBuf::from("/Library/Something/node_modules");
        let result = analyze_risk(&path);
        assert!(result.is_sensitive);
        assert!(result.reason.unwrap().contains("system Library"));
    }

    // === Desktop apps (non-system paths) ===
    // Note: Apps in /opt, /usr, /Applications are caught by system path rules first
    // These tests use paths that aren't in system directories

    #[test]
    fn test_vscode_app_pattern() {
        // Use a home directory path to avoid /usr system path match
        let path = PathBuf::from("/home/user/.local/share/code/node_modules");
        let result = analyze_risk(&path);
        assert!(result.is_sensitive);
        // Will be caught by .local/share pattern
    }

    #[test]
    fn test_discord_pattern_electron() {
        // Discord in a non-system path (user's config)
        let path = PathBuf::from("/home/user/.config/discord/node_modules");
        let result = analyze_risk(&path);
        assert!(result.is_sensitive);
    }

    #[test]
    fn test_slack_pattern() {
        let path = PathBuf::from("/Applications/Slack.app/Contents/node_modules");
        let result = analyze_risk(&path);
        assert!(result.is_sensitive);
        // Caught by /Applications/ rule
    }

    #[test]
    fn test_obsidian_in_user_dir() {
        // Obsidian in a user path
        let home = dirs::home_dir().unwrap();
        let path = home.join(".obsidian/plugins/node_modules");
        let result = analyze_risk(&path);
        assert!(result.is_sensitive);
        // Caught by dotfolder rule
    }

    // === Version managers ===

    #[test]
    fn test_asdf_version_manager() {
        let path = PathBuf::from("/Users/dev/.asdf/installs/nodejs/24.4.1/lib/node_modules");
        let result = analyze_risk(&path);
        assert!(result.is_sensitive);
        assert!(result.reason.unwrap().contains("asdf"));
    }

    #[test]
    fn test_nvm_version_manager() {
        let path = PathBuf::from("/home/user/.nvm/versions/node/v20.0.0/lib/node_modules");
        let result = analyze_risk(&path);
        assert!(result.is_sensitive);
        assert!(result.reason.unwrap().contains("Node Version Manager"));
    }

    #[test]
    fn test_pyenv() {
        let path = PathBuf::from("/home/user/.pyenv/versions/3.11.0/lib/python3.11");
        let result = analyze_risk(&path);
        assert!(result.is_sensitive);
        assert!(result.reason.unwrap().contains("pyenv"));
    }

    #[test]
    fn test_rustup() {
        let path = PathBuf::from("/Users/dev/.rustup/toolchains/stable-x86_64/lib");
        let result = analyze_risk(&path);
        assert!(result.is_sensitive);
        assert!(result.reason.unwrap().contains("rustup"));
    }

    // === Package managers ===

    #[test]
    fn test_bun_package_manager() {
        let path = PathBuf::from("/Users/dev/.bun/install/global/node_modules");
        let result = analyze_risk(&path);
        assert!(result.is_sensitive);
        assert!(result.reason.unwrap().contains("Bun"));
    }

    #[test]
    fn test_cargo_registry() {
        let path = PathBuf::from("/Users/dev/.cargo/registry/index/.cache");
        let result = analyze_risk(&path);
        assert!(result.is_sensitive);
        assert!(result.reason.unwrap().contains("Cargo"));
    }

    #[test]
    fn test_npm_cache() {
        let path = PathBuf::from("/home/user/.npm/_cacache/content-v2");
        let result = analyze_risk(&path);
        assert!(result.is_sensitive);
        assert!(result.reason.unwrap().contains("npm"));
    }

    #[test]
    fn test_yarn_cache() {
        let path = PathBuf::from("/home/user/.yarn/cache/lodash-npm-4.17.21");
        let result = analyze_risk(&path);
        assert!(result.is_sensitive);
    }

    #[test]
    fn test_gradle() {
        let path = PathBuf::from("/Users/dev/.gradle/caches/modules-2");
        let result = analyze_risk(&path);
        assert!(result.is_sensitive);
        assert!(result.reason.unwrap().contains("Gradle"));
    }

    // === IDEs ===

    #[test]
    fn test_windsurf_ide() {
        let path = PathBuf::from("/Users/dev/.windsurf/extensions/some.extension/node_modules");
        let result = analyze_risk(&path);
        assert!(result.is_sensitive);
        assert!(result.reason.unwrap().contains("Windsurf"));
    }

    #[test]
    fn test_cursor_ide() {
        let path = PathBuf::from("/Users/dev/.cursor/extensions/ms-python/node_modules");
        let result = analyze_risk(&path);
        assert!(result.is_sensitive);
        assert!(result.reason.unwrap().contains("Cursor"));
    }

    #[test]
    fn test_vscode_extensions() {
        let path = PathBuf::from("/home/user/.vscode/extensions/some-ext/node_modules");
        let result = analyze_risk(&path);
        assert!(result.is_sensitive);
    }

    #[test]
    fn test_jetbrains_ide() {
        let path = PathBuf::from("/Users/dev/.idea/libraries/some-lib");
        let result = analyze_risk(&path);
        assert!(result.is_sensitive);
    }

    // === Dotfolder rule ===

    #[test]
    fn test_dotfolder_unknown_tool() {
        // Some hypothetical tool we haven't explicitly listed
        let home = dirs::home_dir().unwrap();
        let path = home.join(".some-unknown-tool/cache/stuff");
        let result = analyze_risk(&path);
        assert!(result.is_sensitive);
        assert!(result.reason.unwrap().contains("some-unknown-tool"));
    }

    #[test]
    fn test_dotfolder_cache_root() {
        let home = dirs::home_dir().unwrap();
        let path = home.join(".cache/some-app/data");
        let result = analyze_risk(&path);
        assert!(result.is_sensitive);
        assert!(result.reason.unwrap().contains("cache"));
    }

    #[test]
    fn test_dotfolder_local() {
        let home = dirs::home_dir().unwrap();
        let path = home.join(".local/share/app/node_modules");
        let result = analyze_risk(&path);
        assert!(result.is_sensitive);
    }

    // === Normal project paths (should NOT be sensitive) ===

    #[test]
    fn test_normal_project_path() {
        let path = PathBuf::from("/home/user/projects/myapp/node_modules");
        let result = analyze_risk(&path);
        assert!(!result.is_sensitive);
        assert!(result.reason.is_none());
    }

    #[test]
    fn test_normal_workspace_path() {
        let path = PathBuf::from("/Users/dev/workspace/frontend/node_modules");
        let result = analyze_risk(&path);
        assert!(!result.is_sensitive);
    }

    #[test]
    fn test_documents_folder_ok() {
        let path = PathBuf::from("/Users/dev/Documents/projects/app/node_modules");
        let result = analyze_risk(&path);
        assert!(!result.is_sensitive);
    }

    #[test]
    fn test_desktop_project_ok() {
        let path = PathBuf::from("/Users/dev/Desktop/my-project/node_modules");
        let result = analyze_risk(&path);
        assert!(!result.is_sensitive);
    }

    #[test]
    fn test_nested_cache_in_project_ok() {
        // .cache inside a project is fine
        let path = PathBuf::from("/Users/dev/projects/app/.cache/webpack");
        let result = analyze_risk(&path);
        assert!(!result.is_sensitive);
    }

    // === Helper function tests ===

    #[test]
    fn test_is_user_library_path_macos() {
        assert!(is_user_library_path("/users/apozo/library/caches"));
        assert!(is_user_library_path(
            "/users/john/library/application support"
        ));
    }

    #[test]
    fn test_is_user_library_path_linux() {
        assert!(is_user_library_path("/home/user/library/something"));
    }

    #[test]
    fn test_is_user_library_path_not_user() {
        // System Library should not match
        assert!(!is_user_library_path("/library/something"));
        // Library in project name should not match
        assert!(!is_user_library_path("/users/dev/projects/library/src"));
    }
}
