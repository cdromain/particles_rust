//! main.rs - Desktop simulator for particles algorithm
//! Handles rendering, timing, and user interface

use embedded_graphics::{
    pixelcolor::Rgb565,
    prelude::*,
    primitives::{Rectangle, PrimitiveStyle, Line},
    text::{Text, Baseline},
    mono_font::{ascii::FONT_6X10, MonoTextStyle},
    Pixel,
};
use embedded_graphics_simulator::{
    SimulatorDisplay, Window, OutputSettingsBuilder, SimulatorEvent,
};
use std::time::{Duration, Instant};
use std::thread;

// CHANGE: Import particles module
// REASON: Separation of concerns - main handles UI, particles handles algorithm
use particles_rust::{ParticlesSystem, Settings};

// CHANGE: Separate UI state from particle system
// REASON: Clean separation between rendering and algorithm
struct UiState {
    // User adjustable parameters
    gravity: f32,
    wind: f32,
    max_particles: usize,
    verbose: bool,
    
    // Display info
    info_message: String,
}

impl Default for UiState {
    fn default() -> Self {
        Self {
            gravity: 1.0,
            wind: 0.1,
            max_particles: 6,
            verbose: false,
            info_message: String::new(),
        }
    }
}

// CHANGE: Extract rendering logic to separate function
// REASON: Modularity and maintainability
fn render_particles<const MAX_PARTICLES: usize, const MAX_DUST: usize>(
    display: &mut SimulatorDisplay<Rgb565>,
    system: &ParticlesSystem<MAX_PARTICLES, MAX_DUST>,
    ui: &UiState,
    settings: &Settings,
) {
    // Theme colors
    let bg_color = Rgb565::BLACK;
    let ground_color = Rgb565::new(0, 10, 15);
    let text_color = Rgb565::new(0, 31, 63);
    
    // Clear screen
    display.clear(bg_color).unwrap();
    
    // Draw ground line
    Line::new(
        Point::new(0, settings.ground_level),
        Point::new(settings.screen_width, settings.ground_level),
    )
    .into_styled(PrimitiveStyle::with_stroke(ground_color, 1))
    .draw(display).unwrap();
    
    // Draw particles
    for particle in &system.particle_pool {
        if particle.active {
            // COMPAT: Same brightness calculation as original
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
    for dust in &system.dust_pool {
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
        Point::new(5, settings.screen_height - 15),
        style,
        Baseline::Top,
    )
    .draw(display).unwrap();
    
    // CHANGE: Display normalized outputs instead of scale/pitch
    // REASON: Domain-agnostic display
    let (ground_output, collision_output, _, _) = system.get_outputs();
    let output_text = format!("Output: {} / {}", ground_output, collision_output);
    Text::with_baseline(
        &output_text,
        Point::new(5, 5),
        style,
        Baseline::Top,
    )
    .draw(display).unwrap();
    
    // Gravity display (top right)
    let gravity_text = format!("Gravity: {:.1}", ui.gravity);
    Text::with_baseline(
        &gravity_text,
        Point::new(settings.screen_width - 80, 5),
        style,
        Baseline::Top,
    )
    .draw(display).unwrap();
    
    // Verbose message
    if ui.verbose && system.verbose_timer > 0.0 {
        Text::with_baseline(
            system.verbose_message.as_str(),
            Point::new(5, settings.screen_height - 30),
            style,
            Baseline::Top,
        )
        .draw(display).unwrap();
    }
    
    // Instructions
    let instructions_style = MonoTextStyle::new(&FONT_6X10, Rgb565::new(0, 20, 40));
    Text::with_baseline(
        "Space: Verbose | G: Gravity | W: Wind | P: Particles | Q: Quit",
        Point::new(5, settings.screen_height - 5),
        instructions_style,
        Baseline::Top,
    )
    .draw(display).unwrap();
}

fn main() {
    // CHANGE: Initialize settings with defaults
    // REASON: Configuration externalization
    let mut settings = Settings::default();
    
    // Create display
    let mut display = SimulatorDisplay::new(Size::new(
        settings.screen_width as u32, 
        settings.screen_height as u32
    ));
    let output_settings = OutputSettingsBuilder::new().scale(3).build();
    let mut window = Window::new("Particles - Generative Algorithm", &output_settings);
    
    // CHANGE: Create system with explicit const generics
    // REASON: Compile-time array size specification
    let mut system: ParticlesSystem<12, 50> = ParticlesSystem::new(settings);
    let mut ui = UiState::default();
    
    // Timing
    let mut last_update = Instant::now();
    let target_fps = 60;
    let frame_duration = Duration::from_secs_f32(1.0 / target_fps as f32);
    
    println!("=== Particles - Generative Algorithm (Refactored) ===");
    println!("Controls:");
    println!("  Space: Toggle verbose mode");
    println!("  G: Adjust gravity");
    println!("  W: Adjust wind");
    println!("  P: Adjust max particles");
    println!("  Q: Quit");
    println!("\nNOTE: This refactored version outputs normalized u16 values");
    println!("instead of pitch/scale for embedded system compatibility.");
    
    'main_loop: loop {
        let now = Instant::now();
        let dt = now.duration_since(last_update).as_secs_f32();
        last_update = now;
        
        // Update physics
        system.update(dt);
        
        // Render
        render_particles(&mut display, &system, &ui, &settings);
        window.update(&display);
        
        // Handle events
        for event in window.events() {
            match event {
                SimulatorEvent::Quit => break 'main_loop,
                SimulatorEvent::KeyDown { keycode, .. } => {
                    let key = format!("{:?}", keycode).to_lowercase();
                    match key.as_str() {
                        // Space - toggle verbose
                        "space" => {
                            ui.verbose = !ui.verbose;
                            system.verbose = ui.verbose;
                            println!("Verbose mode: {}", if ui.verbose { "ON" } else { "OFF" });
                        }
                        // G - gravity
                        "g" => {
                            // PERF: Cycle through preset values instead of float increment
                            ui.gravity = match ui.gravity {
                                g if g >= 5.0 => 0.5,
                                g if g >= 4.5 => 5.0,
                                g if g >= 4.0 => 4.5,
                                g if g >= 3.5 => 4.0,
                                g if g >= 3.0 => 3.5,
                                g if g >= 2.5 => 3.0,
                                g if g >= 2.0 => 2.5,
                                g if g >= 1.5 => 2.0,
                                g if g >= 1.0 => 1.5,
                                g if g >= 0.5 => 1.0,
                                _ => 0.5,
                            };
                            settings.gravity = ui.gravity;
                            system.update_settings(settings);
                            println!("Gravity: {:.1}", ui.gravity);
                        }
                        // W - wind
                        "w" => {
                            // PERF: Cycle through preset values
                            ui.wind = match ui.wind {
                                w if w >= 1.0 => 0.0,
                                w if w >= 0.9 => 1.0,
                                w if w >= 0.8 => 0.9,
                                w if w >= 0.7 => 0.8,
                                w if w >= 0.6 => 0.7,
                                w if w >= 0.5 => 0.6,
                                w if w >= 0.4 => 0.5,
                                w if w >= 0.3 => 0.4,
                                w if w >= 0.2 => 0.3,
                                w if w >= 0.1 => 0.2,
                                _ => 0.1,
                            };
                            settings.wind = ui.wind;
                            system.update_settings(settings);
                            println!("Wind: {:.1}", ui.wind);
                        }
                        // P - max particles
                        "p" => {
                            ui.max_particles = if ui.max_particles >= 12 { 1 } else { ui.max_particles + 1 };
                            settings.max_particles = ui.max_particles;
                            system.update_settings(settings);
                            println!("Max particles: {}", ui.max_particles);
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