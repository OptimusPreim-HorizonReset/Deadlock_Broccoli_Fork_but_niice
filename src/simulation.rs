use crate::{
    body::Body,
    config::InformationsConfig,
    galaxy_templates::GalaxyTemplate,
    quadtree::{Oct, Octree},
    renderer,
};

use rayon::prelude::*;
use ultraviolet::{Vec2, Vec3};

pub struct Simulation {
    pub dt: f32,
    pub frame: usize,
    pub bodies: Vec<Body>,
    pub octree: Octree,
    /// Template used to derive accretion spawn parameters.
    accretion_template: GalaxyTemplate,
    config: InformationsConfig,
}

impl Simulation {
    pub fn new(config: &InformationsConfig) -> Self {
        let dt = config.dt;
        let theta = config.theta;
        let epsilon = config.epsilon;

        let accretion_template = GalaxyTemplate::from_config(config);
        let config = config.clone();

        let axis1 = Self::random_inclination_axis();
        let axis2 = Self::random_inclination_axis();
        let clockwise1 = fastrand::bool();
        let clockwise2 = fastrand::bool();

        let mut bodies1 = accretion_template.generate_inclined(Vec2::zero(), Vec2::zero(), axis1, clockwise1);
        let separation = accretion_template.outer_radius * config.galaxy_separation_factor;
        let offset = Vec2::new(separation, 0.0);
        let mut bodies2 = accretion_template.generate_inclined(offset, Vec2::zero(), axis2, clockwise2);

        let m1: f32 = bodies1.iter().map(|b| b.mass).sum();
        let m2: f32 = bodies2.iter().map(|b| b.mass).sum();
        let center1 = bodies1[0].pos;
        let center2 = bodies2[0].pos;
        let (v1, v2) = Self::galaxy_bulk_velocities(&config, m1, m2, center1, center2);
        for body in &mut bodies1 {
            body.vel += v1;
        }
        for body in &mut bodies2 {
            body.vel += v2;
        }

        let mut bodies = Vec::with_capacity(bodies1.len() + bodies2.len());
        bodies.extend(bodies1);
        bodies.extend(bodies2);

        let octree = Octree::new(theta, epsilon);

        Self {
            dt,
            frame: 0,
            bodies,
            octree,
            accretion_template,
            config,
        }
    }

    fn random_inclination_axis() -> Vec3 {
        let theta = fastrand::f32() * std::f32::consts::PI * 0.5;
        let phi = fastrand::f32() * std::f32::consts::TAU;
        let x = theta.sin() * phi.cos();
        let y = theta.sin() * phi.sin();
        let z = theta.cos();
        Vec3::new(x, y, z).normalized()
    }

    fn galaxy_bulk_velocities(
        config: &InformationsConfig,
        m1: f32,
        m2: f32,
        center1: Vec3,
        center2: Vec3,
    ) -> (Vec3, Vec3) {
        let distance = (center2 - center1).mag().max(config.outer_radius * 6.0);
        let mu = 1.0;
        let escape_speed = ((2.0 * mu * (m1 + m2)) / distance).sqrt();

        let roll = fastrand::f32();
        let interaction_type = if roll < config.prob_merge {
            0
        } else if roll < config.prob_merge + config.prob_repeated {
            1
        } else {
            2
        };
        let (speed_factor, lateral_factor, angle) = match interaction_type {
            0 => (
                config.merge_speed_factor,
                config.merge_speed_factor * 0.1,
                config.merge_angle,
            ),
            1 => (
                config.repeated_speed_factor,
                config.repeated_speed_factor * 0.2,
                config.repeated_angle,
            ),
            _ => (
                config.flyby_speed_factor,
                config.flyby_speed_factor * 0.25,
                config.flyby_angle,
            ),
        };

        let direction = (center2 - center1).normalized();
        let perp = Vec3::new(-direction.y, direction.x, 0.0).normalized();
        let sign = if fastrand::bool() { 1.0 } else { -1.0 };

        let v_radial = direction * escape_speed * speed_factor;
        let v_tangent = perp * escape_speed * lateral_factor * sign;
        let v_rel = v_radial * angle.cos() + v_tangent * angle.sin();

        let v1 = v_rel * (m2 / (m1 + m2));
        let v2 = -v_rel * (m1 / (m1 + m2));

        (v1, v2)
    }

