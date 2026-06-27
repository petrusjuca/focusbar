# MCP local — Claude lê seus dados (sem API paga)

O focusbar inclui um **servidor MCP local** (binário `mcp`, empacotado dentro do
`focusbar.app`). Ele lê o mesmo SQLite que o app grava e responde, em linguagem
natural, sobre o seu foco. Você aponta o **seu** Claude (Desktop ou Code) pra
ele — o app **não gasta API**, nada sai da máquina (o servidor abre o banco em
modo somente-leitura).

## Onde fica

- macOS: `/Applications/focusbar.app/Contents/MacOS/mcp`
- O banco lido: `~/Library/Application Support/com.petrusjuca.focusbar/focusbar.db`
  (dá pra sobrescrever com a env `FOCUSBAR_DB`).

A tela **Agente → "CLAUDE LÊ SEUS DADOS (MCP)"** mostra o caminho exato e os
comandos prontos pra copiar.

## Configurar

**Claude Code (terminal):**

```
claude mcp add focusbar -- "/Applications/focusbar.app/Contents/MacOS/mcp"
```

**Claude Desktop** (`claude_desktop_config.json`):

```json
{
  "mcpServers": {
    "focusbar": { "command": "/Applications/focusbar.app/Contents/MacOS/mcp" }
  }
}
```

Reinicie o Claude Desktop depois de editar o JSON.

## Ferramentas expostas (todas somente-leitura)

| Ferramenta | O que entrega |
|---|---|
| `resumo_do_dia` | foco total, top apps e quebra por categoria (padrão: hoje) |
| `blocos_do_dia` | lista cronológica dos blocos: horário, app, atividade, categoria, duração |
| `pomodoros_do_dia` | quantos pomodoros, tempo focado e quantos concluídos |
| `resumo_da_semana` | foco por dia nos últimos 7 dias |

Todas aceitam o argumento opcional `dia` no formato `AAAA-MM-DD`.

## Protocolo

JSON-RPC 2.0 sobre stdio, uma mensagem por linha. Implementa `initialize`,
`tools/list`, `tools/call` e `ping`. Sem dependências de rede.
