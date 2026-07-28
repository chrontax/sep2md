use crate::error::Error;

pub fn fetch_html(url: &str) -> Result<String, Error> {
    Ok(reqwest::blocking::get(url)?.text()?)
}
