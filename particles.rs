//! particles - Core algorithm for embedded systems
//! No heap allocation, no_std compatible

#![no_std]

// CHANGE: Using heapless for collections
// REASON: No heap allocation allowed in embedded context
use heapless::{String, Vec};
use core::fmt::Write;

// CHANGE: Comprehensive settings struct
// REASON: All configuration externalized for compile-time optimization
#[derive(Copy, Clone)]
pub struct Settings {
    // Pool sizes
    pub max_particles: usize,
    pub max_dust: usize,
    pub max_particles_array: usize,  // CHANGE: Compile-time max for arrays
    pub max_dust_array: usize,       // CHANGE: Compile-time max for arrays
    
    // Physics
    pub gravity: f32,
    pub global_fall_speed: f32,
    pub wind: f32,
    
    // Timing
    pub collision_cooldown_time: f32,
    pub trigger_duration: f32,
    pub verbose_duration: f32,
    
    // Display bounds
    pub screen_width: i32,
    pub screen_height: i32,
    pub ground_level: i32,
    
    // Particle generation
    pub particle_spawn_chance: f32,
    pub particle_min_size: f32,
    pub particle_max_size: f32,
    pub particle_sway_speed_min: f32,
    pub particle_sway_speed_max: f32,
    
    // Dust generation
    pub dust_dx_factor: f32,
    pub dust_dy_max: f32,
    pub dust_life_min: f32,
    pub dust_life_max: f32,
    pub dust_brightness_max: u8,
    
    // Output normalization
    pub collision_output_range: f32,
    
    // RNG seed
    pub rng_seed: u32,
}

// CHANGE: Default settings matching original behavior
// REASON: Backward compatibility through default values
impl Default for Settings {
    fn default() -> Self {
        Self {
            max_particles: 6,
            max_dust: 50,
            max_particles_array: 12,
            max_dust_array: 50,
            gravity: 1.0,
            global_fall_speed: 5.0,
            wind: 0.1,
            collision_cooldown_time: 3.0,
            trigger_duration: 0.05,
            verbose_duration: 1.0,
            screen_width: 320,
            screen_height: 170,
            ground_level: 150,
            particle_spawn_chance: 0.2,
            particle_min_size: 3.0,
            particle_max_size: 10.0,
            particle_sway_speed_min: 0.1,
            particle_sway_speed_max: 0.3,
            dust_dx_factor: 10.0,
            dust_dy_max: 5.0,
            dust_life_min: 3.0,
            dust_life_max: 10.0,
            dust_brightness_max: 5,
            collision_output_range: 10.0,
            rng_seed: 0x12345678,
        }
    }
}

// Particle structure
#[derive(Copy, Clone)]
pub struct Particle {
    pub x: f32,
    pub y: f32,
    pub base_speed: f32,
    pub sway: f32,
    pub sway_speed: f32,
    pub wind_sensitivity: f32,
    pub radius: f32,
    // CHANGE: Renamed from 'pitch' to 'particle_type'
    // REASON: Domain-agnostic design
    pub particle_type: u8,
    pub last_collision_time: f32,
    pub active: bool,
}

impl Default for Particle {
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            base_speed: 0.0,
            sway: 0.0,
            sway_speed: 0.0,
            wind_sensitivity: 0.0,
            radius: 0.0,
            particle_type: 0,
            last_collision_time: 0.0,
            active: false,
        }
    }
}

// Dust speck structure
#[derive(Copy, Clone)]
pub struct Dust {
    pub x: f32,
    pub y: f32,
    pub dx: f32,
    pub dy: f32,
    pub brightness: u8,
    pub life: f32,
    pub active: bool,
}

impl Default for Dust {
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            dx: 0.0,
            dy: 0.0,
            brightness: 0,
            life: 0.0,
            active: false,
        }
    }
}

// CHANGE: Generic particle system with const generics
// REASON: Support different array sizes at compile time
pub struct ParticlesSystem<const MAX_PARTICLES: usize, const MAX_DUST: usize> {
    // Object pools
    pub particle_pool: [Particle; MAX_PARTICLES],
    pub dust_pool: [Dust; MAX_DUST],
    pub active_particles: usize,
    pub active_dust: usize,
    
