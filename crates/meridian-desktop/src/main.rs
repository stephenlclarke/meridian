mod app;
mod clock;
mod hud;
mod mesh;
mod profile;
mod renderer;
mod session;
mod storage;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    app::run()
}