    pub fn step(&mut self) {
        self.iterate();
        self.collide();
        self.attract();
        if renderer::SPAWN_ENABLED.load(std::sync::atomic::Ordering::Relaxed) {
            self.spawn_accretion();
        }
        self.frame += 1;
    }

    pub fn attract(&mut self) {
        let oct = Oct::new_containing(&self.bodies);
        let reserve_nodes = self.bodies.len() * 6;
        let reserve_parents = self.bodies.len();
        self.octree.reserve(reserve_nodes, reserve_parents);
        self.octree.clear(oct);

        for body in &self.bodies {
            self.octree.insert(body.pos, body.mass);
        }

        self.octree.propagate();

        self.bodies.par_iter_mut().for_each(|body| {
            body.acc = self.octree.acc(body.pos);
        });

        self.apply_hydrodynamic_inflow();
    }

    fn apply_hydrodynamic_inflow(&mut self) {
        let outer_r = self.accretion_template.outer_radius.max(1.0);
        let inflow_strength = 0.025;
        let restore_strength = 0.08;

        self.bodies.par_iter_mut().for_each(|body| {
            let y = body.pos.y;
            let z = body.pos.z;
            let lateral = if y.abs() > 0.0 {
                -y.signum() * inflow_strength * (1.0 - (y.abs() / outer_r).clamp(0.0, 1.0))
            } else {
                0.0
            };
            let vertical = -z * restore_strength / outer_r;
            body.acc += ultraviolet::Vec3::new(0.0, lateral, vertical);
        });
    }

    pub fn iterate(&mut self) {
        self.bodies.par_iter_mut().for_each(|body| {
            body.update(self.dt);
        });
    }

