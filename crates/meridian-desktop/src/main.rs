mod app;
mod clock;
mod hud;
mod mesh;
mod renderer;
mod session;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    app::run()
}
