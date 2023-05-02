mod engine;

use log::LevelFilter;


fn main() {
    env_logger::Builder::new()
        .filter_module("wgpu_hal", LevelFilter::Warn)
        .init();

    engine::start()
}