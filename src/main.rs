use std::{
    env::args,
    fs::File,
    io::{self, Write},
    iter::Peekable,
    path::Path,
};

use itertools::Itertools;
use scraper::{ElementRef, Html, Node, Selector};

fn main() {
    let mut args = args();
    let mut url = args.nth(1).expect("SEP page URL is required");
    if !url.ends_with('/') {
        url.push('/');
    }
    let outname = args
        .next()
        .unwrap_or_else(|| format!("{}.md", url.rsplit('/').nth(1).unwrap()));
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
        .expect("Document lacks main text")
        .child_elements()
        .peekable();
    let mut main_text = Vec::new();
    while main_els.peek().is_some() {
        main_text.push(Section::from_iter(&mut main_els));
    }

    let html = reqwest::blocking::get(format!("{}notes.html", url))
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
                i,
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
    id: String,
    title: String,
    level: u8,
    text: Vec<Content>,
}

impl Section {
    fn from_iter<'a, I: Iterator<Item = ElementRef<'a>>>(iter: &mut Peekable<I>) -> Self {
        let head = iter.next().unwrap();
        let id = head.value().id().unwrap().to_string();
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
            "{} {} {{#{}}}\n\n{}",
            "#".repeat(self.level as usize),
            self.title,
            self.id,
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
    OrderedList(Vec<String>),
    DefinitionList(Vec<(String, String)>),
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
                    .map(|el| markdownify(el, markdownify_ignore).trim().to_string())
                    .collect(),
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
            other => panic!("Unexpected content tag: {}", other),
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
            Self::OrderedList(l) => {
                l.iter()
                    .enumerate()
                    .map(|(i, str)| format!("{}. {}\n", i, str))
                    .collect::<String>()
                    + "\n\n"
            }
            Self::DefinitionList(d) => d
                .iter()
                .map(|(dt, dd)| format!("{}\n\n: {}\n\n", dt, dd))
                .collect::<String>(),
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
                    let href = el.attr("href").unwrap();
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
