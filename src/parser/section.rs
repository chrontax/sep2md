use std::iter::Peekable;

use scraper::ElementRef;

use crate::config::Config;
use crate::content::Content;
use crate::content::inline::markdownify;

#[derive(Debug)]
pub struct Section {
    pub id: Option<String>,
    pub title: String,
    pub level: u8,
    pub text: Vec<Content>,
}

impl Section {
    pub fn from_iter<'a, I: Iterator<Item = ElementRef<'a>>>(
        iter: &mut Peekable<I>,
        config: &Config,
    ) -> Self {
        let head = iter.next().unwrap();
        let mut id = head.value().id().map(String::from);
        let title;

        if head.children().count() == 1
            && let Some(a) = head.child_elements().next()
            && a.value().name() == "a"
        {
            id = id.and_then(|_| {
                a.value()
                    .attr("name")
                    .and_then(|_| a.value().id())
                    .map(String::from)
            });
            title = markdownify(a, &|_| false, config);
        } else {
            title = markdownify(head, &|_| false, config);
        }
        let level = head.value().name()[1..].parse().unwrap();
        let mut text = Vec::new();

        while iter
            .peek()
            .is_some_and(|el| !el.value().name().starts_with('h'))
        {
            text.push(Content::from_element(iter.next().unwrap(), &|_| false, config));
        }

        Self {
            id,
            title,
            level,
            text,
        }
    }

    pub fn markdown(self) -> String {
        format!(
            "{} {}{}\n\n{}",
            "#".repeat(self.level as usize),
            self.title,
            self.id
                .map(|id| format!(" {{#{id}}}"))
                .unwrap_or_default(),
            self.text
                .into_iter()
                .map(Content::markdown)
                .collect::<String>()
        )
    }
}
