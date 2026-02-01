use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug, Clone)]
#[command(name = "cache-sweep")]
#[command(about = "Find and delete dependency/cache folders to reclaim disk space")]
#[command(version)]
pub struct Args {
    /// Select profiles to search (comma-separated). Use without value to list.
    #[arg(short = 'p', long, value_delimiter = ',')]
    pub profiles: Option<Vec<String>>,

    /// Starting directory for search
    #[arg(short = 'd', long, default_value = ".")]
    pub directory: PathBuf,

    /// Auto-delete all found directories (non-interactive)
    #[arg(short = 'D', long)]
    pub delete_all: bool,

    /// Skip confirmation when using --delete-all
    #[arg(short = 'y')]
    pub yes: bool,

    /// Hide error messages
    #[arg(short = 'e', long)]
    pub hide_errors: bool,

    /// Exclude directories (comma-separated)
    #[arg(short = 'E', long, value_delimiter = ',')]
    pub exclude: Option<Vec<String>>,

    /// Start from home directory
    #[arg(short = 'f', long)]
    pub full: bool,

    /// Display sizes: auto, mb, gb
    #[arg(long, default_value = "auto")]
    pub size_unit: String,

    /// Sort by: size, path, or age
    #[arg(short = 's', long, default_value = "size")]
    pub sort: String,

    /// Search for specific folder names (disables profiles)
    #[arg(short = 't', long, value_delimiter = ',')]
    pub targets: Option<Vec<String>>,

    /// Exclude system-critical directories
    #[arg(short = 'x', long)]
    pub exclude_sensitive: bool,

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
        if let Some(ref targets) = self.targets {
            targets.clone()
        } else {
            let profile_names = self
                .profiles
                .clone()
                .unwrap_or_else(|| vec!["node".to_string()]);
            crate::profiles::get_targets_for_profiles(&profile_names)
        }
    }
}
