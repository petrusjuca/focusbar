# focusbar — Pain points de UX (auditoria)

Categorizado por severidade. ✅ = resolvido nesta leva · 🔜 = recomendado a seguir.

## 🔴 Crítico

### 1. Sobreposição de conceitos (a maior fonte de confusão)
Hoje existem **quatro** ideias parecidas competindo:
- **Foco** (`FocusBar`) — texto livre "no que você quer focar".
- **Task rule** (pílula `task: FINALIZAR WINDOWS / definir task` no card AGORA) — regra por palavra-chave que rotula apps. Obscura e raramente entendida.
- **Tarefas / to-do** (`TodoView`) — a lista de afazeres.
- **Modo Foco** (Pomodoro) — o bloco de tempo.

→ **Recomendação:** unificar em **um** fluxo: a *tarefa* da lista É o foco, e focar nela inicia o bloco. Esconder/fundir a "task rule" por keyword dentro de Categorias (poder de poucos, atrapalha a maioria). **Maior ganho de clareza do app.** 🔜

## 🟠 Alto

### 2. Mini pobre demais ✅
O mini só mostrava 1 direção + 1 ação. **Resolvido:** virou cockpit — parado mostra a to-do list (▶ foca, ○ conclui); rodando mostra o timer grande.

### 3. Timer não era global ✅
O cronômetro só existia dentro do card Pomodoro; trocou de aba, sumiu. **Resolvido:** faixa de timer no topo, visível em qualquer aba enquanto o bloco roda.

### 4. Densidade no topo da janela
Antes da primeira ação útil há: título grande + subtítulo + CTA + selfcheck + pausar + (banner) + card AGORA + abas. Muito scroll pra quem tem TDAH. 🔜 Compactar cabeçalho.

## 🟡 Médio

### 5. Dois botões "focar" com sentidos diferentes
`FocusBar`→"focar" só **salva o texto**; `FocusSessionCard`→"Focar 25min" **inicia o timer**. Mesma palavra, ações diferentes. 🔜 Unificar: focar = iniciar bloco.

### 6. "Pausar rastreamento" ambíguo
Pode ser lido como "pausar o timer". 🔜 Renomear pra "Pausar monitoramento".

## 🟢 Baixo

### 7. Pomodoro sem pausar/retomar
Só dá pra "parar" (zera). Sem pausa real do bloco. 🔜

### 8. Duração fixa (25/50)
Sem escolher um tempo custom. 🔜 Stepper de minutos.

---

**Resolvido nesta leva:** #2, #3 (+ botão do agente no topo, auto-diagnóstico SelfCheck).
**Próximo de maior impacto:** #1 (unificar tarefa = foco = bloco).
