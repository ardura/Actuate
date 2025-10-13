use std::fs;
use std::path::{Path, PathBuf};
use anyhow::{bail, Context, Result};
use serde_json::Value;
use ureq;
use zip::read::ZipArchive;

pub struct ReleaseDownloader {
    owner: String,
    repo: String,
    download_path: PathBuf,
}

impl ReleaseDownloader {
    pub fn new(owner: String, repo: String, path: PathBuf) -> Self {
        Self {
            owner,
            repo,
            download_path: path,
        }
    }

    pub fn download_latest_release(&self) -> Result<()> {
        // Ensure download directory exists
        fs::create_dir_all(&self.download_path)?;

        let url = format!(
            "https://api.github.com/repos/{}/{}/releases/latest",
            self.owner, self.repo
        );

        let response = ureq::get(&url)
            .header("Accept", "application/vnd.github.v3+json")
            .call()?
            .body_mut()
            .read_json::<Value>()?;

        let assets = response["assets"]
            .as_array()
            .context("No assets found in the latest release")?;

        for asset in assets {
            let asset_name = asset["name"]
                .as_str()
                .unwrap_or("unknown_asset");
            if !asset_name.contains("Source") {
                self.download_asset(asset)?;
            }
        }

        Ok(())
    }

    fn download_asset(&self, asset: &Value) -> Result<()> {
        let download_url = asset["browser_download_url"]
            .as_str()
            .context("Invalid download URL")?;

        let asset_name = asset["name"]
            .as_str()
            .unwrap_or("unknown_asset");

        let mut response = ureq::get(download_url).call()?;

        if !response.status().is_success() {
            bail!("Failed to download asset '{}': {}", asset_name, response.status());
        }

        let mut reader = response.body_mut().as_reader();
        let output_path = self.download_path.join(asset_name);
        let mut outfile = fs::File::create(&output_path)?;
        std::io::copy(&mut reader, &mut outfile)
            .with_context(|| format!("Failed to write downloaded file to '{}'", output_path.display()))?;

        if asset_name.ends_with(".zip") {
            self.extract_zip(&output_path)?;
        }

        println!("Downloaded asset: {}", asset_name);
        Ok(())
    }

    fn extract_zip(&self, zip_path: &Path) -> Result<()> {
        let file = fs::File::open(zip_path)?;
        let mut archive = ZipArchive::new(file)?;

        for i in 0..archive.len() {
            let mut file = archive.by_index(i)?;
            let outpath = self.download_path.join(file.name());

            if file.is_dir() {
                fs::create_dir_all(&outpath)?;
            } else {
                if let Some(p) = outpath.parent() {
                    fs::create_dir_all(p)?;
                }
                let mut outfile = fs::File::create(&outpath)?;
                std::io::copy(&mut file, &mut outfile)?;
            }
        }

        // Optionally remove the zip file after extraction
        fs::remove_file(zip_path)?;

        Ok(())
    }
}