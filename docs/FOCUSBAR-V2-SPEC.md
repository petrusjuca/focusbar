# FOCUSBAR V2 — Especificação completa, função por função

> Escrita em 01.07.2026. Este documento descreve POR EXTENSO como cada função do
> focusbar v2 deve funcionar — pra ser revisado por Petrus + João antes de
> qualquer código. Incorpora TODAS as lições pagas no v1 (3 semanas, 2 auditorias
> multi-agente, 61 testes) e todos os feedbacks (ATT 1–3 + Revisão 4).

---

## 0. PRINCÍPIOS NÃO-NEGOCIÁVEIS

1. **Local-first.** Tudo na máquina. Nada sai sem ação explícita do usuário.
2. **Zero API paga.** IA forte = o Claude DO USUÁRIO via MCP (assinatura). IA local é opcional.
3. **À prova de TDAH.** Zero manutenção obrigatória. O app entende sozinho; o usuário só corrige quando quiser. Trocar muito de aba NUNCA é tratado como falha.
4. **Nunca punitivo.** Nenhuma frase de cobrança. "Travou? quer retomar ou começar leve?" — nunca "você procrastinou 2h".
5. **Honestidade dos dados.** Todo minuto tem dono. Na dúvida, o app diz "não sei" em vez de chutar. Nunca atribuir conteúdo à sessão errada.
6. **Leve.** Sem daemon de 2GB. Sem modelo quente à toa. Roda num notebook fraco.

## 1. ARQUITETURA GERAL (o que muda do v1)

```
┌─────────────────────────────────────────────────────────┐
│ CORE (Rust, daemon local)                                │
│  captura (evento+heartbeat) → porteiro → TABELA BRUTA    │
│  derivador → sessões/blocos → marcadores                 │
│  API HTTP local (127.0.0.1:7690) + servidor MCP          │
└──────────────┬───────────────────────────┬──────────────┘
               │                           │
   ┌───────────▼───────────┐   ┌───────────▼────────────┐
   │ UI WEB (localhost)    │   │ Claude (Desktop/Code)  │
   │ dev: navegador        │   │ via MCP — análise forte│
   │ prod: janela Tauri    │   └────────────────────────┘
   │ apontando pra MESMA UI│
   └───────────────────────┘
   ┌───────────────────────┐
   │ EXTENSÃO DE BROWSER   │──► API local (tab_id, URL,
   │ (MV3, própria ou AW)  │    aba fechada, foco×fundo)
   └───────────────────────┘
```

**Decisões estruturais:**
- **UI web em localhost primeiro** (pedido do João): itera rápido no navegador; empacotar em Tauri é a ÚLTIMA fase, sem retrabalho (a janela Tauri só embute a mesma UI).
- **2 tabelas desde o dia 1**: `events` (bruta, append-only, a verdade) e `sessions` (derivada, recalculável). Mudou a lógica de sessionização? Re-deriva o histórico inteiro.
- **Captura por EVENTO** com heartbeat: `SetWinEventHook`/`EVENT_SYSTEM_FOREGROUND` (Win) e `NSWorkspace didActivateApplication` (Mac) como fonte primária; poll a cada 15–30s só como confirmação/fallback.
- **Extensão de browser** como fonte de identidade de aba (resolve URL no Windows, aba fechada, foco×fundo de uma vez).
- **Transplantes do v1** (código testado, não reescrever): porteiro (`redact.rs`), categorias/regras (`category.rs`), âncora anti-alucinação (`ai.rs`), sinais nativos (`signals.rs`), nomes de site + 2 Claudes (`browser.rs`), servidor MCP (`mcp.rs`), máquina de estados (`state.rs`).

---

## 2. CAPTURA — como cada função funciona

