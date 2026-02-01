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
                reason: Some(format!("Part of {} installation", app_name)),
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
