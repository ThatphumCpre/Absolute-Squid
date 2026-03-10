use clap::Parser;
use inquire::{Confirm, Select};
use std::collections::HashMap;
use std::fmt;
use std::fs;
use std::path::PathBuf;
use walkdir::WalkDir;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[arg(default_value = ".")]
    path: PathBuf,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum Env {
    Staging,
    Prod,
    Unknown,
}

#[derive(Clone, Debug, PartialEq)]
enum Kind {
    Application,
    AppProject,
    Unknown,
}

#[derive(Clone, Debug)]
struct ManifestFile {
    path: PathBuf,
    kind: Kind,
    env: Env,
    is_active: bool,
    source_path: Option<String>,
    lines: Vec<String>,
}

impl fmt::Display for ManifestFile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let env_str = match self.env {
            Env::Staging => "[STG]",
            Env::Prod => "[PRD]",
            Env::Unknown => "[???]",
        };
        let kind_str = match self.kind {
            Kind::Application => "App",
            Kind::AppProject => "Proj",
            Kind::Unknown => "???",
        };
        let file_name = self.path.file_name().unwrap_or_default().to_string_lossy();
        write!(f, "{} {} - {}", env_str, kind_str, file_name)
    }
}

#[derive(Clone, Debug)]
struct ManifestGroup {
    name: String,
    env: Env,
    is_active: bool,
    files: Vec<ManifestFile>,
}

impl fmt::Display for ManifestGroup {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let env_str = match self.env {
            Env::Staging => "[STG]",
            Env::Prod => "[PRD]",
            Env::Unknown => "[???]",
        };
        let file_count = self.files.len();
        let s = if file_count == 1 { "" } else { "s" };
        write!(f, "{} {} ({} file{})", env_str, self.name, file_count, s)
    }
}

fn process_file(path: &std::path::Path) -> Option<ManifestFile> {
    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return None,
    };

    let lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();

    let mut kind = Kind::Unknown;
    let mut is_active = false;
    let mut is_argo = false;
    let mut source_path = None;

    for line in &lines {
        let trimmed = line.trim();
        let un_commented = trimmed.trim_start_matches('#').trim_start_matches(' ');

        if un_commented.starts_with("kind: Application") {
            kind = Kind::Application;
            is_argo = true;
            if !trimmed.starts_with('#') {
                is_active = true;
            }
        } else if un_commented.starts_with("kind: AppProject") {
            kind = Kind::AppProject;
            is_argo = true;
            if !trimmed.starts_with('#') {
                is_active = true;
            }
        } else if un_commented.starts_with("path:") {
            // Attempt to capture the path: field under spec.source
            let parts: Vec<&str> = un_commented.splitn(2, ':').collect();
            if parts.len() == 2 {
                source_path = Some(parts[1].trim().to_string());
            }
        }
    }

    if !is_argo {
        return None;
    }

    // Determine environment based on the path structure first
    // E.g. /envs/staging/ or /tequila-workloads/staging/
    let mut env = Env::Unknown;
    let path_str_lower = path.to_string_lossy().to_lowercase();

    // We split path components to specifically look for 'staging', 'stg', 'production', 'prod' directories
    // We check from the end of the path backwards (closest folder rules)
    let components: Vec<_> = path.components().collect();
    for comp in components.iter().rev() {
        if let std::path::Component::Normal(os_str) = comp {
            let s = os_str.to_string_lossy().to_lowercase();
            if s == "staging" || s == "stg" {
                env = Env::Staging;
                break;
            } else if s == "production"
                || s == "prod"
                || s.starts_with("prod-")
                || s.starts_with("production-")
            {
                // adding prod-v1-32 heuristic based on user input
                env = Env::Prod;
                break;
            }
        }
    }

    // Fallback logic
    if env == Env::Unknown {
        let filename_lower = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_lowercase();
        let content_lower = content.to_lowercase();

        if filename_lower.contains("stg")
            || filename_lower.contains("staging")
            || content_lower.contains("staging")
            || content_lower.contains("stg")
        {
            env = Env::Staging;
        } else if filename_lower.contains("prod")
            || filename_lower.contains("production")
            || content_lower.contains("production")
            || content_lower.contains("prod")
        {
            env = Env::Prod;
        }
    }

    Some(ManifestFile {
        path: path.to_path_buf(),
        kind,
        env,
        is_active,
        source_path,
        lines,
    })
}

