//! Porteiro de privacidade: redação de conteúdo sensível antes de QUALQUER coisa
//! ir pro modelo, e zonas de exclusão (sessões que nem devem ser analisadas).
//!
//! Conservador de propósito: melhor deixar passar um pouco de texto normal do que
//! garglar tudo. O que casa vira [REDIGIDO].

use regex::Regex;
use std::sync::OnceLock;

/// Apps/sites que NUNCA devem ser analisados (lista padrão — o usuário ajusta depois).
/// Comparação case-insensitive contra "app + título": termos de UMA palavra exigem
/// fronteira (token exato), termos com espaço usam substring. Evitamos palavras
/// genéricas como "banco"/"caixa" sozinhas (dariam falso-positivo em "banco de dados",
/// "caixa de entrada") — preferimos nomes de marca + termos compostos.
const EXCLUSION_ZONES: &[&str] = &[
    // gerenciadores de senha
    "1password", "bitwarden", "lastpass", "keepass", "dashlane", "keychain",
    "acesso às chaves", "proton pass",
    // bancos / financeiro (marcas + termos compostos, não a palavra "banco" solta)
    "itau", "itaú", "nubank", "bradesco", "santander", "sicoob", "sicredi",
    "picpay", "neon", "c6 bank", "banco inter", "banco do brasil",
    "caixa econômica", "caixa economica", "mercado pago", "online banking",
    "internet banking",
    // saúde
    "prontuário", "prontuario",
];

/// True se `needle` aparece em `hay` como token inteiro (delimitado por não-alfanumérico).
fn word_present(hay: &str, needle: &str) -> bool {
    hay.split(|c: char| !c.is_alphanumeric()).any(|w| w == needle)
}

/// True se a sessão cai numa zona de exclusão (não deve nem ser analisada).
pub fn is_excluded(app: &str, title: &str) -> bool {
    let hay = format!("{} {}", app, title).to_lowercase();
    EXCLUSION_ZONES.iter().any(|z| {
        if z.contains(' ') {
            hay.contains(z) // termo composto: substring
        } else {
            word_present(&hay, z) // palavra única: token exato
        }
    })
}

fn patterns() -> &'static Vec<Regex> {
    static P: OnceLock<Vec<Regex>> = OnceLock::new();
    P.get_or_init(|| {
        let raw = [
            // CPF
            r"\b\d{3}\.?\d{3}\.?\d{3}-?\d{2}\b",
            // cartão de crédito (13-16 dígitos, com espaços/hífens)
            r"\b(?:\d[ -]?){13,16}\b",
            // e-mail
            r"\b[A-Za-z0-9._%+\-]+@[A-Za-z0-9.\-]+\.[A-Za-z]{2,}\b",
            // celular BR (com DDD e o 9; cobre +55, parênteses, hífen/espaço)
            r"\b(?:\+?55\s?)?\(?\d{2}\)?\s?9\d{4}[-\s]?\d{4}\b",
            // JWT (eyJ... . ... . ...)
            r"\beyJ[A-Za-z0-9_\-]+\.[A-Za-z0-9_\-]+\.[A-Za-z0-9_\-]+\b",
            // tokens/segredos longos (chaves de API, etc.)
            r"\b(?:sk|pk|ghp|gho|xox[bp])[-_][A-Za-z0-9_\-]{12,}\b",
            r"\b[A-Za-z0-9_\-]{32,}\b",
            // linhas com "senha"/"password" seguidas de algo
            r"(?i)(senha|password|passwd|pwd)\s*[:=]\s*\S+",
        ];
        raw.iter().filter_map(|p| Regex::new(p).ok()).collect()
    })
}

/// Regex que captura a "cauda" de uma URL (query/fragment), preservando o início.
fn url_tail() -> &'static Regex {
    static URL_TAIL: OnceLock<Regex> = OnceLock::new();
    URL_TAIL.get_or_init(|| Regex::new(r"(https?://[^\s?#]+)[?#]\S*").unwrap())
}