    pub fn collide(&mut self) {
        if self.bodies.len() < 2 {
            return;
        }

        let max_radius = self
            .bodies
            .iter()
            .map(|body| body.effective_radius())
            .fold(0.0_f32, f32::max)
            .max(1.0);
        let cell_size = max_radius * 3.0;

        let mut min_x = f32::MAX;
        let mut min_y = f32::MAX;
        let mut min_z = f32::MAX;
        let mut max_x = f32::MIN;
        let mut max_y = f32::MIN;
        let mut max_z = f32::MIN;
        for body in &self.bodies {
            min_x = min_x.min(body.pos.x);
            min_y = min_y.min(body.pos.y);
            min_z = min_z.min(body.pos.z);
            max_x = max_x.max(body.pos.x);
            max_y = max_y.max(body.pos.y);
            max_z = max_z.max(body.pos.z);
        }

        min_x -= cell_size;
        min_y -= cell_size;
        min_z -= cell_size;
        max_x += cell_size;
        max_y += cell_size;
        max_z += cell_size;

        let size_x = ((max_x - min_x) / cell_size).ceil() as usize + 1;
        let size_y = ((max_y - min_y) / cell_size).ceil() as usize + 1;
        let size_z = ((max_z - min_z) / cell_size).ceil() as usize + 1;
        let total_cells = size_x.saturating_mul(size_y).saturating_mul(size_z);
        if total_cells == 0 {
            return;
        }

        let mut cells = Vec::with_capacity(total_cells);
        cells.resize_with(total_cells, Vec::new);

        let cell_index = |pos: Vec3| {
            let ix = ((pos.x - min_x) / cell_size).floor() as isize;
            let iy = ((pos.y - min_y) / cell_size).floor() as isize;
            let iz = ((pos.z - min_z) / cell_size).floor() as isize;
            ((ix as usize) * size_y + iy as usize) * size_z + iz as usize
        };

        for (index, body) in self.bodies.iter().enumerate() {
            let idx = cell_index(body.pos);
            if idx < cells.len() {
                cells[idx].push(index);
            }
        }

        for x in 0..size_x {
            for y in 0..size_y {
                for z in 0..size_z {
                    let cell_idx = (x * size_y + y) * size_z + z;
                    let indices = &cells[cell_idx];
                    if indices.is_empty() {
                        continue;
                    }

                    for dx in 0..=1 {
                        for dy in 0..=1 {
                            for dz in 0..=1 {
                                let nx = x + dx;
                                let ny = y + dy;
                                let nz = z + dz;
                                if nx >= size_x || ny >= size_y || nz >= size_z {
                                    continue;
                                }
                                let neighbor_idx = (nx * size_y + ny) * size_z + nz;
                                if neighbor_idx >= cells.len() {
                                    continue;
                                }
                                let neighbor_indices = &cells[neighbor_idx];
                                if neighbor_indices.is_empty() {
                                    continue;
                                }

                                for &i in indices {
                                    for &j in neighbor_indices {
                                        if neighbor_idx == cell_idx && i >= j {
                                            continue;
                                        }
                                        self.resolve(i, j);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    /// Spawns accretion particles in the outer ring spawn zone.
    ///
    /// On each step rolls against `accretion_spawn_rate`. A successful roll places
    /// one new particle at a random position within the outer ring spawn zone
    /// (outer third of the galactic disk) and assigns it a circular orbital velocity
    /// derived from the local gravitational acceleration reported by the octree.
    /// Spawns are placed in absolute disk coordinates, avoiding the central bulge.
    fn spawn_accretion(&mut self) {
        let spawn_start_r = self
            .accretion_template
            .outer_ring_spawn_zone_inner_radius;
        let outer_r = self
            .accretion_template
            .outer_ring_spawn_zone_outer_radius;
        if spawn_start_r >= outer_r {
            return;
        }

        if fastrand::f32() > self.accretion_template.accretion_spawn_rate {
            return;
        }

        let (mass_min, mass_max) = self.accretion_template.particle_mass_range;

        // Spawn in absolute disk coordinates, not relative to detected centers.
        // This ensures spawns occur in the outer third of the galactic disk.
        let a = fastrand::f32() * std::f32::consts::TAU;
        let (sin, cos) = a.sin_cos();
        let r = spawn_start_r + fastrand::f32() * (outer_r - spawn_start_r);
        let spawn_pos = Vec3::new(
            cos * r,
            sin * r,
            (fastrand::f32() - 0.5) * outer_r * 0.03,
        );
        // Tangent direction for a clockwise orbit in the disk plane.
        let tangent = Vec3::new(sin, -cos, 0.0);

        let mass = mass_min + fastrand::f32() * (mass_max - mass_min);
        let radius = mass.cbrt();

        let acc = self.octree.acc(spawn_pos);
        let orbital_speed = (acc.mag() * r).sqrt();
        let vel = tangent * orbital_speed;

        let angular_speed = 0.3 + fastrand::f32() * 0.5 + orbital_speed * 0.02;
        self.bodies.push(Body::new(
            spawn_pos,
            vel,
            mass,
            radius,
            angular_speed,
            Vec3::new(0.0, 0.0, 1.0),
        ));
    }

    fn resolve(&mut self, i: usize, j: usize) {
        let b1 = &self.bodies[i];
        let b2 = &self.bodies[j];

        let p1 = b1.pos;
        let p2 = b2.pos;

        let r1 = b1.effective_radius();
        let r2 = b2.effective_radius();

        let d = p2 - p1;
        let r = r1 + r2;

        if d.mag_sq() > r * r {
            return;
        }

        let v1 = b1.vel;
        let v2 = b2.vel;

        let v = v2 - v1;

        let d_dot_v = d.dot(v);

        let m1 = b1.mass;
        let m2 = b2.mass;

        let weight1 = m2 / (m1 + m2);
        let weight2 = m1 / (m1 + m2);

        if d_dot_v >= 0.0 && d != Vec3::zero() {
            let tmp = d * (r / d.mag() - 1.0);
            self.bodies[i].pos -= weight1 * tmp;
            self.bodies[j].pos += weight2 * tmp;
            return;
        }

        let v_sq = v.mag_sq();
        let d_sq = d.mag_sq();
        let r_sq = r * r;

        let t = (d_dot_v + (d_dot_v * d_dot_v - v_sq * (d_sq - r_sq)).max(0.0).sqrt()) / v_sq;

        self.bodies[i].pos -= v1 * t;
        self.bodies[j].pos -= v2 * t;

        let p1 = self.bodies[i].pos;
        let p2 = self.bodies[j].pos;
        let d = p2 - p1;
        let d_dot_v = d.dot(v);
        let d_sq = d.mag_sq();

        let tmp = d * (1.5 * d_dot_v / d_sq);
        let v1 = v1 + tmp * weight1;
        let v2 = v2 - tmp * weight2;

        self.bodies[i].vel = v1;
        self.bodies[j].vel = v2;
        self.bodies[i].pos += v1 * t;
        self.bodies[j].pos += v2 * t;
    }
}
