# FLOWMODE — planejamento de funções (brainstorm Petrus, 04.07.2026)

> Capturado da conversa. É o alvo de UX/produto pós-v0.7.0 — complementa a
> FOCUSBAR-V2-SPEC (arquitetura) e o checklist Rev4. Anotações [claude] marcam
> o que já existe, o que conflita com decisão anterior e o que é novo.

## Ideias de produto

1. **Modo "Relógio Xadrez"** — dois timers iguais (ex.: 4h produtividade ×
   4h não-produtividade). O assistente decide a cada momento qual relógio está
   correndo (pela categoria/juízo ao vivo). Meta: bater as 4h de produção antes
   do relógio da procrastinação. [claude: novo; barato de construir — os
   estados por minuto e categorias já existem. Gamificação honesta.]
2. **Skins da mini-janela** (ovo, tomate…) — diversão periódica. [depois]
3. **Gamificação estilo Forest/RPG medieval** — floresta de pomodoros.
   [explicitamente "só depois, opcional"]

## [ANÁLISE DA SEMANA]
Comparação **semana civil (seg–dom) vs semana anterior** (decisão: possibilidade
2 — a comparação evolui ao longo da semana). Insights relevantes de mudança.
[claude: hoje existe "últimos 7 dias"; falta o corte civil + comparação + vira
aba própria.]

## [INICIA AQUI] Ligando o aplicativo
Ao iniciar o dia, pede a intenção/objetivos — pulável, mas é a ÚNICA coisa na
tela (se deixar pra depois, não faz). [claude: ritual da manhã existe como
card; a mudança é torná-lo a tela de entrada bloqueante-suave.]

## Camadas de IA (setup Petrus)
- ~~3 camadas~~ → **2 camadas**:
  - **Camada 1 — "operário local":** sintetiza/limpa dados. Preferência:
    Minimax M3 multimodal (OAuth por assinatura se der); alternativas DeepSeek
    v4 flash (sem visão → usaria OCR), Gemini flash.
  - **Camada 2 — raciocínio genial:** Claude Opus via MCP ("sai de graça").
- [claude: camada 2 está PRONTA (7 ferramentas MCP, incl. recap/standup/
  corrigir_categoria). Camada 1 é API de nuvem — conflita com "nada sai da
  máquina" (D2/filosofia) e adiciona custo/chave. Recomendação: plugável e
  DESLIGADA por padrão; medir primeiro até onde regras+correções+Claude chegam.]
- Nota do screenpipe: `allow-frames` é o que permite pipe acessar IMAGENS;
  sem isso só texto de OCR/acessibilidade. [claude: no focusbar o equivalente
  é a decisão D1 — screenshots com retenção curta, hoje NÃO salva por design.]

## TELA INICIAL
- Botão grande: **abrir o agente de foco** (janelinha, pomodoros imediatos).
- Botão pequeno: **pausar monitoramento** (timeline registra "pausado"). [existe]
- Índice "o que está sendo feito agora" (debug, sem IA). [existe: card AGORA]
- Navegação: [Hoje] [Análise da Semana] [Configurações] [Lembretes] [Dados].
  [claude: hoje é Hoje/Semana/Assistente/Lembretes/Dados — renomear
  Assistente→Configurações e Semana→Análise da Semana]

## AGENTE DE FOCO (janelinha)
- Nomear o pomodoro antes de começar OU dar play num já nomeado. [parcial]
- Durante: alterar/adicionar tempo em tempo real (estilo timer do Google);
  conta o tempo INTEGRAL real (terminou antes, tempo extra, overtime). [existe]
- Fim de tempo BEM visível/chamativo (não passar batido). [reforçar]
- Pausa: permite ajustar manualmente e salva a duração REAL da pausa. [parcial]
- Visual: círculo de progresso estilo timer do Google. [novo]

