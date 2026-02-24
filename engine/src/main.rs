// we need a system that can identify the node's gpu and give it a score
//
// code-snippet from clap example ref: https://docs.rs/clap/latest/clap/_derive/_tutorial/index.html
use clap::{Parser, Subcommand};
use std::path::PathBuf;

use crate::dht::DHT;

mod client;
mod dht;
mod gossip;
mod gpu;
mod model;
mod scheduling;
mod server;
mod utils;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    /// Optional name to operate on
    name: Option<String>,

    /// Sets a custom config file
    #[arg(short, long, value_name = "FILE")]
    config: Option<PathBuf>,

    /// Turn debugging information on
    #[arg(short, long, action = clap::ArgAction::Count)]
    debug: u8,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    Start {
        #[arg(long)]
        url: String,
        #[arg(short, long)]
        path: String,
    },
    Join {
        #[arg(short, long)]
        swarm_url: String,
    },
    Test {},
}

fn main() {
    let cli = Cli::parse();

    // You can check the value provided by positional arguments, or option arguments
    if let Some(name) = cli.name.as_deref() {
        println!("Value for name: {name}");
    }

    if let Some(config_path) = cli.config.as_deref() {
        println!("Value for config: {}", config_path.display());
    }

    // You can see how many times a particular flag or argument occurred
    // Note, only flags can have multiple occurrences
    match cli.debug {
        0 => println!("Debug mode is off"),
        1 => println!("Debug mode is kind of on"),
        2 => println!("Debug mode is on"),
        _ => println!("Don't be crazy"),
    }

    // You can check for the existence of subcommands, and if found use their
    // matches just as you would the top level cmd
    match &cli.command {
        Some(Commands::Start { url, path }) => {
            if url.is_empty() && path.is_empty() {
                println!("leader url and model path is needed");
            } else {
                println!("inference started");
            }
        }
        Some(Commands::Join { swarm_url }) => {
            if swarm_url.is_empty() {
                println!("leader url is needed");
            } else {
                println!("joining inference swarm");
            }
        }
        Some(Commands::Test {}) => {
            scheduling::main();
        }
        None => {}
    }
    // let dht = DHT::init();
}

struct SystemInfo {
    ram: usize,
    gpu_vram: usize,
}
