//! An example of a program that holds two thunderbox panels.
//!
//! The program owns its data, its terminal, and its event loop. It borrows two
//! thunderbox renderers, and nothing else. The two renderers read the same
//! frames, and each one draws its own picture.
//!
//! Note what this example never does. It never builds a thunderbox data source, a
//! thunderbox runtime, or a thunderbox dashboard. thunderbox never touches the
//! terminal. ADR 0004 in `docs/contributor/adr` states this promise.
//!
//! Run the example with this command:
//!
//!     cargo run -p thunderbox-widgets --example panel_embed

use std::io;
use std::time::{Duration, Instant};

use ratatui::crossterm::event::{self, Event, KeyCode};
use ratatui::layout::{Constraint, Layout};
use ratatui::widgets::{Block, Paragraph};
use thunderbox_core::Threshold;
use thunderbox_core::{Field, Frame};
use thunderbox_widgets::{Marker, PanelData, PanelOptions, StatPanel, Theme, TimeSeriesPanel};

/// The data of the program, in the form that the program uses.
struct HostMetrics {
    started: Instant,
    samples: Vec<(i64, f64)>,
}

impl HostMetrics {
    fn new() -> Self {
        Self {
            started: Instant::now(),
            samples: Vec::new(),
        }
    }

    fn tick(&mut self) {
        let t = self.started.elapsed().as_millis() as i64;
        let phase = t as f64 / 700.0;
        let v = 50.0 + 35.0 * phase.sin() + 8.0 * (phase * 3.1).sin();
        self.samples.push((t, v));
        if self.samples.len() > 600 {
            self.samples.remove(0);
        }
    }

    /// Returns the data of the program as a thunderbox frame.
    ///
    /// This method is the whole connection between the program and thunderbox.
    fn to_frames(&self) -> Vec<Frame> {
        let (times, values): (Vec<i64>, Vec<Option<f64>>) =
            self.samples.iter().map(|(t, v)| (*t, Some(*v))).unzip();
        vec![Frame::named(
            "host",
            vec![Field::time("time", times), Field::f64("throughput", values)],
        )]
    }
}

fn main() -> io::Result<()> {
    let mut terminal = ratatui::init();
    let theme = Theme::default();
    // The stat panel turns red above this value. A time series panel reads the
    // same list, and a later version can colour a line with it.
    let thresholds = [Threshold {
        value: 80.0,
        color: "critical".into(),
    }];
    let mut metrics = HostMetrics::new();
    let mut marker = Marker::Braille;
    let result = loop {
        metrics.tick();
        let frames = metrics.to_frames();

        if let Err(e) = terminal.draw(|f| {
            let [host_area, panel_area, help_area] = Layout::vertical([
                Constraint::Length(3),
                Constraint::Min(6),
                Constraint::Length(1),
            ])
            .areas(f.area());
            let [chart_area, stat_area] =
                Layout::horizontal([Constraint::Min(20), Constraint::Length(20)]).areas(panel_area);

            // The program draws its own border and its own text here.
            f.render_widget(
                Paragraph::new("The program owns this border and this text.")
                    .block(Block::bordered().title("the program")),
                host_area,
            );

            // The program draws two thunderbox panels here. The settings say how
            // to show a number, and both panels read them.
            let options = PanelOptions {
                unit: Some("bytes/s"),
                thresholds: &thresholds,
                ..Default::default()
            };
            f.render_widget(
                TimeSeriesPanel::new("throughput", PanelData::Frames(&frames), &theme)
                    .marker(marker)
                    .options(options),
                chart_area,
            );
            f.render_widget(
                StatPanel::new("now", PanelData::Frames(&frames), &theme)
                    .marker(marker)
                    .options(options),
                stat_area,
            );

            f.render_widget(Paragraph::new("q quit    m change the marker"), help_area);
        }) {
            break Err(e);
        }

        if event::poll(Duration::from_millis(80))?
            && let Event::Key(k) = event::read()?
        {
            match k.code {
                KeyCode::Char('q') | KeyCode::Esc => break Ok(()),
                KeyCode::Char('m') => {
                    marker = match marker {
                        Marker::Braille => Marker::Octant,
                        Marker::Octant => Marker::HalfBlock,
                        Marker::HalfBlock => Marker::Block,
                        Marker::Block => Marker::Braille,
                    }
                }
                _ => {}
            }
        }
    };
    ratatui::restore();
    result
}
