# focusbar — Arquitetura em blocos + comparação com ActivityWatch e screenpipe

> Gerado em 01.07.2026 a partir do código real (auditado por agentes, item a item).
> Legenda: ✅ funciona · 🟠 parcial/precisa validar · ⬜ não existe · 🔵 decisão de produto pendente

---

## CAMADA 1 — CAPTURA (os sentidos)

| Bloco | Status | Detalhe |
|---|---|---|
| 1.1 Janela em foco | ✅ | Polling 1s (`active-win-pos-rs`). Captura por EVENTO ainda não (⬜) — polling funciona, evento é otimização |
| 1.2 URL do browser (Mac) | ✅ | AppleScript (Chrome/Safari/Edge/Brave/Arc/Vivaldi/Opera) + fallback por Acessibilidade pro Opera GX (🟠 em teste) |
| 1.3 URL do browser (Windows) | 🟠 | Fase A (v0.5.0): extensão própria → endpoint local → sampler. Falta validar no Windows/Opera GX do João |
| 1.4 Conteúdo da tela (Mac) | ✅ | Árvore de Acessibilidade + `AXManualAccessibility` (destrava Electron: Claude, Ollama, Code) + mira AXWebArea |
| 1.5 Conteúdo da tela (Windows) | 🟠 | UI Automation implementado; compila, **runtime a validar pelo chefe** (v0.4.0) |
| 1.6 OCR (fallback de pixel) | ✅ | xcap por janela → fallback tela cheia GUARDADO por pid (não atribui à sessão errada). Health-check no startup + botão "Testar os olhos". Imagem **em memória, nunca salva** |
| 1.7 Presença: idle | ✅ | `user-idle`, sinal primário |
| 1.8 Presença: áudio tocando | ✅ | CoreAudio (Mac) + WASAPI (Win) — separa "assistindo" de "AFK" |
| 1.9 Presença: tela bloqueada | ✅ | CGSession (Mac) + input desktop (Win) |
| 1.10 Porteiro (PII) | ✅ | `redact.rs`: senha/CPF/cartão/token (32+ chars) + zonas de exclusão (banco/senha nunca capturados) |
| 1.11 Título limpo | ✅ | Remove o lixo do Chrome ("Uso elevado da memória… 1,2 GB - Google Chrome: perfil") |
| 1.12 Extensão de browser (tab_id, aba fechada, foco×fundo) | ✅ | Fase A (v0.5.0): MV3 própria em `extension/`, só permissão `tabs`, só 127.0.0.1. Aba fechada registrada. Falta: `audible` no payload (roubar do AW) |
| 1.13 Áudio/microfone/Whisper | ⬜ | Por design (privacidade/peso). screenpipe tem |

## CAMADA 2 — ARMAZENAMENTO (a memória)

| Bloco | Status | Detalhe |
|---|---|---|
| 2.1 `focus_events` (sessões) | ✅ | Uma linha por troca de janela, com conteúdo + categoria |
| 2.2 `interval_markers` (pausado/ausente) | ✅ | Todo minuto tem dono |
| 2.3 `pomodoro_log` | ✅ | goal, início, planejado, real, cumpriu? |
| 2.4 `todos` / `notes`(intenção) / `settings` | ✅ | |
| 2.5 Tabela BRUTA separada (re-derivável) | 🟠 | `tab_events` (v0.5.0) é o embrião: eventos crus de aba com retenção 90d. Falta generalizar pra eventos de janela/estado (fase do derivador) |
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
| 1 | Opera GX mostra browser, não sites (Win) | 🟠 Fase A entregue (v0.5.0: extensão própria + endpoint 127.0.0.1:7690) — falta validar no Windows/Opera GX do João |
| 2 | MCP no Claude Code | ✅ validado |
| 3 | UI do timer 1:1 Google | ⬜ |
| 4 | Mostra duração real após acabar | ✅ |
| 5 | Pomodoros pré-prontos por tarefa (estimativa 🍅) | ⬜ ideia nova |
| 6 | BUG: intenção definida some/repergunta | 🟠 investigar |
| 7/8 | Repensar "focar agora" manual + "checar agora" inútil | 🟠 "checar agora" consertado (v0.5.2: julga o último app REAL, não o focusbar); repensar "focar agora" segue aberto |
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
| 20 | Acesso ao navegador (extensão) | ✅ extensão PRÓPRIA MV3 (`extension/`, v0.5.0) — só `tabs`, só loopback |

