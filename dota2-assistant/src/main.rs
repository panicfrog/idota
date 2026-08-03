//! dota2-assistant: locate a Dota 2 installation and set up its Game State
//! Integration (GSI) configuration without manual editing.

mod discovery;
mod gsi_config;

use std::path::{Path, PathBuf};

use anyhow::Result;
use clap::{Parser, Subcommand};
use dota::components::GameState;
use dota::ServerBuilder;

#[derive(Parser, Debug)]
#[command(
    name = "dota2-assistant",
    version,
    about = "Dota 2 assistant: locate the game and set up Game State Integration"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,

    /// Path to the Dota 2 executable, game root, or cfg directory.
    /// Auto-detected when omitted.
    #[arg(long, global = true, value_name = "PATH")]
    path: Option<PathBuf>,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Locate Dota 2 and print its executable and GSI config entry point
    Find,
    /// Locate Dota 2 and write the GSI config file into the game's cfg directory
    Setup {
        /// Port the GSI server will listen on
        #[arg(short, long, default_value_t = 53000)]
        port: u16,

        /// Optional auth token Dota 2 will include in every payload
        #[arg(long)]
        token: Option<String>,
    },
    /// Start the GSI server and log incoming events
    Serve {
        /// Port the GSI server will listen on
        #[arg(short, long, default_value_t = 53000)]
        port: u16,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("info"),
    )
    .init();

    let cli = Cli::parse();
    match cli.command {
        None | Some(Command::Find) => cmd_find(cli.path.as_deref()),
        Some(Command::Setup { port, token }) => {
            cmd_setup(cli.path.as_deref(), port, token.as_deref())
        }
        Some(Command::Serve { port }) => cmd_serve(port).await,
    }
}

fn cmd_find(path: Option<&Path>) -> Result<()> {
    let install = discovery::find_install(path)?;
    print_install(&install);
    Ok(())
}

fn cmd_setup(path: Option<&Path>, port: u16, token: Option<&str>) -> Result<()> {
    let install = discovery::find_install(path)?;
    print_install(&install);

    let uri = format!("http://127.0.0.1:{port}/");
    let bind_uri = format!("127.0.0.1:{port}");
    let cfg_file = gsi_config::write_config(&install.cfg_dir, &uri, token)?;

    println!();
    println!("Wrote Game State Integration config:");
    println!("  {}", cfg_file.display());
    println!();
    println!("Next steps:");
    println!("  1. Restart Dota 2 with the launch option `-gamestateintegration`");
    println!("  2. Run your GSI server bound to `{bind_uri}` so it receives events");
    Ok(())
}

async fn cmd_serve(port: u16) -> Result<()> {
    let bind_uri = format!("127.0.0.1:{port}");
    let http_uri = format!("http://{bind_uri}/");

    // Fail fast with a clear message when the port is unavailable. The server
    // binds inside a spawned task, so without this check the process would
    // print "Listening..." and then hang while the listener silently dies.
    if let Err(e) = tokio::net::TcpListener::bind(&bind_uri).await {
        anyhow::bail!(
            "cannot bind to `{bind_uri}`: {e}\n\
             Is another dota2-assistant instance already running?"
        );
    }

    println!("Starting Game State Integration server on `{bind_uri}` ...");
    let server = ServerBuilder::new(&bind_uri)
        .register(log_event)
        .start()?;

    println!("Listening on {http_uri}");
    println!(
        "Make sure `dota2-assistant setup` was run and Dota 2 was started \
         with the `-gamestateintegration` launch option."
    );

    server.run_forever().await;
    Ok(())
}

/// Log a compact one-line summary of every incoming game state event.
async fn log_event(event: bytes::Bytes) -> Result<(), anyhow::Error> {
    let state: GameState = match serde_json::from_slice(&event) {
        Ok(state) => state,
        Err(e) => {
            // A single malformed payload must not kill the handler task
            // (which would make the listener exit with `NoHandlersAvailable`
            // and leave the process hanging with a dead server).
            log::error!("failed to deserialize event: {e}");
            return Ok(());
        }
    };

    let map = state
        .map
        .as_ref()
        .map(|m| format!("{} / {}", m.name, m.game_state))
        .unwrap_or_else(|| "no map".to_string());
    let hero = state
        .get_hero()
        .and_then(|h| h.name.clone())
        .unwrap_or_else(|| "-".to_string());

    log::info!("event: {map} | hero={hero}");
    Ok(())
}

fn print_install(install: &discovery::Dota2Install) {
    println!("Dota 2 game root: {}", install.game_root.display());
    match &install.executable {
        Some(exe) => println!("Executable:       {exe}", exe = exe.display()),
        None => println!("Executable:       not found (unrecognized install layout)"),
    }
    println!("GSI config dir:   {}", install.cfg_dir.display());
}
