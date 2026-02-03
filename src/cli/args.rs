use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug, Clone)]
#[command(name = "cache-sweep")]
#[command(about = "Find and delete dependency/cache folders to reclaim disk space")]
#[command(version, disable_version_flag = true, arg(clap::Arg::new("version").short('v').long("version").action(clap::ArgAction::Version).global(true)))]
#[allow(clippy::struct_excessive_bools)] // CLI args naturally have many boolean flags
pub struct Args {
    /// Select profiles to search (comma-separated). Use without value to list.
    #[arg(short = 'p', long, value_delimiter = ',', conflicts_with = "targets")]
    pub profiles: Option<Vec<String>>,

    /// Starting directory for search
    #[arg(short = 'd', long, default_value = ".")]
    pub directory: PathBuf,

    /// Exclude directories (comma-separated)
    #[arg(short = 'E', long, value_delimiter = ',')]
    pub exclude: Option<Vec<String>>,

    /// Start from home directory
    #[arg(short = 'f', long)]
    pub full: bool,

    /// Sort by: size, path, or age
    #[arg(short = 's', long, default_value = "size")]
    pub sort: String,

    /// Search for specific folder names (disables profiles)
    #[arg(short = 't', long, value_delimiter = ',')]
    pub targets: Option<Vec<String>>,

    /// Include protected system directories in results (hidden by default)
    #[arg(short = 'X', long)]
    pub show_protected: bool,

    /// Simulate deletion without actually deleting
    #[arg(long)]
    pub dry_run: bool,

    /// Stream each result as JSON (one object per line)
    #[arg(long)]
    pub json_stream: bool,

    /// Output all results as single JSON object
    #[arg(long)]
    pub json: bool,

    /// Follow symbolic links (default: false for safety)
    #[arg(long)]
    pub follow_links: bool,

    /// Respect .gitignore files (default: false - scan everything)
    #[arg(long)]
    pub respect_ignore: bool,
}

impl Args {
    pub fn effective_directory(&self) -> PathBuf {
        if self.full {
            dirs::home_dir().unwrap_or_else(|| self.directory.clone())
        } else {
            self.directory.clone()
        }
    }

    pub fn effective_targets(&self) -> Vec<String> {
        self.targets.as_ref().map_or_else(
            || {
                let profile_names = self
                    .profiles
                    .clone()
                    .unwrap_or_else(|| vec!["all".to_string()]);
                crate::profiles::get_targets_for_profiles(&profile_names)
            },
            Clone::clone,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn default_args() -> Args {
        Args {
            profiles: None,
            directory: PathBuf::from("."),
            exclude: None,
            full: false,
            sort: "size".to_string(),
            targets: None,
            show_protected: false,
            dry_run: false,
            json_stream: false,
            json: false,
            follow_links: false,
            respect_ignore: false,
        }
    }

    #[test]
    fn test_effective_directory_default() {
        let args = default_args();
        assert_eq!(args.effective_directory(), PathBuf::from("."));
    }

    #[test]
    fn test_effective_directory_custom() {
        let mut args = default_args();
        args.directory = PathBuf::from("/custom/path");
        assert_eq!(args.effective_directory(), PathBuf::from("/custom/path"));
    }

    #[test]
    fn test_effective_directory_full_flag() {
        let mut args = default_args();
        args.full = true;
        let result = args.effective_directory();
        // Should be home dir if available, otherwise fallback to directory
        assert!(result != Path::new(".") || dirs::home_dir().is_none());
    }

    #[test]
    fn test_effective_targets_explicit() {
        let mut args = default_args();
        args.targets = Some(vec!["custom_target".to_string()]);
        assert_eq!(args.effective_targets(), vec!["custom_target".to_string()]);
    }

    #[test]
    fn test_effective_targets_default_all_profiles() {
        let args = default_args();
        let targets = args.effective_targets();
        // Default is "all" - should include targets from multiple profiles
        assert!(targets.contains(&"node_modules".to_string()));
        assert!(targets.contains(&"__pycache__".to_string()));
        assert!(targets.contains(&"target".to_string()));
    }

    #[test]
    fn test_effective_targets_python_profile() {
        let mut args = default_args();
        args.profiles = Some(vec!["python".to_string()]);
        let targets = args.effective_targets();
        assert!(targets.contains(&"__pycache__".to_string()));
        assert!(targets.contains(&".venv".to_string()));
    }

    #[test]
    fn test_effective_targets_multiple_profiles() {
        let mut args = default_args();
        args.profiles = Some(vec!["node".to_string(), "rust".to_string()]);
        let targets = args.effective_targets();
        assert!(targets.contains(&"node_modules".to_string()));
        assert!(targets.contains(&"target".to_string()));
    }
}
