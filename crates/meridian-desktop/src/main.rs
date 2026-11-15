mod app;
mod clock;
mod mesh;
mod renderer;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    app::run()
}
