#!/usr/bin/env bash
# Build do focusbar para macOS, ASSINADO com a identidade local estável
# (focusbar-local). Assim as permissões do macOS (Acessibilidade/Automação)
# NÃO resetam a cada rebuild — você concede uma vez e pronto.
#
# Pré-requisito (já configurado uma vez): o cofre de assinatura
# ~/Library/Keychains/focusbar-signing.keychain-db com a identidade focusbar-local.
# Se algum dia precisar recriar, ver docs/ASSINATURA-LOCAL.md.
set -euo pipefail

KCHAIN="$HOME/Library/Keychains/focusbar-signing.keychain-db"
KPASS="focusbar-local-signing"
IDENTITY="focusbar-local"
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
APP="$ROOT/src-tauri/target/release/bundle/macos/focusbar.app"

echo "→ destravando o cofre de assinatura"
security unlock-keychain -p "$KPASS" "$KCHAIN"

echo "→ buildando (release)"
cd "$ROOT"
npm run tauri build -- --bundles app

echo "→ assinando com a identidade estável ($IDENTITY)"
codesign --force --deep --sign "$IDENTITY" --keychain "$KCHAIN" "$APP"
codesign --verify --verbose "$APP"

echo ""
echo "✓ pronto e assinado: $APP"
echo "  instalar:  pkill -x focusbar; rm -rf /Applications/focusbar.app && cp -R \"$APP\" /Applications/ && open -a focusbar"
