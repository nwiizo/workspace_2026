const statusEl = document.getElementById("status");
const memoInput = document.getElementById("memo-text");
const skipFilled = document.getElementById("skip-filled");
const onlyWorked = document.getElementById("only-worked");
const onlyRed = document.getElementById("only-red");
const autoCommit = document.getElementById("auto-commit");

function log(msg) {
  statusEl.textContent = msg;
}

async function sendToContent(action) {
  const tabs = await chrome.tabs.query({ active: true, currentWindow: true });
  const tab = tabs[0];
  if (!tab || !tab.url || !tab.url.includes("work.minagine.net")) {
    log("Minagine の勤怠表ページで実行してください。");
    return null;
  }
  try {
    return await chrome.tabs.sendMessage(tab.id, {
      action,
      memo: memoInput.value,
      skipFilled: skipFilled.checked,
      onlyWorked: onlyWorked.checked,
      onlyRed: onlyRed.checked,
      autoCommit: autoCommit.checked,
    });
  } catch (e) {
    log(
      "ページ側スクリプトに接続できませんでした。ページを再読み込みしてからお試しください。\n" +
        (e?.message || ""),
    );
    return null;
  }
}

document.getElementById("dry-run").addEventListener("click", async () => {
  log("対象を確認中…");
  const res = await sendToContent("dryRun");
  if (!res) return;
  const dates = (res.dates || []).join(", ");
  if (res.mode === "warning") {
    const days = (res.warningDays || []).join(", ");
    log(
      `モード: 警告バナー駆動\n` +
        `対象行: ${res.targets} 件 / 全行: ${res.total} 件\n` +
        (days ? `警告日: ${days}\n` : "") +
        (dates ? `対象日: ${dates}` : ""),
    );
    return;
  }
  const breakdown = [
    `既入力: ${res.alreadyFilled}`,
    `赤以外: ${res.notRed}`,
    `勤務なし: ${res.noWork}`,
  ].join(", ");
  log(
    `モード: フォールバック（警告なし）\n` +
      `対象行: ${res.targets} 件 / 全行: ${res.total} 件\n` +
      `(${breakdown})` +
      (dates ? `\n対象日: ${dates}` : ""),
  );
});

document.getElementById("run").addEventListener("click", async () => {
  if (!memoInput.value.trim()) {
    log("備考テキストを入力してください。");
    return;
  }
  log("実行中… ページを操作しないでください。");
  const res = await sendToContent("run");
  if (!res) return;
  if (res.error) {
    log("エラー: " + res.error);
    return;
  }
  let out =
    `反復: ${res.iterations} 回\n` +
    `入力成功: ${res.success} 件 / 失敗: ${res.failed} 件\n`;
  const remaining = Array.isArray(res.remaining) ? res.remaining : [];
  if (remaining.length === 0) {
    out += "→ 警告バナーは消えました";
  } else {
    out += `→ 残り警告日: ${remaining.join(", ")}`;
    if (res.stalled) out += "\n（同じ警告が解消されなかったため停止）";
  }
  if (Array.isArray(res.failures) && res.failures.length) {
    const lines = res.failures
      .slice(0, 5)
      .map((f) => `  ${f.date}日: ${f.reason}`);
    out += "\n" + lines.join("\n");
    if (res.failures.length > 5) out += `\n  ... 他 ${res.failures.length - 5} 件`;
    out += "\nDevTools コンソールで [minagine-bikou] を確認してください。";
  }
  if (!res.committed && res.success > 0)
    out += "\n※ 「保存する」自動クリックがOFFのため手動で確定してください";
  log(out);
});
