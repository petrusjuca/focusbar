# focusbar → Assistente Pessoal (TDAH) sobre Screenpipe — v2

> **Focusbar transforma seu computador em um assistente de IA que sabe tudo que
> você fez e pode agir em cima disso.** Usa o Screenpipe open-source como base.
> É bom de um jeito que TE ajuda e dá vontade de usar — e pode ser customizado
> pra usos específicos seus. Algo que nenhum outro app te entrega.

> Blueprint de filosofia e arquitetura. **Decidir bem antes de programar.** Sem código ainda.

## Filosofia (a alma do produto)

- *"Aquilo que você lembra, você pode agir em cima."*
- *"Você não pode agir em cima de achismo, apenas de dados."*
- *"Quanto mais informação você tiver, mais você pode melhorar algo."*
- *"O objetivo não é nos levar ao limite da tortura, e sim fazer mais e melhor em
  menos tempo — pra sobrar tempo pra outras coisas também."*

---

## 0. TL;DR — decisões tomadas

1. **Não reconstruir o gravador.** Screenpipe roda **por baixo** (tela + OCR + Whisper + redação).
   Integração **sem fork**, via **API local + MCP**.
2. **Reaproveitar o focusbar** (Tauri + React, dashboard, coach, lembretes) como UI.
3. **IA em dois níveis:**
   - **Local pequeno** (`Llama 3.2 3B`) — filtra/limpa o bruto, destila episódios. Nunca sai da máquina.
   - **SOTA via custo fixo/grátis** — **Claude (Max) por MCP** + opcional **Gemini grátis**.
     **Sem API medida** (custo imprevisível). Só recebe o **destilado e limpo**.
4. **Armazenamento local.** Bruto no Screenpipe; camada de sentido em SQLite **nosso, ao lado**.
5. **Proatividade é MODO** (Foco / Companheiro / Dia ruim), troca no momento.
6. **A IA não define tarefas** — observa execução, no máximo sugere.
7. **Orçamento de tempo: ≤ 10 min/dia** de interação, num período só (pedido do chefe).

---

## 1. A virada conceitual

Isto **não é vigilância de si mesmo** (painel que te julga → veneno pra TDAH, morre em 2 semanas).
É uma **função executiva externa**: memória de trabalho, senso de tempo e iniciação de tarefa
— as 3 coisas que o TDAH sabota e que essa arquitetura terceiriza. O software é a parte do teu
cérebro que **não fica online de forma confiável**.

---

## 2. Os 5 princípios

1. **Captura sem fricção, lembrança com curadoria.** Você nunca alimenta na mão. 99% é ruído —
   a inteligência está no que ele **traz à tona**. *Captura é commodity; curadoria é o produto.*
2. **"Fora da vista, fora da mente" — invertido.** Re-materializa intenções que evaporaram.
3. **Não-punitivo por padrão.** Nunca "você perdeu 2h no YouTube". Sempre "travou depois do
   almoço — retomar ou começar leve?". Tom é arquitetura.
4. **Esquecer é feature.** Bruto de hoje detalhado; semana vira episódio; mês vira padrão.
5. **À prova de TDAH (meta-princípio).** Se manter a ferramenta exigir função executiva, ela morre.
   Zero manutenção, auto-organizável, tolerante a buracos. Sumiu 3 dias? Continua, **sem culpa**.
   Você **não** liga/desliga nem nomeia tarefa — ele faz isso.

---

## 3. Arquitetura

```
┌──────────────────────────────────────────────────────────────┐
│  SCREENPIPE (motor de captura — de fundo)                    │
│  tela · OCR · áudio/Whisper · redação na origem · SQLite/FTS  │
└───────────────┬──────────────────────────────────────────────┘
                │  API local + MCP
                ▼
┌──────────────────────────────────────────────────────────────┐
│  FILTRO LOCAL — Llama 3.2 3B (na máquina)                    │
│  tira senha/2FA/cartão/CPF · zonas de exclusão · destila      │
│  → na dúvida, FICA LOCAL                                       │
└───────────────┬──────────────────────────────────────────────┘
                ▼
┌──────────────────────────────────────────────────────────────┐
│  MEMÓRIA — 4 camadas (SQLite nosso, ao lado)                 │
│  bruto → episódios → loops abertos → autoconhecimento         │
└───────────────┬──────────────────────────────────────────────┘
                ▼ (só o destilado e limpo)
┌──────────────────────────────────────────────────────────────┐
│  CÉREBRO SOTA — custo fixo/grátis                            │
│  Claude (Max) via MCP · Gemini grátis (auto)                  │
│  análise do dia · insights elaborados · debrief               │
└───────────────┬──────────────────────────────────────────────┘
                ▼
┌──────────────────────────────────────────────────────────────┐
│  UI (focusbar — Tauri + React)                              │
│  dashboard · captura rápida · modos · debrief · chat          │
└──────────────────────────────────────────────────────────────┘
```

