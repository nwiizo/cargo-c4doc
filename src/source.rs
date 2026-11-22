use crate::{Error, Result, io_error, model::SourceRef};
use ignore::WalkBuilder;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::Read,
    path::{Component, Path},
};

pub const MAX_SOURCE_BYTES: u64 = 2 * 1024 * 1024;

pub fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

pub fn read_bounded(path: &Path, max_bytes: u64) -> Result<Vec<u8>> {
    let file = fs::File::open(path).map_err(|e| io_error(path, e))?;
    if !file.metadata().map_err(|e| io_error(path, e))?.is_file() {
        return Err(Error::Invalid(format!(
            "{} is not a regular file",
            path.display()
        )));
    }
    let mut bytes = Vec::new();
    file.take(max_bytes.saturating_add(1))
        .read_to_end(&mut bytes)
        .map_err(|e| io_error(path, e))?;
    if bytes.len() as u64 > max_bytes {
        return Err(Error::Invalid(format!(
            "{} exceeds {max_bytes} bytes",
            path.display()
        )));
    }
    Ok(bytes)
}

pub fn validate_relative(path: &str) -> Result<()> {
    if path.is_empty()
        || path.contains(['\\', ':'])
        || path.chars().any(char::is_control)
        || path
            .split('/')
            .any(|part| part.is_empty() || part == "." || part == "..")
        || !Path::new(path)
            .components()
            .all(|part| matches!(part, Component::Normal(_)))
    {
        return Err(Error::Invalid(format!(
            "unsafe repository-relative path: {path:?}"
        )));
    }
    Ok(())
}

pub fn source_bytes(root: &Path, relative: &str) -> Result<Vec<u8>> {
    validate_relative(relative)?;
    let mut path = root.to_path_buf();
    for part in Path::new(relative).components() {
        path.push(part);
        if fs::symlink_metadata(&path)
            .map_err(|e| io_error(&path, e))?
            .file_type()
            .is_symlink()
        {
            return Err(Error::Invalid(format!(
                "source symlinks are not followed: {relative}"
            )));
        }
    }
    read_bounded(&path, MAX_SOURCE_BYTES)
}

pub fn line_count(bytes: &[u8]) -> Result<u64> {
    let text = std::str::from_utf8(bytes)
        .map_err(|_| Error::Invalid("source must be UTF-8 text".into()))?;
    if text.contains('\0') {
        return Err(Error::Invalid("binary source is not supported".into()));
    }
    Ok(text.lines().count() as u64)
}

pub fn cite(
    root: &Path,
    path: String,
    start_line: u64,
    end_line: Option<u64>,
) -> Result<SourceRef> {
    let bytes = source_bytes(root, &path)?;
    let lines = line_count(&bytes)?;
    let end_line = end_line.unwrap_or(lines);
    if start_line == 0 || end_line < start_line || end_line > lines {
        return Err(Error::Invalid(format!(
            "{path}: invalid line range {start_line}..{end_line}; file has {lines} lines"
        )));
    }
    Ok(SourceRef {
        path,
        start_line,
        end_line,
        sha256: digest(&bytes),
    })
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Inventory {
    pub(crate) schema_version: u32,
    pub files: Vec<FileInfo>,
    pub skipped: Vec<Skipped>,
    pub(crate) max_files: usize,
    pub(crate) max_bytes: u64,
    pub truncated: bool,
    limitations: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct FileInfo {
    pub path: String,
    pub(crate) language: String,
    bytes: u64,
    lines: u64,
    pub(crate) sha256: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Skipped {
    path: String,
    reason: String,
}

fn language(path: &Path) -> Option<&'static str> {
    let name = path.file_name()?.to_str()?;
    if matches!(
        name,
        "Dockerfile" | "Containerfile" | "Makefile" | "Justfile" | "justfile"
    ) {
        return Some("build");
    }
    Some(match path.extension()?.to_str()? {
        "rs" => "rust",
        "ts" | "tsx" => "typescript",
        "js" | "jsx" | "mjs" | "cjs" => "javascript",
        "go" => "go",
        "py" => "python",
        "java" => "java",
        "kt" | "kts" => "kotlin",
        "cs" => "csharp",
        "rb" => "ruby",
        "php" => "php",
        "swift" => "swift",
        "c" | "h" | "cc" | "cpp" | "hpp" => "c/c++",
        "scala" => "scala",
        "ex" | "exs" => "elixir",
        "vue" | "svelte" | "html" => "frontend",
        "sh" | "fish" => "shell",
        "sql" => "sql",
        "proto" => "protobuf",
        "graphql" | "gql" => "graphql",
        "toml" | "json" | "yaml" | "yml" | "tf" | "hcl" => "configuration",
        "md" | "mdx" | "rst" | "adoc" | "txt" => "documentation",
        _ => return None,
    })
}

pub fn scan(root: &Path, max_files: usize, max_bytes: u64) -> Result<Inventory> {
    if !root.is_dir() {
        return Err(Error::Invalid(format!(
            "{} is not a directory",
            root.display()
        )));
    }
    let mut inventory = Inventory {
        schema_version: 1,
        files: vec![],
        skipped: vec![],
        max_files,
        max_bytes,
        truncated: false,
        limitations: vec![
            "File inventory only: no semantic analysis, dependency inference, or source contents.",
            "Respects .gitignore/.ignore, including outside Git; hidden paths, symlinks, build and vendor directories are excluded.",
            "Only recognized UTF-8 source, manifest, and documentation files are included. This is not a secret scanner.",
            "Ignored, hidden, unsupported, and pruned paths are not enumerated in skipped; read failures and size limits are.",
        ].into_iter().map(String::from).collect(),
    };
    let walker = WalkBuilder::new(root)
        .require_git(false)
        .follow_links(false)
        .sort_by_file_path(Path::cmp)
        .filter_entry(|entry| {
            entry.depth() == 0
                || !matches!(
                    entry.file_name().to_str(),
                    Some(
                        "target"
                            | "node_modules"
                            | "vendor"
                            | "dist"
                            | "build"
                            | "coverage"
                            | "__pycache__"
                    )
                )
        })
        .build();
    for entry in walker {
        let entry = entry.map_err(|e| Error::Invalid(format!("scan failed: {e}")))?;
        if !entry.file_type().is_some_and(|kind| kind.is_file()) {
            continue;
        }
        let Some(language) = language(entry.path()) else {
            continue;
        };
        if inventory.files.len() == max_files {
            inventory.truncated = true;
            break;
        }
        let path = entry
            .path()
            .strip_prefix(root)
            .map_err(|e| Error::Invalid(e.to_string()))?
            .to_str()
            .ok_or_else(|| Error::Invalid("non-UTF-8 source path".into()))?
            .replace('\\', "/");
        let info = read_bounded(entry.path(), max_bytes).and_then(|bytes| {
            Ok(FileInfo {
                path: path.clone(),
                language: language.into(),
                lines: line_count(&bytes)?,
                bytes: bytes.len() as u64,
                sha256: digest(&bytes),
            })
        });
        match info {
            Ok(info) => inventory.files.push(info),
            Err(error) => inventory.skipped.push(Skipped {
                path,
                reason: error.to_string(),
            }),
        }
    }
    inventory.files.sort_by(|a, b| a.path.cmp(&b.path));
    inventory.skipped.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(inventory)
}