---

# Pesquisa 02.07.2026 — o que roubar do screenpipe v2 e do ActivityWatch

Pesquisa profunda nos dois projetos (código-fonte, docs, issues 2025-2026), feita
depois da Fase A. Duas validações grandes antes da lista:

- **O screenpipe ABANDONOU a captura contínua.** A v2 deles (2025-26) é
  event-driven + accessibility-first, OCR só como fallback — exatamente a
  arquitetura do focusbar. O "grava tudo sempre" morreu de RAM (issues de
  10GB+/OOM). Nossa decisão de OCR efêmero só da janela em foco está certa.
- **O ActivityWatch confirma o desenho da tabela bruta**: event log cru +
  derivação na leitura (nada de gravar categoria no evento). E o AW está
  migrando o manager pra **Tauri** (aw-tauri) — mesma stack nossa.

## Roubar do ActivityWatch (em ordem)

1. **Heartbeat + merge por dado idêntico (pulsetime)** — o watcher nunca "abre/
   fecha" sessão; manda estado pontual e o CORE funde eventos adjacentes iguais
   dentro de uma janela (`pulsetime ≈ intervalo + folga`). Mata a classe inteira
   de bug "sessão órfã de 9h" (crash/sleep/kill). É o miolo da fase do derivador.
2. **Double-heartbeat na transição** — na troca, manda o dado ANTIGO em t-1ms e
   o novo em t: fecha o período anterior com precisão de troca de aba, sem estado.
3. **`audible` no payload da extensão** (`tab.audible` do Chrome) — a forma
   barata de não perder "assistindo vídeo sem tocar no mouse": une audible com
   not-afk na derivação. Encaixa direto no nosso estado Passive.
4. **Browser = fonte de CONTEÚDO; OS = fonte de verdade sobre FOCO/TEMPO** —
   a extensão MV3 mente sobre foco (nem sabe). Casar por interseção de períodos
   com a janela ativa. (O tab_feed da Fase A já faz isso — validado.)
5. **AFK datado retroativamente do último input real**, não de quando o timeout
   estourou. (Nosso open_marker já recua pro último input — validado.)
6. **NÃO copiar do AW:** (a) guardar bruto pra sempre sem agregado materializado
   — dashboards deles ficam lentos com 1-2 anos de dado; nós temos daily_rollups,
   manter e ampliar; (b) casar extensão↔janela por lista hardcoded de nomes de
   browser (manutenção eterna) — nosso matching por token + título já é melhor.

## Roubar do screenpipe v2 (em ordem)

1. **Escada de intervalos por atividade** — polling não precisa ser fixo em 1s:
   input recente → rápido; idle curto → 1s; idle fundo → 2s+. Menos bateria.
2. **Perfis de energia** — na bateria, intervalos 2×; bateria ≤20%, desligar o
   trabalho pesado (OCR); ≤10%, pausar captura (app continua vivo).
3. **Trabalho pesado só quando o CPU está ocioso** — adiar OCR/sumarização até
   o CPU ficar N segundos abaixo de um threshold (não OCRar durante uma call).
4. **Filtro de privacidade NA ENTRADA** (não pós-filtro): domínio bloqueado não
   gera nem evento (match por boundary de domínio: `chase` casa chase.com, não
   purchase.com). Nossas zonas de exclusão já fazem isso pra apps; estender a
   domínios da extensão.
5. **Retenção em camadas ("Lean")** — apagar o payload pesado velho mantendo o
   texto derivado pesquisável. Nosso purge de 90d do tab_events é o embrião.
6. **Anti-armadilha:** backoff exponencial + circuit breaker em QUALQUER loop de
   retry/monitor (o leak de 2.7GB/h deles em 2026 foi um loop de recovery de
   áudio sem backoff).
7. **Produto:** as features que os usuários deles realmente usam são recap do
   dia, standup automático, breakdown de tempo e nudge de distração — todas já
   no nosso roadmap (MCP fim do dia, insights, coach). Nenhum dos dois tem
   pomodoro/intenção — segue sendo NOSSO diferencial.
