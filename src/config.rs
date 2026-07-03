use std::fs;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct InformationsConfig {
    pub dt: f32,
    pub n: usize,
    pub theta: f32,
    pub epsilon: f32,
    pub accretion_spawn_rate: f32,
    pub inner_radius: f32,
    pub outer_radius: f32,
    pub outer_ring_spawn_zone_inner_radius: Option<f32>,
    pub outer_ring_spawn_zone_outer_radius: Option<f32>,
    pub central_mass: f32,
    pub particle_mass_range: (f32, f32),
    pub inflow_strength: f32,
    pub restore_strength: f32,
    pub galaxy_separation_factor: f32,
    pub prob_merge: f32,
    pub prob_repeated: f32,
    pub prob_flyby: f32,
    pub merge_speed_factor: f32,
    pub repeated_speed_factor: f32,
    pub flyby_speed_factor: f32,
    pub merge_angle: f32,
    pub repeated_angle: f32,
    pub flyby_angle: f32,
}

impl Default for InformationsConfig {
    fn default() -> Self {
        Self {
            dt: 0.05,
            n: 50000,
            theta: 1.0,
            epsilon: 1.1,
            accretion_spawn_rate: 0.1,
            inner_radius: 25.0,
            outer_radius: 1118.03,
            outer_ring_spawn_zone_inner_radius: None,
            outer_ring_spawn_zone_outer_radius: None,
            central_mass: 1e6,
            particle_mass_range: (0.5, 2.0),
            inflow_strength: 0.025,
            restore_strength: 0.08,
            galaxy_separation_factor: 5.0,
            prob_merge: 0.35,
            prob_repeated: 0.35,
            prob_flyby: 0.30,
            merge_speed_factor: 0.50,
            repeated_speed_factor: 0.70,
            flyby_speed_factor: 0.90,
            merge_angle: 0.05,
            repeated_angle: 0.25,
            flyby_angle: 0.35,
        }
    }
}

impl InformationsConfig {
    pub fn load() -> Self {
        let path = Path::new("informations.md");
        let file_text = fs::read_to_string(path).unwrap_or_default();
        let mut config = InformationsConfig::default();
        let mut in_block = false;

        for line in file_text.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("```config") {
                in_block = true;
                continue;
            }
            if in_block {
                if trimmed.starts_with("```") {
                    break;
                }
                if trimmed.is_empty() || trimmed.starts_with('#') {
                    continue;
                }
                let pair = trimmed.split_once('=').or_else(|| trimmed.split_once(':'));
                if let Some((key, value)) = pair {
                    let key = key.trim();
                    let value = value.trim();
                    match key {
                        "dt" => {
                            if let Ok(v) = value.parse() {
                                config.dt = v;
                            }
                        }
                        "n" => {
                            if let Ok(v) = value.parse() {
                                config.n = v;
                            }
                        }
                        "theta" => {
                            if let Ok(v) = value.parse() {
                                config.theta = v;
                            }
                        }
                        "epsilon" => {
                            if let Ok(v) = value.parse() {
                                config.epsilon = v;
                            }
                        }
                        "accretion_spawn_rate" => {
                            if let Ok(v) = value.parse() {
                                config.accretion_spawn_rate = v;
                            }
                        }
                        "inner_radius" => {
                            if let Ok(v) = value.parse() {
                                config.inner_radius = v;
                            }
                        }
                        "outer_radius" => {
                            if let Ok(v) = value.parse() {
                                config.outer_radius = v;
                            }
                        }
                        "outer_ring_spawn_zone_inner_radius" => {
                            if let Ok(v) = value.parse() {
                                config.outer_ring_spawn_zone_inner_radius = Some(v);
                            }
                        }
                        "outer_ring_spawn_zone_outer_radius" => {
                            if let Ok(v) = value.parse() {
                                config.outer_ring_spawn_zone_outer_radius = Some(v);
                            }
                        }
                        "central_mass" => {
                            if let Ok(v) = value.parse() {
                                config.central_mass = v;
                            }
                        }
                        "particle_mass_range" => {
                            let parts: Vec<&str> = value.split(',').map(str::trim).collect();
                            if parts.len() == 2 {
                                if let (Ok(min), Ok(max)) = (parts[0].parse(), parts[1].parse()) {
                                    config.particle_mass_range = (min, max);
                                }
                            }
                        }
                        "inflow_strength" => {
                            if let Ok(v) = value.parse() {
                                config.inflow_strength = v;
                            }
                        }
                        "restore_strength" => {
                            if let Ok(v) = value.parse() {
                                config.restore_strength = v;
                            }
                        }
                        "galaxy_separation_factor" => {
                            if let Ok(v) = value.parse() {
                                config.galaxy_separation_factor = v;
                            }
                        }
                        "prob_merge" => {
                            if let Ok(v) = value.parse() {
                                config.prob_merge = v;
                            }
                        }
                        "prob_repeated" => {
                            if let Ok(v) = value.parse() {
                                config.prob_repeated = v;
                            }
                        }
                        "prob_flyby" => {
                            if let Ok(v) = value.parse() {
                                config.prob_flyby = v;
                            }
                        }
                        "merge_speed_factor" => {
                            if let Ok(v) = value.parse() {
                                config.merge_speed_factor = v;
                            }
                        }
                        "repeated_speed_factor" => {
                            if let Ok(v) = value.parse() {
                                config.repeated_speed_factor = v;
                            }
                        }
                        "flyby_speed_factor" => {
                            if let Ok(v) = value.parse() {
                                config.flyby_speed_factor = v;
                            }
                        }
                        "merge_angle" => {
                            if let Ok(v) = value.parse() {
                                config.merge_angle = v;
                            }
                        }
                        "repeated_angle" => {
                            if let Ok(v) = value.parse() {
                                config.repeated_angle = v;
                            }
                        }
                        "flyby_angle" => {
                            if let Ok(v) = value.parse() {
                                config.flyby_angle = v;
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

        config
    }
}
