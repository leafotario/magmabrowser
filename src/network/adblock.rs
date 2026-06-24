use std::collections::HashSet;

#[derive(Clone)]
pub struct AdblockEngine {
    blocked_domains: HashSet<String>,
}

impl AdblockEngine {
    pub fn start() -> Self {
        // Inicializa com uma lista de domínios conhecidos de anúncios e tracking
        let mut domains = HashSet::new();
        let base_list = vec![
            "doubleclick.net",
            "google-analytics.com",
            "googlesyndication.com",
            "adservice.google.com",
            "amazon-adsystem.com",
            "taboola.com",
            "outbrain.com",
            "criteo.com",
            "adsafeprotected.com",
            "adnxs.com",
            "adform.net",
            "facebook.com/tr/",
            "connect.facebook.net",
            "pixel.facebook.com",
            "hotjar.com",
            "clarity.ms",
        ];
        
        for d in base_list {
            domains.insert(d.to_string());
        }

        Self {
            blocked_domains: domains,
        }
    }

    /// Analisa se a URL dada (navegação) pertence a algum domínio bloqueado.
    pub fn should_block(&self, url: &str) -> bool {
        // Normaliza a URL para pegar apenas a parte relevante (ignorando scheme)
        let normalized = url.to_lowercase();
        
        // Exceções para esquemas nativos/locais
        if normalized.starts_with("petal://") 
            || normalized.starts_with("file://")
            || normalized.starts_with("localhost")
            || normalized.starts_with("127.0.0.1") {
            return false;
        }

        for domain in &self.blocked_domains {
            if normalized.contains(domain) {
                // Log útil e sutil apenas quando bloqueia de fato
                println!("🛡️ Adblock interceptou navegação para: {}", domain);
                return true;
            }
        }
        false
    }

    /// Retorna a lista de domínios bloqueados como uma string de Array JSON
    /// Útil para injetar o escudo dinâmico no JS.
    pub fn get_blocked_domains_js_array(&self) -> String {
        let items: Vec<String> = self.blocked_domains.iter().map(|d| format!("'{}'", d)).collect();
        format!("[{}]", items.join(","))
    }
}
