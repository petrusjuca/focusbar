# focusbar → Assistente Pessoal (TDAH) sobre Screenpipe

> Blueprint de filosofia e arquitetura. **Decidir bem antes de programar.**
> Status: design. Nada aqui é código ainda.

## TL;DR — decisões já tomadas

1. **Não reconstruir o gravador.** O Screenpipe roda **por baixo** como motor de
   captura (tela + OCR + áudio/Whisper + redação). A gente constrói **só a camada de sentido**.
2. **Integração sem fork:** consumir o Screenpipe pela **API local + MCP** (`screenpipe-mcp`),
   não modificando o repo dele. Código que mantemos = pequeno e nosso.
3. **Reaproveitar o focusbar:** a casca (Tauri + React), dashboard, coach, lembretes
   e categorias viram a UI deste assistente.
4. **Dois produtos, um código-base de UI:**
   - *focusbar leve* (metadados, sem gravar tela) → uso compartilhado / chefe.
   - *Assistente pessoal* (captura total + IA nuvem) → uso do Petrus (TDAH).
5. **IA na nuvem é OK** (quer o assistente mais inteligente possível), **mas com porteiro local**
   filtrando o que sai.
6. **Proatividade é um MODO** que troca no momento, não um ajuste fixo.
7. **A IA não define tarefas** — no máximo sugere. Ela observa a *execução*.

---

## 1. A virada conceitual

Isto **não é vigilância de si mesmo** (painel que te julga → veneno pra TDAH, aciona
disforia sensível à rejeição e morre em 2 semanas).

Isto é uma **função executiva externa**. Memória de trabalho, senso de tempo e
iniciação de tarefa são as 3 coisas que o TDAH sabota — e as 3 que essa arquitetura
terceiriza. O software é a parte do teu cérebro que **não fica online de forma confiável**.
Todo o resto deriva disso.

---

## 2. Os 5 princípios

1. **Captura sem fricção, lembrança com curadoria.** Você nunca alimenta o sistema na mão.
   Mas 99% do capturado é ruído — a inteligência está no que ele **traz à tona**, não no que grava.
   *Captura é commodity; curadoria é o produto.*
2. **"Fora da vista, fora da mente" — invertido.** Ele re-materializa intenções que evaporaram
   ("ah, eu ia fazer aquilo" das 10h que sumiu às 10h03).
3. **Não-punitivo por padrão, sempre.** Nunca "você perdeu 2h no YouTube". Sempre
   "notei que você travou depois do almoço — retomar de onde parou ou começar leve?".
   Tom é **arquitetura**, não enfeite — define se você ainda usa em 3 meses.
4. **Esquecer é feature.** O bruto de hoje é detalhado; o de uma semana vira episódio resumido;
   o de um mês vira só padrão. Você quase nunca quer o pixel de terça — quer "terça você travou em X".
5. **À prova de TDAH (meta-princípio).** Se manter a ferramenta exigir função executiva, ela morre.
   Zero manutenção, auto-organizável, tolerante a buracos. Sumiu 3 dias? Ela continua de onde dá,
   **sem tela de culpa**.

---

## 3. Arquitetura em camadas

```
┌─────────────────────────────────────────────────────────────┐
│  SCREENPIPE (motor de captura — roda de fundo)              │
│  tela (eventos) · OCR/árvore de acessibilidade · áudio       │
│  (Whisper) · redação na origem · SQLite local + busca FTS    │
└───────────────┬─────────────────────────────────────────────┘
                │  API local + MCP (screenpipe-mcp)
                ▼
┌─────────────────────────────────────────────────────────────┐
│  PORTEIRO LOCAL (modelo pequeno na máquina)                 │
│  zonas de exclusão · redação extra · decide o que pode subir │
└───────────────┬─────────────────────────────────────────────┘
                ▼
┌─────────────────────────────────────────────────────────────┐
│  MEMÓRIA (4 camadas destiladas)                             │
│  bruto → episódios → semântico/loops → autoconhecimento      │
└───────────────┬─────────────────────────────────────────────┘
                ▼
┌─────────────────────────────────────────────────────────────┐
│  ASSISTENTE (IA forte na nuvem — só toca no destilado/limpo) │
│  ritual diário · loops abertos · sugestões · resumos         │
└───────────────┬─────────────────────────────────────────────┘
                ▼
┌─────────────────────────────────────────────────────────────┐
│  UI (focusbar — Tauri + React)                              │
│  dashboard · modos · debrief · chat com o assistente         │
└─────────────────────────────────────────────────────────────┘
```

