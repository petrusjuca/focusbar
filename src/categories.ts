// Taxonomia única de categorias — a MESMA que a IA usa (ai.rs normalize_category)
// e que as regras por app produzem (category.rs). Antes o dropdown de override só
// listava 4 e a IA gerava 7 (Estudo/Comunicação/Pessoal sumiam na hora de corrigir).
export const CATEGORIES: string[] = [
  "Trabalho",
  "Estudo",
  "Comunicação",
  "Pessoal",
  "Ferramenta",
  "Procrastinação",
  "Outro",
];

const CAT_COLORS: Record<string, string> = {
  Trabalho: "#34c759",
  Estudo: "#5ac8fa",
  Comunicação: "#ffcc00",
  Pessoal: "#af52de",
  Ferramenta: "#007aff",
  Procrastinação: "#ff3b30",
  Outro: "#8e8e93",
};

export function catColor(cat: string): string {
  return CAT_COLORS[cat] ?? "#8e8e93";
}
