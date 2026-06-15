import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";

// Aplica o tema salvo antes de renderizar (evita piscar). Padrão: escuro.
document.documentElement.dataset.theme =
  localStorage.getItem("theme") === "light" ? "light" : "dark";

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