## [CONFIGURAÇÕES] (substitui "Assistente" — nome atual é misleading)
- Modos companheiro/foco/dia ruim com as diferenças EXPLÍCITAS na tela. [Rev4 #17]
- Toggles: OCR, IA local (se existir camada 1). [OCR existe]
- Temporário até 1.0: "analisar meu dia com Claude.ai". [existe]
- Config do MCP e como invocar. [existe]
- Só aqui: tema claro + iniciar com o sistema. [autostart existe; tema claro novo]

## [LEMBRETES]
- Lembretes entre sessões (levantar, água…). [existe]
- No alerta: "chega por hoje" (silencia hoje) e "desativar". [novo]
- "Não posso ainda" → o aviso FICA na tela junto da janela de foco até você
  excluir manualmente (aí você lembra de fazer). [novo — melhor que adiar]

## [HOJE]
- **Modo Foco:** tempos pré-configurados + adicionar tempo rápido + campo
  GRANDE de tempo livre (o atual é pequeno demais; parse estilo Google já
  existe). [parcial]
- **REMOVER: "Intenção de hoje + um passo pra chegar lá"** — inútil no formato
  atual (a intenção continua existindo no ritual de início; o que sai é a
  cascata de passos dali). [Petrus: "INÚTIL DO SISTEMA ATUAL"]
- **Tarefas (modo foco):** tarefa com tempo custom próprio (tarefas de 10min ×
  30min); iniciar pomodoro da lista ou da janelinha; opcional: já indicar
  quantos pomodoros a tarefa deve levar (e adicionar extras depois). [Rev4 #5]
- **Pomodoros concluídos:** mesmo nome → numeração automática 1/2/3 e
  agrupamento como "grande tarefa"; cada linha mostra tempo planejado × real ×
  pausa que seguiu. [novo]
- **Dedicado hoje:** barra por projeto (verde) + pausa (amarelo/laranja) +
  total do dia. Depois gráficos (barra/pizza/ambos). [parcial]
- **Gráfico por categoria:**
  - Meta: "Outro" < 5%. Acima disso o sistema AVISA e pede pra revisar a
    categoria Outro — a correção vira memória (aprendizado). [novo, importante]
  - "% tempo pausado" pode entrar/sair do gráfico (horas ativas × horas totais).
  - Mostrar também tempo TOTAL com o focusbar aberto (6h? 12h?). [novo]
- **Linha do tempo:** foco/pausado/ausente/sem dados + recategorizar manual.
  Agrupar sessões vizinhas iguais (senão recategorizar "Planejar notas" 30×).
  [claude: agrupamento resolvido na Fase B (merge do derivador)]
- Botão [histórico] no rodapé.

## [HISTÓRICO]
Minuto a minuto, durações e acessos aba a aba. Substitui "Sessões recentes".
Serve pra debugging. [claude: a tabela crua já existe (tab_events + sessões);
falta a tela]

## [DADOS]
Estatísticas cruas de TODO dado numérico do sistema — debugging e métricas;
o sistema compara entre dias e sugere insights. [existe parcialmente]

---

### Leitura do claude (04.07) — ordem de ataque sugerida
1. Reorganização de telas (renames + REMOVER cascata + campo de tempo grande +
   fim-de-timer chamativo) — barato, alinha o app ao mapa mental do doc.
2. Alerta "Outro > 5%" + revisão guiada (alimenta a memória de categorias — é
   o que faz o recap do Claude ficar bom sozinho).
3. Análise da Semana (civil vs anterior) como aba + ferramenta MCP.
4. Tarefas com tempo próprio + numeração/agrupamento de pomodoros (Rev4 #5/#10).
5. Lembretes: "chega por hoje" + aviso que fica na tela.
6. Relógio Xadrez (usa tudo acima; feature-assinatura do FLOWMODE).
7. D1 (screenshots retenção curta, opt-in) — pré-requisito de "ler imagens".
8. Skins/gamificação — último, como o próprio doc diz.
