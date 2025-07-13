//! particles - Desktop simulator version
//! A nature-inspired generative algorithm
//! Run with: cargo run --release

use embedded_graphics::{
    pixelcolor::Rgb565,
    prelude::*,
    primitives::{Rectangle, PrimitiveStyle, Line},
    text::{Text, Baseline},
    mono_font::{ascii::FONT_6X10, MonoTextStyle},
    Pixel,  // Import Pixel directly from embedded_graphics
};
use embedded_graphics_simulator::{
    SimulatorDisplay, Window, OutputSettingsBuilder, SimulatorEvent,
};
use std::time::{Duration, Instant};
use std::thread;

// Constants
const MAX_PARTICLES: usize = 12;
const MAX_DUST: usize = 50;
const SCREEN_WIDTH: i32 = 320;
const SCREEN_HEIGHT: i32 = 170;
const GROUND_LEVEL: i32 = 150;
const COLLISION_COOLDOWN_TIME: f32 = 3.0;
const TRIGGER_DURATION: f32 = 0.05;
const VERBOSE_DURATION: f32 = 1.0;

// Scale definitions
#[derive(Copy, Clone)]
enum Scale {
    Minor,
    Major,
    Dorian,
    Phrygian,
    Lydian,
    Mixolydian,
    Locrian,
    HarmonicMinor,
    MelodicMinor,
}

impl Scale {
    fn intervals(&self) -> &'static [u8] {
        match self {
            Scale::Minor => &[0, 2, 3, 5, 7, 8, 10],
            Scale::Major => &[0, 2, 4, 5, 7, 9, 11],
            Scale::Dorian => &[0, 2, 3, 5, 7, 9, 10],
            Scale::Phrygian => &[0, 1, 3, 5, 7, 8, 10],
            Scale::Lydian => &[0, 2, 4, 6, 7, 9, 11],
            Scale::Mixolydian => &[0, 2, 4, 5, 7, 9, 10],
            Scale::Locrian => &[0, 1, 3, 5, 6, 8, 10],
            Scale::HarmonicMinor => &[0, 2, 3, 5, 7, 8, 11],
            Scale::MelodicMinor => &[0, 2, 3, 5, 7, 9, 11],
        }
    }

    fn name(&self) -> &'static str {
        match self {
            Scale::Minor => "Minor",
            Scale::Major => "Major",
            Scale::Dorian => "Dorian",
            Scale::Phrygian => "Phrygian",
            Scale::Lydian => "Lydian",
            Scale::Mixolydian => "Mixolydian",
            Scale::Locrian => "Locrian",
            Scale::HarmonicMinor => "Harmonic Minor",
            Scale::MelodicMinor => "Melodic Minor",
        }
    }
}

// Particle structure
#[derive(Copy, Clone)]
struct Particle {
    x: f32,
    y: f32,
    base_speed: f32,
    sway: f32,
    sway_speed: f32,
    wind_sensitivity: f32,
    radius: f32,
    pitch: u8,
    last_collision_time: f32,
    active: bool,
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
            pitch: 0,
            last_collision_time: 0.0,
            active: false,
        }
    }
}

// Dust speck structure
#[derive(Copy, Clone)]
struct Dust {
    x: f32,
    y: f32,
    dx: f32,
    dy: f32,
    brightness: u8,
    life: f32,
    active: bool,
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

// Main particles system
struct ParticlesSystem {
    // Object pools
    particle_pool: [Particle; MAX_PARTICLES],
    dust_pool: [Dust; MAX_DUST],
    active_particles: usize,
    active_dust: usize,
    
    // Timing
    time: f32,
    trigger_timer: f32,
    collision_trigger_timer: f32,
    verbose_timer: f32,
    
    // Outputs
    last_ground_pitch_voltage: f32,
    collision_cv: f32,
    
    // Messages
    verbose_message: String,
    
    // Parameters
    root_note: u8,
    octave: u8,
    scale: Scale,
    global_fall_speed: f32,
    gravity: f32,
    max_particles: usize,
    wind: f32,
    verbose: bool,
    
