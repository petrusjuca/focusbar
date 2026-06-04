# focusbar

Assistente de produtividade local — um "screenpipe leve". Rastreia onde seu
tempo vai (app/janela em foco), categoriza (trabalho / ferramenta /
procrastinação), mostra seu pico de foco, manda lembretes e te dá dicas do que
melhorar no dia. **100% local** (SQLite), só metadados — **não grava a tela**.

## O que ele faz hoje

- ⏱️ **Tempo por app/janela** com troca de foco e debounce (sessões reais)
- 😴 **Detecção de ocioso** (AFK) — tempo parado não conta como foco
- 📊 **Dashboard diário e semanal** (tempo por app, timeline, pico de foco)
- 🏷️ **Categorias** automáticas: Trabalho / Ferramenta / Procrastinação / Outro
- 🎯 **Tasks** por regras (palavra no título → nome da task)
- 🔔 **Lembretes** únicos e recorrentes → notificação nativa
- 🤖 **Coach ao vivo**: alerta de procrastinação prolongada, fragmentação de
  foco, "travado" na mesma janela, e resumo no fim do dia
- 🧠 **Insights** do dia ("seu melhor foco foi às Xh", "procrastinação alta")
- 🟢 Roda em **background** (ícone na barra de menu) e pode **iniciar com o sistema**

## Privacidade

Só guarda **metadados** (nome do app, título da janela, horários). Nenhum pixel
da tela é capturado, nada sai da máquina. O banco fica em:
- macOS: `~/Library/Application Support/com.petrusjuca.focusbar/focusbar.db`
- Windows: `%AppData%\com.petrusjuca.focusbar\focusbar.db`

No macOS, ler o título das janelas exige permissão de **Acessibilidade** (o app
pede na primeira vez).

## Instalar

### macOS
Baixe o `.dmg` na página de [Releases](../../releases), arraste pra Aplicativos.
Como o app não é assinado (ainda), na primeira vez abra com **botão direito →
Abrir**.

### Windows
Baixe o `.exe`/`.msi` na página de [Releases](../../releases). O SmartScreen
pode avisar (app não assinado) → **Mais informações → Executar assim mesmo**.

## Desenvolvimento

Pré-requisitos: [Node 20+](https://nodejs.org) e [Rust](https://rustup.rs).

```bash
npm install
npm run tauri dev      # roda em modo desenvolvimento
npm run tauri build    # gera o instalador do SO atual
```

> O Tauri **não cross-compila**: o instalador do Windows precisa ser gerado no
> Windows (ou pelo CI). Veja `.github/workflows/release.yml`.

## Gerar instaladores (Mac + Windows) via CI

O workflow `release.yml` builda os dois SOs no GitHub Actions. Para publicar uma
versão e gerar os instaladores:

```bash
git tag v0.1.0
git push origin v0.1.0
```

O CI cria uma **Release (rascunho)** com o `.dmg` (macOS) e o `.exe`/`.msi`
(Windows). Publique a release e compartilhe o link.

## Roadmap

- 🧠 Camada de IA (nomear tasks automaticamente + comparar com sua lista de
  tarefas, dizer se está no foco do que precisa fazer) — local (Ollama) ou API
- 📝 Integração com lista de tarefas (Notion / lista manual)
- 🎛️ Regras de categoria/alerta editáveis pela interface
