use anyhow::Result;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Cell, Paragraph, Row, Table, Tabs},
};
use std::io::stdout;

use crossbeam_channel::{unbounded, Sender};
use std::time::{Duration, Instant};

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use sockcan::prelude::*; // <-- ajoute ceci

// ---- données métier pour la table ----
#[derive(Clone, Debug)]
struct CanRow {
    ts: String,
    iface: String,
    id: String, // ex: "118" ou "1DF9050F"
    dlc: u8,
    data: String, // "05 FF 7F 01 ..."
}

fn bytes_to_hex_spaced(data: &[u8]) -> String {
    data.iter().map(|b| format!("{:02X}", b)).collect::<Vec<_>>().join(" ")
}

// ---- état de l'app ----
struct App {
    frames: Vec<CanRow>,
    selected_tab: usize,
    last_tick: Instant,
}

impl App {
    fn new() -> Self {
        Self { frames: Vec::with_capacity(128), selected_tab: 0, last_tick: Instant::now() }
    }

    fn push_frame(&mut self, row: CanRow) {
        self.frames.push(row);
        if self.frames.len() > 5000 {
            let drop = self.frames.len() - 5000;
            self.frames.drain(0..drop);
        }
    }
}

// ---- boucle principale ----
fn main() -> Result<()> {
    // flag d’arrêt (Ctrl-C)
    let stop = Arc::new(AtomicBool::new(false));
    {
        let stop = stop.clone();
        ctrlc::set_handler(move || {
            stop.store(true, Ordering::SeqCst);
        })
        .expect("failed to set Ctrl-C handler");
    }

    // logs
    let _ = env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_timestamp_millis()
        .try_init();

    // canal pour recevoir des frames
    let (tx, rx) = unbounded::<CanRow>();

    // interface (ou ta CLI)
    let iface = std::env::args().nth(1).unwrap_or_else(|| "vcan0".to_string());

    // lancer le lecteur CAN (pas de `?`)
    spawn_can_reader(tx.clone(), iface, stop.clone());

    // init terminal
    enable_raw_mode()?;
    let mut out = stdout();
    execute!(out, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(out);
    let mut terminal = Terminal::new(backend)?;

    // guard de restauration terminal (même en cas de panic/erreur)
    struct TermGuard;
    impl Drop for TermGuard {
        fn drop(&mut self) {
            let _ = disable_raw_mode();
            // on ne peut pas récupérer terminal ici, mais on peut encore envoyer les séquences CSI
            let mut out = stdout();
            let _ = execute!(out, LeaveAlternateScreen, DisableMouseCapture);
        }
    }
    let _guard = TermGuard;

    // app
    let mut app = App::new();

    // event loop
    let tick_rate = Duration::from_millis(100);
    loop {
        if stop.load(Ordering::SeqCst) {
            break;
        }
        // 1) dessine
        terminal.draw(|f| ui(f, &app))?;

        // 2) traite messages entrants (non bloquant)
        for _ in 0..256 {
            match rx.try_recv() {
                Ok(row) => app.push_frame(row),
                Err(crossbeam_channel::TryRecvError::Empty) => break,
                Err(_) => break,
            }
        }

        // 3) clavier / tick
        let timeout =
            tick_rate.checked_sub(app.last_tick.elapsed()).unwrap_or(Duration::from_secs(0));

        if event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => break,
                    // optionnel : traiter Ctrl-C via clavier (en plus du signal)
                    KeyCode::Char('c')
                        if key.modifiers.contains(crossterm::event::KeyModifiers::CONTROL) =>
                    {
                        break
                    },
                    KeyCode::Left => app.selected_tab = app.selected_tab.saturating_sub(1),
                    KeyCode::Right => app.selected_tab = (app.selected_tab + 1).min(2),
                    _ => {},
                }
            }
        }

        if app.last_tick.elapsed() >= tick_rate {
            app.last_tick = Instant::now();
        }
    }

    // restore terminal
    disable_raw_mode()?;
    let backend = terminal.backend_mut();
    execute!(backend, LeaveAlternateScreen, DisableMouseCapture)?;
    Ok(())
}

