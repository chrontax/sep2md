use std::{fs, io::Write};

use crate::{config::Config, error::Error};

pub fn download_figure(url: &str, config: &Config) -> Result<(), Error> {
    eprintln!("Downloading figure to: figures/{url}");
    if !matches!(fs::exists("figures"), Ok(true)) {
        fs::create_dir("figures")?;
    }
    fs::File::create(format!("figures/{url}"))
        .expect("Failed to create figure file")
        .write_all(&reqwest::blocking::get(format!("{}/{}", config.base_url, url))?.bytes()?)?;
    Ok(())
}
