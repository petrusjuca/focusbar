# focusbar — o que mudou

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
