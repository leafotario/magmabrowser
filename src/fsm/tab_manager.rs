use std::time::Instant;
use wry::WebView;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TabState {
    Level0Focus,
    Level1Emptying,
    Level2Suspended,
    Level3Terminal,
}

pub enum FsmAction {
    None,
    RequestDestruction,
}

pub struct TabFSM {
    pub state: TabState,
    pub background_since: Option<Instant>,
}

impl TabFSM {
    pub fn new() -> Self {
        Self { state: TabState::Level0Focus, background_since: None }
    }

    pub fn set_background(&mut self) {
        if self.state == TabState::Level0Focus { self.background_since = Some(Instant::now()); }
    }

    pub fn set_foreground(&mut self, webview: &WebView) -> Result<(), String> {
        self.state = TabState::Level0Focus;
        self.background_since = None;
        #[cfg(target_os = "windows")] return resume_webview_windows(webview);
        #[cfg(not(target_os = "windows"))] return Ok(());
    }

    pub fn tick(&mut self, webview: &WebView) -> Result<FsmAction, String> {
        let mut action = FsmAction::None;
        if let Some(since) = self.background_since {
            let elapsed = since.elapsed();
            if elapsed.as_secs() >= 30 && self.state == TabState::Level0Focus {
                self.state = TabState::Level1Emptying;
                self.apply_level_1(webview)?;
            } else if elapsed.as_secs() >= 120 && self.state == TabState::Level1Emptying {
                self.state = TabState::Level2Suspended;
                self.apply_level_2(webview)?;
            } else if elapsed.as_secs() >= 300 && self.state == TabState::Level2Suspended {
                self.state = TabState::Level3Terminal;
                action = self.apply_level_3(webview)?;
            }
        }
        Ok(action)
    }

    fn apply_level_1(&self, webview: &WebView) -> Result<(), String> {
        #[cfg(target_os = "windows")] return apply_level_1_windows(webview);
        #[cfg(not(target_os = "windows"))] return Ok(()); 
    }

    fn apply_level_2(&self, webview: &WebView) -> Result<(), String> {
        #[cfg(target_os = "windows")] return apply_level_2_windows(webview);
        #[cfg(not(target_os = "windows"))] return Ok(()); 
    }

    fn apply_level_3(&self, webview: &WebView) -> Result<FsmAction, String> {
        let _ = webview.evaluate_script("window.ipc.postMessage('scroll_snapshot:' + window.scrollX + ',' + window.scrollY);");
        #[cfg(target_os = "windows")] let _ = apply_level_3_windows_preview(webview);
        Ok(FsmAction::RequestDestruction)
    }
}

#[cfg(target_os = "windows")]
use wry::WebViewExtWindows;
#[cfg(target_os = "windows")]
use webview2_com::Microsoft::Web::WebView2::Win32::{
    ICoreWebView2Controller, ICoreWebView2_19, ICoreWebView2_3,
    COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_LOW,
    COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_NORMAL,
};
#[cfg(target_os = "windows")]
use windows_core::ComInterface;

#[cfg(target_os = "windows")]
fn get_core_webview2(webview: &WebView) -> Result<ICoreWebView2Controller, String> {
    Ok(webview.controller())
}

#[cfg(target_os = "windows")]
fn apply_level_1_windows(webview: &WebView) -> Result<(), String> {
    unsafe {
        let controller = get_core_webview2(webview)?;
        let core = controller.CoreWebView2().map_err(|e| format!("COM falha: {}", e))?;
        let core_19 = core.cast::<ICoreWebView2_19>().map_err(|e| format!("COM falha: {}", e))?;
        core_19.SetMemoryUsageTargetLevel(COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_LOW).map_err(|e| format!("COM falha: {}", e))?;
        Ok(())
    }
}

#[cfg(target_os = "windows")]
fn apply_level_2_windows(_webview: &WebView) -> Result<(), String> {
    // TrySuspend requires a complex COM callback. For bare-metal, we skip it
    // since OsTrimmer already periodically invokes EmptyWorkingSet on the process.
    Ok(())
}

#[cfg(target_os = "windows")]
fn resume_webview_windows(webview: &WebView) -> Result<(), String> {
    unsafe {
        let controller = get_core_webview2(webview)?;
        let core = controller.CoreWebView2().map_err(|e| format!("COM falha: {}", e))?;
        if let Ok(core_3) = core.cast::<ICoreWebView2_3>() { let _ = core_3.Resume(); }
        if let Ok(core_19) = core.cast::<ICoreWebView2_19>() { let _ = core_19.SetMemoryUsageTargetLevel(COREWEBVIEW2_MEMORY_USAGE_TARGET_LEVEL_NORMAL); }
        Ok(())
    }
}

#[cfg(target_os = "windows")]
fn apply_level_3_windows_preview(webview: &WebView) -> Result<(), String> {
    unsafe {
        let controller = get_core_webview2(webview)?;
        let _core = controller.CoreWebView2().map_err(|e| format!("COM falha: {}", e))?;
        Ok(())
    }
}