    // Timing
    pub time: f32,
    pub trigger_timer: f32,
    pub collision_trigger_timer: f32,
    pub verbose_timer: f32,
    
    // CHANGE: Outputs now normalized to u16 range
    // REASON: Domain-agnostic output values
    pub last_ground_output: u16,
    pub collision_output: u16,
    
    // CHANGE: Using heapless::String for messages
    // REASON: No heap allocation
    pub verbose_message: String<128>,
    
    // Parameters (now minimal, most in Settings)
    pub verbose: bool,
    
    // Random state
    rng_state: u32,
    
    // CHANGE: Reference to settings
    // REASON: All configuration externalized
    settings: Settings,
}

impl<const MAX_PARTICLES: usize, const MAX_DUST: usize> ParticlesSystem<MAX_PARTICLES, MAX_DUST> {
    pub fn new(settings: Settings) -> Self {
        Self {
            particle_pool: [Particle::default(); MAX_PARTICLES],
            dust_pool: [Dust::default(); MAX_DUST],
            active_particles: 0,
            active_dust: 0,
            time: 0.0,
            trigger_timer: 0.0,
            collision_trigger_timer: 0.0,
            verbose_timer: 0.0,
            last_ground_output: 0,
            collision_output: 0,
            verbose_message: String::new(),
            verbose: false,
            rng_state: settings.rng_seed,
            settings,
        }
    }
    
    // Simple PRNG (xorshift32)
    fn random(&mut self) -> f32 {
        self.rng_state ^= self.rng_state << 13;
        self.rng_state ^= self.rng_state >> 17;
        self.rng_state ^= self.rng_state << 5;
        (self.rng_state as f32) / (u32::MAX as f32)
    }
    
    fn random_range(&mut self, min: f32, max: f32) -> f32 {
        min + self.random() * (max - min)
    }
    
    fn random_int(&mut self, min: i32, max: i32) -> i32 {
        (self.random_range(min as f32, max as f32 + 1.0)) as i32
    }
    
    // CHANGE: New function to convert position/type to normalized output
    // REASON: Replace domain-specific pitch/voltage conversion
    fn particle_to_output(settings: &Settings, particle: &Particle) -> u16 {
        // COMPAT: Maintains proportional relationship between particle properties and output
        let position_factor = particle.x / settings.screen_width as f32;
        let type_factor = particle.particle_type as f32 / 7.0;  // Original had 7 scale degrees
        let size_factor = (particle.radius - settings.particle_min_size) / 
                         (settings.particle_max_size - settings.particle_min_size);
        
        // Combine factors to create output similar to original pitch mapping
        let combined = (position_factor * 0.3 + type_factor * 0.5 + size_factor * 0.2);
        (combined * u16::MAX as f32) as u16
    }
    
    // CHANGE: Convert collision value to normalized output
    // REASON: Domain-agnostic output
    fn collision_to_output(&self, value: f32) -> u16 {
        let normalized = (value + self.settings.collision_output_range / 2.0) / self.settings.collision_output_range;
        (normalized.clamp(0.0, 1.0) * u16::MAX as f32) as u16
    }
    
