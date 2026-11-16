mod app;
pub mod celestial;
mod clock;
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
