use std::{fs, io::Write};

use scraper::{ElementRef, Selector};

use crate::config::Config;

use super::Content;
use super::inline::markdownify;

const FIGURE_CLASSES: [&str; 3] = ["figure", "figureright", "inner-fig"];

pub fn handle_paragraph<F: Fn(&scraper::Node) -> bool>(
    el: ElementRef,
    ignore: &F,
    config: &Config,
) -> Content {
    Content::Paragraph(markdownify(el, ignore, config).trim().to_string())
}

pub fn handle_blockquote<F: Fn(&scraper::Node) -> bool>(
    el: ElementRef,
    ignore: &F,
    config: &Config,
) -> Content {
    Content::Blockquote(markdownify(el, ignore, config).trim().to_string())
}

pub fn render_unordered_list<F: Fn(&scraper::Node) -> bool>(
    el: ElementRef,
    ignore: &F,
    config: &Config,
    indent: usize,
) -> String {
    let prefix = "  ".repeat(indent);
    let mut out = String::new();
    for li in el.child_elements() {
        let anchor = li
            .value()
            .id()
            .map(|id| format!("<a id=\"{id}\"></a>"))
            .unwrap_or_default();
        // Collect nested lists separately
        let nested_lists: Vec<String> = li
            .child_elements()
            .filter_map(|child| match child.value().name() {
                "ul" => Some(render_unordered_list(child, ignore, config, indent + 2)),
                "ol" => Some(render_ordered_list(child, ignore, config, indent + 2)),
                _ => None,
            })
            .collect();
        // Process inline content, ignoring nested list elements
        let ignore_lists = |node: &scraper::Node| -> bool {
            if ignore(node) {
                return true;
            }
            node.as_element()
                .is_some_and(|e| matches!(e.name(), "ul" | "ol"))
        };
        let text = markdownify(li, &ignore_lists, config).trim().to_string();
        let text = if anchor.is_empty() {
            text
        } else {
            format!("{anchor} {text}")
        };
        out.push_str(&format!("{prefix}- {text}\n"));
        for nested in nested_lists {
            out.push_str(&nested);
        }
    }
    out
}

pub fn render_ordered_list<F: Fn(&scraper::Node) -> bool>(
    el: ElementRef,
    ignore: &F,
    config: &Config,
    indent: usize,
) -> String {
    let prefix = "  ".repeat(indent);
    let start: usize = el.attr("start").and_then(|s| s.parse().ok()).unwrap_or(1);
    let mut out = String::new();
    for (i, li) in el.child_elements().enumerate() {
        let anchor = li
            .value()
            .id()
            .map(|id| format!("<a id=\"{id}\"></a>"))
            .unwrap_or_default();
        // Collect nested lists separately
        let nested_lists: Vec<String> = li
            .child_elements()
            .filter_map(|child| match child.value().name() {
                "ul" => Some(render_unordered_list(child, ignore, config, indent + 2)),
                "ol" => Some(render_ordered_list(child, ignore, config, indent + 2)),
                _ => None,
            })
            .collect();
        // Process inline content, ignoring nested list elements
        let ignore_lists = |node: &scraper::Node| -> bool {
            if ignore(node) {
                return true;
            }
            node.as_element()
                .is_some_and(|e| matches!(e.name(), "ul" | "ol"))
        };
        let text = markdownify(li, &ignore_lists, config).trim().to_string();
        let text = if anchor.is_empty() {
            text
        } else {
            format!("{anchor} {text}")
        };
        out.push_str(&format!("{prefix}{}. {text}\n", start + i));
        for nested in nested_lists {
            out.push_str(&nested);
        }
    }
    out
}

pub fn handle_unordered_list<F: Fn(&scraper::Node) -> bool>(
    el: ElementRef,
    ignore: &F,
    config: &Config,
) -> Content {
    Content::List(render_unordered_list(el, ignore, config, 0))
}

