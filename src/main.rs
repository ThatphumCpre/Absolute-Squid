use clap::Parser;
use inquire::MultiSelect;
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

#[derive(Clone, Debug, PartialEq)]
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

use std::collections::HashMap;

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
        }
    }

    if !is_argo {
        return None;
    }

    // Determine environment based on filename or content
    let filename_lower = path.file_name().unwrap_or_default().to_string_lossy().to_lowercase();
    let content_lower = content.to_lowercase();
    
    let env = if filename_lower.contains("stg") || filename_lower.contains("staging") || content_lower.contains("staging") || content_lower.contains("stg") {
        Env::Staging
    } else if filename_lower.contains("prod") || filename_lower.contains("production") || content_lower.contains("production") || content_lower.contains("prod") {
        Env::Prod
    } else {
        Env::Unknown
    };

    Some(ManifestFile {
        path: path.to_path_buf(),
        kind,
        env,
        is_active,
        lines,
    })
}

fn toggle_lines(lines: &[String], activate: bool) -> Vec<String> {
    lines.iter().map(|line| {
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
    }).collect()
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
        println!("No ArgoCD Application or AppProject manifests found in {}", args.path.display());
        return;
    }

    // Group the manifests by file stem (base name)
    let mut groups_map: HashMap<String, ManifestGroup> = HashMap::new();

    for manifest in manifests {
        let stem = manifest.path.file_stem().unwrap_or_default().to_string_lossy().to_string();
        let entry = groups_map.entry(stem.clone()).or_insert_with(|| ManifestGroup {
            name: stem,
            env: manifest.env.clone(),
            is_active: true, // we'll update this
            files: Vec::new(),
        });
        
        entry.files.push(manifest);
    }

    let mut groups: Vec<ManifestGroup> = groups_map.into_values().map(|mut g| {
        // A group is considered active only if ALL its manifests are active.
        // If some are off, we treat the group as off.
        g.is_active = g.files.iter().all(|m| m.is_active);
        g
    }).collect();

    // Sort groups for stable output
    groups.sort_by(|a, b| a.name.cmp(&b.name));

    // pre-select indices for OFF groups
    let mut default_selection = Vec::new();
    for (i, g) in groups.iter().enumerate() {
        if !g.is_active {
            default_selection.push(i);
        }
    }
    
    let ans = MultiSelect::new("Select deployments to turn OFF (checked = OFF, unchecked = ON):", groups.clone())
        .with_default(&default_selection)
        .prompt();

    match ans {
        Ok(selections) => {
            let selected_names: Vec<_> = selections.iter().map(|s| &s.name).collect();
            
            for group in &groups {
                let should_be_off = selected_names.contains(&&group.name);
                let should_be_active = !should_be_off;
                
                // if we toggle the group on, turn ALL inner manifests on
                // if we toggle the group off, turn ALL inner manifests off
                for manifest in &group.files {
                    if should_be_active != manifest.is_active || group.is_active != should_be_active {
                        let new_lines = toggle_lines(&manifest.lines, should_be_active);
                        let new_content = new_lines.join("\n") + "\n";
                        
                        if let Err(e) = fs::write(&manifest.path, new_content) {
                            eprintln!("Failed to write to {}: {}", manifest.path.display(), e);
                        } else {
                            let status = if should_be_active { "Turned ON" } else { "Turned OFF" };
                            println!("{} {}", status, manifest.path.display());
                        }
                    }
                }
            }
            println!("Done!");
        }
        Err(e) => println!("Error or canceled: {}", e),
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
