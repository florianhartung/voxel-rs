use log::LevelFilter;

mod engine;

fn main() {
    env_logger::Builder::new()
        .filter_module("wgpu_hal::vulkan::instance", LevelFilter::Off) // suppress invalid vulkan validation layer error (see https://github.com/gfx-rs/wgpu/pull/4002)
        .filter_module("wgpu_hal", LevelFilter::Warn)
        .init();

    engine::start()
}
