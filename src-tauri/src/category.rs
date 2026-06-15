//! Categorização por regras (nível 1): mapeia app + título para uma categoria.
//! Determinístico, offline, sem custo. As listas são fáceis de editar aqui;
//! no futuro dá pra mover pra uma tabela editável pelo usuário.

/// Categorias na ordem de prioridade de match (a primeira que casar vence).
/// (nome_da_categoria, palavras-chave em minúsculo)
const RULES: &[(&str, &[&str])] = &[
    (
        "Trabalho",
        &[
            "proxy", "adspower", "notion", "planilha", "sheets", "docs", "slides",
            "meet", "zoom", "gmail", "outlook", "mail", "calendar", "drive",
            "linear", "jira", "trello", "asana", "slack",
        ],
    ),
    (
        "Ferramenta",
        &[
            "vscode", "visual studio code", "code", "terminal", "iterm", "xcode",
            "github", "gitlab", "localhost", "cargo", "npm", "focusbar", "figma",
            "postman", "docker", "warp",
        ],
    ),
    (
        "Procrastinação",
        &[
            "youtube", "discord", "roblox", "whatsapp", "instagram", "twitter",
            "x.com", "reddit", "tiktok", "netflix", "twitch", "steam", "spotify",
            "tierlist", "game",
        ],
    ),
];

use std::collections::HashMap;

/// Categoria efetiva: usa o override manual do usuário (por nome de app) se
/// existir; senão cai na regra automática.
pub fn effective(overrides: &HashMap<String, String>, app_name: &str, title: &str) -> String {
    if let Some(c) = overrides.get(app_name) {
        if !c.is_empty() {
            return c.clone();
        }
    }
    categorize(app_name, title).to_string()
}

/// Categoria de uma sessão. Casa contra "app título" em minúsculo.
pub fn categorize(app_name: &str, title: &str) -> &'static str {
    let hay = format!("{} {}", app_name, title).to_lowercase();
    for (cat, keywords) in RULES {
        if keywords.iter().any(|k| hay.contains(k)) {
            return cat;
        }
    }
    "Outro"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rules_match_expected_categories() {
        assert_eq!(categorize("YouTube", "gameplay zerando jogo"), "Procrastinação");
        assert_eq!(categorize("Visual Studio Code", "main.rs"), "Ferramenta");
        assert_eq!(categorize("Notion", "roadmap do produto"), "Trabalho");
        assert_eq!(categorize("AppDesconhecido", "janela qualquer"), "Outro");
    }

    #[test]
    fn priority_trabalho_beats_later_rules() {
        // Título com palavra de Trabalho (notion) e de Procrastinação (youtube):
        // a primeira regra na ordem de prioridade vence.
        assert_eq!(
            categorize("Chrome", "notion e youtube na mesma aba"),
            "Trabalho"
        );
    }

    #[test]
    fn matching_is_case_insensitive() {
        assert_eq!(categorize("DISCORD", ""), "Procrastinação");
    }

    #[test]
    fn effective_prefers_user_override() {
        let mut ov = HashMap::new();
        ov.insert("YouTube".to_string(), "Trabalho".to_string());
        // Sem override seria Procrastinação; o override do usuário manda.
        assert_eq!(effective(&ov, "YouTube", "qualquer vídeo"), "Trabalho");
    }

    #[test]
    fn effective_ignores_empty_override_and_falls_back() {
        let mut ov = HashMap::new();
        ov.insert("YouTube".to_string(), String::new());
        assert_eq!(effective(&ov, "YouTube", "vídeo"), "Procrastinação");
    }
}
