mod app;
pub mod asteroid_render;
pub mod celestial;
mod clock;
mod docked;
mod docked_render;
pub mod docked_screens;
mod equipment;
mod flight_lesson;
mod hud;
mod market;
mod mesh;
mod profile;
mod renderer;
mod session;
mod storage;
mod trade_lesson;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    app::run()
}
