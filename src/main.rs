use log::LevelFilter;

mod engine;

fn main() {
    env_logger::Builder::new()
        .filter_module("wgpu_hal", LevelFilter::Warn)
        .init();

    engine::start()
}