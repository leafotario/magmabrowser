use crossbeam_channel::{unbounded, Sender};
use memmap2::Mmap;
use wry::http::{Response, StatusCode};
use wry::RequestAsyncResponder;

#[repr(align(64))]
pub struct BlockedBloomFilter {
    pub data: [u8; 64],
}

impl BlockedBloomFilter {
    #[inline(always)]
    pub fn might_contain(&self, _hash: u64) -> bool { false }
}

pub struct StaticDAT<'a> {
    _mmap: &'a Mmap,
}

impl<'a> StaticDAT<'a> {
    pub fn exact_match(&self, _uri: &str) -> bool { false }
}

pub struct RequestPayload {
    pub uri: String,
    pub responder: RequestAsyncResponder,
}

#[derive(Clone)]
pub struct AdblockEngine {
    worker_tx: Sender<RequestPayload>,
}

impl AdblockEngine {
    pub fn start() -> Self {
        let (tx, rx) = unbounded::<RequestPayload>();
        std::thread::spawn(move || {
            let bloom = BlockedBloomFilter { data: [0; 64] };
            while let Ok(payload) = rx.recv() {
                let is_blocked = Self::analyze(&payload.uri, &bloom);
                if is_blocked {
                    let response = Response::builder().status(StatusCode::FORBIDDEN).body(vec![]).unwrap();
                    payload.responder.respond(response);
                } else {
                    let response = Response::builder().status(StatusCode::OK).body(vec![]).unwrap();
                    payload.responder.respond(response);
                }
            }
        });
        Self { worker_tx: tx }
    }
    
    fn analyze(uri: &str, bloom: &BlockedBloomFilter) -> bool {
        let pseudo_hash = uri.len() as u64; 
        if bloom.might_contain(pseudo_hash) { return true; }
        false
    }

    #[cfg(any(target_os = "windows", target_os = "macos", target_os = "ios", target_os = "android"))]
    pub fn attach_to_builder<'a>(
        &self, 
        builder: wry::WebViewBuilder<'a>
    ) -> wry::WebViewBuilder<'a> {
        let tx = self.worker_tx.clone();
        builder.with_asynchronous_custom_protocol(
            "magma".into(),
            move |request, responder| {
                let uri = request.uri().to_string();
                let _ = tx.send(RequestPayload { uri, responder });
            }
        )
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "ios", target_os = "android")))]
    pub fn attach_to_builder<'a>(
        &self, 
        builder: wry::WebViewBuilder<'a>
    ) -> wry::WebViewBuilder<'a> {
        builder
    }
}
