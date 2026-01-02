import { useEffect, useLayoutEffect, useMemo, useRef, useState } from 'react';
import { useEditor } from './hooks/useEditor';
import { DEFAULT_GUI_THEME, GuiThemeSnapshot, resizeViewport } from './services/backend';

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
  const previousDisplayIndexRef = useRef<number>(0);
  const previousTopLineRef = useRef<number>(0);

  useEffect(() => {
    editorRef.current?.focus();
  }, [snapshot]);

  const bufferLines = useMemo(() => snapshot?.buffer.lines ?? [], [snapshot]);
  const cursorLine = snapshot?.buffer.cursor.line ?? 0;
  const cursorColumn = snapshot?.buffer.cursor.column ?? 0;
  const guiTheme = useMemo<GuiThemeSnapshot>(
    () => snapshot?.theme ?? DEFAULT_GUI_THEME,
    [snapshot],
  );

  useEffect(() => {
    const root = document.documentElement;
    const entries: Array<[string, string]> = [
      ['--altre-app-background', guiTheme.appBackground],
      ['--altre-app-foreground', guiTheme.appForeground],
      ['--altre-focus-ring', guiTheme.focusRing],
      ['--altre-active-line-background', guiTheme.activeLineBackground],
      ['--altre-cursor-background', guiTheme.cursorBackground],
      ['--altre-cursor-foreground', guiTheme.cursorForeground],
      ['--altre-minibuffer-border', guiTheme.minibufferBorder],
      ['--altre-minibuffer-prompt', guiTheme.minibufferPrompt],
      ['--altre-minibuffer-input', guiTheme.minibufferInput],
      ['--altre-minibuffer-info', guiTheme.minibufferInfo],
      ['--altre-minibuffer-error', guiTheme.minibufferError],
      ['--altre-statusline-border', guiTheme.statuslineBorder],
      ['--altre-statusline-background', guiTheme.statuslineBackground],
      ['--altre-statusline-foreground', guiTheme.statuslineForeground],
    ];

    for (const [name, value] of entries) {
      if (value) {
        root.style.setProperty(name, value);
      } else {
        root.style.removeProperty(name);
      }
    }
  }, [guiTheme]);

  const topLine = snapshot?.viewport?.topLine ?? 0;

  // DOM 計測に基づく行高・列幅から viewport を再計算
  const [measuredRows, setMeasuredRows] = useState<number>(snapshot?.viewport?.height ?? 1);
  const [measuredCols, setMeasuredCols] = useState<number>(snapshot?.viewport?.width ?? 80);

  // エディタ表示領域のサイズ変化を監視し、行数・列数を算出
  useLayoutEffect(() => {
    const el = editorRef.current;
    if (!el) return;

    const measure = () => {
      const rect = el.getBoundingClientRect();
      const style = getComputedStyle(el);
      // 1行の高さを測定
      const probe = document.createElement('span');
      probe.textContent = 'A';
      probe.style.visibility = 'hidden';
      probe.style.position = 'absolute';
      probe.style.whiteSpace = 'pre';
      probe.style.lineHeight = style.lineHeight;
      probe.style.fontFamily = style.fontFamily;
      probe.style.fontSize = style.fontSize;
      el.appendChild(probe);
      const lineH = probe.getBoundingClientRect().height || 16;
      // 1桁の幅（近似値）
      probe.textContent = 'M';
      const chW = probe.getBoundingClientRect().width || 8;
      el.removeChild(probe);

      // WebView での端数発生により最終行が見切れないよう、
      // 計測値は切り上げ、行数・列数は安全側（過小見積もり）で算出
      const lineHPx = Math.max(1, Math.ceil(lineH));
      const chWPx = Math.max(1, Math.ceil(chW));
      const paddingV = (parseFloat(style.paddingTop || '0') || 0) + (parseFloat(style.paddingBottom || '0') || 0);
      const paddingH = (parseFloat(style.paddingLeft || '0') || 0) + (parseFloat(style.paddingRight || '0') || 0);
      const contentH = Math.max(0, rect.height - paddingV - 1); // 1px 安全マージン
      const contentW = Math.max(0, rect.width - paddingH - 1);
      const rows = Math.max(1, Math.floor(contentH / lineHPx));
      const cols = Math.max(8, Math.floor(contentW / chWPx));

      setMeasuredRows(rows);
      setMeasuredCols(cols);

      // バックエンドへ通知（内部計算とスナップショット整合のため）
      resizeViewport(rows, cols).catch(() => {/* 非致命 */});
    };

    measure();
    const ro = new ResizeObserver(() => measure());
    ro.observe(el);
    return () => ro.disconnect();
  }, [editorRef]);

  const viewportHeight = Math.max(1, measuredRows);
  // 表示上のカーソル行インデックス（EOF+1 を仮想行として扱う）
  const displayCursorIndex = useMemo(() => {
    if (bufferLines.length === 0) return 0;
    const lastIdx = Math.max(0, bufferLines.length - 1);
    const lastLen = bufferLines[lastIdx]?.length ?? 0;
    const isAtVirtualEOF = cursorLine === lastIdx && cursorColumn >= lastLen && bufferLines[lastIdx] !== '';
    return isAtVirtualEOF ? bufferLines.length : cursorLine;
  }, [bufferLines, cursorLine, cursorColumn]);

  const visibleStart = useMemo(() => {
    // 末尾に 1 行ぶんの余白（仮想行）を加味してスクロール可能範囲を計算
    const totalWithPadding = Math.max(1, bufferLines.length + 1);
    const maxStart = Math.max(0, totalWithPadding - viewportHeight);
    let start = Math.min(Math.max(0, topLine), maxStart);

    // カーソル表示行が確実に見えるように start を補正
    const lastVisible = start + viewportHeight - 1;
    if (displayCursorIndex < start) {
      start = displayCursorIndex;
    } else if (displayCursorIndex > lastVisible) {
      start = Math.min(maxStart, displayCursorIndex - (viewportHeight - 1));
    }

    return Math.min(maxStart, Math.max(0, start));
  }, [bufferLines, topLine, viewportHeight, displayCursorIndex]);
  const visibleLines = useMemo(() => {
    const result: Array<{ content: string; index: number | null; highlights?: Array<{ start: number; end: number; current: boolean }> }> = [];
    for (let offset = 0; offset < viewportHeight; offset += 1) {
      const actualIndex = visibleStart + offset;
      if (actualIndex < bufferLines.length) {
        // 該当行のハイライトを抽出
        const lineHighlights = (snapshot?.highlights || [])
          .filter((h) => h.line === actualIndex && h.kind === 'search')
          .map((h) => ({ start: h.startColumn, end: h.endColumn, current: !!h.isCurrent }))
          .sort((a, b) => a.start - b.start);
        result.push({ content: bufferLines[actualIndex], index: actualIndex, highlights: lineHighlights });
      } else {
        result.push({ content: '', index: null });
      }
    }
    return result;
  }, [bufferLines, visibleStart, viewportHeight, snapshot]);

  const lineCount = useMemo(() => {
    if (!snapshot) {
      return 0;
    }
    return Math.max(1, bufferLines.length);
  }, [snapshot, bufferLines.length]);

  const lineNumberDigits = useMemo(() => {
    const effective = Math.max(1, lineCount);
    return Math.max(3, String(effective).length);
  }, [lineCount]);

  const lineNumberWidth = useMemo(() => `${lineNumberDigits + 1}ch`, [lineNumberDigits]);

  const formatLineNumber = useMemo(() => {
    return (index: number | null) => {
      if (index === null) {
        return '';
      }
      return (index + 1).toString().padStart(lineNumberDigits, ' ');
    };
  }, [lineNumberDigits]);

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

    // I-search の表示（最優先で先頭に挿入）
    const lines: Array<{
      key: string;
      type: 'prompt' | 'info' | 'error';
      prompt?: string;
      input?: string;
      content?: string;
      suffix?: string;
      suffixClass?: 'info' | 'error';
    }> = [];

    const search = snapshot.searchUi;
    if (search && typeof search.pattern === 'string') {
      const isError = search.status === 'not-found';
      const label = search.promptLabel && search.promptLabel.trim().length > 0
        ? search.promptLabel
        : (search.direction === 'backward' ? 'I-search backward' : 'I-search');
      const prompt = `${label}: `;
      const input = search.pattern.length > 0 ? search.pattern : '\u00a0';
      const suffix =
        typeof search.totalMatches === 'number'
          ? ` (${typeof search.currentMatch === 'number' ? search.currentMatch : 0}/${search.totalMatches})`
          : undefined;
      lines.push({
        key: 'isearch',
        // 常にプロンプト形式で表示し、件数はサフィックスに出す
        type: 'prompt',
        prompt,
        input,
        suffix,
        suffixClass: isError ? 'error' : 'info',
      });
      if (search.message && search.message.trim().length > 0) {
        lines.push({ key: 'isearch-msg', type: isError ? 'error' : 'info', content: search.message });
      }
    }

    const restLines: Array<{
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
      restLines.push({
        key: 'prompt',
        type: 'prompt',
        prompt,
        input: input.length > 0 ? input : '\u00a0',
      });

      if (mode === 'goto-line' && statusMessage) {
        restLines.push({
          key: 'goto-status',
          type: 'info',
          content: statusMessage,
        });
      }
    } else if (mode === 'error') {
      if (statusMessage) {
        restLines.push({
          key: 'minibuffer-error',
          type: 'error',
          content: statusMessage,
        });
      }
    } else if (mode === 'info' && statusMessage) {
      restLines.push({
        key: 'minibuffer-info',
        type: 'info',
        content: statusMessage,
      });
    }

    if (lines.length === 0 && restLines.length === 0) {
      const message = globalError ?? globalInfo ?? statusMessage;
      if (message) {
        restLines.push({
          key: 'fallback-message',
          type: globalError ? 'error' : 'info',
          content: message,
        });
      }
    } else if (globalError) {
      restLines.push({
        key: 'global-error',
        type: 'error',
        content: globalError,
      });
    } else if (globalInfo) {
      restLines.push({
        key: 'global-info',
        type: 'info',
        content: globalInfo,
      });
    }

    if (lines.length === 0 && restLines.length === 0) {
      return [
        {
          key: 'inactive',
          type: 'info' as const,
          content: '\u00a0',
        },
      ];
    }

    return [...lines, ...restLines];
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
      previousDisplayIndexRef.current = displayCursorIndex;
      return;
    }

    const prev = previousDisplayIndexRef.current;
    const delta = displayCursorIndex - prev;

    // 大きく下方向へ移動（M-> など）した場合は末尾に合わせてスクロール
    let block: ScrollLogicalPosition = 'nearest';
    if (delta >= 1) {
      block = 'end';
    } else if (delta <= -1) {
      block = 'start';
    }

    activeLineRef.current.scrollIntoView({ block, inline: 'nearest' });

    previousDisplayIndexRef.current = displayCursorIndex;
  }, [displayCursorIndex, bufferLineCount]);

  activeLineRef.current = null;
  firstVisibleLineRef.current = null;

  function renderWithHighlights(text: string, highlights: Array<{ start: number; end: number; current: boolean }>): (string | JSX.Element)[] {
    if (!highlights || highlights.length === 0) {
      return [text || '\u00a0'];
    }
    const parts: (string | JSX.Element)[] = [];
    let cursor = 0;
    highlights.forEach((h, idx) => {
      const start = Math.max(0, Math.min(h.start, text.length));
      const end = Math.max(start, Math.min(h.end, text.length));
      if (cursor < start) {
        parts.push(text.slice(cursor, start));
      }
      const className = h.current ? 'editor-surface__highlight editor-surface__highlight--current' : 'editor-surface__highlight';
      parts.push(
        <span key={`hl-${idx}-${start}-${end}`} className={className}>
          {text.slice(start, end)}
        </span>
      );
      cursor = end;
    });
    if (cursor < text.length) {
      parts.push(text.slice(cursor));
    }
    if (parts.length === 0) return ['\u00a0'];
    return parts;
  }
  return (
    <div className="app">
      <div className="app__minibuffer">
        {minibufferLines.map((line) => {
          if (line.type === 'prompt') {
            return (
              <div key={line.key} className="minibuffer__line">
                <span className="minibuffer__prompt">{line.prompt ?? ''}</span>
                <span className="minibuffer__input">{line.input ?? '\u00a0'}</span>
                {('suffix' in line) && (line as any).suffix ? (
                  <span
                    className={`${(line as any).suffixClass === 'error' ? 'minibuffer__error' : 'minibuffer__message'} minibuffer__suffix`}
                  >
                    {(line as any).suffix}
                  </span>
                ) : null}
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
              const displayIndex = visibleStart + index;
              // 実在行: インデックス一致でアクティブ
              const isRealActive = actualIndex !== null && actualIndex === cursorLine;
              // 仮想行: 最終行の末尾（改行なしEOF）にカーソルがある場合に表示
              const lastIdx = Math.max(0, bufferLines.length - 1);
              const lastLen = bufferLines.length > 0 ? bufferLines[lastIdx].length : 0;
              const isAtVirtualEOF =
                bufferLines.length > 0 && cursorLine === lastIdx && cursorColumn >= lastLen && bufferLines[lastIdx] !== '';
              const isEmptyBufferPhantom = bufferLines.length === 0 && cursorLine === 0 && displayIndex === 0;
              const isPhantomActive =
                actualIndex === null && (isAtVirtualEOF || isEmptyBufferPhantom) && displayIndex === bufferLines.length;
              const isActive = isRealActive || isPhantomActive;
              const isPhantomLine = actualIndex === null;
              const lineNumberText = isPhantomLine ? '' : formatLineNumber(actualIndex);

              const content = line.content;
              const safeColumn = Math.min(cursorColumn, content.length);
              const before = content.slice(0, safeColumn);
              // カーソル位置の文字を消さずに表示するため、
              // カーソルスパン自体には文字を入れず、
              // 残りのテキストはカーソル位置からそのまま描画する
              const after = content.slice(safeColumn);

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
                    <span
                      className={`editor-surface__gutter ${isPhantomLine ? 'editor-surface__gutter--phantom' : ''}`}
                      style={{ width: lineNumberWidth }}
                    >
                      {lineNumberText}
                    </span>
                    <span>
                      {renderWithHighlights(content || '\u00a0', line.highlights || [])}
                    </span>
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
                  <span
                    className={`editor-surface__gutter ${isPhantomLine ? 'editor-surface__gutter--phantom' : ''}`}
                    style={{ width: lineNumberWidth }}
                  >
                    {lineNumberText}
                  </span>
                  <span>
                    {renderWithHighlights(before, line.highlights || [])}
                  </span>
                  <span className="editor-surface__cursor" aria-hidden="true"></span>
                  <span>
                    {renderWithHighlights(after || '', (line.highlights || []).map(h => ({ start: h.start - safeColumn, end: h.end - safeColumn, current: h.current })).filter(h => h.end > 0))}
                  </span>
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
