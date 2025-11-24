use std::{
    env::args,
    fs::File,
    io::{self, Write},
    iter::Peekable,
    path::Path,
};

use itertools::Itertools;
use once_cell::sync::OnceCell;
use scraper::{ElementRef, Html, Node, Selector};

const SKIP_ELEMENTS: [&str; 1] = ["hr"];

// I should do better than adding global state, but this app doesn't need exemplary code.
static BASE_URL: OnceCell<String> = OnceCell::new();

fn main() {
    let mut args = args();
    let url = args.nth(1).expect("SEP page URL is required");
    BASE_URL
        .set(url.rsplit_once('/').unwrap().0.to_string())
        .unwrap();
    let outname = args.next().unwrap_or_else(|| {
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
    let html = reqwest::blocking::get(&url)
        .expect("Failed to request page HTML")
        .text()
        .unwrap();
    let document = Html::parse_document(&html);
    if !document.errors.is_empty() {
        eprintln!("HTML parse errors:\n\t{}", document.errors.join("\n\t"));
    }
    let title = document
        .select(&Selector::parse("h1").unwrap())
        .next()
        .or_else(|| {
            document
                .select(&Selector::parse("#aueditable > h2").unwrap())
                .next()
        })
        .expect("Document lacks title")
        .text()
        .next()
        .expect("Title is empty")
        .trim()
        .to_string();
    let preamble = document
        .select(&Selector::parse("#preamble > *").unwrap())
        .map(|el| Content::from_element(el, &|_| false))
        .collect::<Vec<_>>();
    let mut main_els = document
        .select(&Selector::parse("#main-text").unwrap())
        .next()
        .or_else(|| {
            document // becuase why would subarticles have main-text 🙄
                .select(&Selector::parse("#aueditable").unwrap())
                .next()
        })
        .expect("Document lacks main text")
        .child_elements()
        .peekable();
    let mut main_text = Vec::new();
    while main_els.peek().is_some() {
        main_text.push(Section::from_iter(&mut main_els));
        while main_els
            .peek()
            .is_some_and(|el| SKIP_ELEMENTS.contains(&el.value().name()))
        {
            main_els.next().unwrap();
        }
    }

    let html = reqwest::blocking::get(format!("{}/notes.html", url.rsplit_once('/').unwrap().0))
        .expect("Failed to request notes HTML")
        .text()
        .unwrap();
    let document = Html::parse_document(&html);
    let mut notes = Vec::new();
    for el in document.select(&Selector::parse("#aueditable > div").unwrap()) {
        notes.push(
            el.child_elements()
                .map(|el| {
                    Content::from_element(el, &|node| {
                        node.as_element().is_some_and(|el| el.name() == "a")
                    })
                })
                .collect::<Vec<_>>(),
        );
    }

    let article = Article {
        title,
        main_text,
        preamble,
        notes,
    };

    article
        .write_to_file(outname)
        .expect("Failed to write output");
}

#[derive(Debug)]
struct Article {
    title: String,
    preamble: Vec<Content>,
    main_text: Vec<Section>,
    notes: Vec<Vec<Content>>,
}

impl Article {
    fn write_to_file<P: AsRef<Path>>(self, path: P) -> io::Result<()> {
        let mut f = File::create(path)?;

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

#[derive(Debug)]
struct Section {
    id: Option<String>,
    title: String,
    level: u8,
    text: Vec<Content>,
}

impl Section {
    fn from_iter<'a, I: Iterator<Item = ElementRef<'a>>>(iter: &mut Peekable<I>) -> Self {
        let head = iter.next().unwrap();
        let id = head.value().id().map(String::from);
        let title = markdownify(head, &|_| false);
        let level = head.value().name()[1..].parse().unwrap();
        let mut text = Vec::new();

        while iter
            .peek()
            .is_some_and(|el| !el.value().name().starts_with('h'))
        {
            text.push(Content::from_element(iter.next().unwrap(), &|_| false));
        }

        Self {
            id,
            title,
            level,
            text,
        }
    }

    fn markdown(self) -> String {
        format!(
            "{} {}{}\n\n{}",
            "#".repeat(self.level as usize),
            self.title,
            self.id
                .map(|id| format!(" {{#{}}}", id))
                .unwrap_or_default(),
            self.text
                .into_iter()
                .map(Content::markdown)
                .collect::<String>()
        )
    }
}

#[derive(Debug)]
enum Content {
    Paragraph(String),
    Blockquote(String),
    OrderedList(Vec<(Option<String>, String)>, usize),
    DefinitionList(Vec<(String, String)>),
    Empty,
}

impl Content {
    fn from_element<'a, F: Fn(&Node) -> bool>(el: ElementRef<'a>, markdownify_ignore: &F) -> Self {
        match el.value().name() {
            "blockquote" => {
                Self::Blockquote(markdownify(el, markdownify_ignore).trim().to_string())
            }
            "p" => Self::Paragraph(markdownify(el, markdownify_ignore).trim().to_string()),
            "ol" => Self::OrderedList(
                el.child_elements()
                    .map(|el| {
                        (
                            el.value().id().map(|id| format!("<a id=\"{}\"></a>", id)),
                            markdownify(el, markdownify_ignore).trim().to_string(),
                        )
                    })
                    .collect(),
                el.attr("start").and_then(|s| s.parse().ok()).unwrap_or(1),
            ),
            "dl" => Self::DefinitionList(
                el.child_elements()
                    .tuples()
                    .map(|(dt, dd)| {
                        (
                            markdownify(dt, markdownify_ignore).trim().to_string(),
                            markdownify(dd, markdownify_ignore).trim().to_string(),
                        )
                    })
                    .collect(),
            ),
            "ul" => {
                eprintln!(
                    "Encountered a `ul`. Is this a table of contents?\n{}",
                    el.html()
                );
                Self::Empty
            }
            other => {
                eprintln!("Unexpected content tag: {}. Ignoring...", other);
                Self::Empty
            }
        }
    }

    fn markdown(self) -> String {
        match self {
            Self::Paragraph(p) => p + "\n\n",
            Self::Blockquote(b) => {
                b.lines()
                    .map(|l| String::from("> ") + l)
                    .collect::<String>()
                    + "\n\n"
            }
            Self::OrderedList(l, start) => {
                l.into_iter()
                    .enumerate()
                    .map(|(i, (anchor, str))| {
                        format!("{}. {} {}\n", start + i, anchor.unwrap_or_default(), str)
                    })
                    .collect::<String>()
                    + "\n\n"
            }
            Self::DefinitionList(d) => d
                .iter()
                .map(|(dt, dd)| format!("{}\n\n: {}\n\n", dt, dd))
                .collect::<String>(),
            Self::Empty => String::new(),
        }
    }
}

fn markdownify<F: Fn(&Node) -> bool>(el: ElementRef<'_>, ignore: &F) -> String {
    let mut result = String::new();
    for child in el.children() {
        let value = child.value();
        if ignore(value) {
            continue;
        }
        if value.is_text() {
            result += value.as_text().unwrap();
        } else if value.is_element() {
            let el = value.as_element().unwrap();
            match el.name() {
                "em" => {
                    result.push('*');
                    result += &markdownify(ElementRef::wrap(child).unwrap(), ignore);
                    result.push('*');
                }
                "a" => {
                    let mut href = el.attr("href").unwrap().to_string();
                    if !href.contains('/') && !href.contains('#') {
                        // probably a link to a subarticle
                        href = format!("{}/{}", BASE_URL.get().unwrap(), href);
                    }
                    result += &format!(
                        "[{}]({})",
                        ElementRef::wrap(child).unwrap().text().next().unwrap(),
                        href
                    )
                }
                "sup" => {
                    let a = ElementRef::wrap(child).unwrap().child_elements().next();
                    if let Some(a) = a {
                        result += &format!(
                            "[^{}]",
                            a.value().id().unwrap().strip_prefix("ref-").unwrap()
                        )
                    } else {
                        result += &format!(
                            "<sup>{}</sup>",
                            ElementRef::wrap(child).unwrap().text().next().unwrap()
                        )
                    }
                }
                "p" => result += &markdownify(ElementRef::wrap(child).unwrap(), ignore),
                other => eprintln!("Unexpected tag: {}", other),
            }
        }
    }
    result.replace('\n', " ")
}
