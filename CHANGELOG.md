# focusbar — o que mudou

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
