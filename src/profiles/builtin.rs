use std::collections::HashMap;
use std::sync::LazyLock;

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct Profile {
    pub name: &'static str,
    pub description: &'static str,
    pub targets: &'static [&'static str],
}

pub static PROFILES: LazyLock<HashMap<&'static str, Profile>> = LazyLock::new(|| {
    let mut m = HashMap::new();

    m.insert(
        "node",
        Profile {
            name: "node",
            description: "Node.js dependencies and caches",
            targets: &[
                "node_modules",
                ".npm",
                ".next",
                ".nuxt",
                ".angular",
                ".svelte-kit",
                ".vite",
                ".nx",
                ".turbo",
                ".parcel-cache",
                ".eslintcache",
                ".cache",
                ".jest",
                "coverage",
                "deno_cache",
            ],
        },
    );

    m.insert(
        "python",
        Profile {
            name: "python",
            description: "Python caches and virtual environments",
            targets: &[
                "__pycache__",
                ".pytest_cache",
                ".mypy_cache",
                ".venv",
                "venv",
            ],
        },
    );

    m.insert(
        "data-science",
        Profile {
            name: "data-science",
            description: "Data science and ML caches",
            targets: &[".ipynb_checkpoints", ".dvc", ".mlruns", "outputs"],
        },
    );

    m.insert(
        "java",
        Profile {
            name: "java",
            description: "Java build artifacts",
            targets: &["target", ".gradle", "out"],
        },
    );

    m.insert(
        "android",
        Profile {
            name: "android",
            description: "Android native build caches",
            targets: &[".cxx", "externalNativeBuild"],
        },
    );

    m.insert(
        "swift",
        Profile {
            name: "swift",
            description: "Swift/Xcode build artifacts",
            targets: &["DerivedData", ".swiftpm"],
        },
    );

    m.insert(
        "dotnet",
        Profile {
            name: "dotnet",
            description: ".NET build artifacts",
            targets: &["obj", "TestResults", ".vs"],
        },
    );

    m.insert(
        "rust",
        Profile {
            name: "rust",
            description: "Rust build artifacts",
            targets: &["target"],
        },
    );

    m.insert(
        "ruby",
        Profile {
            name: "ruby",
            description: "Ruby dependencies",
            targets: &[".bundle"],
        },
    );

    m.insert(
        "elixir",
        Profile {
            name: "elixir",
            description: "Elixir build artifacts",
            targets: &["_build", "deps", "cover"],
        },
    );

    m.insert(
        "haskell",
        Profile {
            name: "haskell",
            description: "Haskell build artifacts",
            targets: &["dist-newstyle", ".stack-work"],
        },
    );

    m.insert(
        "scala",
        Profile {
            name: "scala",
            description: "Scala build artifacts",
            targets: &[".bloop", ".metals", "target"],
        },
    );

    m.insert(
        "cpp",
        Profile {
            name: "cpp",
            description: "C++ CMake build artifacts",
            targets: &["CMakeFiles", "cmake-build-debug", "cmake-build-release"],
        },
    );

    m.insert(
        "unity",
        Profile {
            name: "unity",
            description: "Unity project caches",
            targets: &["Library", "Temp", "Obj"],
        },
    );

    m.insert(
        "unreal",
        Profile {
            name: "unreal",
            description: "Unreal Engine caches",
            targets: &["Intermediate", "DerivedDataCache", "Binaries"],
        },
    );

    m.insert(
        "godot",
        Profile {
            name: "godot",
            description: "Godot Engine caches",
            targets: &[".import", ".godot"],
        },
    );

    m.insert(
        "infra",
        Profile {
            name: "infra",
            description: "Infrastructure and deployment caches",
            targets: &[
                ".serverless",
                ".vercel",
                ".netlify",
                ".terraform",
                ".sass-cache",
                "elm_stuff",
                "nimcache",
            ],
        },
    );

    m
});

pub fn get_targets_for_profiles(profile_names: &[String]) -> Vec<String> {
    let mut targets = Vec::new();

    for name in profile_names {
        if name == "all" {
            for profile in PROFILES.values() {
                for target in profile.targets {
                    let t = (*target).to_string();
                    if !targets.contains(&t) {
                        targets.push(t);
                    }
                }
            }
        } else if let Some(profile) = PROFILES.get(name.as_str()) {
            for target in profile.targets {
                let t = (*target).to_string();
                if !targets.contains(&t) {
                    targets.push(t);
                }
            }
        }
    }

    targets
}

#[allow(dead_code)]
pub fn list_profiles() -> Vec<&'static Profile> {
    let mut profiles: Vec<_> = PROFILES.values().collect();
    profiles.sort_by_key(|p| p.name);
    profiles
}
