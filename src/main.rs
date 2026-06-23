mod engine;
mod fsm;
mod memory;
mod network;

use std::num::NonZeroU32;
use softbuffer::{Context, Surface};
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

fn main() {
    let event_loop = EventLoop::new().unwrap();

    let window = WindowBuilder::new()
        .with_title("Magma Browser [Bare-Metal Edition]")
        .with_inner_size(winit::dpi::LogicalSize::new(1280.0, 720.0))
        .build(&event_loop)
        .expect("Falha arquitetural crítica: Falha ao solicitar a janela nativa ao Kernel.");

    let sb_context = unsafe { Context::new(&window).expect("Falha ao criar Contexto Softbuffer") };
    let mut sb_surface = unsafe { Surface::new(&sb_context, &window).expect("Falha ao criar Superfície Softbuffer") };

    let adblock_engine = network::adblock::AdblockEngine::start();
    let ephemeral_context = engine::builder::EphemeralWebContext::new();
    
    let mut _webview = Some(engine::builder::build_webview(&window, &ephemeral_context, &adblock_engine)
        .expect("Falha arquitetural: Não foi possível instanciar o motor WebView."));

    let mut tab_fsm = fsm::tab_manager::TabFSM::new();
    let mut os_trimmer = memory::os_trim::OsTrimmer::new();
    let adblock_engine_loop = adblock_engine.clone();

    event_loop.run(move |event, elwt| {
        elwt.set_control_flow(ControlFlow::Wait);

        match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                elwt.exit();
            }
            Event::WindowEvent {
                event: WindowEvent::Resized(_new_size),
                ..
            } => {}
            Event::WindowEvent {
                event: WindowEvent::MouseInput { .. },
                ..
            } => {
                if _webview.is_none() {
                    let ephemeral = engine::builder::EphemeralWebContext::new();
                    _webview = Some(engine::builder::build_webview(&window, &ephemeral, &adblock_engine_loop)
                        .expect("Falha arquitetural na Reidratação"));
                    
                    let wv = _webview.as_ref().unwrap();
                    let _ = wv.evaluate_script("window.scrollTo(0, 0);"); 
                    let _ = tab_fsm.set_foreground(wv);
                }
            }
            Event::AboutToWait => {
                if let Some(wv) = _webview.as_ref() {
                    if let Ok(fsm::tab_manager::FsmAction::RequestDestruction) = tab_fsm.tick(wv) {
                        _webview = None;
                        let size = window.inner_size();
                        if size.width > 0 && size.height > 0 {
                            let _ = sb_surface.resize(
                                NonZeroU32::new(size.width).unwrap(),
                                NonZeroU32::new(size.height).unwrap(),
                            );
                            if let Ok(mut buffer) = sb_surface.buffer_mut() {
                                for index in 0..(size.width * size.height) {
                                    buffer[index as usize] = 0xFF_1E_1E_1E; 
                                }
                                let _ = buffer.present();
                            }
                        }
                    }
                    if let Ok(memory::os_trim::TrimAction::EmergencyCrash) = os_trimmer.try_trim(_webview.as_ref()) {
                        _webview = None;
                        let ephemeral = engine::builder::EphemeralWebContext::new();
                        _webview = Some(engine::builder::build_webview(&window, &ephemeral, &adblock_engine_loop)
                            .expect("FMEA: Falha na Reidratação de Emergência"));
                        let wv = _webview.as_ref().unwrap();
                        let _ = wv.evaluate_script("window.scrollTo(0, 0);"); 
                        let _ = tab_fsm.set_foreground(wv);
                    }
                }
            }
            _ => (),
        }
    }).unwrap();
}
