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
