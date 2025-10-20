import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import {
  EditorSnapshot,
  KeySequencePayload,
  KeyStrokePayload,
  extractErrorMessage,
  fetchSnapshot,
  openFile,
  sendKeySequence,
} from '../services/backend';

const KEY_SEQUENCE_FLUSH_DELAY_MS = 160;
const IMMEDIATE_FLUSH_KEYS = new Set([
  'Enter',
  'Escape',
  'Tab',
  'Backspace',
  'Delete',
  'ArrowUp',
  'ArrowDown',
  'ArrowLeft',
  'ArrowRight',
]);

interface UseEditorResult {
  snapshot: EditorSnapshot | null;
  loading: boolean;
  error: string | null;
  handleKeyDown: (event: React.KeyboardEvent<HTMLDivElement>) => void;
  requestRefresh: () => Promise<void>;
  requestOpenFile: (path: string) => Promise<void>;
}

export function useEditor(): UseEditorResult {
  const [snapshot, setSnapshot] = useState<EditorSnapshot | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const pendingSequenceRef = useRef<KeyStrokePayload[][]>([]);
  const flushTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const isFlushingRef = useRef(false);
  const flushFnRef = useRef<() => Promise<void>>(async () => {});

  const scheduleFlush = useCallback(() => {
    if (flushTimerRef.current !== null) {
      clearTimeout(flushTimerRef.current);
    }
    flushTimerRef.current = window.setTimeout(() => {
      flushTimerRef.current = null;
      void flushFnRef.current();
    }, KEY_SEQUENCE_FLUSH_DELAY_MS);
  }, []);

  const clearFlushTimer = useCallback(() => {
    if (flushTimerRef.current !== null) {
      clearTimeout(flushTimerRef.current);
      flushTimerRef.current = null;
    }
  }, []);

  const flushPendingSequence = useCallback(async () => {
    if (isFlushingRef.current) {
      return;
    }

    if (pendingSequenceRef.current.length === 0) {
      clearFlushTimer();
      return;
    }

    const sequence = pendingSequenceRef.current.splice(0, pendingSequenceRef.current.length);
    const payload: KeySequencePayload = { sequence };

    isFlushingRef.current = true;
    clearFlushTimer();

    try {
      const next = await sendKeySequence(payload);
      setSnapshot(next);
      setError(null);
    } catch (err) {
      console.error('key-sequence error', err);
      setError(extractErrorMessage(err));
    } finally {
      isFlushingRef.current = false;
      if (pendingSequenceRef.current.length > 0) {
        scheduleFlush();
      }
    }
  }, [clearFlushTimer, scheduleFlush]);

  useEffect(() => {
    flushFnRef.current = () => flushPendingSequence();
    return () => {
      flushFnRef.current = async () => {};
      if (flushTimerRef.current !== null) {
        clearTimeout(flushTimerRef.current);
      }
    };
  }, [flushPendingSequence]);

  const requestRefresh = useCallback(async () => {
    setLoading(true);
    try {
      const next = await fetchSnapshot();
      setSnapshot(next);
      setError(null);
    } catch (err) {
      console.error('snapshot error', err);
      setError(extractErrorMessage(err));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void requestRefresh();
  }, [requestRefresh]);

  const requestOpenFile = useCallback(async (path: string) => {
    setLoading(true);
    try {
      const next = await openFile(path);
      setSnapshot(next);
      setError(null);
    } catch (err) {
      console.error('open-file error', err);
      setError(extractErrorMessage(err));
    } finally {
      setLoading(false);
    }
  }, []);

  const handleKeyDown = useCallback(
    (event: React.KeyboardEvent<HTMLDivElement>) => {
      if (event.isComposing) {
        return;
      }

      const stroke = createKeyStroke(event);
      if (!stroke) {
        return;
      }

      event.preventDefault();
      pendingSequenceRef.current.push([stroke]);
      if (shouldFlushImmediately(event, stroke)) {
        void flushPendingSequence();
      } else {
        scheduleFlush();
      }
    },
    [flushPendingSequence, scheduleFlush],
  );

  return useMemo(
    () => ({
      snapshot,
      loading,
      error,
      handleKeyDown,
      requestRefresh,
      requestOpenFile,
    }),
    [snapshot, loading, error, handleKeyDown, requestRefresh, requestOpenFile],
  );
}

function createKeyStroke(
  event: React.KeyboardEvent<HTMLDivElement>,
): KeyStrokePayload | null {
  const normalized = normalizeKey(event.key);
  if (!normalized) {
    return null;
  }

  const stroke: KeyStrokePayload = {
    key: normalized.key,
    ctrl: event.ctrlKey || event.metaKey,
    alt: event.altKey,
    shift: event.shiftKey && normalized.requiresShift,
  };

  return stroke;
}

function shouldFlushImmediately(
  event: React.KeyboardEvent<HTMLDivElement>,
  stroke: KeyStrokePayload,
): boolean {
  if (event.repeat) {
    return true;
  }

  if (stroke.ctrl || stroke.alt) {
    return true;
  }

  return IMMEDIATE_FLUSH_KEYS.has(event.key);
}

function normalizeKey(
  raw: string,
): { key: string; requiresShift: boolean } | null {
  if (raw.length === 1) {
    return {
      key: raw,
      requiresShift: false,
    };
  }

  switch (raw) {
    case 'Enter':
    case 'Backspace':
    case 'Delete':
    case 'Tab':
    case 'Escape':
    case 'ArrowUp':
    case 'ArrowDown':
    case 'ArrowLeft':
    case 'ArrowRight':
      return {
        key: transformNamedKey(raw),
        requiresShift: true,
      };
    default:
      return null;
  }
}

function transformNamedKey(key: string): string {
  return key
    .replace('Arrow', '')
    .replace('Escape', 'Esc')
    .replace('Backspace', 'Backspace')
    .replace('Delete', 'Delete');
}
