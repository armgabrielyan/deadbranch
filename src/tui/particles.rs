//! Particle system for the Thanos snap dissolution effect.

use ratatui::style::Color;

/// Unicode characters for particle density levels.
/// Index 0 is lowest density (nearly gone), index 4 is highest (just spawned).
const DENSITY_CHARS: [char; 5] = ['·', '░', '▒', '▓', '█'];

/// Maximum number of active particles at any time.
const MAX_PARTICLES: usize = 200;

/// A single particle drifting across the screen.
#[derive(Debug, Clone)]
pub struct Particle {
    /// Column position (fractional for smooth movement).
    pub x: f32,
    /// Row position (fractional for smooth movement).
    pub y: f32,
    /// Horizontal velocity (cells per tick; positive = rightward).
    vx: f32,
    /// Vertical velocity (cells per tick; negative = upward).
    vy: f32,
    /// Density level: 4=█, 3=▓, 2=▒, 1=░, 0=·
    pub density: u8,
    /// Color inherited from the source cell.
    pub color: Color,
    /// Frames remaining before this particle is removed.
    lifetime: u8,
}

impl Particle {
    /// Get the display character for this particle's current density.
    pub fn char(&self) -> char {
        DENSITY_CHARS[self.density as usize]
    }

    /// Whether this particle should be removed (lifetime expired).
    pub fn is_dead(&self) -> bool {
        self.lifetime == 0
    }
}

/// Manages all active particles: spawning, ticking, and culling.
pub struct ParticleSystem {
    pub particles: Vec<Particle>,
}

impl ParticleSystem {
    pub fn new() -> Self {
        Self {
            particles: Vec::with_capacity(MAX_PARTICLES),
        }
    }

    /// Spawn a particle at the given position with the given color.
    /// Silently drops the spawn if at capacity.
    pub fn spawn(&mut self, x: f32, y: f32, color: Color) {
        if self.particles.len() >= MAX_PARTICLES {
            return;
        }
        // Biased up-right: vx positive, vy negative
        let vx = 0.3 + fastrand::f32() * 0.7; // 0.3 to 1.0 rightward
        let vy = -(0.2 + fastrand::f32() * 0.6); // -0.2 to -0.8 upward
        self.particles.push(Particle {
            x,
            y,
            vx,
            vy,
            density: 3 + (fastrand::u8(0..2)), // start at 3 or 4
            color,
            lifetime: 15 + fastrand::u8(0..15), // 15-29 frames
        });
    }

    /// Advance all particles by one frame: move, decay, cull.
    pub fn tick(&mut self, screen_width: u16, screen_height: u16) {
        for p in &mut self.particles {
            // Move
            p.x += p.vx;
            p.y += p.vy;

            // Gravity: slight downward pull (decelerate upward movement)
            p.vy += 0.02;

            // Random jitter
            p.vx += (fastrand::f32() - 0.5) * 0.1;
            p.vy += (fastrand::f32() - 0.5) * 0.05;

            // Density decay: decrease every 4 frames
            if p.lifetime % 4 == 0 && p.density > 0 {
                p.density -= 1;
            }

            p.lifetime = p.lifetime.saturating_sub(1);
        }

        // Cull dead or out-of-bounds particles
        let w = screen_width as f32;
        let h = screen_height as f32;
        self.particles
            .retain(|p| !p.is_dead() && p.x >= 0.0 && p.x < w && p.y >= 0.0 && p.y < h);
    }