Regra de ouro: **a IA forte nunca toca no dado cru.** Só recebe o que o porteiro já
limpou e o que a destilação já resumiu. Isso resolve **custo** e **privacidade** ao mesmo tempo.

---

## 4. As 4 camadas de memória

| Camada | O que é | Quem produz | Cadência |
|---|---|---|---|
| **Bruto** | Eventos com timestamp, texto de tela, transcrições | Screenpipe | contínuo |
| **Episódios** | Blocos com sentido: "9h10–10h45 codando auth, 3 interrupções" | modelo **local** | a cada ~10–15min |
| **Semântico / loops** | Compromissos, intenções, coisas começadas e largadas | local + nuvem | ao fechar episódio + no debrief |
| **Autoconhecimento** | Ritmos de energia, onde você trava, quando o hiperfoco pega | nuvem (longo prazo) | semanal |

A maior parte do **valor pro TDAH** está na camada de **episódios** (transforma borrão
de tempo em narrativa recuperável) e na **semântica/loops** (o detector de laços abertos).

---

## 5. Porteiro de privacidade (inegociável)

Defesa em camadas, do mais forte pro mais fraco:

1. **Zonas de exclusão dura** — apps/domínios que **nunca** são nem lidos. Padrão inicial:
   gerenciador de senhas, telas de banco, portais de saúde, apps de mensagem íntima.
   *Nem entra no banco.* Configurar isso é a **primeira** coisa.
2. **Redação na entrada** — senha, token, 2FA, cartão, CPF: apagados **antes de armazenar**.
3. **Porteiro decide o que sobe** — regra-mãe: **na dúvida, fica local.** O default é privado;
   você *opta* por mandar algo específico pra nuvem.
4. **Efemeridade** — o que for sensível e escapar é descartado rápido. Meta não é perfeição
   (um código vai piscar na tela uma hora), é vazamento **raro e de vida curta**.

> Honestidade: nada disso é 100%. O OCR vai pegar algo que não devia uma hora. Por isso
> camada 4 existe — o risco não se acumula.

---

## 6. Detector de loops abertos (a joia da coroa)

Provavelmente o **subsistema mais importante** do software inteiro. Extrai e devolve
"coisas que você disse que ia fazer e o TDAH apagou".

- **Fontes:** o que você falou numa call/áudio (Whisper), num chat, num e-mail;
  o que você começou e largou; projetos recorrentes; pessoas esperando retorno.
- **Como extrai:** a camada semântica roda LLM sobre os episódios procurando
  compromissos ("vou mandar X", "preciso responder Y", "depois eu faço Z").
- **Como devolve (pró-TDAH):** por **reconhecimento, não memória**. Ele mostra
  "acho que ficou pendente: A, B, C — confere?" e você corrige com um toque.
  Reconhecer é muito mais fácil que lembrar do zero.
- **Estados:** aberto · adiado (snooze) · feito · descartado. Nada cobra; tudo é sugestão.

---

## 7. O ciclo diário (o coração do "me ajudar a melhorar")

Dois momentos curtos, **opt-in**, à prova de abandono:

**Manhã — declarar intenção (≤ 1 min):**
- Ele mostra os loops abertos + o que ficou de ontem.
- Você escolhe 1–3 intenções do dia (ou só confirma as sugeridas). **Um toque.**
- Não vira lista de 200 itens — só o "norte" de hoje.

