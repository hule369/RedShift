#![windows_subsystem = "windows"]

mod overlay;

use eframe::{egui, NativeOptions, IconData};
use std::process::{Command, Child};
use std::sync::Mutex;
use once_cell::sync::OnceCell;
use systray::Application;
use std::thread;
use std::sync::mpsc::{Sender, Receiver, channel};
use std::env;
use winapi::um::winuser::{GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN, SM_CYCAPTION};
use winreg::enums::*;
use winreg::RegKey;
use std::path::PathBuf;
use image::{self, ImageFormat};
use std::fs;
use std::io::Write;

// Define a custom error type that implements necessary traits
#[derive(Debug)]
struct MenuError(String);

impl std::fmt::Display for MenuError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for MenuError {}

// Make MenuError Send + Sync
unsafe impl Send for MenuError {}
unsafe impl Sync for MenuError {}

static WINDOW_VISIBLE: OnceCell<Mutex<bool>> = OnceCell::new();

struct ControllerApp {
    config: overlay::OverlayConfig,
    overlay_process: Option<Child>,
}

impl ControllerApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Create default font definitions
        let mut fonts = egui::FontDefinitions::default();
        
        // Insert Century Gothic Bold into font_data
        fonts.font_data.insert(
            "century_gothic_bold".to_owned(),
            egui::FontData::from_static(include_bytes!("../assets/fonts/century-gothic-bold.ttf")),
        );

        // Insert the font into the Proportional family at position 0
        fonts.families
            .get_mut(&egui::FontFamily::Proportional)
            .unwrap()
            .insert(0, "century_gothic_bold".to_owned());

        // Set the fonts
        cc.egui_ctx.set_fonts(fonts);

        // Continue with the existing initialization
        let config = load_config().unwrap_or_default();
        
        let mut app = Self {
            config,
            overlay_process: None,
        };
        
        if app.config.enabled {
            app.start_overlay();
        }
        
        app
    }

    fn start_overlay(&mut self) {
        if self.overlay_process.is_none() {
            let current_exe = std::env::current_exe().expect("Failed to get current executable path");
            
            self.overlay_process = Some(
                Command::new(&current_exe)
                    .arg("--overlay")
                    .arg("--opacity")
                    .arg(self.config.opacity.to_string())
                    .spawn()
                    .expect("Failed to start overlay process")
            );
        }
    }

    fn stop_overlay(&mut self) {
        if let Some(mut process) = self.overlay_process.take() {
            let _ = process.kill();
        }
    }

    fn update_opacity(&mut self, new_opacity: u8) {
        let clamped_opacity = new_opacity.clamp(90, 200);
        println!("Controller updating opacity to: {}", clamped_opacity);
        self.config.opacity = clamped_opacity;
        save_config(&self.config);
    }
}

