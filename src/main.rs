use clap::Parser;
use log::{info, LevelFilter};

use crate::engine::EngineConfig;

mod engine;

/// Simple program to greet a person
#[derive(Parser, Debug)]
struct Args {
    /// Run a benchmark (unused as of now)
    #[arg(short, long, default_value_t = false)]
    benchmark: bool,
    /// Use vertical sync
    #[arg(short, long, default_value_t = false)]
    vsync: bool,
    /// Set a custom window size
    #[arg(short, long, num_args = 2, default_value = "800 600", value_delimiter = ' ')]
    window_size: Vec<u32>,
    /// Make the window fullscreen
    #[arg(short, long, default_value_t = false)]
    fullscreen: bool,
}

fn main() {
    env_logger::Builder::new()
        .filter_module("wgpu_hal::vulkan::instance", LevelFilter::Off) // suppress invalid vulkan validation layer error (see https://github.com/gfx-rs/wgpu/pull/4002)
        .filter_module("wgpu_hal", LevelFilter::Warn)
        .init();

    let args = Args::parse();
    if args.benchmark {
        info!("Running benchmark...");
    }

    let engine_config = EngineConfig {
        run_benchmark: args.benchmark,
        vsync: args.vsync,
        window_size: (args.window_size[0], args.window_size[1]),
        fullscreen: args.fullscreen,
    };

    engine::start(engine_config)
}