**Regra de ouro:** o cérebro SOTA **nunca toca no bruto** — só no que o filtro local limpou e a
destilação resumiu. Isso resolve **privacidade** (raw nunca sai) e **custo** (volume mínimo) de uma vez.

---

## 4. As 4 camadas de memória

| Camada | O que é | Quem produz |
|---|---|---|
| **Bruto** | Eventos, texto de tela, transcrições | Screenpipe |
| **Episódios** | "9h10–10h45 codando auth, 3 interrupções" | Llama 3.2 3B (local) |
| **Loops abertos** | Compromissos, intenções, coisas largadas | local + SOTA |
| **Autoconhecimento** | Ritmos de energia, onde trava, padrão pós-almoço | SOTA (longo prazo) |

O maior valor pro TDAH está em **episódios** (borrão → narrativa) e **loops abertos**.

---

## 5. Porteiro de privacidade (inegociável)

1. **Zonas de exclusão dura** — apps/sites **nunca** lidos (gerenciador de senha, banco, saúde).
   *Nem entram no banco.* Primeira coisa a configurar.
2. **Redação na entrada** — senha, token, 2FA, cartão, CPF: apagados antes de armazenar.
3. **Filtro decide o que sobe** — regra-mãe: **na dúvida, fica local.** Só o destilado/limpo vai
   pro cérebro SOTA.
4. **Efemeridade** — sensível que escapar é descartado rápido. Meta: vazamento raro e de vida curta.

---

## 6. Detector de loops abertos + intenções com prazo (a joia)

Subsistema mais importante. "Nunca mais esquecer algo — nem anotação sem ação."

- **Fontes:** o que você falou numa call/áudio (Whisper), num chat, num e-mail; o que começou
  e largou; projetos recorrentes; pessoas esperando retorno.
- **Intenção com prazo:** "quero terminar isso até 10h30" — você anota (ou o Whisper pega).
  Se não terminar, ele lembra **na hora (10h30)** ou guarda pro **debrief de fim de dia**.
- **Como devolve (pró-TDAH):** por **reconhecimento, não memória** — "ficou pendente A, B, C —
  confere?". Reconhecer é mais fácil que lembrar.
- **Estados:** aberto · adiado · feito · descartado. Nada cobra; tudo é sugestão.

---

## 7. Captura rápida (quick-capture)

Quer anotar algo? Joga no app **sem formatar nada**. Ele **digere e arquiva certo**:
- É uma atividade? → vira **tarefa** (com prazo, se houver).
- É uma ideia/nota? → vai pro lugar certo, ligada ao contexto do momento.
- Pode ser por texto ou voz (Whisper já está capturando).

Zero fricção: o trabalho de organizar é da IA, não seu.

---

## 8. O ciclo diário (≤ 10 min/dia, num período só)

Restrição dura (pedido do chefe): a interação ativa **não passa de ~10 min/dia**, concentrada.

**Manhã — intenção (≤ 1 min):** mostra loops abertos + o de ontem; você escolhe 1–3 nortes
do dia (ou confirma os sugeridos). Um toque.

**Fim de tarde — debrief (≤ 8 min):** cruza **intenção × execução × tempo/energia**. Tom gentil,
**grosso, não minucioso** (aponta o óbvio caro, não caça 4 min perdidos). Termina com **1 melhoria**
pra amanhã. O debrief **sempre roda** (independe de modo) — anti-abandono.

---

## 9. Modos de proatividade + política de notificação

**Modos** (troca com fricção quase zero; o sistema detecta e oferece):
- **Foco / Não perturbe** — saída proativa **zero**. Bom pro hiperfoco.
- **Companheiro** (padrão) — entrega o que for necessário: loops, contexto, próximo passo.
- **Dia ruim / baixa energia** — extra-gentil: faz o trabalho pesado, sugere o **menor próximo
  passo**, segura cobrança.

**Notificações — silencioso e suave por padrão:**
- Ele "fala" só em 3 casos: (1) quando perguntado, (2) momentos agendados (manhã/debrief),
  (3) interrupções necessárias (levantar, beber água).
