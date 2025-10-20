import { useEffect, useMemo, useRef, useState } from 'react';
import { useEditor } from './hooks/useEditor';

export function App() {
  const { snapshot, loading, error, handleKeyDown, requestRefresh, requestOpenFile } = useEditor();
  const editorRef = useRef<HTMLDivElement>(null);
  const [openPath, setOpenPath] = useState('');

  useEffect(() => {
    editorRef.current?.focus();
  }, [snapshot]);

  const lines = useMemo(() => snapshot?.buffer.lines ?? [], [snapshot]);
  const cursorLine = snapshot?.buffer.cursor.line ?? 0;
  const cursorColumn = snapshot?.buffer.cursor.column ?? 0;

  const minibufferPrompt = snapshot?.minibuffer.prompt ?? 'M-x';
  const minibufferInput = snapshot?.minibuffer.input ?? '';
  const minibufferMessage = error ?? snapshot?.minibuffer.message ?? null;
  const statusLabel = snapshot?.status.label ?? '---';
  const isDirty = snapshot?.status.isModified ?? false;

  const handleOpenSubmit = async (event: React.FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    if (openPath.trim().length === 0) return;
    await requestOpenFile(openPath.trim());
    setOpenPath('');
  };

  return (
    <div className="app">
      <header className="app__header">
        <span>altre (Tauri プロトタイプ)</span>
        <button type="button" onClick={() => void requestRefresh()}>
          リロード
        </button>
      </header>

      {loading && <div className="app__loading">読み込み中...</div>}
      {error && <div className="app__error">{error}</div>}

      <div className="editor-surface">
        <div
          className="editor-surface__buffer"
          tabIndex={0}
          ref={editorRef}
          onKeyDown={handleKeyDown}
        >
          {lines.map((line, index) => renderLine(line, index, cursorLine, cursorColumn))}
          {lines.length === 0 && <span className="editor-surface__line">(空のバッファ)</span>}
        </div>
      </div>

      <div className="app__minibuffer">
        <span className="minibuffer__prompt">{minibufferPrompt}</span>
        <span className="minibuffer__input">{minibufferInput || '\u00a0'}</span>
        {minibufferMessage && <span className="minibuffer__message">{minibufferMessage}</span>}
        <form onSubmit={handleOpenSubmit} style={{ marginLeft: 'auto' }}>
          <input
            type="text"
            placeholder="ファイルパス"
            value={openPath}
            onChange={(event) => setOpenPath(event.target.value)}
            style={{ background: 'transparent', color: 'inherit', border: '1px solid #333' }}
          />
        </form>
      </div>

      <div className="statusline">
        <span>{statusLabel}</span>
        <span className={isDirty ? 'statusline__dirty' : 'statusline__clean'}>
          {isDirty ? '● 未保存' : '✔ 保存済み'}
        </span>
      </div>
    </div>
  );
}

function renderLine(
  content: string,
  index: number,
  cursorLine: number,
  cursorColumn: number,
): JSX.Element {
  if (index !== cursorLine) {
    return (
      <span key={index} className="editor-surface__line">
        {content || '\u00a0'}
      </span>
    );
  }

  const safeColumn = Math.min(cursorColumn, content.length);
  const before = content.slice(0, safeColumn) || '\u00a0';
  const cursorChar = content.charAt(safeColumn) || '\u00a0';
  const after = content.slice(cursorChar === '\u00a0' ? safeColumn : safeColumn + 1);

  return (
    <span key={index} className="editor-surface__line editor-surface__line--active">
      <span>{before}</span>
      <span className="editor-surface__cursor">{cursorChar}</span>
      <span>{after}</span>
    </span>
  );
}
