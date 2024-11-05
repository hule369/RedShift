use pixels::{Pixels, SurfaceTexture};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU8, Ordering};
use std::thread;
use std::time::Duration;
use std::fs;
use std::path::{PathBuf, Path};
use serde::{Serialize, Deserialize};
use winit::{
    dpi::{LogicalPosition, LogicalSize},
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    monitor::MonitorHandle,
    window::{WindowBuilder, WindowId},
};
use winit::platform::windows::WindowExtWindows;

use winapi::{
    shared::windef::HWND,
    um::winuser::{
        GetWindowLongW, SetWindowLongW, SetWindowPos, GWL_EXSTYLE, SWP_NOMOVE, SWP_NOSIZE,
        SWP_NOACTIVATE, WS_EX_LAYERED, WS_EX_TRANSPARENT, WS_EX_TOOLWINDOW, WS_EX_TOPMOST,
        WS_EX_NOACTIVATE, SetLayeredWindowAttributes, LWA_ALPHA, HWND_TOPMOST,
    },
};

// Constants for default red color and opacity
const DEFAULT_RED: u8 = 255;
const DEFAULT_GREEN: u8 = 0;
const DEFAULT_BLUE: u8 = 0;
static CURRENT_ALPHA: AtomicU8 = AtomicU8::new(100);

#[derive(Serialize, Deserialize, Default)]
pub struct OverlayConfig {
    #[serde(default = "default_opacity")]
    pub opacity: u8,
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub launch_on_startup: bool,
}

// Add default value functions
fn default_opacity() -> u8 {
    90 // Default opacity value now starts at minimum allowed
}

// Add a function to clamp opacity values
fn clamp_opacity(opacity: u8) -> u8 {
    opacity.clamp(90, 200)
}

pub fn config_path() -> PathBuf {
    let path = if let Ok(local_app_data) = std::env::var("LOCALAPPDATA") {
        PathBuf::from(local_app_data)
    } else {
        std::env::current_exe()
            .unwrap_or_else(|_| PathBuf::from("."))
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .to_path_buf()
    };
    
    let mut config_path = path;
    config_path.push("RedShift");
    
    // Create directory if it doesn't exist
    if let Err(e) = std::fs::create_dir_all(&config_path) {
        eprintln!("Failed to create config directory: {}", e);
    }
    
    config_path.push("config.json");
    config_path
}

fn watch_opacity_changes() {
    thread::spawn(|| {
        let config_path = config_path();
        let mut last_opacity = CURRENT_ALPHA.load(Ordering::Relaxed);
        println!("Starting opacity watcher with initial opacity: {}", last_opacity);

        loop {
            thread::sleep(Duration::from_millis(100)); // Adjust the frequency as needed
            if let Ok(config_str) = fs::read_to_string(&config_path) {
                if let Ok(config) = serde_json::from_str::<OverlayConfig>(&config_str) {
                    let new_opacity = clamp_opacity(config.opacity);
                    if new_opacity != last_opacity {
                        println!("Opacity changed: {} -> {}", last_opacity, new_opacity);
                        CURRENT_ALPHA.store(new_opacity, Ordering::Relaxed);
                        last_opacity = new_opacity;
                    }
                }
            }
        }
    });
}

