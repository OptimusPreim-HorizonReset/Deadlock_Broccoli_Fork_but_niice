use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

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
    let mut simulation = Simulation::new(&sim_config);
    let last_config_load = Instant::now();
    let config_reload_interval = Duration::from_secs(1); // Alle 1 Sekunde versuchen neu zu laden

    std::thread::spawn(move || {
        let mut simulation = simulation;
        let mut last_config_load = last_config_load;
        let mut last_config_hash = config_hash(&sim_config);

        loop {
            if renderer::PAUSED.load(Ordering::Relaxed) {
                std::thread::yield_now();
            } else {
                // Kontinuierliches Reload: Versuche Config zu laden wenn genug Zeit vergangen ist
                if last_config_load.elapsed() > config_reload_interval {
                    let new_config = config::InformationsConfig::load();
                    let new_hash = config_hash(&new_config);
                    if new_hash != last_config_hash {
                        eprintln!("🔄 Config-Änderung erkannt! Starte Simulation neu...");
                        simulation = Simulation::new(&new_config);
                        last_config_hash = new_hash;
                    }
                    last_config_load = Instant::now();
                }

                // Alt: RESET_REQUESTED Flag (für Benutzer-Aktion)
                if renderer::RESET_REQUESTED.swap(false, Ordering::Relaxed) {
                    let sim_config = config::InformationsConfig::load();
                    simulation = Simulation::new(&sim_config);
                    eprintln!("🔄 Reset via UI angefordert!");
                }

                simulation.step();
                let frame = simulation.frame;
                render(&mut simulation, frame);
            }
        }
    });

    quarkstrom::run::<Renderer>(app_config);
}

/// Berechnet einen einfachen Hash der Config für Änderungserkennung
fn config_hash(config: &config::InformationsConfig) -> u64 {
    let mut hash: u64 = 0;
    hash ^= config.dt.to_bits() as u64;
    hash ^= config.n as u64;
    hash ^= config.theta.to_bits() as u64;
    hash ^= config.epsilon.to_bits() as u64;
    hash ^= config.outer_radius.to_bits() as u64;
    hash ^= config.inner_radius.to_bits() as u64;
    hash ^= config.galaxy_separation_factor.to_bits() as u64;
    hash ^= config.accretion_spawn_rate.to_bits() as u64;
    hash ^= config.central_mass.to_bits() as u64;
    hash ^= config.collision_interval as u64;
    hash ^= config.attract_interval as u64;
    hash ^= config.spacetime_dilation_factor.to_bits() as u64;
    hash
}

fn render(simulation: &mut Simulation, frame: usize) {
    let mut lock = renderer::UPDATE_LOCK.lock();
    if frame % 2 == 0 {
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
}
