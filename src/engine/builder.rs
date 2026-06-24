use std::env;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use winit::window::Window;
use wry::{Rect, WebView, WebViewBuilder};
#[cfg(target_os = "windows")]
use wry::WebViewBuilderExtWindows;
use crate::network::adblock::AdblockEngine;

pub struct EphemeralWebContext {
    pub data_dir: PathBuf,
}

#[cfg(target_os = "windows")]
fn is_process_alive(pid: u32) -> bool {
    use windows_sys::Win32::System::Threading::{OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION, GetExitCodeProcess};
    use windows_sys::Win32::Foundation::CloseHandle;
    unsafe {
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if handle == 0 { return false; }
        let mut exit_code: u32 = 0;
        let success = GetExitCodeProcess(handle, &mut exit_code);
        CloseHandle(handle);
        if success == 0 { return false; }
        exit_code == 259 // STILL_ACTIVE
    }
}

#[cfg(not(target_os = "windows"))]
fn is_process_alive(pid: u32) -> bool {
    unsafe { libc::kill(pid as i32, 0) == 0 || std::io::Error::last_os_error().raw_os_error() == Some(libc::EPERM) }
}

impl EphemeralWebContext {
    pub fn new() -> Self {
        Self::cleanup_abandoned();
        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_nanos();
        let current_pid = std::process::id();
        let mut data_dir = env::temp_dir();
        data_dir.push(format!("petal_volatile_{}_{}", current_pid, timestamp));
        fs::create_dir_all(&data_dir).expect("Falha");
        Self { data_dir }
    }

    fn cleanup_abandoned() {
        let temp = env::temp_dir();
        let current_pid = std::process::id();
        if let Ok(entries) = fs::read_dir(&temp) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                        if name.starts_with("petal_volatile_") {
                            let parts: Vec<&str> = name.split('_').collect();
                            if parts.len() == 4 {
                                if let Ok(pid) = parts[2].parse::<u32>() {
                                    if pid != current_pid && !is_process_alive(pid) {
                                        let _ = fs::remove_dir_all(&path);
                                    }
                                }
                            } else if parts.len() == 3 {
                                // Formato antigo sem PID
                                let _ = fs::remove_dir_all(&path);
                            }
                        }
                    }
                }
            }
        }
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
    url: &str,
    tab_id: u32,
    hardware_acceleration: bool,
    ipc_tx: crossbeam_channel::Sender<String>,
) -> wry::Result<WebView> {
    let mut builder = WebViewBuilder::new(window);

    // Constranger o WebView para deixar a área da barra de abas visível no host Winit
    let size = window.inner_size();
    if size.height > crate::ui::CHROME_HEIGHT {
        let bounds = Rect {
            x: 0,
            y: crate::ui::CHROME_HEIGHT as i32,
            width: size.width,
            height: size.height - crate::ui::CHROME_HEIGHT,
        };
        builder = builder.with_bounds(bounds);
    }

    let tx_nav = ipc_tx.clone();
    builder = builder.with_on_page_load_handler(move |event, url| {
        // Envia apenas quando o carregamento termina ou muda
        if let wry::PageLoadEvent::Finished = event {
            let _ = tx_nav.send(format!("{}|url|{}", tab_id, url));
        }
    });

    let adblock_engine_clone = adblock_engine.clone();
    builder = builder.with_navigation_handler(move |nav_url| {
        if adblock_engine_clone.should_block(&nav_url) {
            return false; // Bloqueia navegação host
        }
        true
    });

    // Injeção de IPC para rastrear Document Title (nativo não suportado cross-platform sem extensões)
    builder = builder.with_ipc_handler(move |request| {
        let msg = request; // request is a String in wry
        let _ = ipc_tx.send(msg);
    });

    let blocked_array_js = adblock_engine.get_blocked_domains_js_array();
    let init_script = format!(r#"
        (function() {{
            const blocked = {};
            function isBlocked(urlStr) {{
                if (!urlStr) return false;
                try {{
                    let parsed = new URL(urlStr, window.location.href);
                    let host = parsed.hostname.toLowerCase();
                    for (let i = 0; i < blocked.length; i++) {{
                        if (host === blocked[i] || host.endsWith('.' + blocked[i])) return true;
                    }}
                }} catch(e) {{}}
                return false;
            }}

            const origFetch = window.fetch;
            window.fetch = async function(...args) {{
                let url = (typeof args[0] === 'string') ? args[0] : (args[0] && args[0].url);
                if (isBlocked(url)) return Promise.reject(new Error('Petal Adblock: Fetch blocked'));
                return origFetch.apply(this, args);
            }};

            const origOpen = XMLHttpRequest.prototype.open;
            XMLHttpRequest.prototype.open = function(...args) {{
                if (isBlocked(args[1])) return;
                return origOpen.apply(this, args);
            }};

            if (navigator.sendBeacon) {{
                const origBeacon = navigator.sendBeacon;
                navigator.sendBeacon = function(url, data) {{
                    if (isBlocked(url)) return false;
                    return origBeacon.call(navigator, url, data);
                }};
            }}

            // Nota técnica: O MutationObserver é "Best Effort".
            // No nível Bare-Metal do WebView nativo, não conseguimos interceptar a rede (network layer)
            // de requisições de media (img, script src) via HTTPS no cross-platform sem extensões.
            // Portanto, o download de alguns desses recursos pode iniciar antes da remoção da DOM.
            new MutationObserver((mutations) => {{
                for (let m of mutations) {{
                    for (let n of m.addedNodes) {{
                        if (n.nodeType === 1) {{
                            if ((n.tagName === 'SCRIPT' || n.tagName === 'IFRAME' || n.tagName === 'IMG') && isBlocked(n.src)) {{
                                n.src = '';
                                n.remove();
                            }}
                        }}
                    }}
                }}
            }}).observe(document.documentElement || document, {{ childList: true, subtree: true }});

            window.addEventListener('keydown', function(e) {{
                if (e.ctrlKey && e.key.toLowerCase() === 'l') {{
                    e.preventDefault();
                    window.ipc.postMessage('{}|focus_omnibox|');
                }}
            }});

            window.ipc.postMessage('{}|title|' + document.title);
            new MutationObserver(function(mutations) {{
                window.ipc.postMessage('{}|title|' + document.title);
            }}).observe(
                document.querySelector('title') || document.head,
                {{ subtree: true, characterData: true, childList: true }}
            );
        }})();
    "#, blocked_array_js, tab_id, tab_id, tab_id);
    builder = builder.with_initialization_script(&init_script);

    #[cfg(target_os = "windows")]
    {
        let mut args = "--js-flags=\"--lite-mode --max-old-space-size=128 --scavenger_max_new_space_capacity_mb=4\" --renderer-process-limit=2".to_string();
        if !hardware_acceleration {
            args.push_str(" --disable-gpu");
        }
        builder = builder.with_additional_browser_args(&args);
    }
    builder.with_url(url)?.build()
}