    // Activate a particle
    fn activate_particle(&mut self) {
        // Find inactive particle
        let mut particle_index = None;
        for i in 0..MAX_PARTICLES {
            if !self.particle_pool[i].active {
                particle_index = Some(i);
                break;
            }
        }
        
        if let Some(idx) = particle_index {
            // Generate random values before mutating the particle
            let size = self.random_int(
                self.settings.particle_min_size as i32, 
                self.settings.particle_max_size as i32
            ) as f32;
            // COMPAT: Exact same speed calculation as original
            let speed_factor = (1.5 * size + 3.0) / 10.0 * self.settings.gravity;
            let x = self.random_range(0.0, self.settings.screen_width as f32);
            // CHANGE: Using core::f32::consts::PI instead of std
            // REASON: no_std compatibility
            let sway = self.random() * 2.0 * core::f32::consts::PI;
            let sway_speed = self.random_range(
                self.settings.particle_sway_speed_min, 
                self.settings.particle_sway_speed_max
            );
            // CHANGE: particle_type instead of pitch, range 1-7 maintained
            // REASON: Domain-agnostic while maintaining same behavior
            let particle_type = self.random_int(1, 7) as u8;
            
            // Now update the particle
            let p = &mut self.particle_pool[idx];
            p.x = x;
            p.y = 0.0;
            p.base_speed = speed_factor;
            p.sway = sway;
            p.sway_speed = sway_speed;
            // COMPAT: Exact same wind sensitivity calculation
            p.wind_sensitivity = 0.7 + 0.3 / size;
            p.radius = size;
            p.particle_type = particle_type;
            p.last_collision_time = self.time - self.settings.collision_cooldown_time;
            p.active = true;
            self.active_particles += 1;
        }
    }
    
    // Activate dust
    fn activate_dust(&mut self) {
        // Find inactive dust
        let mut dust_index = None;
        for i in 0..MAX_DUST {
            if !self.dust_pool[i].active {
                dust_index = Some(i);
                break;
            }
        }
        
        if let Some(idx) = dust_index {
            // Generate random values before mutating
            let x = self.random_range(0.0, self.settings.screen_width as f32);
            let y = self.random_range(0.0, self.settings.ground_level as f32);
            let dx = (self.random() - 0.5) * self.settings.wind * self.settings.dust_dx_factor;
            let dy = (self.random() - 0.5) * self.settings.dust_dy_max;
            let brightness = self.random_int(1, self.settings.dust_brightness_max as i32) as u8;
            let life = self.random_range(self.settings.dust_life_min, self.settings.dust_life_max);
            
            // Now update the dust
            let d = &mut self.dust_pool[idx];
            d.x = x;
            d.y = y;
            d.dx = dx;
            d.dy = dy;
            d.brightness = brightness;
            d.life = life;
            d.active = true;
            self.active_dust += 1;
        }
    }
    
    // CHANGE: Update particles without Vec allocation
    // REASON: No heap allocation allowed
    // PERF: Use fixed-size buffer for particles to deactivate
    fn update_particles(&mut self, dt: f32) {
        // CHANGE: Fixed-size buffer instead of Vec
        // REASON: Avoid heap allocation
        let mut particles_to_deactivate: Vec<usize, MAX_PARTICLES> = Vec::new();
        
        for i in 0..MAX_PARTICLES {
            if self.particle_pool[i].active {
                let p = &mut self.particle_pool[i];
                
                // Update position - COMPAT: Identical physics
                p.y += p.base_speed * self.settings.global_fall_speed * dt;
                p.sway += p.sway_speed * dt;
                // CHANGE: Using libm::sinf for no_std
                // REASON: Core doesn't provide trig functions
                p.x += libm::sinf(p.sway) * self.settings.wind * p.wind_sensitivity * 10.0;
                
                // Handle borders - COMPAT: Identical boundary behavior
                if p.x < 0.0 {
                    p.x = 0.0;
                    p.sway += core::f32::consts::PI / 4.0;
                } else if p.x > self.settings.screen_width as f32 {
                    p.x = self.settings.screen_width as f32;
                    p.sway -= core::f32::consts::PI / 4.0;
                }
                
                // Check ground collision
                if p.y >= self.settings.ground_level as f32 {
                    // CHANGE: Generate normalized output instead of MIDI/voltage
                    // REASON: Domain-agnostic design
                    self.last_ground_output = Self::particle_to_output(&self.settings, p);
                    
                    // CHANGE: Format message using heapless write!
                    // REASON: No heap allocation
                    self.verbose_message.clear();
                    let _ = write!(
                        &mut self.verbose_message,
                        "Particle Output: {}, Trigger: HIGH", 
                        self.last_ground_output
                    );
                    
                    self.verbose_timer = self.settings.verbose_duration;
                    self.trigger_timer = self.settings.trigger_duration;
                    
                    // PERF: Try to add to deactivation list
                    let _ = particles_to_deactivate.push(i);
                }
            }
        }
        
        // Deactivate particles
        for &i in &particles_to_deactivate {
            self.particle_pool[i].active = false;
            self.active_particles -= 1;
        }
        
        // Spawn new particles - COMPAT: Same spawn logic
        if self.active_particles < self.settings.max_particles && 
           self.random() > (1.0 - self.settings.particle_spawn_chance) {
            self.activate_particle();
        }
    }
    
