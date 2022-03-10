use std::collections::HashSet;

use tui::style::{Color, Modifier, Style};
use tui::text::{Span, Spans};
use tui_tree_widget::TreeItem;

#[derive(Debug, Clone, PartialEq)]
pub struct TopicTreeEntry {
    pub topic: String,
    pub leaf: String,
    pub messages: usize,
    pub last_payload: Option<Vec<u8>>,
    pub topics_below: usize,
    pub messages_below: usize,
    pub entries_below: Vec<TopicTreeEntry>,
}

impl<'a> From<&'a TopicTreeEntry> for TreeItem<'a> {
    fn from(entry: &'a TopicTreeEntry) -> Self {
        let children = entry
            .entries_below
            .iter()
            .map(std::convert::Into::into)
            .collect::<Vec<_>>();

        let meta = entry.last_payload.as_ref().map_or_else(
            || {
                format!(
                    "({} topics, {} messages)",
                    entry.topics_below, entry.messages_below
                )
            },
            |payload| format!("= {}", crate::format::payload_as_utf8(payload.clone())),
        );

        let text = vec![Spans::from(vec![
            Span::styled(&entry.leaf, Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(" "),
            Span::styled(meta, Style::default().fg(Color::DarkGray)),
        ])];

        TreeItem::new(text, children)
    }
}

impl TopicTreeEntry {
    #[cfg(test)]
    /// Same examples as `MqttHistory::example`
    pub fn examples() -> Vec<Self> {
        vec![
            Self {
                topic: "foo".into(),
                leaf: "foo".into(),
                messages: 0,
                last_payload: None,
                topics_below: 2,
                messages_below: 2,
                entries_below: vec![
                    Self {
                        topic: "foo/bar".into(),
                        leaf: "bar".into(),
                        messages: 1,
                        last_payload: Some("D".into()),
                        entries_below: vec![],
                        topics_below: 0,
                        messages_below: 0,
                    },
                    Self {
                        topic: "foo/test".into(),
                        leaf: "test".into(),
                        messages: 1,
                        last_payload: Some("B".into()),
                        entries_below: vec![],
                        topics_below: 0,
                        messages_below: 0,
                    },
                ],
            },
            Self {
                topic: "test".into(),
                leaf: "test".into(),
                messages: 2,
                last_payload: Some("C".into()),
                topics_below: 0,
                messages_below: 0,
                entries_below: vec![],
            },
        ]
    }
}

pub fn get_visible<'a, I>(opened: &HashSet<String>, entries: I) -> Vec<&'a TopicTreeEntry>
where
    I: IntoIterator<Item = &'a TopicTreeEntry>,
{
    let mut result = Vec::new();

    for entry in entries {
        result.push(entry);
        if opened.contains(&entry.topic) {
            result.append(&mut get_visible(opened, &entry.entries_below));
        }
    }

    result
}

#[test]
fn visible_topics_none_open_works() {
    let topics = TopicTreeEntry::examples();
    let opened = HashSet::new();
    let visible = get_visible(&opened, &topics);
    let visible = visible.iter().map(|o| o.topic.clone()).collect::<Vec<_>>();
    assert_eq!(visible, ["foo", "test"]);
}

#[test]
fn visible_topics_some_open_works() {
    let topics = TopicTreeEntry::examples();
    let mut opened = HashSet::new();
    opened.insert("foo".to_string());
    let visible = get_visible(&opened, &topics);
    let visible = visible.iter().map(|o| o.topic.clone()).collect::<Vec<_>>();
    assert_eq!(visible, ["foo", "foo/bar", "foo/test", "test"]);
}