pub fn run() {
    watch_opacity_changes();
    let event_loop = EventLoop::new();
    let monitors: Vec<MonitorHandle> = event_loop.available_monitors().collect();
    let mut windows: HashMap<WindowId, (winit::window::Window, Pixels)> = HashMap::new();

    for monitor in monitors {
        create_overlay_window(&event_loop, &monitor, &mut windows);
    }

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;

        match event {
            Event::WindowEvent { event, window_id, .. } => match event {
                WindowEvent::CloseRequested => {
                    *control_flow = ControlFlow::Exit;
                }
                WindowEvent::Resized(size) => {
                    if let Some((_window, pixels)) = windows.get_mut(&window_id) {
                        pixels.resize_surface(size.width, size.height).unwrap_or_else(|e| {
                            eprintln!("Failed to resize surface: {}", e);
                        });
                    }
                }
                _ => (),
            },
            Event::MainEventsCleared => {
                for (_id, (window, pixels)) in windows.iter_mut() {
                    let current_opacity = CURRENT_ALPHA.load(Ordering::Relaxed);
                    
                    let frame = pixels.frame_mut();
                    for pixel in frame.chunks_exact_mut(4) {
                        pixel.copy_from_slice(&[
                            DEFAULT_RED,
                            DEFAULT_GREEN,
                            DEFAULT_BLUE,
                            current_opacity
                        ]);
                    }
                    
                    if let Err(e) = pixels.render() {
                        eprintln!("Failed to render pixels: {}", e);
                    }

                    unsafe {
                        let hwnd = window.hwnd() as HWND;
                        SetLayeredWindowAttributes(
                            hwnd,
                            0,
                            current_opacity,
                            LWA_ALPHA
                        );

                        // Keep window on top
                        SetWindowPos(
                            hwnd,
                            HWND_TOPMOST,
                            0,
                            0,
                            0,
                            0,
                            SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
                        );
                    }

                    window.request_redraw();
                }
            }
            Event::RedrawRequested(_) => {
                // Handled in MainEventsCleared
            }
            _ => (),
        }
    });
}

fn create_overlay_window(
    event_loop: &EventLoop<()>,
    monitor: &MonitorHandle,
    windows: &mut HashMap<WindowId, (winit::window::Window, Pixels)>,
) {
    let size = monitor.size();
    let position = monitor.position();

    let mut window_builder = WindowBuilder::new()
        .with_title("Red Overlay")
        .with_inner_size(LogicalSize::new(size.width as f64, size.height as f64))
        .with_position(LogicalPosition::new(position.x as f64, position.y as f64))
        .with_decorations(false)
        .with_transparent(true);

    #[cfg(target_os = "windows")]
    {
        use winit::platform::windows::WindowBuilderExtWindows;
        window_builder = window_builder.with_skip_taskbar(true);
    }

    let window = window_builder.build(event_loop).expect("Failed to build window");

    #[cfg(target_os = "windows")]
    {
        let hwnd = window.hwnd() as HWND;
        let ex_style = unsafe { GetWindowLongW(hwnd, GWL_EXSTYLE) } as u32;
        let new_ex_style = ex_style
            | WS_EX_LAYERED
            | WS_EX_TRANSPARENT
            | WS_EX_TOOLWINDOW
            | WS_EX_TOPMOST
            | WS_EX_NOACTIVATE;

        unsafe {
            SetWindowLongW(hwnd, GWL_EXSTYLE, new_ex_style as i32);
            SetLayeredWindowAttributes(hwnd, 0, CURRENT_ALPHA.load(Ordering::Relaxed), LWA_ALPHA);
            
            // Keep window on top
            SetWindowPos(
                hwnd,
                HWND_TOPMOST,
                0,
                0,
                0,
                0,
                SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
            );
        }

        // Store the HWND value for the background thread
        let hwnd_raw = hwnd as isize;
        thread::spawn(move || {
            loop {
                thread::sleep(Duration::from_millis(1000)); // Adjust the frequency as needed
                unsafe {
                    SetWindowPos(
                        hwnd_raw as HWND,
                        HWND_TOPMOST,
                        0,
                        0,
                        0,
                        0,
                        SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
                    );
                }
            }
        });
    }

    let window_size = window.inner_size();
    let surface_texture = SurfaceTexture::new(window_size.width, window_size.height, &window);
    let pixels = Pixels::new(
        window_size.width,
        window_size.height,
        surface_texture,
    )
    .expect("Failed to create Pixels");

    windows.insert(window.id(), (window, pixels));
}