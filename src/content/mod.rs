pub mod block;
pub mod inline;

use scraper::ElementRef;

use crate::config::Config;

#[derive(Debug)]
pub enum Content {
    Paragraph(String),
    Blockquote(String),
    List(String),
    DefinitionList(Vec<(String, String)>),
    Table {
        caption: Option<String>,
        rows: Vec<Vec<String>>,
    },
    Figure { caption: String, path: String },
    Empty,
}

impl Content {
    pub fn from_element<F: Fn(&scraper::Node) -> bool>(
        el: ElementRef,
        ignore: &F,
        config: &Config,
    ) -> Self {
        match el.value().name() {
            "p" => block::handle_paragraph(el, ignore, config),
            "blockquote" => block::handle_blockquote(el, ignore, config),
            "ol" => block::handle_ordered_list(el, ignore, config),
            "dl" => block::handle_definition_list(el, ignore, config),
            "div" => block::handle_div(el, ignore, config),
            "ul" => block::handle_unordered_list(el, ignore, config),
            "table" => block::handle_table(el, ignore, config),
            other => {
                eprintln!("Unexpected content tag: {other}. Ignoring...");
                Self::Empty
            }
        }
    }

    pub fn markdown(self) -> String {
        match self {
            Self::Paragraph(p) => p + "\n\n",
            Self::Blockquote(b) => {
                b.lines()
                    .map(|l| format!("> {l}"))
                    .collect::<String>()
                    + "\n\n"
            }
            Self::List(s) => s + "\n\n",
            Self::DefinitionList(d) => d
                .iter()
                .map(|(dt, dd)| format!("{dt}\n\n: {dd}\n\n"))
                .collect::<String>(),
            Self::Table { caption, rows } => {
                let mut out = String::new();
                if let Some(c) = caption {
                    out.push_str(&format!("**{c}**\n\n"));
                }
                if rows.is_empty() {
                    return out;
                }
                let num_cols = rows.iter().map(|r| r.len()).max().unwrap_or(0);
                // Header row
                if let Some(first) = rows.first() {
                    out.push('|');
                    for cell in first {
                        out.push_str(&format!(" {cell} |"));
                    }
                    for _ in first.len()..num_cols {
                        out.push_str(" |");
                    }
                    out.push('\n');
                    // Separator
                    out.push('|');
                    for _ in 0..num_cols {
                        out.push_str(" --- |");
                    }
                    out.push('\n');
                }
                // Data rows
                for row in rows.iter().skip(1) {
                    out.push('|');
                    for cell in row {
                        out.push_str(&format!(" {cell} |"));
                    }
                    for _ in row.len()..num_cols {
                        out.push_str(" |");
                    }
                    out.push('\n');
                }
                out.push('\n');
                out
            }
            Self::Empty => String::new(),
            Self::Figure { caption, path } => format!("![{caption}](figures/{path})\n\n"),
        }
    }
}
