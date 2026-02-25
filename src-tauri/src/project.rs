use tauri::command;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Serialize, Deserialize)]
pub struct ProjectInfo {
    name: String,
    scripts: Vec<String>,
    path: String,
}

#[derive(Deserialize)]
struct PackageJson {
    name: Option<String>,
    scripts: Option<std::collections::HashMap<String, String>>,
}

#[derive(Serialize, Deserialize)]
pub struct DirEntry {
    name: String,
    #[serde(rename = "isDirectory")]
    is_directory: bool,
}

#[command]
pub fn read_dir(path: String) -> Result<Vec<DirEntry>, String> {
    let mut entries = Vec::new();
    let dir = fs::read_dir(&path).map_err(|e| e.to_string())?;
    
    for entry in dir {
        if let Ok(entry) = entry {
            if let Ok(file_type) = entry.file_type() {
                entries.push(DirEntry {
                    name: entry.file_name().to_string_lossy().to_string(),
                    is_directory: file_type.is_dir(),
                });
            }
        }
    }
    
    Ok(entries)
}

#[command]
pub fn scan_project(path: String) -> Result<ProjectInfo, String> {
    let project_path = Path::new(&path);
    let package_json_path = project_path.join("package.json");

    if !package_json_path.exists() {
        return Err("package.json not found".to_string());
    }

    let content = fs::read_to_string(package_json_path).map_err(|e| e.to_string())?;
    let pkg: PackageJson = serde_json::from_str(&content).map_err(|e| e.to_string())?;

    let mut scripts: Vec<String> = pkg.scripts.unwrap_or_default().keys().cloned().collect();
    scripts.sort();
    
    let name = pkg.name.unwrap_or_else(|| project_path.file_name().unwrap().to_str().unwrap().to_string());

    Ok(ProjectInfo {
        name,
        scripts,
        path,
    })
}
