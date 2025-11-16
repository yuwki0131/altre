import { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import {
  EditorSnapshot,
  KeyStrokePayload,
  SaveResult,
  extractErrorMessage,
  fetchSnapshot,
  openFile,
  pickOpenFile,
  saveFile,
  sendKeySequence,
} from '../services/backend';

const SNAPSHOT_POLL_INTERVAL_MS = 120;

interface FetchOptions {
  showLoader?: boolean;
  force?: boolean;
}

interface UseEditorResult {
  snapshot: EditorSnapshot | null;
  loading: boolean;
  error: string | null;
  info: string | null;
  handleKeyDown: (event: React.KeyboardEvent<HTMLDivElement>) => void;
  requestRefresh: () => Promise<void>;
  requestOpenFile: (path: string) => Promise<void>;
  requestOpenFileDialog: () => Promise<void>;
  requestSaveFile: () => Promise<void>;
}

export function useEditor(): UseEditorResult {
  const [snapshot, setSnapshot] = useState<EditorSnapshot | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [info, setInfo] = useState<string | null>(null);
  const snapshotTimerRef = useRef<ReturnType<typeof setInterval> | null>(null);
  const isFetchingSnapshotRef = useRef(false);
  const pendingFetchRef = useRef<FetchOptions | null>(null);
  const needsInitialSnapshotRef = useRef(true);

  const fetchLatestSnapshot = useCallback(async function fetchLatestSnapshotImpl(
    options?: FetchOptions,
  ) {
    if (isFetchingSnapshotRef.current) {
      if (options?.force) {
        const previous = pendingFetchRef.current;
        pendingFetchRef.current = {
          force: true,
          showLoader: options.showLoader || previous?.showLoader,
        };
      }
      return;
    }

    const shouldShowLoader = options?.showLoader || needsInitialSnapshotRef.current;
    if (shouldShowLoader) {
      setLoading(true);
    }

    isFetchingSnapshotRef.current = true;
    try {
      const next = await fetchSnapshot();
      setSnapshot(next);
      needsInitialSnapshotRef.current = false;
      setError(null);
      setInfo(null);
    } catch (err) {
      console.error('snapshot error', err);
      setError(extractErrorMessage(err));
      setInfo(null);
    } finally {
      isFetchingSnapshotRef.current = false;
      if (shouldShowLoader) {
        setLoading(false);
        needsInitialSnapshotRef.current = false;
      }
      const pending = pendingFetchRef.current;
      pendingFetchRef.current = null;
      if (pending) {
        void fetchLatestSnapshotImpl(pending);
      }
    }
  }, []);

  useEffect(() => {
    void fetchLatestSnapshot({ showLoader: true });
    snapshotTimerRef.current = window.setInterval(() => {
      void fetchLatestSnapshot();
    }, SNAPSHOT_POLL_INTERVAL_MS);

    return () => {
      if (snapshotTimerRef.current !== null) {
        clearInterval(snapshotTimerRef.current);
        snapshotTimerRef.current = null;
      }
    };
  }, [fetchLatestSnapshot]);

  const requestRefresh = useCallback(async () => {
    await fetchLatestSnapshot({ force: true, showLoader: true });
  }, [fetchLatestSnapshot]);

  const requestOpenFile = useCallback(async (path: string) => {
    setLoading(true);
    setInfo(null);
    try {
      const next = await openFile(path);
      setSnapshot(next);
      needsInitialSnapshotRef.current = false;
      setError(null);
      setInfo(null);
    } catch (err) {
      console.error('open-file error', err);
      setError(extractErrorMessage(err));
      setInfo(null);
    } finally {
      setLoading(false);
    }
  }, []);

  const requestOpenFileDialog = useCallback(async () => {
    try {
      const selected = await pickOpenFile();
      if (!selected) {
        return;
      }
      await requestOpenFile(selected);
    } catch (err) {
      console.error('open-dialog error', err);
      setError(extractErrorMessage(err));
      setInfo(null);
    }
  }, [requestOpenFile]);

  const requestSaveFile = useCallback(async () => {
    setLoading(true);
    try {
      const result: SaveResult = await saveFile();
      setSnapshot(result.snapshot);
      needsInitialSnapshotRef.current = false;
      if (result.success) {
        setError(null);
        setInfo(result.message ?? '保存しました');
      } else {
        setError(result.message ?? '保存に失敗しました');
        setInfo(null);
      }
    } catch (err) {
      console.error('save-file error', err);
      setError(extractErrorMessage(err));
      setInfo(null);
    } finally {
      setLoading(false);
    }
  }, []);

  const dispatchKeySequence = useCallback(
    async (sequence: KeyStrokePayload[][]) => {
      try {
        await sendKeySequence({ sequence });
        setError(null);
        setInfo(null);
        void fetchLatestSnapshot({ force: true });
      } catch (err) {
        console.error('key-sequence error', err);
        setError(extractErrorMessage(err));
        setInfo(null);
      }
    },
    [fetchLatestSnapshot],
  );

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
      void dispatchKeySequence([[stroke]]);
    },
    [dispatchKeySequence],
  );

  return useMemo(
    () => ({
      snapshot,
      loading,
      error,
      info,
      handleKeyDown,
      requestRefresh,
      requestOpenFile,
      requestOpenFileDialog,
      requestSaveFile,
    }),
    [snapshot, loading, error, info, handleKeyDown, requestRefresh, requestOpenFile, requestOpenFileDialog, requestSaveFile],
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