    /// Whether there are any active particles.
    pub fn is_empty(&self) -> bool {
        self.particles.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_particle_char_mapping() {
        let p = Particle {
            x: 0.0,
            y: 0.0,
            vx: 0.0,
            vy: 0.0,
            density: 4,
            color: Color::White,
            lifetime: 10,
        };
        assert_eq!(p.char(), '█');

        let p0 = Particle {
            density: 0,
            ..p.clone()
        };
        assert_eq!(p0.char(), '·');

        let p2 = Particle {
            density: 2,
            ..p.clone()
        };
        assert_eq!(p2.char(), '▒');
    }

    #[test]
    fn test_particle_is_dead() {
        let alive = Particle {
            x: 0.0,
            y: 0.0,
            vx: 0.0,
            vy: 0.0,
            density: 4,
            color: Color::White,
            lifetime: 5,
        };
        assert!(!alive.is_dead());

        let dead = Particle {
            lifetime: 0,
            ..alive
        };
        assert!(dead.is_dead());
    }

    #[test]
    fn test_spawn_adds_particle() {
        let mut ps = ParticleSystem::new();
        assert!(ps.is_empty());
        ps.spawn(10.0, 5.0, Color::Green);
        assert_eq!(ps.particles.len(), 1);
        assert!(!ps.is_empty());
    }

    #[test]
    fn test_spawn_respects_cap() {
        let mut ps = ParticleSystem::new();
        for _ in 0..MAX_PARTICLES + 50 {
            ps.spawn(10.0, 5.0, Color::Green);
        }
        assert_eq!(ps.particles.len(), MAX_PARTICLES);
    }

    #[test]
    fn test_tick_moves_particles() {
        let mut ps = ParticleSystem::new();
        ps.particles.push(Particle {
            x: 10.0,
            y: 10.0,
            vx: 1.0,
            vy: -0.5,
            density: 4,
            color: Color::White,
            lifetime: 30,
        });
        let old_x = ps.particles[0].x;
        let old_y = ps.particles[0].y;
        ps.tick(80, 24);
        assert!(ps.particles[0].x > old_x);
        assert!(ps.particles[0].y < old_y);
    }

    #[test]
    fn test_tick_removes_dead_particles() {
        let mut ps = ParticleSystem::new();
        ps.particles.push(Particle {
            x: 10.0,
            y: 10.0,
            vx: 0.0,
            vy: 0.0,
            density: 4,
            color: Color::White,
            lifetime: 1,
        });
        ps.tick(80, 24);
        assert!(ps.is_empty());
    }

    #[test]
    fn test_tick_removes_out_of_bounds() {
        let mut ps = ParticleSystem::new();
        ps.particles.push(Particle {
            x: 100.0,
            y: 10.0,
            vx: 1.0,
            vy: 0.0,
            density: 4,
            color: Color::White,
            lifetime: 30,
        });
        ps.tick(80, 24);
        assert!(ps.is_empty());
    }

    #[test]
    fn test_tick_removes_above_screen() {
        let mut ps = ParticleSystem::new();
        ps.particles.push(Particle {
            x: 10.0,
            y: -1.0,
            vx: 0.0,
            vy: -1.0,
            density: 4,
            color: Color::White,
            lifetime: 30,
        });
        ps.tick(80, 24);
        assert!(ps.is_empty());
    }

    #[test]
    fn test_density_decays_over_time() {
        let mut ps = ParticleSystem::new();
        ps.particles.push(Particle {
            x: 10.0,
            y: 10.0,
            vx: 0.0,
            vy: 0.0,
            density: 4,
            color: Color::White,
            lifetime: 30,
        });
        // Decay checks lifetime % 4 == 0 before decrementing:
        // Tick 1: 30%4=2 (no), → 29. Tick 2: 29%4=1 (no), → 28. Tick 3: 28%4=0 (decay!), → 27.
        for _ in 0..3 {
            ps.tick(80, 24);
        }
        assert!(ps.particles[0].density < 4);
    }

    #[test]
    fn test_gravity_pulls_down_over_time() {
        let mut ps = ParticleSystem::new();
        ps.particles.push(Particle {
            x: 40.0,
            y: 12.0,
            vx: 0.0,
            vy: -0.5,
            density: 4,
            color: Color::White,
            lifetime: 60,
        });
        for _ in 0..40 {
            ps.tick(80, 24);
        }
        assert!(ps.particles[0].vy > -0.5);
    }
}
