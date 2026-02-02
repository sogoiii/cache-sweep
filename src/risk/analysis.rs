use std::path::Path;

#[derive(Debug, Clone, Default)]
pub struct RiskAnalysis {
    pub is_sensitive: bool,
    pub reason: Option<String>,
}

// Known applications that depend on node_modules (use path patterns)
const SENSITIVE_APP_PATTERNS: &[(&str, &str)] = &[
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
];

pub fn analyze_risk(path: &Path) -> RiskAnalysis {
    let path_str = path.to_string_lossy().to_lowercase();

    // Check for system paths
    if path_str.contains("/applications/")
        || path_str.contains("/library/")
        || path_str.contains("/system/")
        || path_str.contains("program files")
        || path_str.contains("/appdata/")
    {
        return RiskAnalysis {
            is_sensitive: true,
            reason: Some("System or application directory".to_string()),
        };
    }

    // Check for known sensitive applications using path patterns
    for (pattern, app_name) in SENSITIVE_APP_PATTERNS {
        if path_str.contains(*pattern) {
            return RiskAnalysis {
                is_sensitive: true,
                reason: Some(format!("Part of {app_name} installation")),
            };
        }
    }

    // Check for common user application paths
    if path_str.contains("/.vscode/")
        || path_str.contains("/.config/")
        || path_str.contains("/.local/share/")
    {
        return RiskAnalysis {
            is_sensitive: true,
            reason: Some("User configuration or application data".to_string()),
        };
    }

    RiskAnalysis::default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_system_path_applications() {
        let path = PathBuf::from("/Applications/MyApp.app/node_modules");
        let result = analyze_risk(&path);
        assert!(result.is_sensitive);
        assert!(result.reason.unwrap().contains("System"));
    }

    #[test]
    fn test_system_path_library() {
        let path = PathBuf::from("/Library/Something/node_modules");
        let result = analyze_risk(&path);
        assert!(result.is_sensitive);
    }

    #[test]
    fn test_system_path_program_files() {
        let path = PathBuf::from("C:/Program Files/App/node_modules");
        let result = analyze_risk(&path);
        assert!(result.is_sensitive);
    }

    #[test]
    fn test_system_path_appdata() {
        let path = PathBuf::from("C:/Users/User/AppData/Local/App/node_modules");
        let result = analyze_risk(&path);
        assert!(result.is_sensitive);
    }

    #[test]
    fn test_vscode_app_pattern() {
        let path = PathBuf::from("/usr/share/code.app/resources/node_modules");
        let result = analyze_risk(&path);
        assert!(result.is_sensitive);
        assert!(result.reason.unwrap().contains("Visual Studio Code"));
    }

    #[test]
    fn test_discord_pattern() {
        let path = PathBuf::from("/home/user/.config/discord/node_modules");
        let result = analyze_risk(&path);
        assert!(result.is_sensitive);
        // Should match discord pattern first (before .config)
        assert!(result.reason.unwrap().contains("Discord"));
    }

    #[test]
    fn test_slack_pattern() {
        let path = PathBuf::from("/Applications/Slack.app/Contents/node_modules");
        let result = analyze_risk(&path);
        assert!(result.is_sensitive);
    }

    #[test]
    fn test_obsidian_pattern() {
        let path = PathBuf::from("/opt/obsidian/node_modules");
        let result = analyze_risk(&path);
        assert!(result.is_sensitive);
        assert!(result.reason.unwrap().contains("Obsidian"));
    }

    #[test]
    fn test_user_config_vscode() {
        let path = PathBuf::from("/home/user/.vscode/extensions/node_modules");
        let result = analyze_risk(&path);
        assert!(result.is_sensitive);
        assert!(result.reason.unwrap().contains("User configuration"));
    }

    #[test]
    fn test_user_config_dotconfig() {
        let path = PathBuf::from("/home/user/.config/some-app/node_modules");
        let result = analyze_risk(&path);
        assert!(result.is_sensitive);
    }

    #[test]
    fn test_user_local_share() {
        let path = PathBuf::from("/home/user/.local/share/app/node_modules");
        let result = analyze_risk(&path);
        assert!(result.is_sensitive);
    }

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
    fn test_case_insensitive_matching() {
        let path = PathBuf::from("/APPLICATIONS/MyApp/node_modules");
        let result = analyze_risk(&path);
        assert!(result.is_sensitive);
    }
}
