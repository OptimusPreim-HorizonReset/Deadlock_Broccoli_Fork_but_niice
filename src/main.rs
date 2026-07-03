use std::sync::atomic::Ordering;

mod body;
mod config;
mod galaxy_templates;
mod quadtree;
mod renderer;
mod simulation;
mod utils;

use renderer::Renderer;
use simulation::Simulation;

fn main() {
    let app_config = quarkstrom::Config {
        window_mode: quarkstrom::WindowMode::Windowed(900, 900),
    };

    let sim_config = config::InformationsConfig::load();
    let sim_config_clone = sim_config.clone();
    let simulation = Simulation::new(&sim_config);

    std::thread::spawn(move || {
        let mut simulation = simulation;
        let sim_config = sim_config_clone;
        loop {
            if renderer::PAUSED.load(Ordering::Relaxed) {
                std::thread::yield_now();
            } else {
                if renderer::RESET_REQUESTED.swap(false, Ordering::Relaxed) {
                    simulation = Simulation::new(&sim_config);
                }
                simulation.step();
            }
            render(&mut simulation);
        }
    });

    quarkstrom::run::<Renderer>(app_config);
}

fn render(simulation: &mut Simulation) {
    let mut lock = renderer::UPDATE_LOCK.lock();
    {
        let mut lock = renderer::BODIES.lock();
        lock.clear();
        lock.extend_from_slice(&simulation.bodies);
    }
    {
        let mut lock = renderer::QUADTREE.lock();
        lock.clear();
        lock.extend_from_slice(&simulation.octree.nodes);
    }
    *lock |= true;
}
