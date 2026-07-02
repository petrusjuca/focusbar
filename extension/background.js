// focusbar — extensão de abas (Fase A).
//
// Reporta à API local do focusbar (127.0.0.1:7690) qual aba está ativa, quando
// ela muda e quando fecha. É o que dá URL CERTA onde o app sozinho não alcança
// (Opera GX, Windows). Tudo fica na máquina: o único destino é loopback.

const ENDPOINT = "http://127.0.0.1:7690/api/tab-event";

// Qual navegador somos? A API local usa isso pra só aplicar a aba quando o
// app em foco é ESTE navegador (dois browsers abertos não se contaminam).
function browserName() {
  const brands = navigator.userAgentData?.brands?.map((b) => b.brand) ?? [];
  for (const b of brands) {
    const n = b.toLowerCase();
    if (n.includes("opera gx")) return "opera gx";
    if (n.includes("opera")) return "opera";
    if (n.includes("edge")) return "edge";
    if (n.includes("brave")) return "brave";
    if (n.includes("vivaldi")) return "vivaldi";
  }
  const ua = navigator.userAgent;
  if (ua.includes("OPR/")) return ua.includes("GX") ? "opera gx" : "opera";
  if (ua.includes("Edg/")) return "edge";
  if (brands.some((b) => b.toLowerCase().includes("google chrome"))) return "chrome";
  if (ua.includes("Chrome/")) return "chrome";
  return "chromium";
}
const BROWSER = browserName();

// Só URL http(s), e já SEM query/fragment (tokens, ?email=, session id — não
// saem nem pro loopback). Páginas internas (opera://, chrome://) viram "".
function cleanUrl(raw) {
  try {
    const u = new URL(raw ?? "");
    if (u.protocol !== "http:" && u.protocol !== "https:") return "";
    return (u.origin + u.pathname).replace(/\/$/, "");
  } catch {
    return "";
  }
}

// Fire-and-forget: se o focusbar não estiver aberto, silêncio (sem retry — o
// próximo evento de aba tenta de novo sozinho).
function send(action, tab) {
  fetch(ENDPOINT, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({
      action,
      browser: BROWSER,
      tab_id: String(tab?.id ?? ""),
      url: cleanUrl(tab?.url),
      title: tab?.title ?? "",
    }),
  }).catch(() => {});
}

// Trocou de aba.
chrome.tabs.onActivated.addListener(({ tabId }) => {
  chrome.tabs.get(tabId).then((tab) => send("activated", tab)).catch(() => {});
});

// A aba ativa navegou (URL nova) ou o título estabilizou.
chrome.tabs.onUpdated.addListener((tabId, changeInfo, tab) => {
  if (tab.active && (changeInfo.url || changeInfo.title)) send("updated", tab);
});

// Fechou a aba (a API limpa o feed se era a ativa).
chrome.tabs.onRemoved.addListener((tabId) => {
  send("removed", { id: tabId });
});

// Mudou de JANELA do navegador → a aba ativa é outra.
chrome.windows.onFocusChanged.addListener((windowId) => {
  if (windowId === chrome.windows.WINDOW_ID_NONE) return;
  chrome.tabs
    .query({ active: true, windowId })
    .then(([tab]) => tab && send("activated", tab))
    .catch(() => {});
});

// Ao (re)instalar/acordar, reporta a aba ativa atual — sem esperar uma troca.
chrome.runtime.onStartup?.addListener(reportCurrent);
chrome.runtime.onInstalled.addListener(reportCurrent);
function reportCurrent() {
  chrome.tabs
    .query({ active: true, lastFocusedWindow: true })
    .then(([tab]) => tab && send("activated", tab))
    .catch(() => {});
}