    // Update dust
    fn update_dust(&mut self, dt: f32) {
        for d in &mut self.dust_pool {
            if d.active {
                d.x += d.dx * dt;
                d.y += d.dy * dt;
                d.life -= dt;
                
                if d.life <= 0.0 {
                    d.active = false;
                    self.active_dust -= 1;
                }
            }
        }
        
        // Spawn new dust - COMPAT: Same spawn logic
        while self.active_dust < self.settings.max_particles * 8 && 
              self.active_dust < self.settings.max_dust {
            self.activate_dust();
        }
    }
    
    // Check collisions
    fn check_collisions(&mut self) {
        for i in 0..MAX_PARTICLES {
            if !self.particle_pool[i].active { continue; }
            
            for j in (i + 1)..MAX_PARTICLES {
                if !self.particle_pool[j].active { continue; }
                
                let p1 = self.particle_pool[i];
                let p2 = self.particle_pool[j];
                
                // Box collision detection - COMPAT: Identical collision logic
                if p1.x < p2.x + p2.radius &&
                   p1.x + p1.radius > p2.x &&
                   p1.y < p2.y + p2.radius &&
                   p1.y + p1.radius > p2.y 
                {
                    // Check cooldown
                    if self.time - p1.last_collision_time >= self.settings.collision_cooldown_time &&
                       self.time - p2.last_collision_time >= self.settings.collision_cooldown_time 
                    {
                        // CHANGE: Generate normalized collision output
                        // REASON: Domain-agnostic design
                        let collision_value = self.random_range(
                            -self.settings.collision_output_range / 2.0, 
                            self.settings.collision_output_range / 2.0
                        );
                        self.collision_output = self.collision_to_output(collision_value);
                        
                        // CHANGE: Format message using heapless
                        // REASON: No heap allocation
                        self.verbose_message.clear();
                        let _ = write!(
                            &mut self.verbose_message,
                            "Collision Output: {}, Trigger: HIGH", 
                            self.collision_output
                        );
                        
                        self.verbose_timer = self.settings.verbose_duration;
                        self.collision_trigger_timer = self.settings.trigger_duration;
                        
                        self.particle_pool[i].last_collision_time = self.time;
                        self.particle_pool[j].last_collision_time = self.time;
                    }
                }
            }
        }
    }
    
    // Update system
    pub fn update(&mut self, dt: f32) {
        self.time += dt;
        
        // Update timers
        if self.verbose_timer > 0.0 {
            self.verbose_timer -= dt;
        }
        if self.trigger_timer > 0.0 {
            self.trigger_timer -= dt;
        }
        if self.collision_trigger_timer > 0.0 {
            self.collision_trigger_timer -= dt;
        }
        
        self.update_particles(dt);
        self.update_dust(dt);
        self.check_collisions();
    }
    
    // CHANGE: Get current outputs as normalized values
    // REASON: Clean interface for embedded systems
    pub fn get_outputs(&self) -> (u16, u16, bool, bool) {
        (
            self.last_ground_output,
            self.collision_output,
            self.trigger_timer > 0.0,
            self.collision_trigger_timer > 0.0,
        )
    }
    
    // CHANGE: Update settings at runtime if needed
    // REASON: Support dynamic reconfiguration
    pub fn update_settings(&mut self, settings: Settings) {
        self.settings = settings;
    }
}

// CHANGE: Add module-level documentation
// REASON: Professional code quality
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_particle_system_creation() {
        let settings = Settings::default();
        let _system: ParticlesSystem<12, 50> = ParticlesSystem::new(settings);
    }
}