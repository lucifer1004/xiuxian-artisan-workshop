use std::fs;
use std::path::Path;

use crate::error::QianjiError;

pub(super) fn copy_template_dir(template_dir: &Path, target_dir: &Path) -> Result<(), QianjiError> {
    if !template_dir.is_dir() {
        return Err(QianjiError::Topology(format!(
            "Flowhub module template directory `{}` is missing",
            template_dir.display()
        )));
    }

    fs::create_dir_all(target_dir).map_err(|error| {
        QianjiError::Topology(format!(
            "Failed to create materialized target directory `{}`: {error}",
            target_dir.display()
        ))
    })?;

    copy_dir_recursive(template_dir, target_dir)
}

fn copy_dir_recursive(source_dir: &Path, target_dir: &Path) -> Result<(), QianjiError> {
    for entry in fs::read_dir(source_dir).map_err(|error| {
        QianjiError::Topology(format!(
            "Failed to read Flowhub template directory `{}`: {error}",
            source_dir.display()
        ))
    })? {
        let entry = entry.map_err(|error| {
            QianjiError::Topology(format!(
                "Failed to inspect Flowhub template directory `{}`: {error}",
                source_dir.display()
            ))
        })?;
        let source_path = entry.path();
        let target_path = target_dir.join(entry.file_name());
        let file_type = entry.file_type().map_err(|error| {
            QianjiError::Topology(format!(
                "Failed to inspect Flowhub template entry `{}`: {error}",
                source_path.display()
            ))
        })?;

        if file_type.is_dir() {
            fs::create_dir_all(&target_path).map_err(|error| {
                QianjiError::Topology(format!(
                    "Failed to create materialized directory `{}`: {error}",
                    target_path.display()
                ))
            })?;
            copy_dir_recursive(&source_path, &target_path)?;
            continue;
        }

        fs::copy(&source_path, &target_path).map_err(|error| {
            QianjiError::Topology(format!(
                "Failed to copy Flowhub template file `{}` into `{}`: {error}",
                source_path.display(),
                target_path.display()
            ))
        })?;
    }

    Ok(())
}
