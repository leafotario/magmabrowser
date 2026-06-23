use std::env;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use winit::window::Window;
use wry::{WebView, WebViewBuilder};
#[cfg(target_os = "windows")]
use wry::WebViewBuilderExtWindows;
use crate::network::adblock::AdblockEngine;

pub struct EphemeralWebContext {
    pub data_dir: PathBuf,
}

impl EphemeralWebContext {
    pub fn new() -> Self {
        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_nanos();
        let mut data_dir = env::temp_dir();
        data_dir.push(format!("magma_volatile_{}", timestamp));
        fs::create_dir_all(&data_dir).expect("Falha");
        Self { data_dir }
    }
}

impl Drop for EphemeralWebContext {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.data_dir);
    }
}

pub fn build_webview(
    window: &Window,
    _ephemeral_context: &EphemeralWebContext,
    adblock_engine: &AdblockEngine,
) -> wry::Result<WebView> {
    let mut builder = WebViewBuilder::new(window);

    #[cfg(target_os = "windows")]
    {
        builder = builder.with_additional_browser_args(
            "--js-flags=\"--lite-mode --max-old-space-size=128 --scavenger_max_new_space_capacity_mb=4\" --renderer-process-limit=2"
        );
    }
    
    let builder = adblock_engine.attach_to_builder(builder);
    builder.with_url("https://magma.browser/local_cache")?.build()
}
