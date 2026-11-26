mod app;
mod asteroid_render;
pub mod audio;
mod authored_hull;
pub mod celestial;
mod clock;
mod docked;
mod docked_render;
pub mod docked_screens;
mod equipment;
mod flight_lesson;
mod help;
mod hud;
mod market;
mod mesh;
mod mule_render;
mod profile;
mod renderer;
mod session;
mod storage;
mod trade_lesson;
mod xyzzy;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    app::run()
}