// ---- rendu UI ----
fn ui(f: &mut ratatui::Frame, app: &App) {
    // layout principal: [tabs] en haut, table au centre, status en bas
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(3), Constraint::Length(1)])
        .split(f.area());

    // Tabs
    let titles = ["Frames", "Stats", "Settings"].iter().map(|t| Line::from(*t));
    let tabs = Tabs::new(titles)
        .block(Block::default().borders(Borders::ALL).title("bms-view"))
        .select(app.selected_tab)
        .highlight_style(Style::default().add_modifier(Modifier::BOLD));
    f.render_widget(tabs, chunks[0]);

    // Table des frames
    let header = Row::new([
        Cell::from("TS"),
        Cell::from("IFACE"),
        Cell::from("ID"),
        Cell::from("DLC"),
        Cell::from("DATA"),
    ])
    .style(Style::default().add_modifier(Modifier::BOLD));

    // affiche les dernières lignes (évite d’imprimer 5000 lignes si la fenêtre est petite)
    let height = chunks[1].height.saturating_sub(3) as usize; // - header/borders
    let start = app.frames.len().saturating_sub(height);
    let rows = app.frames[start..].iter().map(|r| {
        Row::new([
            Cell::from(r.ts.as_str()),
            Cell::from(r.iface.as_str()),
            Cell::from(r.id.as_str()),
            Cell::from(format!("{}", r.dlc)),
            Cell::from(r.data.as_str()),
        ])
    });

    let table = Table::new(
        rows,
        [
            Constraint::Length(18),
            Constraint::Length(8),
            Constraint::Length(12),
            Constraint::Length(5),
            Constraint::Percentage(100),
        ],
    )
    .header(header)
    .block(Block::default().title("CAN frames").borders(Borders::ALL))
    .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
    .column_spacing(1);

    f.render_widget(table, chunks[1]);

    // barre d’état
    let status = Line::from(format!(
        "q:quit  ←/→:tabs   frames:{}   now:{}",
        app.frames.len(),
        chrono::Local::now().format("%H:%M:%S")
    ));
    let p = Paragraph::new(status).block(Block::default().borders(Borders::ALL));
    f.render_widget(p, chunks[2]);
}

fn spawn_can_reader(tx: Sender<CanRow>, iface: String, stop: Arc<AtomicBool>) {
    std::thread::spawn(move || {
        // OUVERTURE DANS LE THREAD → pas besoin que SockCanHandle soit Send
        let sock = match SockCanHandle::open_raw(iface.as_str(), CanTimeStamp::CLASSIC) {
            Ok(s) => s,
            Err(e) => {
                log::error!("open_raw({}): {}", iface, e);
                return;
            },
        };

        // Optionnel: non-bloquant / timeouts
        // let _ = sock.set_blocking(false);
        // let _ = sock.set_timeout(500, 500);

        loop {
            if stop.load(Ordering::SeqCst) {
                break;
            }

            let msg = sock.get_can_frame();

            let iface_idx = msg.get_iface();
            // let stamp = msg.get_stamp(); // utile si tu veux l’afficher
            let (id, dlc, data) = match msg.get_raw() {
                CanAnyFrame::RawStd(f) => {
                    (f.get_id(), f.get_len(), &f.get_data()[..usize::from(f.get_len())])
                },
                CanAnyFrame::RawFd(f) => {
                    (f.get_id(), f.get_len(), &f.get_data()[..usize::from(f.get_len())])
                },
                CanAnyFrame::Err(e) => {
                    log::warn!("CAN read error: {}", e);
                    continue;
                },
                CanAnyFrame::None(_canid) => {
                    // timeout/aucune trame
                    continue;
                },
            };
            let ifname = sock.get_ifname(iface_idx).unwrap_or_else(|_| iface_idx.to_string());

            let row = CanRow {
                ts: format!("{}", chrono::Local::now().format("%H:%M:%S%.3f")),
                // si tu veux le nom d’interface au lieu de l’index, remplace ci-dessous :
                // let ifname = sock.get_ifname(iface_idx).unwrap_or_else(|_| iface_idx.to_string());
                // iface: ifname,
                iface: ifname,
                id: format!("{:X}", id),
                dlc,
                data: bytes_to_hex_spaced(data),
            };

            if tx.send(row).is_err() {
                break; // UI fermée
            }
        }

        log::info!("can-reader: stopped");
    });
}
