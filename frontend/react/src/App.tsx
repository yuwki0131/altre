import { useEffect, useMemo, useRef } from 'react';
import { useEditor } from './hooks/useEditor';

export function App() {
  const {
    snapshot,
    loading,
    error,
    info,
    handleKeyDown,
  } = useEditor();
  const editorRef = useRef<HTMLDivElement>(null);
  const activeLineRef = useRef<HTMLSpanElement | null>(null);

  useEffect(() => {
    editorRef.current?.focus();
  }, [snapshot]);

  const lines = useMemo(() => snapshot?.buffer.lines ?? [], [snapshot]);
  const cursorLine = snapshot?.buffer.cursor.line ?? 0;
  const cursorColumn = snapshot?.buffer.cursor.column ?? 0;

  const lineCount = useMemo(() => {
    if (!snapshot) {
      return 0;
    }
    return Math.max(1, snapshot.buffer.lines.length);
  }, [snapshot]);

  const statusText = useMemo(() => {
    if (!snapshot) {
      return ' 起動中...';
    }

    const modifiedFlag = snapshot.status.isModified ? '*' : ' ';
    const label = snapshot.status.label || 'scratch';
    const line = snapshot.buffer.cursor.line + 1;
    const column = snapshot.buffer.cursor.column + 1;
    const fpsDisplay = '--';

    return ` ${modifiedFlag} ${label}  Ln ${line}, Col ${column}  ${lineCount} lines  FPS: ${fpsDisplay}`;
  }, [snapshot, lineCount]);

  const minibufferLines = useMemo(() => {
    if (!snapshot) {
      return [
        {
          key: 'loading',
          type: 'info' as const,
          content: loading ? '読み込み中...' : '初期化中...',
        },
      ];
    }

    const lines: Array<{
      key: string;
      type: 'prompt' | 'info' | 'error';
      prompt?: string;
      input?: string;
      content?: string;
    }> = [];

    const mode = snapshot.minibuffer.mode;
    const prompt = snapshot.minibuffer.prompt;
    const input = snapshot.minibuffer.input;
    const statusMessage = snapshot.minibuffer.message;

    const globalError = error ?? null;
    const globalInfo = info ?? null;

    const interactiveModes = new Set([
      'find-file',
      'execute-command',
      'eval-expression',
      'write-file',
      'switch-buffer',
      'kill-buffer',
      'query-replace-pattern',
      'query-replace-replacement',
      'goto-line',
      'save-confirmation',
    ]);

    if (interactiveModes.has(mode)) {
      lines.push({
        key: 'prompt',
        type: 'prompt',
        prompt,
        input: input.length > 0 ? input : '\u00a0',
      });

      if (mode === 'goto-line' && statusMessage) {
        lines.push({
          key: 'goto-status',
          type: 'info',
          content: statusMessage,
        });
      }
    } else if (mode === 'error') {
      if (statusMessage) {
        lines.push({
          key: 'minibuffer-error',
          type: 'error',
          content: statusMessage,
        });
      }
    } else if (mode === 'info' && statusMessage) {
      lines.push({
        key: 'minibuffer-info',
        type: 'info',
        content: statusMessage,
      });
    }

    if (lines.length === 0) {
      const message = globalError ?? globalInfo ?? statusMessage;
      if (message) {
        lines.push({
          key: 'fallback-message',
          type: globalError ? 'error' : 'info',
          content: message,
        });
      }
    } else if (globalError) {
      lines.push({
        key: 'global-error',
        type: 'error',
        content: globalError,
      });
    } else if (globalInfo) {
      lines.push({
        key: 'global-info',
        type: 'info',
        content: globalInfo,
      });
    }

    if (lines.length === 0) {
      return [
        {
          key: 'inactive',
          type: 'info' as const,
          content: '\u00a0',
        },
      ];
    }

    return lines;
  }, [snapshot, error, info, loading]);

  const bufferLineCount = snapshot?.buffer.lines.length ?? 0;

  useEffect(() => {
    if (activeLineRef.current) {
      activeLineRef.current.scrollIntoView({ block: 'nearest' });
    }
  }, [cursorLine, cursorColumn, bufferLineCount]);

  return (
    <div className="app">
      <div className="app__minibuffer">
        {minibufferLines.map((line) => {
          if (line.type === 'prompt') {
            return (
              <div key={line.key} className="minibuffer__line">
                <span className="minibuffer__prompt">{line.prompt ?? ''}</span>
                <span className="minibuffer__input">{line.input ?? '\u00a0'}</span>
              </div>
            );
          }

          const className =
            line.type === 'error' ? 'minibuffer__error' : 'minibuffer__message';
          return (
            <div key={line.key} className="minibuffer__line">
              <span className={className}>{line.content ?? '\u00a0'}</span>
            </div>
          );
        })}
      </div>

      <div className="editor-surface">
        <div
          className="editor-surface__buffer"
          tabIndex={0}
          ref={editorRef}
          onKeyDown={handleKeyDown}
        >
          {lines.length === 0 ? (
            <span className="editor-surface__line">(空のバッファ)</span>
          ) : (
            lines.map((line, index) => {
              const isActive = index === cursorLine;
              if (!isActive) {
                return (
                  <span key={index} className="editor-surface__line">
                    {line || '\u00a0'}
                  </span>
                );
              }

              const safeColumn = Math.min(cursorColumn, line.length);
              const before = line.slice(0, safeColumn) || '\u00a0';
              const cursorChar = line.charAt(safeColumn) || '\u00a0';
              const after = line.slice(cursorChar === '\u00a0' ? safeColumn : safeColumn + 1);

              return (
                <span
                  key={index}
                  className="editor-surface__line editor-surface__line--active"
                  ref={activeLineRef}
                >
                  <span>{before}</span>
                  <span className="editor-surface__cursor">{cursorChar}</span>
                  <span>{after}</span>
                </span>
              );
            })
          )}
        </div>
      </div>

      <div className="statusline">
        <span className="statusline__content">{statusText}</span>
      </div>
    </div>
  );
}