impl eframe::App for ControllerApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        let mut style = (*ctx.style()).clone();
        style.text_styles = [
            (egui::TextStyle::Heading, egui::FontId::new(24.0, egui::FontFamily::Proportional)),
            (egui::TextStyle::Body, egui::FontId::new(16.0, egui::FontFamily::Proportional)),
            (egui::TextStyle::Button, egui::FontId::new(14.0, egui::FontFamily::Proportional)),
        ].into();
        
        // Pure black background
        style.visuals.window_fill = egui::Color32::from_rgb(0, 0, 0);
        style.visuals.panel_fill = egui::Color32::from_rgb(0, 0, 0);
        style.visuals.widgets.noninteractive.bg_fill = egui::Color32::from_rgb(15, 15, 15);
        style.visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(25, 25, 25);
        style.visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(35, 35, 35);
        style.visuals.widgets.active.bg_fill = egui::Color32::from_rgb(220, 40, 40);
        style.visuals.selection.bg_fill = egui::Color32::from_rgb(220, 40, 40);
        ctx.set_style(style.clone());

        // Window visibility check
        if let Some(visible) = WINDOW_VISIBLE.get() {
            let is_visible = *visible.lock().unwrap();
            if !is_visible {
                frame.set_visible(false);
                return;
            } else {
                frame.set_visible(true);
            }
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.spacing_mut().item_spacing = egui::vec2(0.0, 4.0);
            
            // Title with custom font
            ui.vertical_centered(|ui| {
                ui.heading(
                    egui::RichText::new("R E D S H I F T")
                        .color(egui::Color32::from_rgb(220, 40, 40))
                        .size(24.0)
                        .strong()
                        .text_style(egui::TextStyle::Heading)
                );
                
                // Add the new text here
                ui.label(
                    egui::RichText::new("Intensity")
                        .color(egui::Color32::WHITE)
                        .size(13.0)  // Half of 24.0
                        .text_style(egui::TextStyle::Heading)
                );
            });
            
            ui.add_space(4.0);
            
            // Define all positioning variables for opacity control
            let mut opacity = self.config.opacity as f32;
            
            // Window dimensions
            let window_width = 270.0;
            let window_height = 150.0;
            
            // Slider dimensions
            let slider_width = 220.0;
            let slider_height = 24.0;
            
            // Calculate positions
            let slider_x = 70.0;  // Center horizontally
            let slider_y = 55.0;  // Position from top
            
            // Custom slider style
            let mut slider_style = (*ctx.style()).clone();
            slider_style.visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(15, 15, 15);  // Darker background
            slider_style.visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(25, 25, 25);   // Slightly lighter when hovered
            slider_style.visuals.widgets.active.bg_fill = egui::Color32::from_rgb(220, 40, 40);   // Red when dragging
            slider_style.visuals.widgets.inactive.rounding = 2.0.into();  // Slightly rounded corners
            ctx.set_style(slider_style);

            // Draw slider with percentage
            if ui.put(
                egui::Rect::from_min_size(
                    egui::pos2(slider_x, slider_y),
                    egui::Vec2::new(slider_width, slider_height)
                ),
                egui::Slider::new(&mut opacity, 90.0..=200.0)
                    .text("")
                    .custom_formatter(|value, _| {
                        // Simple percentage calculation
                        let percentage = ((value - 90.0) / 110.0 * 100.0).round() as i32;
                        format!("{}%", percentage)
                    })
            ).changed() {
                self.update_opacity(opacity as u8);
            }

            ui.add_space(4.0);

            // Add space to push elements to bottom
            ui.add_space(ui.available_height() - 40.0);  // Reserve space for bottom row

            // Bottom row with checkboxes and minimize button
            ui.horizontal(|ui| {
                ui.add_space(0.0);  // Changed from 20.0 to -5.0 (moved 25 pixels left)
                
                // Left side: Checkboxes
                ui.vertical(|ui| {
                    let mut enabled = self.config.enabled;
                    ui.checkbox(&mut enabled, egui::RichText::new("Enable Overlay").size(14.0));
                    
                    if enabled != self.config.enabled {
                        self.config.enabled = enabled;
                        save_config(&self.config);
                        if enabled {
                            self.start_overlay();
                        } else {
                            self.stop_overlay();
                        }
                    }

                    let mut launch_on_startup = self.config.launch_on_startup;
                    ui.checkbox(&mut launch_on_startup, 
                        egui::RichText::new("Launch on Startup").size(14.0));
                    
                    if launch_on_startup != self.config.launch_on_startup {
                        self.config.launch_on_startup = launch_on_startup;
                        if let Err(e) = set_launch_on_startup(launch_on_startup) {
                            eprintln!("Failed to set startup: {}", e);
                        }
                        save_config(&self.config);
                    }
                });

                // Push minimize button to the right
                ui.with_layout(egui::Layout::right_to_left(egui::Align::RIGHT), |ui| {
                    ui.add_space(0.0);  // Reduced from 20.0 to 10.0 to move button right
                    if ui.button(
                        egui::RichText::new("Minimize to Tray")
                            .size(13.0)
                            .color(egui::Color32::from_rgb(180, 180, 180))
                    ).clicked() 
                    {
                        if let Some(visible) = WINDOW_VISIBLE.get() {
                            *visible.lock().unwrap() = false;
                        }
                        ctx.request_repaint();
                    }
                });
            });
        });
    }

    fn on_close_event(&mut self) -> bool {
        if let Some(visible) = WINDOW_VISIBLE.get() {
            *visible.lock().unwrap() = false;
        }
        false
    }
}

fn save_config(config: &overlay::OverlayConfig) {
    let config_path = overlay::config_path();
    let config_str = serde_json::to_string_pretty(config).unwrap();
    if let Err(e) = std::fs::write(&config_path, config_str) {
        eprintln!("Failed to save config: {}", e);
    }
}

fn load_config() -> Option<overlay::OverlayConfig> {
    let config_path = overlay::config_path();
    let config_str = std::fs::read_to_string(config_path).ok()?;
    serde_json::from_str(&config_str).ok()
}

enum TrayAction {
    ShowSettings,
    Exit,
}

fn kill_processes_by_name(name: &str) {
    if cfg!(target_os = "windows") {
        println!("Attempting to kill process: {}", name);
        let output = Command::new("taskkill")
            .args(["/F", "/IM", name])
            .output();
        
        match output {
            Ok(output) => {
                println!("Taskkill output: {}", String::from_utf8_lossy(&output.stdout));
                if !output.stderr.is_empty() {
                    eprintln!("Taskkill error: {}", String::from_utf8_lossy(&output.stderr));
                }
            }
            Err(e) => eprintln!("Failed to execute taskkill: {}", e),
        }
    }
}

fn set_launch_on_startup(enable: bool) -> Result<(), Box<dyn std::error::Error>> {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let path = "Software\\Microsoft\\Windows\\CurrentVersion\\Run";
    let (key, _) = hkcu.create_subkey(path)?;
    
    let exe_path = std::env::current_exe()?;
    let exe_path_str = exe_path.to_str().unwrap();

    if enable {
        key.set_value("RedShift", &exe_path_str)?;
    } else {
        key.delete_value("RedShift").ok(); // Ignore if not exists
    }
    Ok(())
}