/// Redige conteúdo sensível, trocando por [REDIGIDO].
pub fn redact(input: &str) -> String {
    // 1) Tira query/fragment de qualquer URL (tokens/PII vivem no ?...#...).
    //    Roda ANTES dos demais padrões e cobre títulos antigos já no banco.
    let mut out = url_tail().replace_all(input, "$1").into_owned();
    // 2) Padrões de segredo → [REDIGIDO].
    for re in patterns() {
        out = re.replace_all(&out, "[REDIGIDO]").into_owned();
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redacts_cpf() {
        let out = redact("meu CPF é 123.456.789-00 ok");
        assert!(out.contains("[REDIGIDO]"));
        assert!(!out.contains("123.456.789-00"));
    }

    #[test]
    fn redacts_password_line() {
        let out = redact("senha: hunter2supersecreta");
        assert!(out.contains("[REDIGIDO]"));
        assert!(!out.contains("hunter2supersecreta"));
    }

    #[test]
    fn redacts_long_token() {
        let out = redact("token ghp_abcdefghijklmnopqrstuvwx ativo");
        assert!(out.contains("[REDIGIDO]"));
        assert!(!out.contains("ghp_abcdefghijklmnopqrstuvwx"));
    }

    #[test]
    fn keeps_normal_text() {
        let s = "reuniao sobre o roadmap as 14h";
        assert_eq!(redact(s), s);
    }

    #[test]
    fn redacts_email() {
        let out = redact("contato: joao.silva@empresa.com.br aqui");
        assert!(out.contains("[REDIGIDO]"));
        assert!(!out.contains("joao.silva@empresa.com.br"));
    }

    #[test]
    fn redacts_jwt() {
        let out = redact("auth eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.dozjgNryP4J3jVmNHl0w abc");
        assert!(out.contains("[REDIGIDO]"));
        assert!(!out.contains("eyJhbGciOiJIUzI1NiJ9"));
    }

    #[test]
    fn strips_url_query_and_fragment_with_tokens() {
        // reset token na query e access_token no fragment não podem sobrar
        let a = redact("abrindo https://app.com/reset?token=abcd1234&user=joao agora");
        assert!(!a.contains("token=abcd1234"));
        assert!(a.contains("https://app.com/reset"));
        let b = redact("cb https://x.com/cb#access_token=ya29.SHORT&t=1");
        assert!(!b.contains("access_token"));
        assert!(b.contains("https://x.com/cb"));
    }

    #[test]
    fn url_without_tail_is_preserved() {
        let s = "vendo https://www.youtube.com/watch";
        assert_eq!(redact(s), s);
    }

    #[test]
    fn generic_words_are_not_excluded() {
        // "banco"/"caixa" soltos não devem excluir trabalho legítimo
        assert!(!is_excluded("DBeaver", "banco de dados - produção"));
        assert!(!is_excluded("Chrome", "Caixa de entrada - Gmail"));
        assert!(!is_excluded("Figma", "banco de imagens do projeto"));
    }

    #[test]
    fn brand_banks_still_excluded() {
        assert!(is_excluded("Chrome", "Nubank — minha conta"));
        assert!(is_excluded("Safari", "Caixa Econômica - login"));
        assert!(is_excluded("Chrome", "Banco do Brasil - extrato"));
    }

    #[test]
    fn exclusion_zones_match_password_and_bank_apps() {
        assert!(is_excluded("1Password", "Cofre pessoal"));
        assert!(is_excluded("Chrome", "Nubank - minha conta"));
        assert!(is_excluded("Bitwarden", ""));
    }

    #[test]
    fn normal_apps_not_excluded() {
        assert!(!is_excluded("Chrome", "youtube.com - video"));
        assert!(!is_excluded("Visual Studio Code", "main.rs"));
    }
}
