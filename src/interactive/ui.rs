use crate::format;
use crate::interactive::app::App;
use crate::mqtt_history::{self, HistoryEntry};
use crate::topic_view::{self, TopicTreeEntry};
use std::cmp::min;
use std::error::Error;
use tui::{
    backend::Backend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::Spans,
    widgets::{Block, Borders, List, ListItem, Paragraph, Wrap},
    Frame,
};
use tui_tree_widget::{Tree, TreeState};

mod history;

pub fn draw<B: Backend>(f: &mut Frame<B>, app: &mut App) -> Result<(), Box<dyn Error>> {
    let chunks = Layout::default()
        .constraints([Constraint::Length(2 + 3), Constraint::Min(8)].as_ref())
        .split(f.size());
    draw_info_header(f, chunks[0], app);
    draw_main(f, chunks[1], app)?;
    Ok(())
}

fn draw_info_header<B>(f: &mut Frame<B>, area: Rect, app: &App)
where
    B: Backend,
{
    let host = format!("MQTT Broker: {} (Port {})", app.host, app.port);
    let subscribed = format!("Subscribed Topic: {}", app.subscribe_topic);
    let mut text = vec![Spans::from(host), Spans::from(subscribed)];

    if let Some(topic) = &app.selected_topic {
        text.push(Spans::from(format!("Selected Topic: {}", topic)));
    }

    let title = format!("MQTT TUI {}", env!("CARGO_PKG_VERSION"));
    let block = Block::default().borders(Borders::ALL).title(title);
    let paragraph = Paragraph::new(text).block(block).wrap(Wrap { trim: true });
    f.render_widget(paragraph, area);
}

fn draw_main<B>(f: &mut Frame<B>, area: Rect, app: &mut App) -> Result<(), Box<dyn Error>>
where
    B: Backend,
{
    let history = &app
        .history
        .lock()
        .map_err(|err| format!("failed to aquire lock of mqtt history: {}", err))?;

    let topics = mqtt_history::history_to_tmlp(history.iter());
    let tree_items = topic_view::get_tmlp_as_tree(&topics);

    // Move opened_topics over to TreeState
    app.topic_overview_state.close_all();
    for topic in &app.opened_topics {
        app.topic_overview_state
            .open(topic_view::get_identifier_of_topic(&tree_items, topic).unwrap_or_default());
    }

    // Ensure selected topic is selected index
    app.topic_overview_state.select(
        app.selected_topic
            .as_ref()
            .and_then(|selected_topic| {
                topic_view::get_identifier_of_topic(&tree_items, selected_topic)
            })
            .unwrap_or_default(),
    );

    #[allow(clippy::option_if_let_else)]
    let overview_area = if let Some(topic_history) = app
        .selected_topic
        .as_ref()
        .and_then(|selected_topic| history.get(selected_topic))
    {
        let chunks = Layout::default()
            .constraints([Constraint::Percentage(35), Constraint::Percentage(65)].as_ref())
            .direction(Direction::Horizontal)
            .split(area);

        draw_details(f, chunks[1], topic_history);

        chunks[0]
    } else {
        area
    };

    draw_overview(
        f,
        overview_area,
        topics.len(),
        &tree_items,
        &mut app.topic_overview_state,
    );
    Ok(())
}

fn draw_overview<B>(
    f: &mut Frame<B>,
    area: Rect,
    topic_amount: usize,
    tree_items: &[TopicTreeEntry],
    state: &mut TreeState,
) where
    B: Backend,
{
    let title = format!("Topics ({})", topic_amount);

    let tree_items = topic_view::tree_items_from_tmlp_tree(&tree_items);

    let widget = Tree::new(tree_items)
        .block(Block::default().borders(Borders::ALL).title(title))
        .highlight_style(Style::default().fg(Color::Black).bg(Color::LightGreen));
    f.render_stateful_widget(widget, area, state);
}

fn draw_details<B>(f: &mut Frame<B>, area: Rect, topic_history: &[HistoryEntry])
where
    B: Backend,
{
    let last = topic_history.last().unwrap();
    let payload_length = last.packet.payload.len();
    let payload_json = format::payload_as_json(last.packet.payload.to_vec());

    let payload = payload_json.map_or(
        format::payload_as_utf8(last.packet.payload.to_vec()),
        |payload| json::stringify_pretty(payload, 2),
    );
    let lines = payload.matches('\n').count().saturating_add(1);

    let chunks = Layout::default()
        .constraints(
            [
                #[allow(clippy::cast_possible_truncation)]
                Constraint::Length(min(area.height as usize / 3, 2 + lines) as u16),
                Constraint::Min(16),
            ]
            .as_ref(),
        )
        .split(area);

    draw_payload(f, chunks[0], payload_length, &payload);
    history::draw(f, chunks[1], topic_history);
}

fn draw_payload<B>(f: &mut Frame<B>, area: Rect, bytes: usize, payload: &str)
where
    B: Backend,
{
    let title = format!("Payload (Bytes: {})", bytes);
    let items = payload.lines().map(ListItem::new).collect::<Vec<_>>();
    let widget = List::new(items)
        .block(Block::default().borders(Borders::ALL).title(title))
        .highlight_style(Style::default().fg(Color::Black).bg(Color::LightGreen));
    f.render_widget(widget, area);
}