const WINDOW_ICON_BYTES: &[u8] = include_bytes!("../assets/RSICONICO.ico");
const BUTTON_ICON_BYTES: &[u8] = include_bytes!("../assets/RSICONICO.ico");

fn load_icon() -> IconData {
    let image = image::load_from_memory_with_format(WINDOW_ICON_BYTES, ImageFormat::Ico)
        .expect("Failed to load embedded window icon");
    let image = image.to_rgba8();
    let (width, height) = image.dimensions();
    IconData {
        rgba: image.into_raw(),
        width,
        height,
    }
}

fn run_controller() {
    WINDOW_VISIBLE.set(Mutex::new(true)).unwrap();
    
    let (tx, _rx): (Sender<TrayAction>, Receiver<TrayAction>) = channel();
    
    thread::spawn(move || {
        let mut app = Application::new().expect("Systray initialization failed");
    
        // Write embedded icon to a temporary file
        let icon_path = {
            let mut path = std::env::temp_dir();
            path.push("RSICONICO_temp.ico");
            let mut file = fs::File::create(&path).expect("Failed to create temporary icon file");
            file.write_all(WINDOW_ICON_BYTES).expect("Failed to write icon data to temporary file");
            path
        };

        if let Err(e) = app.set_icon_from_file(&icon_path.to_string_lossy()) {
            eprintln!("Failed to set tray icon: {}", e);
        }
    
        let exe_dir = std::env::current_exe()
            .expect("Failed to get executable path")
            .parent()
            .expect("Failed to get executable directory")
            .to_path_buf();
        
        let icon_path = exe_dir.join("assets").join("icon").join("merk.ico");
        println!("Looking for icon at: {}", icon_path.display());
        
        if let Err(e) = app.set_icon_from_file(&icon_path.to_string_lossy()) {
            eprintln!("Failed to set tray icon: {}. Using default icon.", e);
            let absolute_path = std::path::PathBuf::from("G:/Cursor Projects/RedShiftBundle/redshiftbundle/assets/icon/merk.ico");
            if let Err(e) = app.set_icon_from_file(&absolute_path.to_string_lossy()) {
                eprintln!("Failed to set tray icon with absolute path: {}", e);
            }
        }
        
        let _ = app.set_tooltip("RedShift Controller");
    
        let tx_clone = tx.clone();
        if let Err(e) = app.add_menu_item("Show Settings", move |_| -> Result<(), MenuError> {
            if let Some(visible) = WINDOW_VISIBLE.get() {
                *visible.lock().unwrap() = true;
            }
            tx_clone.send(TrayAction::ShowSettings)
                .map_err(|e| MenuError(e.to_string()))?;
            Ok(())
        }) {
            eprintln!("Failed to add Show Settings menu item: {}", e);
        }
    
        let tx_clone = tx.clone();
        if let Err(e) = app.add_menu_item("Exit", move |_| -> Result<(), MenuError> {
            kill_processes_by_name("redshift.exe");
            
            thread::sleep(std::time::Duration::from_millis(100));
            
            tx_clone.send(TrayAction::Exit)
                .map_err(|e| MenuError(e.to_string()))?;
            
            std::process::exit(0);
            
            #[allow(unreachable_code)]
            Ok(())
        }) {
            eprintln!("Failed to add Exit menu item: {}", e);
        }
    
        let _ = app.wait_for_message();
    });

    // Get screen dimensions using WinAPI
    let screen_width;
    let screen_height;
    let taskbar_height = 90;     // Tripled from 60 to raise the window much higher
    let right_margin = 60.0;      // Tripled from 20 to move further from right edge
    
    unsafe {
        screen_width = GetSystemMetrics(SM_CXSCREEN);
        screen_height = GetSystemMetrics(SM_CYSCREEN);
    }

    let window_width = 270.0;
    let window_height = 150.0;
    
    // Calculate position (bottom right, above taskbar)
    let x = (screen_width as f32) - window_width - right_margin;  // Move left by right_margin
    let y = (screen_height as f32) - window_height - taskbar_height as f32;  // Raise higher

    let options = NativeOptions {
        initial_window_size: Some(egui::vec2(window_width, window_height)),
        initial_window_pos: Some(egui::pos2(x, y)),
        resizable: false,
        decorated: true,
        transparent: true,
        always_on_top: true,
        min_window_size: Some(egui::vec2(window_width, window_height)),
        max_window_size: Some(egui::vec2(window_width, window_height)),
        icon_data: Some(load_icon()),
        ..Default::default()
    };

    let app_creator: Box<dyn FnOnce(&eframe::CreationContext) -> Box<dyn eframe::App>> = 
        Box::new(|cc| Box::new(ControllerApp::new(cc)));

    eframe::run_native(
        "RedShift Controller",
        options,
        app_creator
    ).expect("Failed to run eframe");
}

fn main() {
    let args: Vec<String> = env::args().collect();
    
    if args.len() > 1 && args[1] == "--overlay" {
        overlay::run();
    } else {
        run_controller();
    }
}