import { invoke } from "@tauri-apps/api/core";
import DOMPurify from "dompurify";
import { marked } from "marked";

import "./styles.css";

type DiaryState = {
  date: string;
  dates: string[];
  dirty: boolean;
  saving: boolean;
};

const state: DiaryState = {
  date: localToday(),
  dates: [],
  dirty: false,
  saving: false,
};

function requiredElement<T extends Element>(selector: string): T {
  const element = document.querySelector<T>(selector);
  if (!element) {
    throw new Error(`必要な画面要素が見つかりません: ${selector}`);
  }
  return element;
}

function localToday(): string {
  const date = new Date();
  const year = date.getFullYear();
  const month = String(date.getMonth() + 1).padStart(2, "0");
  const day = String(date.getDate()).padStart(2, "0");
  return `${year}-${month}-${day}`;
}

function displayDate(value: string): string {
  const [year, month, day] = value.split("-").map(Number);
  return new Intl.DateTimeFormat("ja-JP", {
    year: "numeric",
    month: "long",
    day: "numeric",
    weekday: "short",
  }).format(new Date(year, month - 1, day));
}

function titleFromMarkdown(markdown: string): string | null {
  const heading = markdown.match(/^\s*#\s+(.+?)\s*#*\s*$/m);
  return heading?.[1]?.trim() || null;
}

function errorMessage(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

window.addEventListener("DOMContentLoaded", async () => {
  const editor = requiredElement<HTMLTextAreaElement>("#editor");
  const dateInput = requiredElement<HTMLInputElement>("#entry-date");
  const saveButton = requiredElement<HTMLButtonElement>("#save-button");
  const todayButton = requiredElement<HTMLButtonElement>("#today-button");
  const entryList = requiredElement<HTMLElement>("#entry-list");
  const entryCount = requiredElement<HTMLElement>("#entry-count");
  const entryTitle = requiredElement<HTMLElement>("#entry-title");
  const preview = requiredElement<HTMLElement>("#preview");
  const characterCount = requiredElement<HTMLElement>("#character-count");
  const statusMessage = requiredElement<HTMLElement>("#status-message");
  const statusDot = requiredElement<HTMLElement>("#status-dot");

  marked.setOptions({ gfm: true, breaks: true });

  function setStatus(message: string, kind: "idle" | "dirty" | "error" = "idle") {
    statusMessage.textContent = message;
    statusDot.dataset.kind = kind;
  }

  function renderEditor() {
    const markdown = editor.value;
    entryTitle.textContent = titleFromMarkdown(markdown) ?? displayDate(state.date);
    characterCount.textContent = `${markdown.length.toLocaleString("ja-JP")} 文字`;

    if (!markdown.trim()) {
      preview.replaceChildren();
      const placeholder = document.createElement("p");
      placeholder.className = "preview-placeholder";
      placeholder.textContent = "書き始めると、ここにプレビューが表示されます。";
      preview.append(placeholder);
      return;
    }

    const parsed = marked.parse(markdown, { async: false });
    preview.innerHTML = DOMPurify.sanitize(parsed, {
      USE_PROFILES: { html: true },
    });
  }

  function renderEntryList() {
    entryList.replaceChildren();
    entryCount.textContent = state.dates.length.toLocaleString("ja-JP");

    if (state.dates.length === 0) {
      const empty = document.createElement("p");
      empty.className = "empty-list";
      empty.textContent = "最初の日記を書いてみましょう。";
      entryList.append(empty);
      return;
    }

    for (const date of state.dates) {
      const button = document.createElement("button");
      button.type = "button";
      button.className = "entry-button";
      button.classList.toggle("is-current", date === state.date);
      button.dataset.date = date;

      const primary = document.createElement("span");
      primary.textContent = displayDate(date);
      const secondary = document.createElement("span");
      secondary.textContent = date;
      button.append(primary, secondary);
      button.addEventListener("click", () => void openDate(date));
      entryList.append(button);
    }
  }

  function setDirty(dirty: boolean) {
    state.dirty = dirty;
    saveButton.disabled = !dirty || state.saving;
    setStatus(dirty ? "未保存の変更があります" : "すべて保存済み", dirty ? "dirty" : "idle");
  }

  async function openDate(date: string) {
    if (date === state.date && dateInput.value === date) {
      return;
    }
    if (state.dirty && !window.confirm("未保存の変更を破棄して、別の日記を開きますか？")) {
      dateInput.value = state.date;
      return;
    }

    try {
      const content = await invoke<string | null>("read_entry", { date });
      state.date = date;
      dateInput.value = date;
      editor.value = content ?? "";
      setDirty(false);
      renderEditor();
      renderEntryList();
      editor.focus();
    } catch (error) {
      dateInput.value = state.date;
      setStatus(`読込に失敗しました: ${errorMessage(error)}`, "error");
    }
  }

  async function saveCurrentEntry() {
    if (!state.dirty || state.saving) {
      return;
    }

    state.saving = true;
    saveButton.disabled = true;
    setStatus("保存しています…");
    try {
      await invoke("save_entry", { date: state.date, content: editor.value });
      if (!state.dates.includes(state.date)) {
        state.dates = [...state.dates, state.date].sort((left, right) =>
          right.localeCompare(left),
        );
      }
      setDirty(false);
      renderEntryList();
    } catch (error) {
      setStatus(`保存に失敗しました: ${errorMessage(error)}`, "error");
      saveButton.disabled = false;
    } finally {
      state.saving = false;
    }
  }

  editor.addEventListener("input", () => {
    setDirty(true);
    renderEditor();
  });
  dateInput.addEventListener("change", () => {
    if (dateInput.value) {
      void openDate(dateInput.value);
    }
  });
  saveButton.addEventListener("click", () => void saveCurrentEntry());
  todayButton.addEventListener("click", () => void openDate(localToday()));
  preview.addEventListener("click", (event) => {
    if (event.target instanceof Element && event.target.closest("a")) {
      event.preventDefault();
    }
  });
  window.addEventListener("keydown", (event) => {
    if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "s") {
      event.preventDefault();
      void saveCurrentEntry();
    }
  });
  window.addEventListener("beforeunload", (event) => {
    if (state.dirty) {
      event.preventDefault();
    }
  });

  dateInput.value = state.date;
  try {
    state.dates = await invoke<string[]>("list_entry_dates");
    const initialContent = await invoke<string | null>("read_entry", { date: state.date });
    editor.value = initialContent ?? "";
    renderEntryList();
    renderEditor();
    setDirty(false);
    editor.focus();
  } catch (error) {
    setStatus(`起動に失敗しました: ${errorMessage(error)}`, "error");
  }
});
