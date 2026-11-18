//! A developer aid: with `GTAVI_COUNTDOWN_SCREENSHOT=out.png` set, the app
//! saves a screenshot of its window once the scene has faded in, then quits.
//! The README screenshots are taken this way.

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use eframe::egui::{self, ViewportCommand};

/// Long enough for the scene to fade in and the first tick to land.
const DELAY: Duration = Duration::from_millis(2500);

pub struct Screenshot {
    path: PathBuf,
    started: Instant,
    requested: bool,
    done: Arc<AtomicBool>,
}

impl Screenshot {
    pub fn from_env() -> Option<Self> {
        let path = std::env::var_os("GTAVI_COUNTDOWN_SCREENSHOT")?;
        Some(Screenshot { path: path.into(), started: Instant::now(), requested: false, done: Arc::default() })
    }

    pub fn update(&mut self, ctx: &egui::Context) {
        if self.done.load(Ordering::Relaxed) {
            ctx.send_viewport_cmd(ViewportCommand::Close);
            return;
        }
        if !self.requested && self.started.elapsed() >= DELAY {
            self.requested = true;
            let (path, done, repaint) = (self.path.clone(), self.done.clone(), ctx.clone());
            ctx.send_viewport_cmd(ViewportCommand::Screenshot(egui::ScreenshotCallback::new(move |image| {
                let [width, height] = image.size;
                let pixels: Vec<u8> = image.pixels.iter().flat_map(|p| p.to_srgba_unmultiplied()).collect();
                if let Err(error) = image::save_buffer(&path, &pixels, width as u32, height as u32, image::ColorType::Rgba8) {
                    eprintln!("Could not save the screenshot to {}: {error}", path.display());
                }
                done.store(true, Ordering::Relaxed);
                repaint.request_repaint();
            })));
        }
        ctx.request_repaint_after(Duration::from_millis(100));
    }
}
