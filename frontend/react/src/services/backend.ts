import { invoke } from '@tauri-apps/api/core';

export interface EditorSnapshot {
  buffer: BufferSnapshot;
  minibuffer: MinibufferSnapshot;
  status: StatusSnapshot;
}

export interface BufferSnapshot {
  lines: string[];
  cursor: CursorSnapshot;
}

export interface CursorSnapshot {
  line: number;
  column: number;
}

export interface MinibufferSnapshot {
  mode: string;
  prompt: string;
  input: string;
  completions: string[];
  message?: string | null;
}

export interface StatusSnapshot {
  label: string;
  isModified: boolean;
}

export interface KeyStrokePayload {
  key: string;
  ctrl?: boolean;
  alt?: boolean;
  shift?: boolean;
}

export interface KeySequencePayload {
  sequence: KeyStrokePayload[][];
}

let fallbackBuffer = [
  'Tauri GUI は準備中です。',
  'Rust バックエンドと接続できないため、ローカルサンプルを表示しています。',
  '依存が揃ったら Tauri コマンドを実装し、invoke() が成功するようにしてください。',
];

export async function fetchSnapshot(): Promise<EditorSnapshot> {
  return invokeWithFallback<EditorSnapshot>('editor_snapshot', undefined, createFallbackSnapshot);
}

export async function sendKeySequence(payload: KeySequencePayload): Promise<EditorSnapshot> {
  return invokeWithFallback<EditorSnapshot>(
    'editor_handle_keys',
    { payload },
    () => updateFallbackBuffer(payload),
  );
}

export async function openFile(path: string): Promise<EditorSnapshot> {
  return invokeWithFallback<EditorSnapshot>(
    'editor_open_file',
    { path },
    () => appendFallbackMessage(`open-file: ${path}`),
  );
}

async function invokeWithFallback<T>(
  command: string,
  payload: unknown,
  fallback: () => T,
): Promise<T> {
  if (!isTauriRuntime()) {
    return fallback();
  }

  try {
    return await invoke<T>(command, payload);
  } catch (error) {
    console.warn(`[Tauri invoke: ${command}]`, error);
    return fallback();
  }
}

function isTauriRuntime(): boolean {
  return typeof window !== 'undefined' && '__TAURI_IPC__' in window;
}

function createFallbackSnapshot(): EditorSnapshot {
  return {
    buffer: {
      lines: [...fallbackBuffer],
      cursor: { line: fallbackBuffer.length - 1, column: lastLineLength() },
    },
    minibuffer: {
      mode: 'inactive',
      prompt: 'M-x',
      input: '',
      completions: [],
      message: 'Tauri backend 未接続 (fallback)',
    },
    status: {
      label: 'scratch (fallback)',
      isModified: false,
    },
  };
}

function updateFallbackBuffer(payload: KeySequencePayload): EditorSnapshot {
  for (const key of flattenSequence(payload)) {
    if (!key.ctrl && !key.alt && key.key.length === 1) {
      const current = lastLine();
      const updated = `${current}${key.key}`;
      replaceLastLine(updated);
    } else if (key.key === 'Enter') {
      appendLine('');
    } else if (key.key === 'Backspace') {
      const current = lastLine();
      const updated = current.slice(0, -1);
      replaceLastLine(updated);
    } else {
      appendLine(`[fallback] ${formatKeyStroke(key)}`);
    }
  }
  return createFallbackSnapshot();
}

function appendFallbackMessage(message: string): EditorSnapshot {
  appendLine(`[fallback] ${message}`);
  return createFallbackSnapshot();
}

function flattenSequence(payload: KeySequencePayload): KeyStrokePayload[] {
  return payload.sequence.flatMap((chunk) => chunk);
}

function formatKeyStroke(stroke: KeyStrokePayload): string {
  const modifiers = [
    stroke.ctrl ? 'C' : null,
    stroke.alt ? 'M' : null,
    stroke.shift ? 'S' : null,
  ].filter(Boolean);

  return [...modifiers, stroke.key].join('-');
}

function lastLine(): string {
  return fallbackBuffer.length ? fallbackBuffer[fallbackBuffer.length - 1] : '';
}

function lastLineLength(): number {
  return lastLine().length;
}

function replaceLastLine(line: string): void {
  if (!fallbackBuffer.length) {
    fallbackBuffer = [line];
    return;
  }
  fallbackBuffer = [...fallbackBuffer.slice(0, fallbackBuffer.length - 1), line];
}

function appendLine(line: string): void {
  fallbackBuffer = [...fallbackBuffer, line];
}