    // Random state
    rng_state: u32,
}

impl ParticlesSystem {
    fn new() -> Self {
        Self {
            particle_pool: [Particle::default(); MAX_PARTICLES],
            dust_pool: [Dust::default(); MAX_DUST],
            active_particles: 0,
            active_dust: 0,
            time: 0.0,
            trigger_timer: 0.0,
            collision_trigger_timer: 0.0,
            verbose_timer: 0.0,
            last_ground_pitch_voltage: 0.0,
            collision_cv: 0.0,
            verbose_message: String::new(),
            root_note: 0,
            octave: 2,
            scale: Scale::Minor,
            global_fall_speed: 5.0,
            gravity: 1.0,
            max_particles: 6,
            wind: 0.1,
            verbose: false,
            rng_state: 0x12345678,
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
    
    // Convert MIDI note to voltage (1V/oct)
    fn note_to_voltage(note: u8) -> f32 {
        note as f32 / 12.0
    }
    
    // Convert scale degree to MIDI note
    fn scale_to_midi(scale_degree: u8, scale: Scale, root: u8, octave: u8) -> u8 {
        let intervals = scale.intervals();
        let degree_index = ((scale_degree - 1) as usize) % intervals.len();
        let note_in_scale = intervals[degree_index];
        (octave * 12) + root + note_in_scale
    }
    
    // Activate a particle - fixed borrow checker issues
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
            let size = self.random_int(3, 10) as f32;
            let speed_factor = (1.5 * size + 3.0) / 10.0 * self.gravity;
            let x = self.random_range(0.0, SCREEN_WIDTH as f32);
            let sway = self.random() * 2.0 * std::f32::consts::PI;
            let sway_speed = self.random_range(0.1, 0.3);
            let pitch = self.random_int(1, self.scale.intervals().len() as i32) as u8;
            
            // Now update the particle
            let p = &mut self.particle_pool[idx];
            p.x = x;
            p.y = 0.0;
            p.base_speed = speed_factor;
            p.sway = sway;
            p.sway_speed = sway_speed;
            p.wind_sensitivity = 0.7 + 0.3 / size;
            p.radius = size;
            p.pitch = pitch;
            p.last_collision_time = self.time - COLLISION_COOLDOWN_TIME;
            p.active = true;
            self.active_particles += 1;
        }
    }
    
    // Activate dust - fixed borrow checker issues
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
            let x = self.random_range(0.0, SCREEN_WIDTH as f32);
            let y = self.random_range(0.0, GROUND_LEVEL as f32);
            let dx = (self.random() - 0.5) * self.wind * 10.0;
            let dy = (self.random() - 0.5) * 5.0;
            let brightness = self.random_int(1, 5) as u8;
            let life = self.random_range(3.0, 10.0);
            
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
    
