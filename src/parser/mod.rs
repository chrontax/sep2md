use std::fs::File;
use std::io::{self, Write};

use scraper::{Html, Selector};

use crate::config::Config;
use crate::content::Content;
use crate::content::inline::markdownify;

mod section;
pub use section::Section;

const SKIP_ELEMENTS: [&str; 1] = ["hr"];

#[derive(Debug)]
pub struct Article {
    pub title: String,
    pub pub_info: String,
    pub preamble: Vec<Content>,
    pub main_text: Vec<Section>,
    pub bibliography: Vec<Section>,
    pub other_resources: Vec<Section>,
    pub related_entries: Vec<Section>,
    pub copyright: String,
    pub notes: Vec<Vec<Content>>,
}

impl Article {
    pub fn parse(
        html: &str,
        notes_html: &str,
        config: &Config,
    ) -> Result<Self, crate::error::Error> {
        let document = Html::parse_document(html);
        if !document.errors.is_empty() {
            let errors: Vec<String> = document.errors.iter().map(|e| e.to_string()).collect();
            return Err(crate::error::Error::HtmlParse(errors));
        }
        let title = parse_title(&document)?;
        let pub_info = parse_pub_info(&document, config);
        let preamble = parse_preamble(&document, config);
        let main_text = parse_main_text(&document, config)?;
        let bibliography = parse_section_div(&document, "#bibliography", config);
        let other_resources = parse_section_div(&document, "#other-internet-resources", config);
        let related_entries = parse_section_div(&document, "#related-entries", config);
        let copyright = parse_copyright(&document, config);
        let notes = parse_notes(notes_html, config)?;
        Ok(Article {
            title,
            pub_info,
            preamble,
            main_text,
            bibliography,
            other_resources,
            related_entries,
            copyright,
            notes,
        })
    }

    pub fn write_to_file(self, config: &Config) -> io::Result<()> {
        let mut f = File::create(&config.outname)?;
        writeln!(f, "# {}\n", self.title)?;
        if !self.pub_info.is_empty() {
            writeln!(f, "{}\n", self.pub_info)?;
        }

        for content in self.preamble {
            f.write_all(content.markdown().as_bytes())?;
        }

        for s in self.main_text {
            f.write_all(s.markdown().as_bytes())?;
        }

        for s in self.bibliography {
            f.write_all(s.markdown().as_bytes())?;
        }

        for s in self.other_resources {
            f.write_all(s.markdown().as_bytes())?;
        }

        for s in self.related_entries {
            f.write_all(s.markdown().as_bytes())?;
        }

        f.write_all(b"<div style=\"page-break-after: always;\"></div>\n\n")?;

        if !self.copyright.is_empty() {
            writeln!(f, "{}\n", self.copyright)?;
        }

        let date = chrono::Local::now().format("%Y-%m-%d");
        writeln!(
            f,
            "Generated on {date} from [{url}]({url}) using [sep2md](https://github.com/chrontax/sep2md).\n",
            url = config.url
        )?;

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
        .flat_map(|el| Content::flatten_element(el, &|_| false, config))
        .collect()
}

fn parse_main_text(document: &Html, config: &Config) -> Result<Vec<Section>, crate::error::Error> {
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

fn parse_section_div(document: &Html, selector: &str, config: &Config) -> Vec<Section> {
    let mut els = match document.select(&Selector::parse(selector).unwrap()).next() {
        Some(el) => el.child_elements().peekable(),
        None => return Vec::new(),
    };

    let mut sections = Vec::new();
    while els.peek().is_some() {
        sections.push(Section::from_iter(&mut els, config));
        while els
            .peek()
            .is_some_and(|el| SKIP_ELEMENTS.contains(&el.value().name()))
        {
            els.next();
        }
    }
    sections
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

fn parse_pub_info(document: &Html, config: &Config) -> String {
    document
        .select(&Selector::parse("#pubinfo").unwrap())
        .next()
        .map(|el| markdownify(el, &|_| false, config).trim().to_string())
        .unwrap_or_default()
}

fn parse_copyright(document: &Html, config: &Config) -> String {
    document
        .select(&Selector::parse("#article-copyright").unwrap())
        .next()
        .map(|el| markdownify(el, &|_| false, config).trim().to_string())
        .unwrap_or_default()
}

fn parse_notes(html: &str, config: &Config) -> Result<Vec<Vec<Content>>, crate::error::Error> {
    let document = Html::parse_document(html);
    let mut notes = Vec::new();
    for el in document.select(&Selector::parse("#aueditable > div").unwrap()) {
        notes.push(
            el.child_elements()
                .flat_map(|el| {
                    Content::flatten_element(
                        el,
                        &|node| node.as_element().is_some_and(|el| el.name() == "a"),
                        config,
                    )
                })
                .collect::<Vec<_>>(),
        );
    }
    Ok(notes)
}
