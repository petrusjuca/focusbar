# focusbar — extensão de abas

Reporta a aba ativa ao focusbar rodando **nesta** máquina (`127.0.0.1:7690`).
Nada sai do computador. É o que faz o relatório mostrar **os sites de verdade**
(WhatsApp, YouTube, Miro…) em vez de "4h51 no Opera GX" — em qualquer
navegador Chromium, em qualquer sistema.

## Instalar (Opera GX / Opera)

1. Abra `opera://extensions`
2. Ligue o **Modo de desenvolvedor** (canto superior direito)
3. Clique **Carregar sem compactação** e escolha esta pasta (`extension/`)
4. Pronto — não tem passo 4. Confira em `http://127.0.0.1:7690/api/health`
   (com o focusbar aberto): `ext_last_event_ts` > 0 = conectada.

## Instalar (Chrome / Edge / Brave / Vivaldi)

Mesma coisa, em `chrome://extensions` (ou `edge://extensions`) → Modo de
desenvolvedor → **Carregar sem compactação** → esta pasta.

## O que ela manda (e o que NÃO manda)

- Manda: ação (`activated`/`updated`/`removed`), navegador, id da aba,
  **URL sem query/fragment** (tokens e afins nunca saem) e título da aba.
- Destino: só `http://127.0.0.1:7690` (loopback). Sem focusbar aberto, o
  `fetch` falha em silêncio e ninguém guarda nada.
- Permissões: só `tabs` (pra saber qual aba está ativa). Não lê o conteúdo
  das páginas.
