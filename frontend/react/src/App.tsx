import { useEffect, useState } from 'react';

export function App() {
  const [message, setMessage] = useState('Tauri GUI は準備中です');

  useEffect(() => {
    // TODO: バックエンド連携時に初期化処理を追加する
    setMessage('Tauri GUI の実装タスクを開始してください');
  }, []);

  return (
    <div style={{ padding: '1rem', fontFamily: 'system-ui' }}>
      <h1>altre (Tauri プロトタイプ)</h1>
      <p>{message}</p>
      <p>React フロントエンドの雛形です。バックエンド API が実装され次第、UI を拡張してください。</p>
    </div>
  );
}