    // Update particles
    fn update_particles(&mut self, dt: f32) {
        let mut particles_to_deactivate = Vec::new();
        
        for i in 0..MAX_PARTICLES {
            if self.particle_pool[i].active {
                let p = &mut self.particle_pool[i];
                
                // Update position
                p.y += p.base_speed * self.global_fall_speed * dt;
                p.sway += p.sway_speed * dt;
                p.x += p.sway.sin() * self.wind * p.wind_sensitivity * 10.0;
                
                // Handle borders
                if p.x < 0.0 {
                    p.x = 0.0;
                    p.sway += std::f32::consts::PI / 4.0;
                } else if p.x > SCREEN_WIDTH as f32 {
                    p.x = SCREEN_WIDTH as f32;
                    p.sway -= std::f32::consts::PI / 4.0;
                }
                
                // Check ground collision
                if p.y >= GROUND_LEVEL as f32 {
                    let midi_note = Self::scale_to_midi(
                        p.pitch, 
                        self.scale, 
                        self.root_note, 
                        self.octave
                    );
                    self.last_ground_pitch_voltage = Self::note_to_voltage(midi_note);
                    
                    self.verbose_message = format!(
                        "Particle CV: {:.2}V, Trigger: 5V", 
                        self.last_ground_pitch_voltage
                    );
                    
                    self.verbose_timer = VERBOSE_DURATION;
                    self.trigger_timer = TRIGGER_DURATION;
                    particles_to_deactivate.push(i);
                }
            }
        }
        
        // Deactivate particles
        for &i in &particles_to_deactivate {
            self.particle_pool[i].active = false;
            self.active_particles -= 1;
        }
        
        // Spawn new particles
        if self.active_particles < self.max_particles && self.random() > 0.8 {
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
        
        // Spawn new dust
        while self.active_dust < self.max_particles * 8 && self.active_dust < MAX_DUST {
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
                
                // Box collision detection
                if p1.x < p2.x + p2.radius &&
                   p1.x + p1.radius > p2.x &&
                   p1.y < p2.y + p2.radius &&
                   p1.y + p1.radius > p2.y 
                {
                    // Check cooldown
                    if self.time - p1.last_collision_time >= COLLISION_COOLDOWN_TIME &&
                       self.time - p2.last_collision_time >= COLLISION_COOLDOWN_TIME 
                    {
                        self.collision_cv = self.random_range(-5.0, 5.0);
                        self.verbose_message = format!(
                            "Collision CV: {:.2}V, Trigger: 5V", 
                            self.collision_cv
                        );
                        
                        self.verbose_timer = VERBOSE_DURATION;
                        self.collision_trigger_timer = TRIGGER_DURATION;
                        
                        self.particle_pool[i].last_collision_time = self.time;
                        self.particle_pool[j].last_collision_time = self.time;
                    }
                }
            }
        }
    }
    
