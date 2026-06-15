# Assinatura & Notarização (tirar o aviso de "app não verificado")

Hoje os instaladores **não são assinados**. Por isso:

- **macOS (Gatekeeper):** ao abrir, aparece "focusbar não pode ser aberto porque
  o desenvolvedor não pode ser verificado". Contorno do usuário: clicar com o
  botão direito → **Abrir** (uma vez), ou Ajustes → Privacidade e Segurança →
  **Abrir mesmo assim**.
- **Windows (SmartScreen):** "O Windows protegeu seu PC". Contorno: **Mais
  informações → Executar assim mesmo**.

Funciona, mas passa insegurança. Para remover os avisos de vez, é preciso
assinar — o que exige contas/certificados **pagos** (não dá pra automatizar sem eles).

## macOS — Developer ID + notarização

Pré-requisitos (custo: **Apple Developer Program, US$99/ano**):

1. Certificado **Developer ID Application** (criado no portal da Apple, instalado no Keychain).
2. Uma **app-specific password** da sua conta Apple (ou chave da App Store Connect API).

No CI (`.github/workflows/release.yml`), passar estes secrets pro `tauri-action`:

```yaml
      - name: Build app + bundle installers
        uses: tauri-apps/tauri-action@v0
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
          APPLE_CERTIFICATE: ${{ secrets.APPLE_CERTIFICATE }}            # .p12 em base64
          APPLE_CERTIFICATE_PASSWORD: ${{ secrets.APPLE_CERTIFICATE_PASSWORD }}
          APPLE_SIGNING_IDENTITY: ${{ secrets.APPLE_SIGNING_IDENTITY }} # "Developer ID Application: Nome (TEAMID)"
          APPLE_ID: ${{ secrets.APPLE_ID }}
          APPLE_PASSWORD: ${{ secrets.APPLE_PASSWORD }}                 # app-specific password
          APPLE_TEAM_ID: ${{ secrets.APPLE_TEAM_ID }}
```

O `tauri-action` assina e **notariza** automaticamente quando essas variáveis existem.

## Windows — code signing

Pré-requisitos (custo: **certificado de code signing**, ~US$100–400/ano de uma CA;
certificados EV reduzem o atrito do SmartScreen mais rápido).

Configurar em `src-tauri/tauri.conf.json` → `bundle.windows.certificateThumbprint`
(ou via `tauri-action` com os secrets do certificado). Detalhes:
https://tauri.app/distribute/sign/windows/

## Enquanto não houver certificados

Os contornos no topo deste arquivo funcionam para uso pessoal/interno (você + o chefe).
A assinatura só vale a pena quando for distribuir pra mais gente.
