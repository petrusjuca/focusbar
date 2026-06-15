# Assinatura local estável (acaba com o reset de permissões)

## O problema que isso resolve

Toda vez que o app era recompilado (assinatura "ad-hoc", diferente a cada build),
o macOS tratava o focusbar como um app **novo** e **apagava** as permissões de
Acessibilidade/Automação — obrigando a reconceder a cada build. Chato pra caramba.

## A solução

Assinamos o app com uma **identidade de code-signing local e fixa** (`focusbar-local`,
auto-assinada, guardada num cofre dedicado). Como a identidade **não muda** entre
builds, o macOS reconhece o app como o mesmo de sempre e **mantém as permissões**.

> Isso NÃO é a notarização da Apple (que tira o aviso "app não verificado" e custa
> US$99/ano — ver `ASSINATURA.md`). É só pra estabilidade local das permissões.

## Como buildar a partir de agora

```sh
./scripts/build-mac.sh
```

Esse script destrava o cofre, builda em release e **assina** com `focusbar-local`.
No fim ele imprime o comando pra instalar no /Applications.

Se você buildar com `npm run tauri build` direto (sem o script), o app sai **ad-hoc**
de novo e as permissões resetam — então **use o script** pra builds do dia a dia.

## O cofre (keychain)

- Arquivo: `~/Library/Keychains/focusbar-signing.keychain-db`
- Senha: `focusbar-local-signing`
- Identidade: `focusbar-local` (cert auto-assinado, válido por 10 anos)

Depois de **reiniciar o Mac**, o cofre fica travado — o `build-mac.sh` já o destrava
sozinho com a senha acima, então não precisa fazer nada manual.

## Recriar do zero (se o cofre for perdido)

```sh
OSSL=/opt/homebrew/opt/openssl@3/bin/openssl   # precisa do OpenSSL 3 (brew), não o LibreSSL do sistema
KCHAIN="$HOME/Library/Keychains/focusbar-signing.keychain-db"
KPASS="focusbar-local-signing"
mkdir -p /tmp/fbsign && cd /tmp/fbsign

"$OSSL" req -x509 -newkey rsa:2048 -keyout fb.key -out fb.crt -days 3650 -nodes \
  -subj "/CN=focusbar-local" \
  -addext "keyUsage=critical,digitalSignature" \
  -addext "extendedKeyUsage=critical,codeSigning" \
  -addext "basicConstraints=critical,CA:false"
# IMPORTANTE: -legacy (OpenSSL 3 senão o `security` do macOS não importa)
"$OSSL" pkcs12 -export -legacy -out fb.p12 -inkey fb.key -in fb.crt -passout pass:focusbar -name "focusbar-local"

security delete-keychain "$KCHAIN" 2>/dev/null || true
security create-keychain -p "$KPASS" "$KCHAIN"
security set-keychain-settings "$KCHAIN"
security unlock-keychain -p "$KPASS" "$KCHAIN"
security import fb.p12 -k "$KCHAIN" -P focusbar -T /usr/bin/codesign -A
security set-key-partition-list -S apple-tool:,apple:,codesign: -s -k "$KPASS" "$KCHAIN"
# adiciona o cofre à lista de busca (preservando os existentes)
security list-keychains -d user -s "$KCHAIN" $(security list-keychains -d user | sed -e 's/^[[:space:]]*"//' -e 's/"$//')
```

Depois de recriar, conceda as permissões uma vez — e elas ficam permanentes.