### 2.1 Janela em foco (evento + heartbeat)
- **Windows:** `SetWinEventHook(EVENT_SYSTEM_FOREGROUND)` → callback com HWND → resolve processo (pid, exe) + título. Timestamp exato da troca.
- **Mac:** observer do `NSWorkspace` (app ativado) + leitura do título via AX (`AXFocusedWindow.AXTitle` — só exige permissão de Acessibilidade, NÃO Gravação de Tela).
- **Heartbeat (15–30s):** confirma que a janela do último evento ainda está na frente; se divergir (evento perdido), emite um evento sintético `source=poll`.
- Cada troca grava UMA linha na tabela BRUTA: `ts, app, exe, título, pid, source(evento|poll)`.
- **Debounce:** trocas < 2s não geram sessão derivada (mas FICAM na bruta — nada se perde).

### 2.2 URL e identidade de aba (browser)
Três fontes, em ordem de preferência:
1. **Extensão (MV3, própria — ~200 linhas):** escuta `tabs.onActivated`, `tabs.onUpdated`, `tabs.onRemoved`, `windows.onFocusChanged` e POSTa pra `127.0.0.1:7690/api/tab-event`. Dá: URL real da aba ativa, tab_id estável, evento de FECHAR aba, e distinção aba-em-foco × aba-no-fundo. Funciona em Chrome/Edge/Brave/**Opera GX**/Firefox — nos DOIS SOs. Segurança: só fala com localhost, nenhum dado sai.
2. **Mac sem extensão:** AppleScript (Chrome/Safari/Edge/Brave/Arc/Vivaldi/Opera) + fallback por AX pro Opera GX (lê a barra de endereço na árvore).
3. **Windows sem extensão:** UI Automation — achar o controle da barra de endereço (`UIA_EditControlTypeId` + name heurístico) do processo do browser.
- A URL passa por `clean_url` (corta query/fragment — tokens/PII moram lá) e vira `site_label` ("YouTube", "Claude (cowork)", …).
- ⚠️ Lição do v1: **Opera GX não é scriptável por AppleScript** — por isso a extensão é o caminho definitivo.

### 2.3 Conteúdo da tela (os "olhos")
Pipeline em 3 estágios, rodando UMA vez por sessão estável (~2s após a troca):
1. **Árvore de acessibilidade** — Mac: AX walk (Value/Title/Description, bounded: profundidade 10, 320 nós, 1500 chars). Windows: UI Automation walk equivalente.
   - ⚠️ Lição: **Chrome/Electron escondem a página da acessibilidade**. Setar `AXManualAccessibility=true` no app destrava Electron (Claude, VS Code, Ollama). Pro corpo de página do Chrome, mirar a `AXWebArea`; se não existir, cair pro OCR.
2. **Título limpo como base** — remover o lixo do Chrome ("Uso elevado da memória em… : 1,2 GB - Google Chrome: perfil", "(NN)", "- Reprodução de áudio"). O nome real da página já é sinal bom.
3. **OCR (fallback de pixel)** — captura da janela do **pid da sessão** (nunca "a janela em foco agora" — o OCR leva segundos e o foco pode mudar pra um app sensível). Se a enumeração de janelas falhar (⚠️ lição: **xcap é intermitente** — às vezes lista 2 janelas em vez de 18), fallback pra captura de MONITOR **somente se** a janela em foco ainda é a da sessão (`focused_pid == pid`) — senão devolve nada (honestidade > cobertura). OCR nativo: Apple Vision (Mac) / Windows.Media.Ocr (Win). **Imagem em memória, nunca em disco** (ver decisão pendente D1).
- Todo texto passa pelo **porteiro** antes de persistir.

### 2.4 Porteiro (privacidade) — transplante do v1
- Redige: sequências 32+ chars (tokens/chaves), CPF, cartão, e-mails em query, `?token=`/`#access_token=`.
- **Zonas de exclusão:** apps/sites de banco, gerenciador de senhas, saúde → a sessão é registrada ("Nubank, 4min") mas o CONTEÚDO nunca é capturado.
- Senhas em campo de senha já vêm mascaradas pela acessibilidade do SO; o OCR não as vê.

