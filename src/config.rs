pub struct Config {
    pub url: String,
    pub base_url: String,
    pub outname: String,
}

impl Config {
    pub fn from_args(url: String, outname: Option<String>) -> Self {
        let base_url = url.trim_end_matches('/').to_string();
        let outname = outname.unwrap_or_else(|| {
            format!(
                "{}.md",
                if url.ends_with('/') {
                    url[..url.len() - 1].rsplit_once('/').unwrap().1
                } else {
                    let tmp = url.rsplit_once('/').unwrap().1;
                    tmp.rsplit_once('.').map(|(base, _)| base).unwrap_or(tmp)
                }
            )
        });
        Self {
            url,
            base_url,
            outname,
        }
    }
}
