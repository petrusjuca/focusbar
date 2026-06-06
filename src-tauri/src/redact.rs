//! Porteiro de privacidade: redação de conteúdo sensível antes de QUALQUER coisa
//! ir pro modelo, e zonas de exclusão (sessões que nem devem ser analisadas).
//!
//! Conservador de propósito: melhor deixar passar um pouco de texto normal do que
//! garglar tudo. O que casa vira [REDIGIDO].

use regex::Regex;
use std::sync::OnceLock;

/// Apps/sites que NUNCA devem ser analisados (lista padrão — o usuário ajusta depois).
/// Comparação é case-insensitive contra "app + título".
const EXCLUSION_ZONES: &[&str] = &[
    // gerenciadores de senha
    "1password", "bitwarden", "lastpass", "keepass", "dashlane", "keychain",
    "acesso às chaves", "proton pass",
    // bancos / financeiro
    "banco", "itau", "itaú", "nubank", "bradesco", "santander", "caixa",
    "banco inter", "banco do brasil", "c6 bank", "online banking", "internet banking",
    // saúde
    "prontuário", "prontuario",
];

/// True se a sessão cai numa zona de exclusão (não deve nem ser analisada).
pub fn is_excluded(app: &str, title: &str) -> bool {
    let hay = format!("{} {}", app, title).to_lowercase();
    EXCLUSION_ZONES.iter().any(|z| hay.contains(z))
}

fn patterns() -> &'static Vec<Regex> {
    static P: OnceLock<Vec<Regex>> = OnceLock::new();
    P.get_or_init(|| {
        let raw = [
            // CPF
            r"\b\d{3}\.?\d{3}\.?\d{3}-?\d{2}\b",
            // cartão de crédito (13-16 dígitos, com espaços/hífens)
            r"\b(?:\d[ -]?){13,16}\b",
            // tokens/segredos longos (chaves de API, etc.)
            r"\b(?:sk|pk|ghp|gho|xox[bp])[-_][A-Za-z0-9_\-]{12,}\b",
            r"\b[A-Za-z0-9_\-]{32,}\b",
            // linhas com "senha"/"password" seguidas de algo
            r"(?i)(senha|password|passwd|pwd)\s*[:=]\s*\S+",
        ];
        raw.iter().filter_map(|p| Regex::new(p).ok()).collect()
    })
}

/// Redige conteúdo sensível, trocando por [REDIGIDO].
pub fn redact(input: &str) -> String {
    let mut out = input.to_string();
    for re in patterns() {
        out = re.replace_all(&out, "[REDIGIDO]").into_owned();
    }
    out
}
