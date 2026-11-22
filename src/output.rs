use crate::{Error, Result, io_error, source};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fs,
    io::Write,
    path::{Component, Path, PathBuf},
};

const MANIFEST: &str = ".c4doc-manifest.json";

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    schema_version: u32,
    files: BTreeMap<String, String>,
}

/// Resolve without following symlinks, including existing ancestor directories.
pub fn destination(path: &Path) -> Result<PathBuf> {
    let cwd = std::env::current_dir().map_err(|e| io_error(".", e))?;
    let absolute = if path.is_absolute() {
        path.to_owned()
    } else {
        cwd.join(path)
    };
    let mut resolved = PathBuf::new();
    for part in absolute.components() {
        match part {
            Component::CurDir => continue,
            Component::ParentDir => {
                resolved.pop();
            }
            _ => resolved.push(part.as_os_str()),
        }
        match fs::symlink_metadata(&resolved) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                return Err(Error::Invalid(format!(
                    "output paths cannot traverse symlinks: {}",
                    resolved.display()
                )));
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(io_error(&resolved, error)),
        }
    }
    // Windows may spell an existing ancestor with a short (8.3) name. Use
    // the same filesystem spelling as the source root even for a new output.
    for ancestor in resolved.ancestors() {
        match dunce::canonicalize(ancestor) {
            Ok(existing) => {
                let suffix = resolved.strip_prefix(ancestor).map_err(|error| {
                    Error::Invalid(format!(
                        "invalid output path {}: {error}",
                        resolved.display()
                    ))
                })?;
                return Ok(if suffix.as_os_str().is_empty() {
                    existing
                } else {
                    existing.join(suffix)
                });
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => return Err(io_error(ancestor, error)),
        }
    }
    Err(Error::Invalid(format!(
        "output has no existing ancestor: {}",
        resolved.display()
    )))
}

pub fn write_new(path: &Path, bytes: &[u8]) -> Result<()> {
    let path = destination(path)?;
    let parent = path
        .parent()
        .ok_or_else(|| Error::Invalid("output needs a parent directory".into()))?;
    fs::create_dir_all(parent).map_err(|e| io_error(parent, e))?;
    let mut file = tempfile::NamedTempFile::new_in(parent).map_err(|e| io_error(parent, e))?;
    file.write_all(bytes).map_err(|e| io_error(&path, e))?;
    file.persist_noclobber(&path)
        .map_err(|e| io_error(&path, e.error))?;
    Ok(())
}

pub fn publish(path: &Path, documents: &BTreeMap<String, String>, check_only: bool) -> Result<()> {
    let path = destination(path)?;
    let exists = path.try_exists().map_err(|e| io_error(&path, e))?;
    if exists {
        verify_owned(&path)?;
    }
    let manifest = Manifest {
        schema_version: 1,
        files: documents
            .iter()
            .map(|(name, text)| (name.clone(), source::digest(text.as_bytes())))
            .collect(),
    };
    let mut files = documents.clone();
    files.insert(
        MANIFEST.into(),
        format!("{}\n", serde_json::to_string_pretty(&manifest)?),
    );
    if check_only {
        if !exists {
            return Err(Error::Invalid(format!(
                "generated documents are missing: {}",
                path.display()
            )));
        }
        let entries = fs::read_dir(&path)
            .map_err(|e| io_error(&path, e))?
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|e| io_error(&path, e))?;
        if entries.len() != files.len() {
            return Err(Error::Invalid(
                "generated file list is stale; run cargo c4doc render".into(),
            ));
        }
        for (name, expected) in &files {
            let current = fs::read(path.join(name)).map_err(|e| io_error(path.join(name), e))?;
            if current != expected.as_bytes() {
                return Err(Error::Invalid(format!(
                    "generated file is stale: {name}; run cargo c4doc render"
                )));
            }
        }
        return Ok(());
    }
    let parent = path
        .parent()
        .ok_or_else(|| Error::Invalid("output needs a parent directory".into()))?;
    fs::create_dir_all(parent).map_err(|e| io_error(parent, e))?;
    let stage = tempfile::tempdir_in(parent).map_err(|e| io_error(parent, e))?;
    let next = stage.path().join("next");
    fs::create_dir(&next).map_err(|e| io_error(&next, e))?;
    for (name, contents) in &files {
        // Flat, validated filenames keep publication independent of model-controlled paths.
        source::validate_relative(name)?;
        if name.contains('/') {
            return Err(Error::Invalid("generated filenames must be flat".into()));
        }
        fs::write(next.join(name), contents).map_err(|e| io_error(next.join(name), e))?;
    }
    let previous = stage.path().join("previous");
    if exists {
        fs::rename(&path, &previous).map_err(|e| io_error(&path, e))?;
    }
    if let Err(error) = fs::rename(&next, &path) {
        if exists && let Err(restore_error) = fs::rename(&previous, &path) {
            let recovery = stage.keep();
            return Err(Error::Invalid(format!(
                "publish failed: {error}; restore failed: {restore_error}; previous files preserved in {}",
                recovery.join("previous").display()
            )));
        }
        return Err(io_error(&path, error));
    }
    Ok(())
}

fn verify_owned(path: &Path) -> Result<()> {
    if !path.is_dir() {
        return Err(Error::Invalid(format!(
            "{} is not a document directory",
            path.display()
        )));
    }
    let manifest_path = path.join(MANIFEST);
    if !manifest_path.exists() {
        return Err(Error::Invalid(format!(
            "{} is not managed by cargo-c4doc; choose a new output directory",
            path.display()
        )));
    }
    if fs::symlink_metadata(&manifest_path)
        .map_err(|e| io_error(&manifest_path, e))?
        .file_type()
        .is_symlink()
    {
        return Err(Error::Invalid(
            "document manifest cannot be a symlink".into(),
        ));
    }
    let manifest: Manifest =
        serde_json::from_slice(&source::read_bounded(&manifest_path, 8 * 1024 * 1024)?)?;
    if manifest.schema_version != 1 {
        return Err(Error::Invalid(
            "unsupported document manifest version".into(),
        ));
    }
    let mut seen = 0;
    for entry in fs::read_dir(path).map_err(|e| io_error(path, e))? {
        let entry = entry.map_err(|e| io_error(path, e))?;
        let name = entry
            .file_name()
            .into_string()
            .map_err(|_| Error::Invalid("non-UTF-8 document filename".into()))?;
        if name == MANIFEST {
            continue;
        }
        let Some(expected) = manifest.files.get(&name) else {
            return Err(Error::Invalid(format!(
                "unmanaged file {name}; keep hand-written notes outside the generated directory"
            )));
        };
        if !entry
            .file_type()
            .map_err(|e| io_error(entry.path(), e))?
            .is_file()
        {
            return Err(Error::Invalid(format!(
                "{name} is no longer a regular generated file"
            )));
        }
        let bytes = source::read_bounded(&entry.path(), 32 * 1024 * 1024)?;
        if source::digest(&bytes) != *expected {
            return Err(Error::Invalid(format!(
                "generated file was edited: {name}; preserve your edits before regenerating"
            )));
        }
        seen += 1;
    }
    if seen != manifest.files.len() {
        return Err(Error::Invalid(
            "generated files were removed; restore them or choose a new output directory".into(),
        ));
    }
    Ok(())
}