pub fn handle_ordered_list<F: Fn(&scraper::Node) -> bool>(
    el: ElementRef,
    ignore: &F,
    config: &Config,
) -> Content {
    Content::List(render_ordered_list(el, ignore, config, 0))
}

pub fn handle_definition_list<F: Fn(&scraper::Node) -> bool>(
    el: ElementRef,
    ignore: &F,
    config: &Config,
) -> Content {
    let mut pairs = Vec::new();
    let mut children = el.child_elements();
    while let Some(dt) = children.next() {
        if let Some(dd) = children.next() {
            let term = markdownify(dt, ignore, config).trim().to_string();
            let def = flatten_dd(dd, ignore, config);
            pairs.push((term, def));
        }
    }
    Content::DefinitionList(pairs)
}

fn flatten_dd<F: Fn(&scraper::Node) -> bool>(
    el: ElementRef,
    ignore: &F,
    config: &Config,
) -> String {
    let mut out = String::new();
    let mut inline = String::new();
    for child in el.children() {
        match child.value() {
            scraper::Node::Text(t) => inline.push_str(t),
            scraper::Node::Element(e) => {
                if matches!(e.name(), "ul" | "ol" | "dl" | "table" | "blockquote") {
                    if !inline.is_empty() {
                        out.push_str(&inline);
                        inline.clear();
                    }
                    let child_el = ElementRef::wrap(child).unwrap();
                    for c in Content::flatten_element(child_el, ignore, config) {
                        out.push_str(&c.markdown());
                    }
                } else {
                    inline.push_str(&markdownify(
                        ElementRef::wrap(child).unwrap(),
                        ignore,
                        config,
                    ));
                }
            }
            _ => {}
        }
    }
    out.push_str(&inline);
    out.trim().to_string()
}

pub fn handle_table<F: Fn(&scraper::Node) -> bool>(
    el: ElementRef,
    ignore: &F,
    config: &Config,
) -> Content {
    let caption = el
        .select(&Selector::parse("caption").unwrap())
        .next()
        .map(|c| markdownify(c, ignore, config).trim().to_string());
    let rows: Vec<Vec<String>> = el
        .select(&Selector::parse("tr").unwrap())
        .map(|tr| {
            tr.select(&Selector::parse("td").unwrap())
                .map(|td| markdownify(td, ignore, config).trim().to_string())
                .collect()
        })
        .filter(|row: &Vec<String>| !row.is_empty() || row.len() > 1)
        .collect();
    Content::Table { caption, rows }
}

fn handle_figure<F: Fn(&scraper::Node) -> bool>(
    el: ElementRef,
    ignore: &F,
    config: &Config,
) -> Content {
    let img = el
        .select(&Selector::parse("img").unwrap())
        .next()
        .expect("Figure lacks image");
    let path = img.value().attr("src").unwrap().to_string();
    let caption = el
        .select(&Selector::parse("p").unwrap())
        .next()
        .map(|p| markdownify(p, ignore, config).trim().to_string())
        .unwrap_or_else(|| img.value().attr("alt").unwrap_or("").to_string());
    eprintln!("Downloading figure to: figures/{path}");
    if !matches!(fs::exists("figures"), Ok(true)) {
        fs::create_dir("figures").expect("Failed to create figures directory");
    }
    std::fs::File::create(format!("figures/{path}"))
        .expect("Failed to create figure file")
        .write_all(
            &reqwest::blocking::get(format!("{}/{}", config.base_url, path))
                .expect("Failed to download figure")
                .bytes()
                .expect("Failed to read figure bytes"),
        )
        .expect("Failed to write figure file");
    Content::Figure { caption, path }
}

pub fn flatten_div<F: Fn(&scraper::Node) -> bool>(
    el: ElementRef,
    ignore: &F,
    config: &Config,
) -> Vec<Content> {
    if el
        .value()
        .classes()
        .find(|c| FIGURE_CLASSES.contains(c))
        .is_some()
    {
        return vec![handle_figure(el, ignore, config)];
    }
    el.child_elements()
        .flat_map(|child| Content::flatten_element(child, ignore, config))
        .collect()
}
