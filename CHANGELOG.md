# focusbar — o que mudou

## v0.10.0 — o batch do checklist: 🍅 por tarefa, renomear em andamento, Relógio Xadrez e mais
- **Tarefa com tempo próprio + estimativa 🍅 (Rev4 #5):** ao criar a tarefa você
  diz "leva 30min" e "2 🍅" — o ▶ focar usa O TEMPO DELA, não um 25min cravado.
- **Renomear o pomodoro EM ANDAMENTO (Rev4 #10):** clica no 🎯, digita, Enter —
  o tempo nem pisca. Vale no card e no mini.
- **Pomodoro neutro claro (Rev4 #11):** sem tarefa escolhida o bloco vale
  igual, dizemos isso na cara, e dá pra nomear depois clicando no 🎯.
- **♟️ Relógio Xadrez (ideia do Petrus):** dois relógios com a mesma meta
  (padrão 4h, configurável) — produtividade × distração. Vença o seu dia.
- **Análise da Semana (seg–dom) × semana anterior:** compara o MESMO trecho já
  vivido ("quarta × quarta") + a anterior inteira + a maior mudança por
  categoria. A comparação evolui com a semana.
- **Lembretes v2:** o aviso agora FICA NA TELA até você resolver — com
  "✓ feito" e "chega por hoje" (silencia até amanhã, volta sozinho).
- **Fim de dia no SEU horário:** o "acabou por hoje?" tem hora configurável em
  Configurações (era 18h cravado).
- **Insights por regra saíram do Hoje (Rev4 #13):** o "maior ladrão" e cia.
  agora vivem no recap do Claude via MCP — com contexto, não com regrinha.

## v0.9.0 — screenshots com retenção curta (decisão D1 executada)
- **"Ver em que aba estava" existe agora:** o app salva UM screenshot por
  sessão estável (a mesma captura que alimenta o OCR — nada de fotografar
  duas vezes), local, e cada bloco do dia ganha um 📸 que abre a imagem.
- **Retenção curta por contrato:** os shots somem sozinhos em 48h (varrida no
  startup e a cada 6h). Zonas de exclusão (banco/senha) continuam NUNCA sendo
  capturadas, e o toggle fica em Configurações → Olhos.
- É a matéria-prima pro julgamento por conteúdo: o Claude (e uma futura
  camada multimodal) passa a ter o que olhar quando o texto não bastar.

## v0.8.1 — timer clicável, timeline com UMA linguagem de cor, diário fora
- **Clica no relógio e digita o tempo** (igual o timer do Google): "10",
  "1h30", "25:00" — Enter e o restante vira exatamente isso, no mini e no
  card. Funciona até no overtime (ganha um novo fôlego) e na pausa.
- **Cores da timeline consertadas (Rev4 #18):** era DOIS sistemas misturados
  (cor aleatória por app × legenda por estado). Agora é um só: verde = foco,
  listrado = pausado, âmbar = ausente, trilho = sem dados — a legenda sempre
  visível e batendo com o que você vê. Quem diz o app é o tooltip e os blocos.
- **Diário/nota rápida REMOVIDO** (decisão D3 / item #14 do João): anotação
  vive fora do focusbar; a intenção do dia continua no ritual da manhã.
- **Identifica mais coisa sozinho:** Roblox Studio, Sketchfab, Blender, Unity
  (criar ≠ jogar), Claude/ChatGPT/Cursor, Miro e Canva não caem mais em
  "Outro"/"Procrastinação" por engano.

## v0.8.0 — timer do Google + revisão do "Outro"
- **Anel de progresso estilo timer do Google** (pedido do João, Rev4 #3): o
  círculo esvazia conforme o tempo passa, dígitos grandes no centro — no mini
  E no card do Modo Foco. Pausa muda a cor; no overtime o anel fica cheio e o
  card pisca.
- **Meta: "Outro" < 5% do dia.** Quando passa disso, o Hoje mostra um aviso
  ("X% do teu dia está em Outro — me ensina?") que abre a REVISÃO GUIADA: só
  os blocos sem categoria clara, maiores primeiro, corrige no toque e cada
  correção vira memória permanente. É o que faz o recap do Claude melhorar
  sozinho, dia após dia.

## v0.7.1 — reorganização FLOWMODE (telas no lugar do teu mapa mental)
- **"Assistente" virou "Configurações"** (o nome antigo era misleading) e
  "Semana" virou "Análise da Semana". Ordem das abas segue o planejamento.
- **Modos do dia com as diferenças POR EXTENSO** em cards dentro de
  Configurações (companheiro ~20min · foco ~10min · dia ruim = silêncio) —
  nada de decorar; tema claro e "iniciar com o sistema" também moraram pra lá.
- **Saiu a cascata "intenção → um passo"** do Hoje (decisão FLOWMODE: a
  intenção vive no ritual da manhã e no mini; tarefas vivem na lista).
- **Campo de tempo livre GRANDE** no Modo Foco (dá pra enxergar o que digita).
- **Fim de timer chamativo:** o card pulsa e o relógio pisca quando o tempo
  cumpre (e na volta da pausa) — pra não passar batido.


## v0.7.0 — o cérebro é o Claude (Ollama removido) + intenção que não some
- **Ollama removido de vez** (decisão D2 / pedido do João): o app não roda LLM
  local. Ao vivo, o juiz de foco usa só camadas confiáveis — regra que você
  ensinou (1 clique), match com a intenção, categoria. Rápido, leve, zero
  alucinação. App mais enxuto (menos uma dependência de rede inteira).
- **O julgamento profundo é do Claude, via MCP — e agora o veredito VOLTA:**
  - `recap_do_dia`: o dossiê do fim do dia num tiro só (intenção declarada,
    resumo, blocos, pomodoros, média da semana + instruções de análise gentil).
  - `standup`: ontem + hoje condensados pro update de standup sair pronto.
  - `corrigir_categoria`: a ÚNICA escrita do MCP — o Claude corrige a categoria
    de um bloco ("esses 40min de YouTube eram estudo") e o dashboard amanhece
    certo. Validação das 7 categorias; todo o resto segue read-only.
- **Bug da intenção que some/repergunta (Rev4 #6) consertado** — eram 4 juntos:
  o "dia" do front era UTC (depois das 21h o ritual da manhã voltava a
  perguntar); o "☀️ Bom dia" aparecia à meia-noite pra quem estava acordado
  (agora só a partir das 4h); e definir/apagar intenção agora reflete NA HORA
  na cascata e no mini (antes esperava o próximo poll).
- **Painel dos sentidos:** o auto-diagnóstico agora inclui a extensão de
  browser (conectada ✓ / parou de reportar ⚠) e perdeu o sensor de Ollama.
- Aba Assistente reescrita: MCP em primeiro, copiar-pro-Claude como fallback.

## v0.6.0 — Fase B: heartbeat + derivador (dado que sobrevive a crash)
- **Nenhuma sessão se perde mais.** Antes, a sessão só era gravada quando
  FECHAVA — crash, forçar-sair ou bateria acabando no meio de 2h de trabalho =
  2h perdidas. Agora a sessão nasce no banco assim que se firma (2s) e o fim é
  empurrado a cada batida do rastreador (modelo do ActivityWatch): o erro
  máximo passa a ser 1 segundo, sempre.
- **Blocos mais honestos (derivador da v2 transplantado):** "Code → 10s de
  WhatsApp → Code" agora vira UM bloco de Code (com o WhatsApp como blocozinho
  no meio) — a visita curta não fragmenta mais o trabalho em três pedaços. A
  duração soma só o tempo REAL (o gap tolerado não conta).
- Bônus: a sessão em andamento aparece no "Hoje" crescendo ao vivo (antes só
  aparecia quando você trocava de janela).

## v0.5.2 — "checar agora" julga o que você FAZIA, não o próprio focusbar
- Clicar em **"checar agora"** foca o próprio focusbar — e aí o juiz via…
  o focusbar, sempre. Agora, nesse caso, ele julga o **último app real** que
  você estava usando (visto pelo rastreador nos últimos 2min), passando pelos
  mesmos porteiros de privacidade (zona de exclusão, título limpo, redação).

## v0.5.1 — o juiz de foco não cita mais o lixo do Chrome
- O agente de foco julgava (e citava como "evidência") o aviso que o Chrome
  enfia no título da janela — "**Uso elevado da memória**", contador "(NN)",
  memória em GB. Agora o juiz recebe o **título limpo** (mesma limpeza que o
  histórico já usava) e, quando a Acessibilidade só devolve a moldura do
  Chrome, cai pro título limpo + OCR em vez de julgar a barra de abas.

## v0.5.0 — extensão de browser (Fase A do roadmap)
- **Sites de verdade em QUALQUER navegador:** nova extensão (pasta `extension/`,
  instala descompactada no Opera GX/Chrome/Edge) reporta a aba ativa ao focusbar
  via `127.0.0.1:7690`. Acaba o "4h51 no Opera GX" sem dizer onde — agora é
  WhatsApp, YouTube, Miro… também no **Windows** e no **Opera GX**.
- **API local nova** (`127.0.0.1:7690`, só loopback): `POST /api/tab-event`
  (extensão) e `GET /api/health` (sinal de vida). URL gravada **sem query/
  fragment** (mesma regra de privacidade de sempre — duas vezes: na extensão
  e no app).
- **Registro cru de abas** (`tab_events`): ativou/mudou/fechou, com retenção
  de 90 dias — o embrião da tabela bruta do roadmap.
- Ordem de resolução de URL no rastreador: AppleScript → **extensão** →
  Acessibilidade. Com dois navegadores abertos, a aba só é usada se o
  navegador dela é o que está em foco (sem contaminação).

## v0.3.0 — auditoria geral (46 pendências resolvidas)
Release grande de qualidade, a partir de uma auditoria multi-agente do app inteiro.

- **Privacidade blindada (porteiro):**
  - A **checagem de foco** agora pula zonas de exclusão (banco/senha/saúde) — antes,
    o título dessas janelas podia ir pro modelo. Corrigido.
  - **URLs limpas:** query e fragment (onde vivem tokens, reset links, `access_token`,
    `?email=`) são removidos antes de gravar/analisar.
  - Redação cobre mais: **e-mail, JWT, celular** (além de CPF/cartão/senha/token).
  - Zonas de exclusão mais espertas: não confunde "**banco de dados**"/"**caixa de
    entrada**" com banco; pega bancos digitais (Nubank, Inter, C6, PicPay…).
  - **Local-first de verdade:** o endpoint da IA só sai da máquina com opt-in explícito + https.
- **Insights que mandam ação** (sem IA, só regra): maior **ladrão de tempo** (nomeado),
  **intenção declarada vs. realidade**, maior **bloco de foco contínuo**, comparação com
  a **média da semana**, e **nº de trocas de tarefa**.
- **Analisar no Claude.ai num clique:** botão em destaque na aba **Hoje**; copia o resumo
  **e abre o Claude.ai** sozinho (você só dá Cmd+V). Funciona pra **hoje ou ontem**.
  O resumo agora leva **data, total e top categorias**.
- **Polish:** estado de **carregando**, aba **Semana** com estado vazio, **erros amigáveis**,
  card "AGORA" respeita a **pausa**, e a **janela pequena** volta ao tamanho certo.
- **Robustez:** fronteira de dia à prova de **horário de verão**; leitura de URL do
  navegador com **timeout** (não trava o rastreador); trava de segurança contra deadlock do DB.
- **Velocidade:** dashboard recarrega **por evento** (não 40×/min) — ~700 queries/min a menos;
  **bundle 60% menor** (596KB → 240KB, gráficos sob demanda).
- **Build:** instalador do Mac agora é **universal** (Intel + Apple Silicon).

## v0.2.6 — categorias editáveis
- **Corrige a categorização "burra":** se um app/site estiver na categoria errada
  (ex.: YouTube de estudo marcado como procrastinação), você ajusta num clique em
  **Hoje → Por categoria → "ajustar"**. Ele lembra pra sempre.
- O override vale em tudo: gráfico, insights, alerta do coach e resumo da IA.

## v0.2.5 — ritual diário
- **Diário de hoje** (aba Hoje): escreva sua **intenção do dia** ou uma **nota rápida**.
- A IA **compara o que você queria fazer com o que realmente fez** no resumo.

## v0.2.4 — janela compacta
- Botão **"Janela pequena"** (rodapé): vira um cantinho **sempre no topo** com o app
  atual + foco do dia + pause. Pra você nunca esquecer dele. Clique ⤢ pra voltar.

## v0.2.3 — feedback do chefe
- **Navegador por site:** WhatsApp, Miro, YouTube viram entradas próprias no "tempo
  por app" (não mais "Chrome"/"Opera" engolindo tudo).
- **Pausar/Retomar** rastreamento (no app e no ícone da barra de menu) — pausado não
  conta como nada (nem procrastinação).
- **Alerta de fragmentação removido** (incomodava quem usa muitas janelas).
- **"Copiar resumo do dia"** (aba Assistente) → cola no Claude.ai pra uma análise
  inteligente, sem custo.
- Mensagem do Ollama mais clara (abrir o app + testar em localhost:11434).

## v0.2.0–v0.2.2 — camada de assistente (IA local)
- **Resumo do dia com IA local** (Llama via Ollama) — episódios, como foi o dia, 1 melhoria.
- **Porteiro de privacidade:** redige senha/CPF/cartão/token; pula apps de banco/senha.
- **Setup da IA por clique** (instala Ollama + baixa modelo, sem terminal).

## v0.1.0 — base
Rastreio de tempo por app/janela (metadados, local), troca de foco, ociosidade,
dashboard diário/semanal, timeline, categorias, tasks, lembretes nativos, coach com
alertas e insights, roda em background (tray) e inicia com o sistema.

---

## Roadmap (próximos saltos)
- **Cérebro forte (Claude):** integração de verdade pra análise inteligente — hoje é copiar-colar.
- **Olhos/ouvidos (Screenpipe):** OCR da tela + áudio pra ele *saber* o que você faz (precisa de macOS recente).
- **Fechar o loop:** transformar insight em ação (lembrar do que você disse que ia fazer).
- **Assinatura** dos instaladores (tirar o aviso do SmartScreen/Gatekeeper).

> Privacidade: tudo local (SQLite), só metadados — não grava a tela. A redação é
> melhor-esforço (rede, não muro): pode escapar algo sensível eventualmente.
