//! Project scaffolding for FastC
//!
//! This module provides functionality to create new FastC projects
//! with appropriate directory structure and build files.

mod templates;

use miette::{bail, IntoDiagnostic, Result};
use std::fs;
use std::path::Path;

/// Type of project to create
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectType {
    /// A binary application with main.fc
    Binary,
    /// A library with lib.fc
    Library,
    /// An FFI wrapper library with header generation
    FfiWrapper,
}

impl std::str::FromStr for ProjectType {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "binary" | "bin" => Ok(ProjectType::Binary),
            "library" | "lib" => Ok(ProjectType::Library),
            "ffi-wrapper" | "ffi" => Ok(ProjectType::FfiWrapper),
            _ => Err(format!(
                "Unknown project type: '{}'. Use 'binary', 'library', or 'ffi-wrapper'.",
                s
            )),
        }
    }
}

impl std::fmt::Display for ProjectType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProjectType::Binary => write!(f, "binary"),
            ProjectType::Library => write!(f, "library"),
            ProjectType::FfiWrapper => write!(f, "ffi-wrapper"),
        }
    }
}

/// Build system template to use
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuildTemplate {
    /// GNU Make
    Make,
    /// CMake
    CMake,
    /// Meson
    Meson,
}

impl std::str::FromStr for BuildTemplate {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "make" => Ok(BuildTemplate::Make),
            "cmake" => Ok(BuildTemplate::CMake),
            "meson" => Ok(BuildTemplate::Meson),
            _ => Err(format!(
                "Unknown build template: '{}'. Use 'make', 'cmake', or 'meson'.",
                s
            )),
        }
    }
}

impl std::fmt::Display for BuildTemplate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BuildTemplate::Make => write!(f, "make"),
            BuildTemplate::CMake => write!(f, "cmake"),
            BuildTemplate::Meson => write!(f, "meson"),
        }
    }
}

/// Create a new FastC project
pub fn create_project(
    name: &str,
    path: &Path,
    project_type: ProjectType,
    build_template: BuildTemplate,
) -> Result<()> {
    // Validate project name
    if name.is_empty() {
        bail!("Project name cannot be empty");
    }
    if !name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    {
        bail!("Project name must contain only alphanumeric characters, underscores, and hyphens");
    }

    // Create project directory
    let project_dir = path.join(name);
    if project_dir.exists() {
        bail!("Directory '{}' already exists", project_dir.display());
    }

    fs::create_dir_all(&project_dir).into_diagnostic()?;

    // Create subdirectories
    fs::create_dir_all(project_dir.join("src")).into_diagnostic()?;

    if project_type == ProjectType::FfiWrapper {
        fs::create_dir_all(project_dir.join("include")).into_diagnostic()?;
    }

    // Write files
    write_manifest(&project_dir, name, project_type)?;
    write_source_files(&project_dir, name, project_type)?;
    write_build_files(&project_dir, name, project_type, build_template)?;
    write_gitignore(&project_dir)?;
    write_readme(&project_dir, name, project_type, build_template)?;

    eprintln!("Created {} project '{}' at {}", project_type, name, project_dir.display());
    eprintln!();
    eprintln!("To get started:");
    eprintln!("  cd {}", name);
    match build_template {
        BuildTemplate::Make => eprintln!("  make"),
        BuildTemplate::CMake => {
            eprintln!("  mkdir build && cd build");
            eprintln!("  cmake ..");
            eprintln!("  make");
        }
        BuildTemplate::Meson => {
            eprintln!("  meson setup build");
            eprintln!("  meson compile -C build");
        }
    }

    Ok(())
}

/// Initialize a FastC project in an existing directory
pub fn init_project(
    path: &Path,
    project_type: ProjectType,
    build_template: BuildTemplate,
) -> Result<()> {
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("project");

    // Check if already initialized
    if path.join("fastc.toml").exists() {
        bail!("Directory already contains a fastc.toml file");
    }

    // Create subdirectories if needed
    let src_dir = path.join("src");
    if !src_dir.exists() {
        fs::create_dir_all(&src_dir).into_diagnostic()?;
    }

    if project_type == ProjectType::FfiWrapper {
        let include_dir = path.join("include");
        if !include_dir.exists() {
            fs::create_dir_all(&include_dir).into_diagnostic()?;
        }
    }

    // Write files (skip if they exist)
    write_manifest(path, name, project_type)?;

    let main_file = match project_type {
        ProjectType::Binary => src_dir.join("main.fc"),
        ProjectType::Library | ProjectType::FfiWrapper => src_dir.join("lib.fc"),
    };
    if !main_file.exists() {
        write_source_files(path, name, project_type)?;
    }

    // Write build files
    write_build_files(path, name, project_type, build_template)?;

    // Write .gitignore if not present
    if !path.join(".gitignore").exists() {
        write_gitignore(path)?;
    }

    // Write README if not present
    if !path.join("README.md").exists() {
        write_readme(path, name, project_type, build_template)?;
    }

    eprintln!("Initialized {} project in {}", project_type, path.display());

    Ok(())
}

fn write_manifest(project_dir: &Path, name: &str, project_type: ProjectType) -> Result<()> {
    let content = templates::manifest(name, project_type);
    fs::write(project_dir.join("fastc.toml"), content).into_diagnostic()
}

fn write_source_files(project_dir: &Path, name: &str, project_type: ProjectType) -> Result<()> {
    let (filename, content) = match project_type {
        ProjectType::Binary => ("main.fc", templates::main_fc(name)),
        ProjectType::Library => ("lib.fc", templates::lib_fc(name)),
        ProjectType::FfiWrapper => ("lib.fc", templates::ffi_lib_fc(name)),
    };

    fs::write(project_dir.join("src").join(filename), content).into_diagnostic()
}

fn write_build_files(
    project_dir: &Path,
    name: &str,
    project_type: ProjectType,
    build_template: BuildTemplate,
) -> Result<()> {
    match build_template {
        BuildTemplate::Make => {
            let content = templates::makefile(name, project_type);
            fs::write(project_dir.join("Makefile"), content).into_diagnostic()
        }
        BuildTemplate::CMake => {
            let content = templates::cmakelists(name, project_type);
            fs::write(project_dir.join("CMakeLists.txt"), content).into_diagnostic()
        }
        BuildTemplate::Meson => {
            let content = templates::meson_build(name, project_type);
            fs::write(project_dir.join("meson.build"), content).into_diagnostic()
        }
    }
}

fn write_gitignore(project_dir: &Path) -> Result<()> {
    let content = templates::gitignore();
    fs::write(project_dir.join(".gitignore"), content).into_diagnostic()
}

fn write_readme(
    project_dir: &Path,
    name: &str,
    project_type: ProjectType,
    build_template: BuildTemplate,
) -> Result<()> {
    let content = templates::readme(name, project_type, build_template);
    fs::write(project_dir.join("README.md"), content).into_diagnostic()
}
