use std::io::Write;

use scraper::{Html, Selector};

use crate::config::Config;
use crate::content::Content;

mod section;
pub use section::Section;

const SKIP_ELEMENTS: [&str; 1] = ["hr"];

#[derive(Debug)]
pub struct Article {
    pub title: String,
    pub preamble: Vec<Content>,
    pub main_text: Vec<Section>,
    pub notes: Vec<Vec<Content>>,
}

impl Article {
    pub fn parse(html: &str, notes_html: &str, config: &Config) -> Result<Self, crate::error::Error> {
        let document = Html::parse_document(html);
        if !document.errors.is_empty() {
            let errors: Vec<String> = document.errors.iter().map(|e| e.to_string()).collect();
            return Err(crate::error::Error::HtmlParse(errors));
        }
        let title = parse_title(&document)?;
        let preamble = parse_preamble(&document, config);
        let main_text = parse_main_text(&document, config)?;
        let notes = parse_notes(notes_html, config)?;
        Ok(Article {
            title,
            preamble,
            main_text,
            notes,
        })
    }

    pub fn write_to_file<P: AsRef<std::path::Path>>(self, path: P) -> std::io::Result<()> {
        let mut f = std::fs::File::create(path)?;
        writeln!(f, "# {}\n", self.title)?;

        for content in self.preamble {
            f.write_all(content.markdown().as_bytes())?;
        }

        for s in self.main_text {
            f.write_all(s.markdown().as_bytes())?;
        }

        for (i, note) in self.notes.into_iter().enumerate() {
            write!(
                f,
                "[^{}]: {}",
                i + 1,
                note.into_iter()
                    .map(Content::markdown)
                    .reduce(|acc, content| acc + "    " + &content)
                    .unwrap()
            )?;
        }

        Ok(())
    }
}

fn parse_preamble(document: &Html, config: &Config) -> Vec<Content> {
    document
        .select(&Selector::parse("#preamble > *").unwrap())
        .map(|el| Content::from_element(el, &|_| false, config))
        .collect()
}

fn parse_main_text(
    document: &Html,
    config: &Config,
) -> Result<Vec<Section>, crate::error::Error> {
    let mut main_els = document
        .select(&Selector::parse("#main-text").unwrap())
        .next()
        .or_else(|| {
            document
                .select(&Selector::parse("#aueditable").unwrap())
                .next()
        })
        .ok_or(crate::error::Error::MissingElement("#main-text"))?
        .child_elements()
        .peekable();

    let mut main_text = Vec::new();
    while main_els.peek().is_some() {
        main_text.push(Section::from_iter(&mut main_els, config));
        while main_els
            .peek()
            .is_some_and(|el| SKIP_ELEMENTS.contains(&el.value().name()))
        {
            main_els.next();
        }
    }
    Ok(main_text)
}

fn parse_title(document: &Html) -> Result<String, crate::error::Error> {
    let title = document
        .select(&Selector::parse("h1").unwrap())
        .next()
        .or_else(|| {
            document
                .select(&Selector::parse("#aueditable > h2").unwrap())
                .next()
        })
        .ok_or(crate::error::Error::MissingElement("h1"))?
        .text()
        .next()
        .ok_or(crate::error::Error::MissingElement("h1 text"))?
        .trim()
        .to_string();
    Ok(title)
}

fn parse_notes(html: &str, config: &Config) -> Result<Vec<Vec<Content>>, crate::error::Error> {
    let document = Html::parse_document(html);
    let mut notes = Vec::new();
    for el in document.select(&Selector::parse("#aueditable > div").unwrap()) {
        notes.push(
            el.child_elements()
                .map(|el| {
                    Content::from_element(el, &|node| {
                        node.as_element().is_some_and(|el| el.name() == "a")
                    }, config)
                })
                .collect::<Vec<_>>(),
        );
    }
    Ok(notes)
}
