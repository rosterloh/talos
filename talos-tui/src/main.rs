mod client;
mod input;
mod state;
mod ui;

use std::io;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crossterm::event::{self, Event, KeyEventKind};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use talos_common::protocol::messages::Request;
use tokio::sync::mpsc;

use state::AppState;

/// Parse `--socket <path>` and `--remote <addr:port>` from argv.
fn parse_args(args: &[String]) -> (Option<String>, Option<String>) {
    let mut socket = None::<String>;
    let mut remote = None::<String>;
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--socket" | "-s" => {
                i += 1;
                if i < args.len() {
                    socket = Some(args[i].clone());
                }
            }
            "--remote" | "-r" => {
                i += 1;
                if i < args.len() {
                    remote = Some(args[i].clone());
                }
            }
            _ => {}
        }
        i += 1;
    }
    (socket, remote)
}

fn build_client_config(
    socket_path: Option<String>,
    remote_addr: Option<String>,
) -> client::ClientConfig {
    #[cfg(feature = "quic")]
    if let Some(addr) = remote_addr {
        return client::ClientConfig::Quic { addr };
    }
    #[cfg(not(feature = "quic"))]
    if remote_addr.is_some() {
        eprintln!("error: this build was compiled without QUIC support (--remote not available)");
        std::process::exit(1);
    }
    client::ClientConfig::Uds {
        socket_path: socket_path.unwrap_or_else(|| "/tmp/talos.sock".to_string()),
    }
}

fn main() -> io::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let (socket_path, remote_addr) = parse_args(&args);

    if socket_path.is_some() && remote_addr.is_some() {
        eprintln!("error: --socket and --remote are mutually exclusive");
        std::process::exit(1);
    }

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let state = Arc::new(Mutex::new(AppState::default()));
    let (cmd_tx, cmd_rx) = mpsc::unbounded_channel::<Request>();

    let client_config = build_client_config(socket_path, remote_addr);

    // Spawn IPC client on tokio runtime
    let rt = tokio::runtime::Runtime::new().unwrap();
    let client_state = Arc::clone(&state);
    rt.spawn(async move {
        client::run(client_config, client_state, cmd_rx).await;
    });

    let result = run_app(&mut terminal, &state, &cmd_tx);

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    if let Err(e) = result {
        eprintln!("Error: {e}");
    }

    Ok(())
}

fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    state: &Arc<Mutex<AppState>>,
    cmd_tx: &mpsc::UnboundedSender<Request>,
) -> io::Result<()> {
    let tick_rate = Duration::from_millis(50); // ~20 FPS

    loop {
        {
            let s = state.lock().unwrap();
            terminal.draw(|f| ui::draw(f, &s))?;
        }

        if event::poll(tick_rate)?
            && let Event::Key(key) = event::read()?
        {
            if key.kind != KeyEventKind::Press {
                continue;
            }

            let mut s = state.lock().unwrap();
            if input::handle_key_event(&mut s, cmd_tx, key) == input::AppAction::Quit {
                return Ok(());
            }
        }
    }
}
