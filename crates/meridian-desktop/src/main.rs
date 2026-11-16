mod app;
mod clock;
mod equipment;
mod hud;
mod market;
mod mesh;
mod profile;
mod renderer;
mod session;
mod storage;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    app::run()
}
