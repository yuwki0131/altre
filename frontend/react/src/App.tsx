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
  const firstVisibleLineRef = useRef<HTMLSpanElement | null>(null);
  const previousCursorLineRef = useRef<number>(0);
  const previousTopLineRef = useRef<number>(0);

  useEffect(() => {
    editorRef.current?.focus();
  }, [snapshot]);

  const bufferLines = useMemo(() => snapshot?.buffer.lines ?? [], [snapshot]);
  const cursorLine = snapshot?.buffer.cursor.line ?? 0;
  const cursorColumn = snapshot?.buffer.cursor.column ?? 0;
  const searchState = snapshot?.search ?? null;

  const topLine = snapshot?.viewport?.topLine ?? 0;
  const viewportHeight = Math.max(1, snapshot?.viewport?.height ?? (bufferLines.length || 1));
  const visibleStart = useMemo(() => {
    const maxStart = Math.max(0, bufferLines.length - viewportHeight);
    return Math.min(Math.max(0, topLine), maxStart);
  }, [bufferLines.length, topLine, viewportHeight]);
  const visibleLines = useMemo(() => {
    const result: Array<{ content: string; index: number | null }> = [];
    for (let offset = 0; offset < viewportHeight; offset += 1) {
      const actualIndex = visibleStart + offset;
      if (actualIndex < bufferLines.length) {
        result.push({ content: bufferLines[actualIndex], index: actualIndex });
      } else {
        result.push({ content: '', index: null });
      }
    }
    return result;
  }, [bufferLines, visibleStart, viewportHeight]);

  const lineCount = useMemo(() => {
    if (!snapshot) {
      return 0;
    }
    return Math.max(1, bufferLines.length);
  }, [snapshot, bufferLines.length]);

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

    if (searchState) {
      const patternDisplay = searchState.pattern.length > 0 ? searchState.pattern : '\u00a0';
      const matchInfo =
        searchState.totalMatches > 0
          ? ` ${searchState.currentMatch ?? 0}/${searchState.totalMatches}`
          : '';
      const flags: string[] = [];
      if (searchState.status === 'not-found') {
        flags.push('Failing');
      }
      if (searchState.wrapped) {
        flags.push('Wrapped');
      }
      const suffix = flags.length > 0 ? ` (${flags.join(', ')})` : '';

      lines.push({
        key: 'search-state',
        type: searchState.status === 'not-found' ? 'error' : 'info',
        content: `${searchState.prompt}: ${patternDisplay}${matchInfo}${suffix}`,
      });

      if (searchState.message) {
        lines.push({
          key: 'search-message',
          type: searchState.status === 'not-found' ? 'error' : 'info',
          content: searchState.message,
        });
      }

      return lines;
    }

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

  const bufferLineCount = bufferLines.length;

  useEffect(() => {
    if (visibleStart === previousTopLineRef.current) {
      return;
    }
    previousTopLineRef.current = visibleStart;
    if (firstVisibleLineRef.current) {
      firstVisibleLineRef.current.scrollIntoView({ block: 'start', inline: 'nearest' });
    }
  }, [visibleStart]);

  useEffect(() => {
    if (!activeLineRef.current) {
      previousCursorLineRef.current = cursorLine;
      return;
    }

    const previousLine = previousCursorLineRef.current;
    const delta = cursorLine - previousLine;

    let block: ScrollLogicalPosition = 'nearest';
    if (delta > 1) {
      block = 'end';
    } else if (delta < -1) {
      block = 'start';
    }

    activeLineRef.current.scrollIntoView({ block, inline: 'nearest' });

    previousCursorLineRef.current = cursorLine;
  }, [cursorLine, cursorColumn, bufferLineCount]);

  activeLineRef.current = null;
  firstVisibleLineRef.current = null;

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
          {visibleLines.length === 0 ? (
            <span className="editor-surface__line">(空のバッファ)</span>
          ) : (
            visibleLines.map((line, index) => {
              const actualIndex = line.index;
              const key = actualIndex ?? `placeholder-${visibleStart + index}`;
              const isActive = actualIndex !== null && actualIndex === cursorLine;

              const content = line.content;
              const safeColumn = Math.min(cursorColumn, content.length);
              const before = content.slice(0, safeColumn);
              const cursorChar = content.charAt(safeColumn) || '\u00a0';
              const after = content.slice(cursorChar === '\u00a0' ? safeColumn : safeColumn + 1);

              if (!isActive) {
                return (
                  <span
                    key={key}
                    className="editor-surface__line"
                    ref={(el) => {
                      if (index === 0) {
                        firstVisibleLineRef.current = el;
                      }
                    }}
                  >
                    {before === '\u00a0' && after === '' ? '\u00a0' : content || '\u00a0'}
                  </span>
                );
              }

              return (
                <span
                  key={key}
                  className="editor-surface__line editor-surface__line--active"
                  ref={(el) => {
                    if (index === 0) {
                      firstVisibleLineRef.current = el;
                    }
                    activeLineRef.current = el;
                  }}
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
