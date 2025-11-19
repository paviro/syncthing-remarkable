use std::ffi::OsStr;
use std::fs::File;
use std::path::Path;

use flate2::read::GzDecoder;
use tar::Archive;
use tokio::task;
use zip::ZipArchive;

use crate::types::MonitorError;

pub async fn extract_zip_archive(zip_path: &Path, extract_dir: &Path) -> Result<(), MonitorError> {
    let zip_path = zip_path.to_path_buf();
    let extract_dir = extract_dir.to_path_buf();

    task::spawn_blocking(move || -> Result<(), MonitorError> {
        let file = File::open(&zip_path)?;
        let mut archive = ZipArchive::new(file)
            .map_err(|err| MonitorError::Config(format!("Failed to open zip archive: {}", err)))?;

        for index in 0..archive.len() {
            let mut file = archive.by_index(index).map_err(|err| {
                MonitorError::Config(format!("Failed to read zip entry: {}", err))
            })?;

            let outpath = match file.enclosed_name() {
                Some(path) => extract_dir.join(path),
                None => continue,
            };

            if file.name().ends_with('/') {
                std::fs::create_dir_all(&outpath)?;
            } else {
                if let Some(parent) = outpath.parent() {
                    std::fs::create_dir_all(parent)?;
                }
                let mut outfile = File::create(&outpath)?;
                std::io::copy(&mut file, &mut outfile)?;
            }

            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                if let Some(mode) = file.unix_mode() {
                    std::fs::set_permissions(&outpath, std::fs::Permissions::from_mode(mode))?;
                }
            }
        }

        Ok(())
    })
    .await
    .map_err(|err| MonitorError::Config(format!("Extraction task failed: {}", err)))??;

    Ok(())
}

pub async fn extract_tarball_entry(
    tarball_path: &Path,
    entry_name: &str,
    destination: &Path,
) -> Result<(), MonitorError> {
    let tarball_path = tarball_path.to_path_buf();
    let entry_name = entry_name.to_string();
    let destination = destination.to_path_buf();

    task::spawn_blocking(move || -> Result<(), MonitorError> {
        let file = std::fs::File::open(&tarball_path)?;
        let decoder = GzDecoder::new(file);
        let mut archive = Archive::new(decoder);
        let mut found = false;

        for entry_result in archive.entries()? {
            let mut entry = entry_result?;
            let matches = entry
                .path()?
                .file_name()
                .and_then(OsStr::to_str)
                .map(|name| name == entry_name)
                .unwrap_or(false);

            if matches {
                entry.unpack(&destination)?;
                found = true;
                break;
            }
        }

        if !found {
            return Err(MonitorError::Config(format!(
                "Archive did not contain expected entry '{}'",
                entry_name
            )));
        }

        Ok(())
    })
    .await
    .map_err(|err| {
        MonitorError::Config(format!("Extraction task failed to complete: {}", err))
    })??;

    Ok(())
}

