(() => {
  if (window.__minagineBikouInjected) return;
  window.__minagineBikouInjected = true;

  const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

  // Wait for a condition to become truthy (returns the resolved value).
  async function waitFor(fn, { timeout = 5000, interval = 80 } = {}) {
    const start = Date.now();
    while (Date.now() - start < timeout) {
      const v = fn();
      if (v) return v;
      await sleep(interval);
    }
    return null;
  }

  // React-compatible value setter (MUI inputs/textareas use React onChange).
  function setReactValue(el, value) {
    const proto = el instanceof HTMLTextAreaElement
      ? HTMLTextAreaElement.prototype
      : HTMLInputElement.prototype;
    const setter = Object.getOwnPropertyDescriptor(proto, "value").set;
    setter.call(el, value);
    el.dispatchEvent(new Event("input", { bubbles: true }));
    el.dispatchEvent(new Event("change", { bubbles: true }));
  }

  // The rows are flex children inside the table area. Each data row contains
  // a date span (txtDate) and a memo button (btnMemo). Header rows have
  // txtDate too, so we look for rows that include btnMemo.
  function getRows() {
    const buttons = document.querySelectorAll('button[data-testid="btnMemo"]');
    const rows = [];
    buttons.forEach((btn) => {
      // Walk up to the row container. Each row contains txtDate + txtWorkStatus.
      let row = btn;
      for (let i = 0; i < 12 && row; i++) {
        if (row.querySelector && row.querySelector('[data-testid="txtDate"]')) {
          break;
        }
        row = row.parentElement;
      }
      if (!row) return;
      // Skip rows we've already captured (same row element).
      if (rows.some((r) => r.row === row)) return;
      rows.push({ row, btn });
    });
    return rows;
  }

  function getRowInfo(row) {
    const dateEl = row.querySelector('[data-testid="txtDate"]');
    const statusEl = row.querySelector('[data-testid="txtWorkStatus"]');
    const workHourEl = row.querySelector('[data-testid="workHour"]');
    return {
      date: dateEl ? dateEl.textContent.trim() : "",
      status: statusEl ? statusEl.textContent.trim() : "",
      workHour: workHourEl ? workHourEl.textContent.trim() : "",
    };
  }

  // Detect whether a memo button represents a row that already has a memo.
  // Empty memo SVG path starts with "M13.834 0.960938..." (pencil only),
  // a filled memo uses a different path "M2.19922 9.86719...". We use SVG
  // path d-attribute prefix as the signal.
  function isMemoFilled(btn) {
    const path = btn.querySelector("svg path");
    if (!path) return false;
    const d = path.getAttribute("d") || "";
    return d.startsWith("M2.19922");
  }

  function hasWork(row) {
    const info = getRowInfo(row);
    if (!info.workHour) return false;
    return /[1-9]/.test(info.workHour);
  }

  // Treat an RGB triple as "red-ish" when red dominates green and blue.
  // Minagine highlights cells that require 備考 with a red foreground/background;
  // we sniff for that without relying on emotion-generated class names.
  function isRedColor(c) {
    if (!c) return false;
    const m = c.match(/rgba?\((\d+),\s*(\d+),\s*(\d+)(?:,\s*([0-9.]+))?\)/);
    if (!m) return false;
    const r = +m[1], g = +m[2], b = +m[3];
    const a = m[4] === undefined ? 1 : +m[4];
    if (a < 0.05) return false;
    return r >= 150 && r - g >= 60 && r - b >= 60;
  }

  // True if any descendant inside the row has red text or red background.
  function isRowRed(row) {
    const candidates = [row, ...row.querySelectorAll("*")];
    for (const el of candidates) {
      const style = getComputedStyle(el);
      if (isRedColor(style.color)) return true;
      if (isRedColor(style.backgroundColor)) return true;
    }
    return false;
  }

  const DIALOG_SELECTOR =
    '.MuiDialog-root, .MuiPopover-root, .MuiModal-root, [role="dialog"]';
  const CANCEL_RE = /キャンセル|閉じる|Cancel|戻る|×|✕/;

  // After clicking btnMemo, locate the popup container, fill the text field
  // and click the save button. We poll for a visible textarea inside any MUI
  // dialog/modal — comparing "before vs after" element sets is unreliable
  // because MUI may keep the dialog mounted across opens.
  function isVisible(el) {
    if (!el || !document.body.contains(el)) return false;
    if (el.offsetParent !== null) return true;
    const cs = getComputedStyle(el);
    return cs.visibility !== "hidden" && cs.display !== "none";
  }

  function findVisibleDialogTextarea() {
    const focused = document.activeElement;
    if (
      focused &&
      focused.tagName === "TEXTAREA" &&
      !focused.readOnly &&
      !focused.disabled &&
      isVisible(focused)
    ) {
      return focused;
    }
    const dialogs = document.querySelectorAll(DIALOG_SELECTOR);
    for (const d of dialogs) {
      if (!isVisible(d)) continue;
      const ta = d.querySelector("textarea:not([readonly]):not([disabled])");
      if (ta && isVisible(ta)) return ta;
    }
    return null;
  }

  async function openAndFillDialog(btn, memoText) {
    btn.click();

    const input = await waitFor(findVisibleDialogTextarea, { timeout: 4500 });
    if (!input) {
      const stuck = Array.from(document.querySelectorAll(DIALOG_SELECTOR)).find(
        (d) => isVisible(d),
      );
      if (stuck) await closeDialog(stuck);
      throw new Error(
        "memo textarea not found inside any visible dialog after click",
      );
    }

    const container =
      input.closest(DIALOG_SELECTOR) ||
      input.closest(".MuiPaper-root") ||
      input.closest("form") ||
      input.parentElement;

    input.focus();
    setReactValue(input, memoText);
    await sleep(150);

    // Set both 開始時刻 / 終了時刻 selects under "PCログ打刻差異の報告" to
    // "自己啓発・研鑽". The dialog has no save button — we always close with
    // the × icon and let the final outer form save commit everything.
    const dialogRoot = container.matches(DIALOG_SELECTOR)
      ? container
      : container.closest(DIALOG_SELECTOR) || container;
    try {
      await selectReason(dialogRoot, "reasonStartSelected", memoText);
      await selectReason(dialogRoot, "reasonEndSelected", memoText);
    } catch (e) {
      console.warn("[minagine-bikou] reason select failed:", e);
    }

    await closeDialog(dialogRoot);
    // Wait for the dialog to fully disappear before moving to the next row.
    await waitFor(
      () =>
        !document.body.contains(input) ||
        input.offsetParent === null ||
        dialogRoot.offsetParent === null,
      { timeout: 4500 },
    );
    await sleep(220);
  }

  // Locate the outer form's primary 保存 button (submit) so we can commit
  // all per-day changes in a single network call at the end of the run.
  function findFormSaveButton() {
    const buttons = Array.from(document.querySelectorAll('button[type="submit"]'));
    const visible = buttons.filter((b) => !b.disabled && b.offsetParent !== null);
    const labeled = visible.find((b) =>
      /保存|更新|登録/.test((b.textContent || "").trim()),
    );
    return labeled || visible[visible.length - 1] || null;
  }

  // Open the labelled MUI Select and click the option whose text matches.
  async function selectReason(scope, nativeName, optionText) {
    const native = scope.querySelector(`input[name="${nativeName}"]`);
    if (!native) return;
    const combobox = native
      .closest(".MuiInputBase-root")
      ?.querySelector('[role="combobox"]');
    if (!combobox) return;

    if ((combobox.textContent || "").trim() === optionText) return;

    combobox.focus();
    combobox.dispatchEvent(
      new MouseEvent("mousedown", { bubbles: true, button: 0 }),
    );
    combobox.click();

    const listbox = await waitFor(
      () => document.querySelector('[role="listbox"]'),
      { timeout: 2000 },
    );
    if (!listbox) return;

    const options = Array.from(listbox.querySelectorAll('[role="option"]'));
    const match =
      options.find((o) => (o.textContent || "").trim() === optionText) ||
      options.find((o) => (o.textContent || "").includes(optionText));
    if (match) {
      match.click();
    } else {
      document.dispatchEvent(
        new KeyboardEvent("keydown", { key: "Escape", bubbles: true }),
      );
    }
    await sleep(150);
  }

  async function closeDialog(dialog) {
    // Prefer the × close icon (SVG-only button at the top of the MUI dialog).
    const buttons = Array.from(dialog.querySelectorAll("button")).filter(
      (b) => !b.disabled && b.offsetParent !== null,
    );
    const closeIcon = buttons.find((b) => {
      const text = (b.textContent || "").trim();
      if (text) return false; // text buttons are not the × close icon
      return b.querySelector("svg") !== null;
    });
    if (closeIcon) {
      closeIcon.click();
      await sleep(180);
      return;
    }
    // Fallback: a labelled cancel/close button, then Escape.
    const labelled = buttons.find((b) =>
      CANCEL_RE.test((b.textContent || "").trim()),
    );
    if (labelled) {
      labelled.click();
      await sleep(180);
      return;
    }
    document.dispatchEvent(
      new KeyboardEvent("keydown", { key: "Escape", bubbles: true }),
    );
    await sleep(180);
  }

  // Auto-fill 開始時刻 / 終了時刻 from the displayed PC-log timestamp when
  // the申請 input is blank. Times are clamped into the 07:00–21:55 window per
  // the user's spec, so a PC log of 06:13 becomes 07:00 and 26:28 becomes
  // 21:55 (since "2628" means just past midnight, i.e. outside the window).
  const TIME_MIN = 700; // 07:00
  const TIME_MAX = 2155; // 21:55

  function clampHHMM(raw) {
    if (!raw || !/^\d{3,4}$/.test(raw)) return null;
    let n = parseInt(raw, 10);
    if (n < TIME_MIN) n = TIME_MIN;
    if (n > TIME_MAX) n = TIME_MAX;
    return String(n).padStart(4, "0");
  }

  // The PC-log timestamp is rendered as a <p> sibling inside the same MUI
  // Stack as the申請 input. An empty paragraph means no PC log for that day.
  function pcLogForInput(input) {
    const stack = input.closest(".MuiStack-root");
    if (!stack) return null;
    const p = stack.querySelector("p");
    if (!p) return null;
    const txt = (p.textContent || "").trim();
    return txt || null;
  }

  // Fire a richer event sequence that mimics real keyboard input. React Hook
  // Form's `register` listens on `input`, `change`, and `blur`; if we only
  // fire `input` once with a generic Event, the form sometimes drops the
  // value (especially on the second input we touch in the same tick).
  async function typeFormValue(input, value) {
    input.dispatchEvent(new MouseEvent("mousedown", { bubbles: true }));
    input.dispatchEvent(new MouseEvent("mouseup", { bubbles: true }));
    input.dispatchEvent(new MouseEvent("click", { bubbles: true }));
    input.focus();
    input.dispatchEvent(new FocusEvent("focus", { bubbles: true }));
    try {
      input.setSelectionRange(0, (input.value || "").length);
    } catch {}
    await sleep(30);

    const proto = HTMLInputElement.prototype;
    const setter = Object.getOwnPropertyDescriptor(proto, "value").set;
    setter.call(input, value);
    input.dispatchEvent(
      new InputEvent("input", {
        data: value,
        inputType: "insertText",
        bubbles: true,
      }),
    );
    input.dispatchEvent(new Event("change", { bubbles: true }));
    await new Promise((r) => requestAnimationFrame(() => requestAnimationFrame(r)));
    input.dispatchEvent(new FocusEvent("blur", { bubbles: true }));
    input.blur();
    await sleep(60);
  }

  // Status strings that indicate the day is a company-defined non-work day
  // (regular off or scheduled holiday-work). We must not auto-fill the申請
  // start/end times for those rows — only the memo/reason fields.
  const NON_WORK_STATUS = new Set(["所休", "所出"]);

  async function ensureTimesFilled(row) {
    const info = getRowInfo(row);
    if (NON_WORK_STATUS.has(info.status)) {
      console.log(
        `[minagine-bikou] skip times for ${info.date} (status=${info.status})`,
      );
      return;
    }
    // React re-renders the row when we mutate the first input, so the second
    // input reference can become detached. Re-resolve every input from the
    // document via the row's data-rowindex to always touch the live element.
    const seed = row.querySelector("input[data-rowindex]");
    if (!seed) return;
    const rowindex = seed.getAttribute("data-rowindex");
    if (!rowindex) return;

    const filled = [];
    for (const testid of ["edtStartTime", "edtEndTime"]) {
      const input = document.querySelector(
        `input[data-rowindex="${rowindex}"][data-testid="${testid}"]`,
      );
      if (!input) continue;
      if (input.disabled || input.readOnly) continue;
      if ((input.value || "").trim() !== "") continue;
      const pc = pcLogForInput(input);
      const value = clampHHMM(pc);
      if (!value) {
        console.log(
          `[minagine-bikou] skip ${testid} rowindex=${rowindex}: pc="${pc}"`,
        );
        continue;
      }
      await typeFormValue(input, value);

      // Re-resolve the live input (React may have swapped the node) and
      // verify the value stuck. If it didn't, retry once with a longer
      // settle window.
      let live = document.querySelector(
        `input[data-rowindex="${rowindex}"][data-testid="${testid}"]`,
      );
      if (live && (live.value || "").trim() !== value) {
        await sleep(200);
        await typeFormValue(live, value);
        live = document.querySelector(
          `input[data-rowindex="${rowindex}"][data-testid="${testid}"]`,
        );
      }
      const final = live ? (live.value || "").trim() : "";
      filled.push(`${testid}=${value}(actual=${final})`);
      await sleep(120);
    }
    if (filled.length) {
      console.log(
        `[minagine-bikou] times filled (row=${rowindex}):`,
        filled.join(", "),
      );
    }
  }

  // Scrape every "MM/DD" pill from the warning banner. The banner lists every
  // day that still needs attention (打刻漏れ / 打刻差異 / 勤怠実績なし …) and
  // disappears once all categories are resolved, so this is the signal we
  // loop against.
  function parseWarningDates() {
    const dates = new Set();
    document.querySelectorAll("span").forEach((s) => {
      const txt = (s.textContent || "").trim();
      const m = txt.match(/^(\d{1,2})\/(\d{1,2})$/);
      if (m) dates.add(parseInt(m[2], 10));
    });
    return dates;
  }

  function rowsForDays(daySet) {
    return getRows().filter(({ row }) => {
      const day = parseInt(getRowInfo(row).date, 10);
      return Number.isFinite(day) && daySet.has(day);
    });
  }

  // After a form save Minagine re-renders the table via React state, so we
  // wait for the DOM to settle before parsing the next warning snapshot.
  async function waitForRerender() {
    // Submit button briefly transitions to disabled while the request is in
    // flight; afterwards the warning banner is reconciled.
    await sleep(800);
    await waitFor(
      () => {
        const submit = findFormSaveButton();
        return submit && !submit.disabled;
      },
      { timeout: 6000 },
    );
    await sleep(400);
  }

  function classifyRows({ skipFilled, onlyWorked, onlyRed }) {
    const rows = getRows();
    const total = rows.length;
    let alreadyFilled = 0;
    let noWork = 0;
    let notRed = 0;
    const targets = [];
    for (const r of rows) {
      const filled = isMemoFilled(r.btn);
      if (skipFilled && filled) {
        alreadyFilled++;
        continue;
      }
      if (onlyRed && !isRowRed(r.row)) {
        notRed++;
        continue;
      }
      if (onlyWorked && !hasWork(r.row)) {
        noWork++;
        continue;
      }
      targets.push(r);
    }
    return { total, alreadyFilled, noWork, notRed, targets };
  }

  chrome.runtime.onMessage.addListener((msg, _sender, sendResponse) => {
    (async () => {
      try {
        if (msg.action === "dryRun") {
          const warningDays = parseWarningDates();
          if (warningDays.size > 0) {
            const targetRows = rowsForDays(warningDays);
            sendResponse({
              mode: "warning",
              total: getRows().length,
              targets: targetRows.length,
              dates: targetRows.map((r) => getRowInfo(r.row).date),
              warningDays: Array.from(warningDays).sort((a, b) => a - b),
            });
            return;
          }
          const { total, alreadyFilled, noWork, notRed, targets } = classifyRows({
            skipFilled: msg.skipFilled,
            onlyWorked: msg.onlyWorked,
            onlyRed: msg.onlyRed,
          });
          sendResponse({
            mode: "fallback",
            total,
            alreadyFilled,
            noWork,
            notRed,
            targets: targets.length,
            dates: targets.map((r) => getRowInfo(r.row).date),
          });
          return;
        }
        if (msg.action === "run") {
          const maxIterations = Math.max(1, msg.maxIterations || 6);
          const autoCommit = msg.autoCommit !== false;
          let totalSuccess = 0;
          let totalFailed = 0;
          const allFailures = [];
          let iterations = 0;
          let lastWarningKey = "";
          let stalled = false;

          while (iterations < maxIterations) {
            const warningDays = parseWarningDates();
            if (warningDays.size === 0) break;

            const key = Array.from(warningDays).sort().join(",");
            if (key === lastWarningKey) {
              // The banner kept the same days as the previous iteration — no
              // progress was made, stop instead of looping forever.
              stalled = true;
              break;
            }
            lastWarningKey = key;
            iterations++;

            const targets = rowsForDays(warningDays);
            if (targets.length === 0) {
              // Days exist in the banner but no matching row visible (month
              // boundary, hidden week filter, …). Bail out gracefully.
              stalled = true;
              break;
            }

            for (const r of targets) {
              const info = getRowInfo(r.row);
              try {
                await ensureTimesFilled(r.row);
                await openAndFillDialog(r.btn, msg.memo);
                totalSuccess++;
                console.log("[minagine-bikou] saved:", info);
                await sleep(250);
              } catch (e) {
                totalFailed++;
                const reason = e?.message || String(e);
                allFailures.push({ date: info.date, reason });
                console.warn("[minagine-bikou] failed:", info, e);
                const open = document.querySelector(DIALOG_SELECTOR);
                if (open) await closeDialog(open);
                await sleep(250);
              }
            }

            if (!autoCommit) break;
            const submit = findFormSaveButton();
            if (!submit) {
              stalled = true;
              break;
            }
            submit.click();
            await waitForRerender();
          }

          const remaining = Array.from(parseWarningDates()).sort((a, b) => a - b);
          sendResponse({
            iterations,
            success: totalSuccess,
            failed: totalFailed,
            failures: allFailures,
            committed: autoCommit && iterations > 0,
            remaining,
            stalled,
          });
          return;
        }
      } catch (e) {
        sendResponse({ error: e?.message || String(e) });
      }
    })();
    return true; // async response
  });
})();
