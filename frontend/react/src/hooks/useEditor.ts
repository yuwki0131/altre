import { useCallback, useEffect, useMemo, useState } from 'react';
import {
  EditorSnapshot,
  KeySequencePayload,
  KeyStrokePayload,
  fetchSnapshot,
  openFile,
  sendKeySequence,
} from '../services/backend';

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

  const requestRefresh = useCallback(async () => {
    try {
      setLoading(true);
      const next = await fetchSnapshot();
      setSnapshot(next);
      setError(null);
    } catch (err) {
      console.error('snapshot error', err);
      setError('バックエンドとの通信に失敗しました');
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void requestRefresh();
  }, [requestRefresh]);

  const requestOpenFile = useCallback(async (path: string) => {
    try {
      setLoading(true);
      const next = await openFile(path);
      setSnapshot(next);
      setError(null);
    } catch (err) {
      console.error('open-file error', err);
      setError('ファイルを開けませんでした');
    } finally {
      setLoading(false);
    }
  }, []);

  const handleKeyDown = useCallback(
    async (event: React.KeyboardEvent<HTMLDivElement>) => {
      // 変換できないキーはブラウザに任せる
      const payload = createKeySequence(event);
      if (!payload) {
        return;
      }

      event.preventDefault();

      try {
        const next = await sendKeySequence(payload);
        setSnapshot(next);
        setError(null);
      } catch (err) {
        console.error('key-sequence error', err);
        setError('キー入力の送信に失敗しました');
      }
    },
    [],
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

function createKeySequence(
  event: React.KeyboardEvent<HTMLDivElement>,
): KeySequencePayload | null {
  const key = normalizeKey(event.key);
  if (!key) {
    return null;
  }

  const stroke: KeyStrokePayload = {
    key,
    ctrl: event.ctrlKey || event.metaKey,
    alt: event.altKey,
    shift: event.shiftKey && key.length > 1, // 1文字の場合は後段で表示
  };

  return { keys: [stroke] };
}

function normalizeKey(raw: string): string | null {
  if (raw.length === 1) {
    return raw;
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
      return transformNamedKey(raw);
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
