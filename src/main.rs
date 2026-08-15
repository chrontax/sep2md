mod config;
mod content;
mod error;
mod fetch;
mod parser;
mod utils;

use config::Config;
use parser::Article;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let url = args.get(1).expect("Usage: sep2md <URL> [OUTPUT-PATH]");
    let outname = args.get(2).cloned();

    let config = Config::from_args(url.clone(), outname);

    let html = fetch::fetch_html(&config.url).expect("Failed to fetch page");
    let notes_url = format!("{}/notes.html", config.base_url);
    let notes_html = fetch::fetch_html(&notes_url).expect("Failed to fetch notes page");

    let article = Article::parse(&html, &notes_html, &config).expect("Failed to parse article");

    article
        .write_to_file(&config)
        .expect("Failed to write output");
}