### 2.5 Presença (idle / áudio / lock) — transplante do v1
Sinais por tick: `idle_secs` (SO), `audio_active` (CoreAudio/WASAPI `GetPeakValue>0.01`), `locked` (CGSession/OpenInputDesktop).
Máquina de estados (primeira que casar vence):
1. pausado manualmente → PAUSADO
2. tela bloqueada → AUSENTE
3. idle < 60s → ATIVO
4. idle ≥ 60s + áudio → PASSIVO (assistindo)
5. idle 60s–5min sem áudio → OCIOSO curto
6. idle ≥ 5min sem áudio → AUSENTE
7. idle ≥ 15min mesmo com áudio → AUSENTE-INCERTO (flag)
Limiares configuráveis. ⚠️ Lições Windows: `IMMDevice::Activate` exige features `Win32_System_Com_StructuredStorage` **e** `Win32_System_Variant`; lock via `OpenInputDesktop` (desktop ≠ "Default" = travado).

---

## 3. DADOS — como o armazenamento funciona

### 3.1 Tabela BRUTA `events` (append-only, a verdade)
`id, ts_ms, kind(foreground|tab|state|pomodoro|ui_click), app, título, url, tab_id, pid, source(evento|poll|extensão), estado_presença, payload_json`
- NUNCA se edita. Re-derivável. Inclui **cada clique de botão do focusbar** (pedido Revisão 4: "cada apertar de botão trackeado").

### 3.2 Derivada `sessions`
`id, início, fim, app/site, título/url, tab_id, dur_ativa, dur_passiva, dur_ociosa, categoria, categoria_fonte(user|ia|regra), confiança, conteúdo(porteirado), activity_ai`
- Derivador roda incremental (a cada fechamento de sessão) e pode RE-DERIVAR tudo do zero.
- **Sessionização:** mesma chave de atividade (app + tab_id/título) reaparecendo em ≤90s = mesma sessão. Blocos = sessões encadeadas da mesma chave.

### 3.3 `markers` (gaps): pausado / ausente / sem-dados — todo minuto tem dono. "Sem dados" honesto quando o app não estava rodando.

### 3.4 Demais: `pomodoro_log` (goal, início, planejado, real, cumpriu, pausas_puladas, demorou_pra_pausar), `todos`, `intentions`, `settings`.

### 3.5 Migração: importador lê o `focusbar.db` v1 e converte pro schema novo. Histórico não se perde.

---

## 4. ENTENDIMENTO — como a categorização funciona

