import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { Todo } from "../types";

// Estado compartilhado de tarefas (usado na aba e no widget).
export function useTodos(pollMs = 0) {
  const [todos, setTodos] = useState<Todo[]>([]);

  async function refresh() {
    try {
      setTodos(await invoke<Todo[]>("list_todos"));
    } catch {
      /* ignore */
    }
  }

  useEffect(() => {
    refresh();
    if (pollMs > 0) {
      const id = setInterval(refresh, pollMs);
      return () => clearInterval(id);
    }
  }, [pollMs]);

  async function add(text: string) {
    if (!text.trim()) return;
    await invoke("add_todo", { text: text.trim() });
    refresh();
  }
  async function toggle(id: number) {
    await invoke("toggle_todo", { id });
    refresh();
  }
  async function remove(id: number) {
    await invoke("delete_todo", { id });
    refresh();
  }

  const open = todos.filter((t) => !t.done);
  const done = todos.filter((t) => t.done);

  return { todos, open, done, add, toggle, remove, refresh };
}
