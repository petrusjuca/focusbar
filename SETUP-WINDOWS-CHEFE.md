# Prompt pra setar o focusbar no PC Windows (copiar e colar numa conversa nova do Claude)

---

Você é um assistente técnico configurando meu PC **Windows**. Faça tudo você mesmo pelo terminal (PowerShell), me explicando em **português simples** e me avisando só quando eu precisar clicar em algo. **Não sou técnico**, então capriche e tenha paciência.

**OBJETIVO:** instalar e deixar funcionando o **focusbar** — um app de produtividade local que rastreia onde meu tempo vai e tem um **assistente de IA que roda 100% na minha máquina** (nada vai pra internet). Ele usa o **Ollama** (IA local) por baixo.

Faça nesta ordem, **verificando cada passo antes de seguir**:

**1) Confirme o ambiente**
- Windows 10 ou 11, 64-bit.
- Confirme que o `winget` existe: `winget --version`. Se não existir, me ajude a instalar o "App Installer" pela Microsoft Store.

**2) Instale o Ollama (motor da IA local)**
- `winget install --id Ollama.Ollama -e --accept-source-agreements --accept-package-agreements`
- Inicie o Ollama (abre pelo menu Iniciar; ele fica rodando em segundo plano) e confirme que responde:
  `Invoke-RestMethod http://localhost:11434/api/version`

**3) Baixe o modelo de IA (~2GB, leve)**
- `ollama pull llama3.2:3b`
- Faça um teste rápido pedindo uma frase curta e confirme que ele respondeu.

**4) Baixe e instale o focusbar**
- Baixe o instalador (PowerShell):
  `Invoke-WebRequest -Uri "https://github.com/petrusjuca/focusbar/releases/download/v0.2.1/focusbar_0.2.1_x64-setup.exe" -OutFile "$env:USERPROFILE\Downloads\focusbar-setup.exe"`
- Rode o instalador (silencioso): `& "$env:USERPROFILE\Downloads\focusbar-setup.exe" /S`
- Se aparecer o aviso **"O Windows protegeu seu PC" (SmartScreen)**, me oriente a clicar em **Mais informações → Executar assim mesmo** (o app não tem certificado pago, mas é seguro).
- Abra o focusbar.

**5) Verifique que está tudo funcionando**
- O focusbar abriu e a aba **Hoje** mostra o app atual sendo rastreado.
- Na aba **Assistente**, ao clicar **"Gerar resumo do dia com IA"**, ele usa o Llama local (precisa do Ollama rodando — confirme que está no ar).
- Deixe pra **iniciar com o Windows**: o Ollama já faz isso; no focusbar, marque **"iniciar com o sistema"** no rodapé.

**6) Me explique em 3 linhas** como usar: as abas (Hoje / Semana / Assistente / Lembretes), que é **tudo local e privado** (só metadados — NÃO grava a tela), e que a IA roda na minha própria máquina.

Se algo falhar, **tente uma alternativa antes de me pedir ajuda** (ex.: se o `winget` falhar, baixe o Ollama em https://ollama.com/download/windows; se o download do focusbar falhar, me avise). Vai com calma comigo.
