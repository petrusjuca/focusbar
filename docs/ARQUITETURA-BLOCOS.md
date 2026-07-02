# focusbar — Arquitetura em blocos + comparação com ActivityWatch e screenpipe

> Gerado em 01.07.2026 a partir do código real (auditado por agentes, item a item).
> Legenda: ✅ funciona · 🟠 parcial/precisa validar · ⬜ não existe · 🔵 decisão de produto pendente

---

## CAMADA 1 — CAPTURA (os sentidos)

| Bloco | Status | Detalhe |
|---|---|---|
| 1.1 Janela em foco | ✅ | Polling 1s (`active-win-pos-rs`). Captura por EVENTO ainda não (⬜) — polling funciona, evento é otimização |
| 1.2 URL do browser (Mac) | ✅ | AppleScript (Chrome/Safari/Edge/Brave/Arc/Vivaldi/Opera) + fallback por Acessibilidade pro Opera GX (🟠 em teste) |
| 1.3 URL do browser (Windows) | ⬜ | **Maior buraco atual** — no Win não isola a URL, então não separa sites (Revisão4 #1) |
| 1.4 Conteúdo da tela (Mac) | ✅ | Árvore de Acessibilidade + `AXManualAccessibility` (destrava Electron: Claude, Ollama, Code) + mira AXWebArea |
| 1.5 Conteúdo da tela (Windows) | 🟠 | UI Automation implementado; compila, **runtime a validar pelo chefe** (v0.4.0) |
| 1.6 OCR (fallback de pixel) | ✅ | xcap por janela → fallback tela cheia GUARDADO por pid (não atribui à sessão errada). Health-check no startup + botão "Testar os olhos". Imagem **em memória, nunca salva** |
| 1.7 Presença: idle | ✅ | `user-idle`, sinal primário |
| 1.8 Presença: áudio tocando | ✅ | CoreAudio (Mac) + WASAPI (Win) — separa "assistindo" de "AFK" |
| 1.9 Presença: tela bloqueada | ✅ | CGSession (Mac) + input desktop (Win) |
| 1.10 Porteiro (PII) | ✅ | `redact.rs`: senha/CPF/cartão/token (32+ chars) + zonas de exclusão (banco/senha nunca capturados) |
| 1.11 Título limpo | ✅ | Remove o lixo do Chrome ("Uso elevado da memória… 1,2 GB - Google Chrome: perfil") |
| 1.12 Extensão de browser (tab_id, aba fechada, foco×fundo) | ⬜ | Não existe. **ActivityWatch já tem isso pronto** — ver estratégia no fim |
| 1.13 Áudio/microfone/Whisper | ⬜ | Por design (privacidade/peso). screenpipe tem |

## CAMADA 2 — ARMAZENAMENTO (a memória)

| Bloco | Status | Detalhe |
|---|---|---|
| 2.1 `focus_events` (sessões) | ✅ | Uma linha por troca de janela, com conteúdo + categoria |
| 2.2 `interval_markers` (pausado/ausente) | ✅ | Todo minuto tem dono |
| 2.3 `pomodoro_log` | ✅ | goal, início, planejado, real, cumpriu? |
| 2.4 `todos` / `notes`(intenção) / `settings` | ✅ | |
| 2.5 Tabela BRUTA separada (re-derivável) | ⬜ | Só existe a de sessões; blocos são derivados em memória |
| 2.6 Screenshots salvos em disco | 🔵 | **HOJE NÃO SALVA (por privacidade)**. Revisão4 #16 pede pra salvar (estilo screenpipe). DECISÃO DE PRODUTO — trade-off privacidade × análise posterior |

## CAMADA 3 — ENTENDIMENTO (o cérebro)

| Bloco | Status | Detalhe |
|---|---|---|
| 3.1 Sessionização (gap 90s → blocos) | ✅ | Mata o confete; testes unitários |
| 3.2 Estados por minuto | ✅ | ativo/passivo/ocioso/ausente/pausado/sem-dados |
| 3.3 Categoria por regra de app | ✅ | Trabalho/Ferramenta/Procrastinação/Outro |
| 3.4 Categoria por CONTEÚDO (IA local) | ✅ | Com âncora anti-alucinação: IA só contraria a regra COM PROVA no texto da tela |
| 3.5 Override por app + recategorizar BLOCO 1-clique | ✅ | Taxonomia única de 7 categorias |
| 3.6 IA nomeia o bloco (`activity_ai`) | 🟠 | Nomeia; falta UI pra renomear |
| 3.7 Check "no foco?" ao vivo | 🟠 | Camadas baratas → IA ancorada. Confiabilidade limitada pelo 3B; captura consertada ajudou. Revisão4 #8: botão "checar agora" confuso (checa com o focusbar em foco) |
| 3.8 Insights por regra | 🟠 | Fragmentação REMOVIDA ✓. "Maior ladrão"/"pico" ainda existem — Revisão4 #13 pede: mover pro Claude/MCP no fim do dia |
| 3.9 Ollama (IA local) | 🔵 | Revisão4 #12 pede REMOVER ("só pesa"). Opções: remover de vez / desligado por padrão. Sem ele: regras + match de tópico + Claude via MCP |

## CAMADA 4 — ASSISTENTE (o "JARVIS")

| Bloco | Status | Detalhe |
|---|---|---|
| 4.1 Pomodoro mundo-real | ✅ | Tempo flexível (1h30/90/25:00), lembra última duração, overtime contado, CONFIRMA pausa, pular pausa (contado), confirma retomar, encerrar cedo, +5min, tudo salvo |
| 4.2 Duração da PAUSA configurável | 🟠 | Fixa 5min (dá +5 na hora); falta setting |
| 4.3 Ajustar/renomear pomodoro EM ANDAMENTO | ⬜ | Revisão4 #10 |
| 4.4 Pomodoro "neutro" (sem tarefa) | 🟠 | Dá se não houver foco definido; UX não deixa claro (Revisão4 #11) |
| 4.5 Estimativa em pomodoros por tarefa ("isso leva 2 🍅") | ⬜ | Revisão4 #5 — ideia nova |
| 4.6 Auto-completar tarefa quando termina | 🟠 | Completa ao clicar "terminei"; não automático no timer |
| 4.7 Intenção → tarefas → foco (cascata manual) | ✅ | Revisão4 #6: BUG a investigar — intenção definida não fica visível/repergunta |
| 4.8 Ritual de manhã (intenção) | ✅ | |
| 4.9 Ritual fim de dia ("Acabou por hoje?") | 🟠 | Funciona; horário fixo 18h sem UI pra mudar |
| 4.10 Coach não-punitivo + modos | 🟠 | Foco=cutuca aos 10min / Companheiro=20min / Dia ruim=silêncio. Revisão4 #17: documentar/refinar diferenças |
| 4.11 Mini-agente protagonista | ✅ | Abre no mini, cockpit, 1 direção por vez |
| 4.12 Diário/nota rápida | 🔵 | Revisão4 #14 pede REMOVER (anotar fora do focusbar) |

## CAMADA 5 — INTERFACES

| Bloco | Status | Detalhe |
|---|---|---|
| 5.1 Abas Hoje/Semana/Assistente/Lembretes/Dados | ✅ | |
| 5.2 Timeline (ribbon com gaps) | 🟠 | Revisão4 #18: cores confusas (2 definições misturadas) |
| 5.3 Painel de Dados | ✅ | Pomodoros, média, pausas, trocas, apps, presença. Revisão4: trackear TODO clique de botão (parcial) |
| 5.4 Blocos do dia + recategorizar | ✅ | |
| 5.5 UI do timer 1:1 com o do Google | ⬜ | Revisão4 #3 |
| 5.6 Auto-diagnóstico REAL | ✅ | Verifica que o OCR LÊ de verdade (não só permissão) + botão "Testar os olhos" |

## CAMADA 6 — INTEGRAÇÕES

| Bloco | Status | Detalhe |
|---|---|---|
| 6.1 Servidor MCP local | ✅ | 4 ferramentas (resumo/blocos/pomodoros/semana), read-only, zero API. Chefe validou (#2) |
| 6.2 Copiar pro Claude web | 🔵 | Revisão4 #19: deletar quando o MCP estiver consolidado |
| 6.3 Instaladores (CI) | 🟠 | Windows ✅ (v0.4.0 entregue); Mac corrigido, falta re-validar no CI |
| 6.4 MCP empacotado no Windows (.exe no instalador) | 🟠 | Compila; não embutido no NSIS ainda |

---

# COMPARAÇÃO — focusbar × ActivityWatch × screenpipe

| Capacidade | focusbar | ActivityWatch | screenpipe |
|---|---|---|---|
| Janela/app em foco | ✅ polling 1s | ✅ heartbeat maduro | ✅ evento |
| URL por aba (real, por extensão) | ⬜ | **✅ extensão oficial** (Chrome/FF), aba foco×fundo | 🟠 via OCR/AX |
| Conteúdo da tela (texto) | ✅ AX/UIA + OCR efêmero | ⬜ | **✅ OCR de tudo, sempre** |
| Screenshots salvos | ⬜ (por design) | ⬜ | ✅ (5–15 GB/mês) |
| Áudio + transcrição | ⬜ | ⬜ | ✅ Whisper |
| Ocioso/AFK | ✅ 5 estados (áudio distingue passivo) | ✅ binário AFK | 🟠 |
| Categorização | ✅ regra + IA por conteúdo ancorada + 1-clique | 🟠 regex manual | ⬜ (dado bruto) |
| Pomodoro mundo-real | **✅ único** | ⬜ | ⬜ |
| Intenção→tarefas→foco | **✅ único** | ⬜ | ⬜ |
| Coach não-punitivo + modos TDAH | **✅ único** | ⬜ | ⬜ |
| Rituais manhã/fim de dia | **✅ único** | ⬜ | ⬜ |
| MCP pro Claude | ✅ curado (resumos prontos) | ⬜ (REST) | ✅ (busca bruta) |
| Peso | leve (~0 quando ocioso) | leve | 0.5–3 GB RAM, 5–10% CPU |
| Privacidade | nada sai/nada salvo | local, sem conteúdo | local mas GRAVA TUDO |
| Licença/custo | nosso, grátis | MPL-2.0, grátis | **comercial = licença paga ($25+/mês/seat)** |
| Maturidade | 3 semanas, bugs em queima | ~10 anos, estável | ~2 anos, pesado, issues de recurso |

## Veredito honesto sobre o "90% em 5 prompts"

**O chefe está certo pela metade — e a metade em que ele está certo importa.**

- ✅ **Certo:** a camada de CAPTURA + dashboard é commoditizada. AW + screenpipe entregam captura madura (inclusive URL por aba, que nos falta no Windows) em minutos. Continuar reconstruindo captura na mão é a parte do focusbar que mais deu bug e menos diferencia.
- ❌ **Incompleto:** os "10%" que faltam são exatamente O PRODUTO que vocês descreveram na filosofia: pomodoro do mundo real (todas as regras que o próprio chefe pediu), intenção→tarefas, coach não-punitivo com modos, rituais, mini-agente, categorização com julgamento ancorado, MCP com camadas destiladas (episódios, não dado bruto). **Nada disso existe no AW nem no screenpipe** — e não sai em 5 prompts; saiu em ~3 semanas de iteração com feedback de vocês dois.
- ⚠️ E o screenpipe cobra **licença comercial**, pesa 0.5–3 GB de RAM e grava tudo em disco — três coisas que contrariam o que já foi decidido (zero custo, leve, nada sai/nada salvo).

## Recomendação estratégica (3 opções)

1. **Híbrido (recomendada):** focusbar continua sendo o ASSISTENTE (camadas 3–6) e a captura leve própria continua pra janela/conteúdo. Pra identidade de aba (URL no Windows, aba fechada, foco×fundo), **adotar a extensão do ActivityWatch** (open-source, MPL) apontada pra um endpoint local nosso — resolve o maior buraco sem construir extensão do zero e sem adotar o screenpipe.
2. **Rebuild sobre AW+SP:** portar a camada assistente pra cima do ActivityWatch (e screenpipe se aceitarem licença+peso). Ganha captura madura, perde leveza/privacidade/zero-custo, e re-paga o custo de integração.
3. **Tudo próprio:** continuar como está e construir extensão própria + evento + 2 tabelas. Mais controle, mais tempo.

---

# Checklist Revisão 4 (30.06) — status atual

| # | Item | Status |
|---|---|---|
| 1 | Opera GX mostra browser, não sites (Win) | ⬜ URL no Win não existe — prioridade nº 1 |
| 2 | MCP no Claude Code | ✅ validado |
| 3 | UI do timer 1:1 Google | ⬜ |
| 4 | Mostra duração real após acabar | ✅ |
| 5 | Pomodoros pré-prontos por tarefa (estimativa 🍅) | ⬜ ideia nova |
| 6 | BUG: intenção definida some/repergunta | 🟠 investigar |
| 7/8 | Repensar "focar agora" manual + "checar agora" inútil | 🔵 decisão UX |
| 9 | +5min conta no mesmo pomodoro? | ✅ SIM — estica o mesmo bloco e conta no tempo real |
| 10 | Ajustar/renomear timer em andamento | ⬜ |
| 11 | Pomodoro neutro | 🟠 dá sem foco definido; UX confusa |
| 12 | Tirar Ollama | 🔵 decisão de produto |
| 13 | "Maior ladrão" → Claude/MCP no fim do dia | 🟠 alinhado, falta fazer |
| 14 | Tirar diário/nota rápida | 🔵 pendente |
| 15 | Não usa screenpipe (PII próprio) | ✅ confirmado — captura própria, regex próprio |
| 16 | "Garantir que salva screenshots" | 🔵 **CONFLITO**: hoje NÃO salva por design. Decidir |
| 17 | Diferença entre os modos | 🟠 documentado acima; refinar |
| 18 | Cores da timeline | ⬜ ajustar |
| 19 | Deletar "Analisar no Claude web" pós-MCP | 🔵 pendente |
| 20 | Acesso ao navegador (extensão) | ⬜ ver estratégia híbrida |