- **Som leve só nesses importantes** (pra não passar batido). **Notificações menores: sem som.**
- No modo Foco, **só o estritamente físico fura** (ex.: 3h sem pausa). O resto fica mudo.
- **Modo Foco decai com segurança** (expira ou o debrief sempre roda) — não engole o dia.

---

## 10. Descanso como cidadão de primeira classe

Contrapeso ao hiperfoco. O padrão TDAH não é preguiça — é travar ligado e esquecer o corpo até
desabar. O sistema otimiza **funcionamento sustentável**, não output bruto: tem permissão de te
tirar do hiperfoco **mesmo quando está rendendo**, porque sabe o que vem depois do colapso.

---

## 11. Stack de IA — custo fixo/grátis (M1 8GB)

**Hardware real:** Apple M1, **8GB RAM**. Screenpipe (0,5–3GB) + modelo + macOS não cabem juntos
folgados → **nada roda 24/7 ao mesmo tempo**.

- **Filtro local — `Llama 3.2 3B`** (Ollama, ~2GB): limpa o bruto + destila episódios.
  **Carregado sob demanda** (a cada ~15min e nos rituais), descarregado depois. Nunca sai da máquina.
- **Cérebro SOTA — custo fixo/grátis (a parte que sai, já limpa):**
  - **Claude (Max) via MCP** — o Screenpipe expõe MCP; o Claude lê o destilado sob a tua
    assinatura (custo fixo, não API medida). Melhor qualidade. Pra conversar com teu dia + debrief.
  - **Gemini grátis (AI Studio)** — chave gratuita com limites; boa pra passos 100% automáticos.
  - ❌ **Sem API por token** (Anthropic/OpenAI medida) — é o custo imprevisível que você quer evitar.
- **Regras antes de modelo:** categorias, foco, idle, alertas (já prontos no focusbar) — custo zero.

> Upgrade fácil: mais RAM → Llama maior local; ou trocar o canal SOTA — **sem mexer no resto**.

---

## 12. Política de esquecimento (aging)

Hoje: bruto detalhado. ~1 semana: episódios resumidos (bruto descartável). ~1 mês: só padrões.
Benefício duplo: **privacidade** (risco não acumula) + **peso mental** (memória envelhece como a humana).

---

## 13. Inteligência que evolui

Os agentes melhoram com o tempo. Conhecendo teus hábitos, ele te avisa quando algo está **errado**
ou quando vê **padrões negativos**: o que te faz **dormir mal**, o que está **constantemente errado**,
o "após o almoço seu ritmo desanda". Isso é a camada de autoconhecimento virando ação.

---

## 14. Presença na tela (depois)

Ideia futura: ele **"estar presente o dia todo"** num cantinho da tela, pra você nunca esquecer
dele — sem gastar espaço demais. Funciona melhor com **2+ monitores**, então fica pra uma fase posterior.

---

## 15. O que reaproveitamos do focusbar

Casca Tauri + React (UI, tray, background, autostart) · dashboard/timeline/categorias/coach/lembretes
→ camada de apresentação · detecção de foco/idle → sinal barato sem depender só do Screenpipe.

---

## 16. Roadmap de implementação (quando codar)

1. Subir Screenpipe local + validar API/MCP.
2. **Porteiro v1** (zonas de exclusão + redação). *Privacidade antes da captura total.*
3. **Captura rápida** (inbox → IA digere em tarefa/nota).
4. **Episódios** (destilação local) no nosso SQLite.
5. **Loops abertos + intenção com prazo** + UI "confere?".
6. **Ciclo diário** (manhã + debrief, ≤10min).
7. **Modos + política de som** das notificações.
8. **Cérebro SOTA via MCP/Max** (chat + insights sobre o destilado).
9. **Autoconhecimento + esquecimento** (aging).

---

## 17. Decisões

**Fechadas:**
- IA dois níveis: **Llama 3.2 3B local** + **Claude(Max) via MCP / Gemini grátis**. Sem API medida.
- Banco da camada de sentido: **SQLite nosso, ao lado**.
- Ritual: manhã + debrief, **≤ 10 min/dia**.
- Captura Screenpipe em **perfil leve** primeiro, medir antes de subir.

**Ainda só você responde (input pessoal):**
- **Zona de exclusão dura:** qual **gerenciador de senha** e qual **banco** você usa, + apps/sites
  que nunca podem ser lidos.