### 4.1 Camadas (da mais barata pra mais cara; primeira que resolve, para):
1. **Correção do usuário** (sticky, por app+foco) — manda sempre.
2. **Regra por site/app** (transplante `category.rs`, match por token inteiro, sem substring).
3. **Conteúdo → IA local ancorada** (SE Ollama presente — ver D2): só pode CONTRARIAR a regra com prova (palavra ≥4 letras da resposta presente no texto real, token inteiro). "INCERTO" cai pra regra. Nunca inventa.
4. **Claude via MCP** (fim do dia): análise de blocos, multitarefa, intenção×realidade — coisas que regra nenhuma faz (Revisão 4 #13).

### 4.2 Blocos: a IA nomeia (`activity_ai`); UI permite **renomear** e **recategorizar em 1 clique** (vale só pro bloco; a correção vira sinal sticky por app+conteúdo).

### 4.3 Insights por regra: **mínimos**. Sem "maior ladrão", sem "fragmentação" (NUNCA cobrar troca de aba), sem "intenção×realidade por palavra". Isso tudo migra pro Claude/MCP que enxerga o dia inteiro com contexto.

---

## 5. ASSISTENTE — como cada função funciona

### 5.1 Pomodoro do mundo real (regras completas, do feedback do João)
- **Iniciar:** por tarefa (▶ na to-do) ou **neutro** (botão sempre visível, sem tarefa). Duração: campo estilo timer do Google — aceita `1h30`, `90`, `25:00`, `45s`; presets 15/25/50; **o valor digitado no campo alimenta os botões em tempo real**; lembra a última duração POR TAREFA.
- **Durante:** pode **+5min**, pode **renomear**, pode **ajustar o tempo restante**, pode pausar o bloco.
- **Fim do tempo:** NÃO inicia pausa. Vira **overtime** (conta no tempo real da tarefa). 3+ min de overtime → anotado "demorou pra encerrar"; 10+ min → pergunta na tela (1 vez, suave).
- **Pausa:** só com confirmação. Duração configurável (não fixa em 5). "Pular pausa" existe e é CONTADO. Fim da pausa → **não auto-retoma**: "bora pro próximo?" — até clicar, o tempo conta como pausa.
- **Encerrar cedo:** "✓ terminei" a qualquer momento → grava tempo real, marca a to-do como completa **automaticamente**, registra sucesso.
- **Tudo salvo:** nome, horário, planejado×real, cumpriu?, pausas puladas → base pro Claude achar "seu horário de mais sucesso".
- **Estimativa em 🍅 (Revisão 4 #5):** to-do aceita estimativa ("acho que 2 pomodoros"); o app compara estimado × real.

### 5.2 Intenção → tarefas → foco (UM conceito só)
- **Manhã (dia novo):** "Qual a intenção de hoje?" (1 frase, o NORTE). Pode pular.
- A intenção fica **SEMPRE visível** (mini + topo da aba Hoje) — corrige o bug Revisão 4 #6.
- Você quebra a intenção em tarefas MANUALMENTE (sem IA inventando). Cada tarefa: ▶ = vira o foco + inicia pomodoro.
- **Não existe** "foco manual" separado (Revisão 4 #7/#8): o foco É a tarefa ativa. Sem tarefa ativa = sem checagem ao vivo. O botão "checar agora" morre.

### 5.3 Coach + modos
- **Foco:** cutuca cedo (10min fora), som suave. **Companheiro (padrão):** cutuca aos 20min, 1×/15min no máx. **Dia ruim:** zero cobrança; só encorajamento e celebração de qualquer entrega.
- Copy sempre não-punitiva. A checagem "está na tarefa?" só roda com pomodoro ativo E só notifica com evidência forte (conteúdo real capturado); na dúvida, silêncio.

### 5.4 Rituais
- Manhã: intenção (acima). **Fim de dia:** horário CONFIGURÁVEL na UI (default 18h, snooze +1h) → "Acabou por hoje?" → 1 clique abre o resumo do dia (e, com MCP configurado, o botão profundo: "analisar com Claude").

### 5.5 Mini-agente (protagonista)
Card flutuante sólido, 1 direção por vez: tarefa atual + timer + estado. Janela grande = histórico/config. Sem drenar bateria (nenhuma IA em loop; atualização por evento).

### 5.6 Removidos (decisões Revisão 4): diário/nota rápida (#14 — ver D3), "Analisar no Claude web" morre quando o MCP estiver consolidado (#19).

---

## 6. INTERFACES

- **Aba Hoje:** intenção (sempre visível) → tarefas (▶) → pomodoro → blocos do dia (recategorizar/renomear 1 clique) → timeline.
- **Timeline:** UMA legenda só (cores por categoria; gaps hachurados com rótulo) — corrige Revisão 4 #18.
- **Aba Dados:** TUDO exposto (pomodoros, média, pausas, puladas, trocas, sites, presença, cliques de UI). Feio ok, completo obrigatório.
- **Aba Config:** modos, horário fim de dia, duração de pausa, limiares de presença, OCR on/off, zonas de exclusão, MCP (comandos prontos), auto-diagnóstico.
- **Auto-diagnóstico REAL:** health-check no startup que OCRa a própria tela e grava resultado + botão "Testar os olhos agora" (mostra o texto lido). Permissão OK ≠ funcionando — o teste é a prova.

## 7. INTEGRAÇÕES

- **MCP local** (transplante): `resumo_do_dia`, `blocos_do_dia`, `pomodoros_do_dia`, `resumo_da_semana` + v2: `sessões(intervalo)`, `intenção_do_dia`, `historico_pomodoros(n_dias)`. Read-only, stdio, robusto a lixo no stdin (lição v1).
- **API HTTP local** (`127.0.0.1:7690`): a mesma que a UI usa; a extensão posta eventos nela.
- ⚠️ Lição: Ollama/tudo local usa `127.0.0.1`, NUNCA `localhost` (IPv6 quebra).

---

## 8. LIÇÕES PAGAS NO V1 (embutir no v2 desde o dia 1)

1. xcap lista janelas de forma INTERMITENTE → captura por janela com retry + fallback monitor GUARDADO por pid.
2. Chrome esconde a página da acessibilidade → `AXManualAccessibility` + AXWebArea + OCR.
3. Opera GX não tem AppleScript → extensão/AX.
4. Preflight de permissão de Gravação de Tela MENTE no macOS → não gatear; tentar e falhar gracioso; health-check real.
5. IA 3B alucina por função do app → âncora: só aceitar com prova textual (token inteiro, não substring).
6. `localhost` → IPv6 → usar `127.0.0.1`.
7. Windows crate: `IMMDevice::Activate` precisa `Win32_System_Com_StructuredStorage` + `Win32_System_Variant`; checar compilação com target `x86_64-pc-windows-gnu` LOCALMENTE antes do CI.
8. Título do Chrome vem decorado com lixo de memória/contadores → limpar sempre.
9. Fallback de captura NUNCA pode atribuir conteúdo de outra janela à sessão (guard por pid).
10. DST: meia-noite pode não existir → `earliest()` com fallback 01:00.
11. Nunca alertar fragmentação/troca de aba (TDAH — é o modo normal de trabalhar).
12. Tauri universal build quebra com 2º binário → Mac nativo no CI, ou lipo manual.

## 9. DECISÕES ✅ TOMADAS (Petrus, 02.07.2026)

- **D1 — Screenshots: SALVAR com retenção curta (24–48h).** Local, com auto-limpeza e toggle. Dá o "ver em que aba estava" sem virar arquivo eterno.
- **D2 — Ollama: REMOVER de vez.** Categorização = regras + correção 1-clique + Claude via MCP no fim do dia. App leve.
- **D3 — Diário/nota rápida: REMOVER.** Anotação vive fora do focusbar. A intenção do dia continua.
- **D4 — Extensão de browser: PRÓPRIA.** MV3 mínima, fala só com 127.0.0.1.

## 10. PLANO DE CONSTRUÇÃO (fases com critério de aceite verificável)

| Fase | Entrega | Critério de aceite |
|---|---|---|
| 0 | Spec revisada por vocês 2 | decisões D1–D4 tomadas |
| 1 | Core: captura evento+heartbeat → tabela bruta → derivador → API local | rodar 1 dia; bruta bate com a realidade; re-derivar produz os mesmos blocos |
| 2 | UI web (localhost): Hoje + timeline + dados | usável no navegador; blocos corretos |
| 3 | Extensão de browser → tab events | URL certa no Opera GX/Win; aba fechada registrada |
| 4 | Conteúdo (AX/UIA/OCR) + porteiro + categorização | auto-teste OCR ok:N; zero conteúdo em sessão errada |
| 5 | Pomodoro completo + intenção/tarefas + rituais + modos | checklist do João (ATT2 itens 7–14 + Rev4 #5,9,10,11) |
| 6 | MCP + migração do histórico v1 | Claude responde "como foi meu dia" com dados reais |
| 7 | Empacotar (Tauri) + instaladores CI Mac/Win | .exe e .app instalam e rodam |

Cada fase fecha com verificação REAL (compila + testes + rodar + olhar o banco), não "parece pronto".
