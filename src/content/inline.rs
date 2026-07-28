use scraper::{ElementRef, Node};

use crate::config::Config;

type InlineHandler = fn(ElementRef, &mut String, &Config);

fn emphasis_handler(el: ElementRef, out: &mut String, config: &Config) {
    out.push('*');
    out.push_str(&markdownify(el, &|_| false, config));
    out.push('*');
}

fn link_handler(el: ElementRef, out: &mut String, config: &Config) {
    let a = el.value();
    let mut href = a.attr("href").unwrap().to_string();
    if !href.contains('/') && !href.contains('#') {
        href = format!("{}/{}", config.base_url, href);
    }
    let text = el.text().next().unwrap_or("");
    out.push_str(&format!("[{text}]({href})"));
}

fn superscript_handler(el: ElementRef, out: &mut String, _config: &Config) {
    let a = el.child_elements().next();
    if let Some(a) = a {
        let id = a.value().id().unwrap().strip_prefix("ref-").unwrap();
        out.push_str(&format!("[^{id}]"));
    } else {
        let text = el.text().next().unwrap_or("");
        out.push_str(&format!("<sup>{text}</sup>"));
    }
}

fn inline_p_handler(el: ElementRef, out: &mut String, config: &Config) {
    out.push_str(&markdownify(el, &|_| false, config));
}

fn strong_handler(el: ElementRef, out: &mut String, config: &Config) {
    let inner = markdownify(el, &|_| false, config);
    out.push_str(&format!("**{inner}**"));
}

fn sub_handler(el: ElementRef, out: &mut String, config: &Config) {
    let inner = markdownify(el, &|_| false, config);
    out.push_str(&format!("<sub>{inner}</sub>"));
}

fn nested_ul_handler(el: ElementRef, out: &mut String, config: &Config) {
    let md = super::block::render_unordered_list(el, &|_| false, config, 0);
    let indented: String = md
        .lines()
        .map(|line| format!("  {line}"))
        .collect::<Vec<_>>()
        .join("\n");
    out.push_str("\n\x00BLOCK_START\x00");
    out.push_str(&indented);
    out.push_str("\x00BLOCK_END\x00");
}

fn nested_ol_handler(el: ElementRef, out: &mut String, config: &Config) {
    let md = super::block::render_ordered_list(el, &|_| false, config, 0);
    let indented: String = md
        .lines()
        .map(|line| format!("  {line}"))
        .collect::<Vec<_>>()
        .join("\n");
    out.push_str("\n\x00BLOCK_START\x00");
    out.push_str(&indented);
    out.push_str("\x00BLOCK_END\x00");
}

fn inline_handlers() -> Vec<(&'static str, InlineHandler)> {
    vec![
        ("em", emphasis_handler as InlineHandler),
        ("i", emphasis_handler as InlineHandler),
        ("a", link_handler as InlineHandler),
        ("sup", superscript_handler as InlineHandler),
        ("sub", sub_handler as InlineHandler),
        ("p", inline_p_handler as InlineHandler),
        ("strong", strong_handler as InlineHandler),
        ("b", strong_handler as InlineHandler),
        ("ul", nested_ul_handler as InlineHandler),
        ("ol", nested_ol_handler as InlineHandler),
    ]
}

pub fn markdownify<F: Fn(&Node) -> bool>(
    el: ElementRef<'_>,
    ignore: &F,
    config: &Config,
) -> String {
    let handlers = inline_handlers();
    let mut result = String::new();
    for child in el.children() {
        let value = child.value();
        if ignore(value) {
            continue;
        }
        if value.is_text() {
            result += value.as_text().unwrap();
        } else if value.is_element() {
            let child_el = value.as_element().unwrap();
            if let Some((_, handler)) = handlers.iter().find(|(tag, _)| *tag == child_el.name()) {
                handler(ElementRef::wrap(child).unwrap(), &mut result, config);
            } else {
                eprintln!("Unexpected inline tag: {}", child_el.name());
            }
        }
    }
    // Protect block-level content from newline collapsing
    let mut blocks: Vec<String> = Vec::new();
    let mut i = 0;
    while let Some(start) = result[i..].find("\x00BLOCK_START\x00") {
        let abs_start = i + start;
        if let Some(end) = result[abs_start..].find("\x00BLOCK_END\x00") {
            let content_start = abs_start + "\x00BLOCK_START\x00".len();
            let content_end = abs_start + end;
            let block = result[content_start..content_end].to_string();
            let placeholder = format!("\x00BLOCK{}\x00", blocks.len());
            blocks.push(block);
            result.replace_range(abs_start..content_end + "\x00BLOCK_END\x00".len(), &placeholder);
            i = abs_start + placeholder.len();
        } else {
            break;
        }
    }
    result = result.replace('\n', " ");
    for (idx, block) in blocks.into_iter().enumerate() {
        let placeholder = format!("\x00BLOCK{idx}\x00");
        result = result.replace(&placeholder, &block);
    }
    result
}
