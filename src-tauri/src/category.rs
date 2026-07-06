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
            "miro", "canva",
            // Windows / Office nativo
            "excel", "word", "winword", "powerpoint", "powerpnt", "msoffice",
            "access", "visio", "onenote", "teams",
        ],
    ),
    (
        "Ferramenta",
        &[
            "vscode", "visual studio code", "code", "terminal", "iterm", "xcode",
            "github", "gitlab", "localhost", "cargo", "npm", "focusbar", "figma",
            "postman", "docker", "warp",
            // IAs de trabalho e criação (identificar > cair em "Outro")
            "claude", "chatgpt", "gemini", "copilot", "cursor",
            // Criação: jogos, 3D, mídia — "roblox studio" ANTES da regra
            // "roblox" (Procrastinação): quem cria não está jogando.
            "roblox studio", "sketchfab", "blender", "unity", "unreal",
            "godot", "photoshop", "premiere", "after effects", "davinci",
            "obs", "audacity",
            // Windows nativo
            "notepad", "notepad++", "powershell", "cmd", "command prompt",
            "explorer", "visual studio", "rider", "intellij", "sublime", "wsl",
        ],
    ),
    (
        "Procrastinação",
        &[
            "youtube", "discord", "roblox", "whatsapp", "instagram", "twitter",
            "x.com", "reddit", "tiktok", "netflix", "twitch", "steam", "spotify",
            "tierlist", "game", "epic games", "xbox", "bethesda", "battle.net",
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
/// Chaves de uma palavra casam por TOKEN inteiro (evita "word" dentro de
/// "password", "access" em "accessibility"); chaves com espaço/pontuação
/// (ex.: "visual studio code", "x.com", "notepad++") casam por substring.
pub fn categorize(app_name: &str, title: &str) -> &'static str {
    let hay = format!("{} {}", app_name, title).to_lowercase();
    let tokens: std::collections::HashSet<&str> = hay
        .split(|c: char| !c.is_alphanumeric())
        .filter(|s| !s.is_empty())
        .collect();
    for (cat, keywords) in RULES {
        let hit = keywords.iter().any(|k| {
            if k.chars().any(|c| !c.is_alphanumeric()) {
                hay.contains(*k) // chave composta/pontuada: substring
            } else {
                tokens.contains(*k) // palavra única: token exato
            }
        });
        if hit {
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
    fn roblox_studio_e_criacao_nao_e_procrastinacao() {
        // "roblox studio" (Ferramenta, chave composta) vence "roblox"
        // (Procrastinação): criar jogo ≠ jogar.
        assert_eq!(categorize("Roblox Studio", "MeuJogo.rbxl"), "Ferramenta");
        assert_eq!(categorize("Roblox", "jogando adopt me"), "Procrastinação");
        assert_eq!(categorize("Sketchfab", "modelo 3d"), "Ferramenta");
        assert_eq!(categorize("Miro", "quadro do projeto"), "Trabalho");
        assert_eq!(categorize("Claude", "conversa"), "Ferramenta");
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

    #[test]
    fn windows_apps_categorize() {
        assert_eq!(categorize("Excel", "Orçamento 2026"), "Trabalho");
        assert_eq!(categorize("Notepad", "notas.txt"), "Ferramenta");
        assert_eq!(categorize("Xbox", "Halo Infinite"), "Procrastinação");
    }

    #[test]
    fn no_substring_false_positives() {
        // chaves de uma palavra não casam DENTRO de palavras maiores:
        assert_eq!(categorize("Chrome", "password manager"), "Outro"); // word ⊄ password
        assert_eq!(categorize("Chrome", "accessibility options"), "Outro"); // access ⊄ ...
        assert_eq!(categorize("Chrome", "codecademy lesson"), "Outro"); // code ⊄ codecademy
    }
}