    // Update system
    fn update(&mut self, dt: f32) {
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
    
    // Render
    fn render(&self, display: &mut SimulatorDisplay<Rgb565>) {
        // Theme colors
        let bg_color = Rgb565::BLACK;
        let ground_color = Rgb565::new(0, 10, 15);
        let text_color = Rgb565::new(0, 31, 63);
        
        // Clear screen
        display.clear(bg_color).unwrap();
        
        // Draw ground line
        Line::new(
            Point::new(0, GROUND_LEVEL),
            Point::new(SCREEN_WIDTH, GROUND_LEVEL),
        )
        .into_styled(PrimitiveStyle::with_stroke(ground_color, 1))
        .draw(display).unwrap();
        
        // Draw particles
        for particle in &self.particle_pool {
            if particle.active {
                let brightness = ((particle.radius * 1.5) as u8).min(31);
                let color = Rgb565::new(0, brightness * 2, brightness * 3);
                
                Rectangle::new(
                    Point::new(particle.x as i32, particle.y as i32),
                    Size::new(particle.radius as u32, particle.radius as u32),
                )
                .into_styled(PrimitiveStyle::with_fill(color))
                .draw(display).unwrap();
            }
        }
        
        // Draw dust
        for dust in &self.dust_pool {
            if dust.active {
                let brightness = dust.brightness.min(31);
                let color = Rgb565::new(0, brightness, brightness * 2);
                
                Pixel(Point::new(dust.x as i32, dust.y as i32), color)
                    .draw(display).unwrap();
            }
        }
        
        // Draw UI
        let style = MonoTextStyle::new(&FONT_6X10, text_color);
        
        // Title
        Text::with_baseline(
            "particles",
            Point::new(5, SCREEN_HEIGHT - 15),
            style,
            Baseline::Top,
        )
        .draw(display).unwrap();
        
        // Scale info (top left)
        let note_names = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"];
        let scale_text = format!("{} {}{}", self.scale.name(), 
            note_names[self.root_note as usize],
            self.octave
        );
        Text::with_baseline(
            &scale_text,
            Point::new(5, 5),
            style,
            Baseline::Top,
        )
        .draw(display).unwrap();
        
        // Gravity display (top right)
        let gravity_text = format!("Gravity: {:.1}", self.gravity);
        Text::with_baseline(
            &gravity_text,
            Point::new(SCREEN_WIDTH - 80, 5),
            style,
            Baseline::Top,
        )
        .draw(display).unwrap();
        
        // Verbose message
        if self.verbose && self.verbose_timer > 0.0 {
            Text::with_baseline(
                &self.verbose_message,
                Point::new(5, SCREEN_HEIGHT - 30),
                style,
                Baseline::Top,
            )
            .draw(display).unwrap();
        }
        
        // Instructions
        let instructions_style = MonoTextStyle::new(&FONT_6X10, Rgb565::new(0, 20, 40));
        Text::with_baseline(
            "Space: Toggle Verbose | R/O/S/G/W: Adjust params | Q: Quit",
            Point::new(5, SCREEN_HEIGHT - 5),
            instructions_style,
            Baseline::Top,
        )
        .draw(display).unwrap();
    }
}

fn main() {
    // Create display
    let mut display = SimulatorDisplay::new(Size::new(SCREEN_WIDTH as u32, SCREEN_HEIGHT as u32));
    let output_settings = OutputSettingsBuilder::new().scale(3).build();
    let mut window = Window::new("Particles - Generative Algorithm", &output_settings);
    
    // Create system
    let mut system = ParticlesSystem::new();
    
    // Timing
    let mut last_update = Instant::now();
    let target_fps = 60;
    let frame_duration = Duration::from_secs_f32(1.0 / target_fps as f32);
    
    println!("=== Particles - Generative Algorithm ===");
    println!("Controls:");
    println!("  Space: Toggle verbose mode");
    println!("  R: Change root note");
    println!("  O: Change octave");
    println!("  S: Change scale");
    println!("  G: Adjust gravity");
    println!("  W: Adjust wind");
    println!("  Q: Quit");
    
    'main_loop: loop {
        let now = Instant::now();
        let dt = now.duration_since(last_update).as_secs_f32();
        last_update = now;
        
        // Update physics
        system.update(dt);
        
        // Render
        system.render(&mut display);
        window.update(&display);
        
        // Handle events
        for event in window.events() {
            match event {
                SimulatorEvent::Quit => break 'main_loop,
                SimulatorEvent::KeyDown { keycode, .. } => {
                    // Convert keycode to string and match (case insensitive)
                    let key = format!("{:?}", keycode).to_lowercase();
                    match key.as_str() {
                        // Space - toggle verbose
                        "space" => {
                            system.verbose = !system.verbose;
                            println!("Verbose mode: {}", if system.verbose { "ON" } else { "OFF" });
                        }
                        // R - root note
                        "r" => {
                            system.root_note = (system.root_note + 1) % 12;
                            let note_names = ["C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B"];
                            println!("Root note: {}", note_names[system.root_note as usize]);
                        }
                        // O - octave
                        "o" => {
                            system.octave = (system.octave % 8) + 1;
                            println!("Octave: {}", system.octave);
                        }
                        // S - scale
                        "s" => {
                            system.scale = match system.scale {
                                Scale::Minor => Scale::Major,
                                Scale::Major => Scale::Dorian,
                                Scale::Dorian => Scale::Phrygian,
                                Scale::Phrygian => Scale::Lydian,
                                Scale::Lydian => Scale::Mixolydian,
                                Scale::Mixolydian => Scale::Locrian,
                                Scale::Locrian => Scale::HarmonicMinor,
                                Scale::HarmonicMinor => Scale::MelodicMinor,
                                Scale::MelodicMinor => Scale::Minor,
                            };
                            println!("Scale: {}", system.scale.name());
                        }
                        // G - gravity
                        "g" => {
                            system.gravity = if system.gravity >= 5.0 { 0.5 } else { system.gravity + 0.5 };
                            println!("Gravity: {:.1}", system.gravity);
                        }
                        // W - wind
                        "w" => {
                            system.wind = if system.wind >= 1.0 { 0.0 } else { system.wind + 0.1 };
                            println!("Wind: {:.1}", system.wind);
                        }
                        // Q - quit
                        "q" => break 'main_loop,
                        _ => {}
                    }
                }
                _ => {}
            }
        }
        
        // Frame rate limiting
        let elapsed = now.elapsed();
        if elapsed < frame_duration {
            thread::sleep(frame_duration - elapsed);
        }
    }
    
    println!("Thanks for playing with particles!");
}