**Fim de tarde — debrief (2–3 min):**
- Cruza **intenção × execução × tempo/energia**: o que saiu, o que não, e o custo real.
- Tom gentil e **grosso, não minucioso**: aponta o óbvio caro ("3h numa tarefa de 30min
  porque ficou alternando com 5 coisas"), não caça 4 minutos perdidos.
- Termina com **1 melhoria pequena** pra amanhã, não um relatório.

Regra anti-abandono: o debrief **sempre roda** (independe de modo). Se você sumir,
ele resume o período e segue — sem culpa.

---

## 8. Proatividade como MODO (não ajuste fixo)

O TDAH oscila entre "me ajuda" e "me deixa em paz" no mesmo dia. Três modos:

- **Foco / Não perturbe** — captura continua, saída proativa vai a **zero**. O que ele
  notaria fica numa fila silenciosa pra depois.
- **Companheiro** — traz loops à tona, devolve contexto após interrupção, sugere próximo passo.
- **Dia ruim / baixa energia** — extra-gentil: faz mais do trabalho pesado, sugere só o
  **menor próximo passo possível**, segura qualquer cobrança.

Críticos:
- **Troca de modo com fricção quase zero** (um atalho/toque). Se der trabalho, você esquece.
- **O sistema detecta e oferece** o modo ("parece foco profundo — fico quieto?").
- **Modo Foco decai com segurança** — expira sozinho ou o debrief sempre roda; senão o
  "não perturbe" engole o dia inteiro.
- **Decisão fechada:** no modo Foco, **só alertas estritamente físicos furam** (ex.: 3h sem
  pausa/água). O resto fica mudo. (É quando o hiperfoco mais esquece o corpo.)

---

## 9. Descanso como cidadão de primeira classe

Não é "lembrete de pausa" genérico — é **contrapeso ao hiperfoco**. O padrão TDAH não é
preguiça, é **travar ligado e esquecer o corpo até desabar**. O sistema otimiza pra
**funcionamento sustentável**, não output bruto. Ele tem permissão de te tirar do hiperfoco
**mesmo quando está rendendo**, porque sabe (autoconhecimento) o que vem depois do colapso.

---

## 10. Stack de IA (local pequeno + nuvem forte)

- **Modelo local pequeno** (ex.: via Ollama, 3–8B): porteiro + destilação bruto→episódio.
  Roda o tempo todo, de graça, privado. Nunca manda nada pra fora.
- **Modelo forte na nuvem** (Claude etc.): o assistente que raciocina — ritual diário,
  loops, resumos, sugestões. Só recebe o **destilado e limpo**.
- **Por que isso controla custo:** mandar captura 24/7 crua pro modelo forte seria caro e lento.
  A destilação local reduz 99% do volume antes de chegar na nuvem.

---

## 11. Política de esquecimento (aging)

- **Hoje:** bruto detalhado.
- **~1 semana:** vira episódios resumidos (bruto descartável).
- **~1 mês:** vira só padrões/autoconhecimento.

Benefício duplo: **privacidade** (risco não acumula) + **peso mental** (a memória envelhece
como a humana — você quer o significado, não o pixel).

---

## 12. O que reaproveitamos do focusbar

- **Casca Tauri + React** (UI, tray, background, autostart).
- **Dashboard / timeline / categorias / coach / lembretes** → viram a camada de apresentação.
- **Detecção de foco/idle** → ainda útil como sinal barato (sem depender só do Screenpipe).

---

## 13. Roadmap de implementação (quando a gente for codar)

1. **Subir o Screenpipe** local e validar a API/MCP (o que dá pra consultar, formato).
2. **Porteiro v1** — zonas de exclusão + redação. *Privacidade antes da captura total.*
3. **Camada de episódios** — destilação local bruto→episódio, gravada no nosso banco.
4. **Detector de loops abertos v1** — extrair compromissos dos episódios, UI de "confere?".
5. **Ciclo diário** — manhã + debrief na UI do focusbar.
6. **Modos de proatividade** + alertas de corpo.
7. **Assistente nuvem** — chat que lê as camadas destiladas (não o bruto).
8. **Autoconhecimento** + política de esquecimento.

---

## 14. Decisões ainda em aberto

- Quais apps/domínios entram na **zona de exclusão dura** inicial (lista tua).
- Quem é o **modelo local** do porteiro (Ollama qual modelo) e qual a **nuvem** (Claude API?).
- Horário do **ritual diário** (manhã/fim de tarde) e gatilhos de auto-detecção de modo.
- Onde mora o **banco da camada de sentido**: dentro do SQLite do Screenpipe ou um nosso ao lado.