fn toggle_lines(lines: &[String], activate: bool) -> Vec<String> {
    lines
        .iter()
        .map(|line| {
            if activate {
                // activate: remove leading `# ` or `#`
                if line.starts_with("# ") {
                    line[2..].to_string()
                } else if line.starts_with('#') {
                    line[1..].to_string()
                } else {
                    line.clone()
                }
            } else {
                // deactivate: add leading `# ` if not empty or already commented
                if line.trim().is_empty() || line.starts_with('#') {
                    line.clone()
                } else {
                    format!("# {}", line)
                }
            }
        })
        .collect()
}

fn main() {
    let args = Args::parse();

    let mut manifests = Vec::new();

    for entry in WalkDir::new(&args.path).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        if path.is_file() {
            if let Some(ext) = path.extension() {
                if ext == "yaml" || ext == "yml" {
                    if let Some(manifest) = process_file(path) {
                        manifests.push(manifest);
                    }
                }
            }
        }
    }

    if manifests.is_empty() {
        println!(
            "No ArgoCD Application or AppProject manifests found in {}",
            args.path.display()
        );
        return;
    }

    // Group the manifests by Environment AND file stem (base name)
    // This prevents a Staging `seer-api` and a Prod `seer-api` from being grouped together
    let mut groups_map: HashMap<(Env, String), ManifestGroup> = HashMap::new();

    for manifest in manifests {
        let stem = manifest
            .path
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        let key = (manifest.env.clone(), stem.clone());
        let entry = groups_map
            .entry(key.clone())
            .or_insert_with(|| ManifestGroup {
                name: stem,
                env: manifest.env.clone(),
                is_active: true, // we'll update this
                files: Vec::new(),
            });

        let path_to_scan = manifest.source_path.clone();
        let manifest_is_active = manifest.is_active;
        entry.files.push(manifest);

        // If this is an Application and specifies a path, scan that target path too!
        if let Some(target_path_str) = path_to_scan {
            let target_dir = args.path.join(target_path_str);
            if target_dir.is_dir() {
                for inner_entry in WalkDir::new(&target_dir).into_iter().filter_map(|e| e.ok()) {
                    let inner_path = inner_entry.path();
                    if inner_path.is_file() {
                        if let Some(inner_ext) = inner_path.extension() {
                            if inner_ext == "yaml" || inner_ext == "yml" {
                                if let Ok(inner_content) = fs::read_to_string(inner_path) {
                                    // Treat these associated files as active/inactive based on the application's state,
                                    // or infer it locally. For simplicity, we just parse it lightly
                                    // to wrap it in a ManifestFile object to be toggled.
                                    let inner_lines: Vec<String> =
                                        inner_content.lines().map(|s| s.to_string()).collect();

                                    let is_inner_active = !inner_lines
                                        .iter()
                                        .all(|l| l.trim().is_empty() || l.starts_with('#'));

                                    entry.files.push(ManifestFile {
                                        path: inner_path.to_path_buf(),
                                        kind: Kind::Unknown,
                                        env: key.0.clone(), // inherit the environment from the parent Application
                                        is_active: is_inner_active,
                                        source_path: None,
                                        lines: inner_lines,
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    let mut groups: Vec<ManifestGroup> = groups_map
        .into_values()
        .map(|mut g| {
            // A group is considered active only if ALL its manifests are active.
            // If some are off, we treat the group as off.
            g.is_active = g.files.iter().all(|m| m.is_active);
            g
        })
        .collect();

    // Group by Environment
    let mut env_map: HashMap<Env, Vec<ManifestGroup>> = HashMap::new();
    for group in groups {
        env_map.entry(group.env.clone()).or_default().push(group);
    }

    #[derive(Clone, Debug, PartialEq)]
    struct EnvOption {
        env: Env,
        is_all_active: bool,
        group_count: usize,
    }

    impl fmt::Display for EnvOption {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            let env_str = match self.env {
                Env::Staging => "Staging",
                Env::Prod => "Production",
                Env::Unknown => "Unknown",
            };
            let s = if self.group_count == 1 { "" } else { "s" };
            write!(f, "{} Environment ({} app{})", env_str, self.group_count, s)
        }
    }

    let mut env_options = Vec::new();
    for (env, env_groups) in &env_map {
        let is_all_active = env_groups.iter().all(|g| g.is_active);
        env_options.push(EnvOption {
            env: env.clone(),
            is_all_active,
            group_count: env_groups.len(),
        });
    }

    // Sort options to have Staging, Prod, Unknown order predictably
    env_options.sort_by_key(|opt| match opt.env {
        Env::Staging => 1,
        Env::Prod => 2,
        Env::Unknown => 3,
    });

    let env_ans = Select::new("Which environment do you want to manage?", env_options.clone()).prompt();

    match env_ans {
        Ok(selected_env_opt) => {
            let env = selected_env_opt.env;
            if let Some(mut env_groups) = env_map.remove(&env) {
                // Sort the groups inside the environment alphabetically
                env_groups.sort_by(|a, b| a.name.cmp(&b.name));
                
                let app_ans = Select::new(
                    "Which project name?",
                    env_groups,
                )
                .prompt();

                match app_ans {
                    Ok(selected_group) => {
                        let current_state_str = if selected_group.is_active { "ON" } else { "OFF" };
                        let opposite_state = !selected_group.is_active;
                        let opposite_state_str = if opposite_state { "ON" } else { "OFF" };
                        
                        let confirm_prompt = format!("Current state is {}. Do you want to turn it {} rn?", current_state_str, opposite_state_str);
                        let confirm_ans = Confirm::new(&confirm_prompt)
                            .with_default(true)
                            .prompt();
                            
                        match confirm_ans {
                            Ok(true) => {
                                for manifest in &selected_group.files {
                                    if opposite_state != manifest.is_active || selected_group.is_active != opposite_state {
                                        let new_lines = toggle_lines(&manifest.lines, opposite_state);
                                        let new_content = new_lines.join("\n") + "\n";
                                        
                                        if let Err(e) = fs::write(&manifest.path, new_content) {
                                            eprintln!("Failed to write to {}: {}", manifest.path.display(), e);
                                        } else {
                                            let status = if opposite_state { "Turned ON" } else { "Turned OFF" };
                                            println!("{} {}", status, manifest.path.display());
                                        }
                                    }
                                }
                                println!("Done!");
                            }
                            Ok(false) => println!("Operation canceled."),
                            Err(_) => println!("Error or canceled."),
                        }
                    }
                    Err(_) => println!("Error or canceled."),
                }
            }
        }
        Err(_) => println!("Error or canceled."),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_toggle_lines_activate() {
        let lines: Vec<String> = vec![
            "# apiVersion: argoproj.io/v1alpha1".to_string(),
            "# kind: Application".to_string(),
            "  # metadata:".to_string(), // Keep this as is if it's not starting with #
            "#   name: app".to_string(),
        ];
        // Note: the current logic strips leading "# " or "#"
        let activated = toggle_lines(&lines, true);
        assert_eq!(activated[0], "apiVersion: argoproj.io/v1alpha1");
        assert_eq!(activated[1], "kind: Application");
        assert_eq!(activated[2], "  # metadata:"); // the toggle only untoggles at start of string! Wait!
        assert_eq!(activated[3], "  name: app");
    }

    #[test]
    fn test_toggle_lines_deactivate() {
        let lines: Vec<String> = vec![
            "apiVersion: argoproj.io/v1alpha1".to_string(),
            "kind: Application".to_string(),
            "".to_string(),
            "  name: app".to_string(),
        ];
        let deactivated = toggle_lines(&lines, false);
        assert_eq!(deactivated[0], "# apiVersion: argoproj.io/v1alpha1");
        assert_eq!(deactivated[1], "# kind: Application");
        assert_eq!(deactivated[2], ""); // empty line shouldn't be commented
        assert_eq!(deactivated[3], "#   name: app");
    }
}
