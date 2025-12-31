import { invoke } from '@tauri-apps/api/core';

export interface ViewportSnapshot {
  topLine: number;
  height: number;
  scrollX: number;
  width: number;
}

export interface EditorSnapshot {
  buffer: BufferSnapshot;
  minibuffer: MinibufferSnapshot;
  status: StatusSnapshot;
  viewport: ViewportSnapshot;
  theme: GuiThemeSnapshot;
  searchUi?: SearchUISnapshot | null;
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

export interface GuiThemeSnapshot {
  appBackground: string;
  appForeground: string;
  focusRing: string;
  activeLineBackground: string;
  cursorBackground: string;
  cursorForeground: string;
  minibufferBorder: string;
  minibufferPrompt: string;
  minibufferInput: string;
  minibufferInfo: string;
  minibufferError: string;
  statuslineBorder: string;
  statuslineBackground: string;
  statuslineForeground: string;
}

export interface SearchUISnapshot {
  promptLabel: string;
  pattern: string;
  status: 'active' | 'not-found' | 'wrapped';
  currentMatch?: number | null;
  totalMatches: number;
  wrapped: boolean;
  message?: string | null;
  direction: 'forward' | 'backward';
}

export const DEFAULT_GUI_THEME: GuiThemeSnapshot = {
  appBackground: '#FFFFFF',
  appForeground: '#101010',
  focusRing: '#0997B633',
  activeLineBackground: '#F0F0F0',
  cursorBackground: '#E5266A',
  cursorForeground: '#FFFFFF',
  minibufferBorder: '#F0F0F0',
  minibufferPrompt: '#0997B6',
  minibufferInput: '#101010',
  minibufferInfo: '#FF4C00',
  minibufferError: '#E5266A',
  statuslineBorder: '#F0F0F0',
  statuslineBackground: '#F0F0F0',
  statuslineForeground: '#101010',
};

export interface KeyStrokePayload {
  key: string;
  ctrl?: boolean;
  alt?: boolean;
  shift?: boolean;
}

export interface KeySequencePayload {
  sequence: KeyStrokePayload[][];
}

export interface SaveResult {
  snapshot: EditorSnapshot;
  success: boolean;
  message?: string;
}

let fallbackBuffer = [
  'Tauri GUI は準備中です。',
  'Rust バックエンドと接続できないため、ローカルサンプルを表示しています。',
  '依存が揃ったら Tauri コマンドを実装し、invoke() が成功するようにしてください。',
];

export async function fetchSnapshot(): Promise<EditorSnapshot> {
  if (!isTauriRuntime()) {
    return createFallbackSnapshot();
  }

  try {
    return await invoke<EditorSnapshot>('editor_snapshot');
  } catch (error) {
    throw formatBackendError('editor_snapshot', error);
  }
}

export async function sendKeySequence(payload: KeySequencePayload): Promise<boolean> {
  if (!isTauriRuntime()) {
    updateFallbackBuffer(payload);
    return false;
  }

  try {
    return await invoke<boolean>('editor_handle_keys', { payload });
  } catch (error) {
    throw formatBackendError('editor_handle_keys', error);
  }
}

export async function openFile(path: string): Promise<EditorSnapshot> {
  if (!isTauriRuntime()) {
    return appendFallbackMessage(`open-file: ${path}`);
  }

  try {
    return await invoke<EditorSnapshot>('editor_open_file', { path });
  } catch (error) {
    throw formatBackendError('editor_open_file', error);
  }
}

export async function saveFile(): Promise<SaveResult> {
  if (!isTauriRuntime()) {
    return {
      snapshot: appendFallbackMessage('save-file (fallback)'),
      success: false,
      message: 'Tauri backend が利用できないため保存できません',
    };
  }

  try {
    const response = await invoke<SaveResult>('editor_save_file');
    return response;
  } catch (error) {
    throw formatBackendError('editor_save_file', error);
  }
}

export async function resizeViewport(height: number, width?: number): Promise<EditorSnapshot> {
  if (!isTauriRuntime()) {
    // fallback: 高さだけ反映したスナップショットを返す
    const snap = createFallbackSnapshot();
    return {
      ...snap,
      viewport: {
        ...snap.viewport,
        height: Math.max(1, Math.floor(height) || 1),
        width: width && width > 0 ? Math.floor(width) : snap.viewport.width,
      },
    };
  }

  try {
    return await invoke<EditorSnapshot>('editor_resize_viewport', {
      height: Math.max(1, Math.floor(height) || 1),
      width: width && width > 0 ? Math.floor(width) : null,
    });
  } catch (error) {
    throw formatBackendError('editor_resize_viewport', error);
  }
}

export async function pickOpenFile(): Promise<string | null> {
  try {
    if (!isTauriRuntime()) {
      return promptForPath();
    }

    const internals = (window as Record<string, unknown> & {
      __TAURI_INTERNALS__?: {
        dialog?: {
          open?: (options: { multiple: boolean }) => Promise<string | string[] | null>;
        };
      };
    }).__TAURI_INTERNALS__;

    const dialog = internals?.dialog?.open;
    if (typeof dialog === 'function') {
      const selected = await dialog({ multiple: false });
      if (Array.isArray(selected)) {
        return selected[0] ?? null;
      }
      return selected ?? null;
    }

    return promptForPath();
  } catch (error) {
    throw formatBackendError('dialog.open', error);
  }
}

const TAURI_DETECTION_KEYS = ['__TAURI_IPC__', '__TAURI_INTERNALS__', '__TAURI_METADATA__'];

function isTauriRuntime(): boolean {
  if (typeof window === 'undefined') {
    return false;
  }
  return TAURI_DETECTION_KEYS.some((key) => key in (window as Record<string, unknown>));
}

function formatBackendError(command: string, error: unknown): Error {
  const message = extractErrorMessage(error);
  return new Error(`コマンド ${command} の実行に失敗しました: ${message}`);
}

export function extractErrorMessage(error: unknown): string {
  if (!error) {
    return '不明なエラー';
  }
  if (error instanceof Error) {
    return error.message;
  }
  if (typeof error === 'string') {
    return error;
  }
  try {
    return JSON.stringify(error);
  } catch {
    return String(error);
  }
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
    viewport: createFallbackViewport(),
    theme: { ...DEFAULT_GUI_THEME },
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

function createFallbackViewport(): ViewportSnapshot {
  const height = Math.max(1, fallbackBuffer.length);
  const maxWidth = fallbackBuffer.reduce((acc, line) => Math.max(acc, line.length), 0);
  return {
    topLine: 0,
    height,
    scrollX: 0,
    width: maxWidth,
  };
}

function promptForPath(): string | null {
  if (typeof window === 'undefined') {
    return null;
  }
  const input = window.prompt('開くファイルのパスを入力してください');
  if (!input) {
    return null;
  }
  const trimmed = input.trim();
  return trimmed.length > 0 ? trimmed : null;
}
