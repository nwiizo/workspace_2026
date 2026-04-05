const DEFAULT_CODE = `fn main() {
    let s = String::from("hello");
    let t = s;
    let r = &t;
}
`;
const DIFF_BEFORE = `fn main() {\n    let s = String::from("hello");\n    let t = s;\n}\n`;
const DIFF_AFTER = `fn main() {\n    let s = String::from("hello");\n    let t = s.clone();\n}\n`;

const EVENT_ICONS = {
  Bind: '\u{1F4E6}', Move: '\u{27A1}\u{FE0F}', BorrowStart: '\u{1F517}',
  BorrowEnd: '\u{1F513}', Clone: '\u{1F4CB}', Drop: '\u{1F5D1}\u{FE0F}',
  CompileError: '\u{26D4}'
};

const EVENT_COLORS = {
  Bind: '#9ece6a', Move: '#ff9e64', BorrowStart: '#7aa2f7',
  BorrowEnd: '#7aa2f7', Clone: '#7dcfff', Drop: '#f7768e',
  CompileError: '#f7768e'
};

// i18n UI strings
const I18N = {
  ja: {
    pressAnalyze: '"Analyze" を押して解析を開始',
    noEvents: '所有権イベントが検出されませんでした',
    noIssues: '所有権の問題は検出されませんでした',
    compileOk: 'コンパイル成功！',
    compileFail: 'コンパイル失敗',
    borrowsFrom: 'を借用中',
    lentTo: 'に貸出中',
    movedExpl: '所有権が移動済み。この変数は使用できません',
    ptrHeap: 'ヒープデータ',
    quizTitle: 'エラー予測クイズ',
    quizSub: 'このコードはコンパイルできる？ できない？',
    compilesOk: 'コンパイルOK',
    compileError: 'コンパイルエラー',
    correct: '正解！',
    incorrect: '不正解',
    nextQ: '次の問題',
    score: 'スコア',
    streak: '連続正解',
    diffTitle: 'Before / After 比較',
    compare: '比較',
    noChanges: '所有権の変化なし',
    concepts: {
      Bind: 'Rustでは値には必ず1つの所有者がいます。let束縛で変数が値の所有者になります。',
      Move: '非Copy型の代入は「移動」です。元の変数は無効になり、二重解放（double free）を防ぎます。',
      BorrowStart: '借用は所有権を移さずに値を参照できます。&=共有参照（複数OK）、&mut=排他参照（1つだけ）。',
      BorrowEnd: '参照がスコープを抜けると借用が終了し、所有者が再び自由に使えます。',
      Clone: '.clone()はヒープデータも含めた深いコピーを作ります。コストはかかりますが、所有権問題を回避できます。',
      Drop: 'スコープ終了時に自動的にDropが呼ばれます。宣言の逆順（LIFO）で破棄されます。',
      CompileError: 'コンパイラが所有権ルール違反を検出しました。メモリ安全性を保証するためのエラーです。',
    },
  },
  en: {
    pressAnalyze: 'Press "Analyze" to start',
    noEvents: 'No ownership events detected',
    noIssues: 'No ownership issues detected',
    compileOk: 'Compilation successful!',
    compileFail: 'Compilation failed',
    borrowsFrom: 'borrows from',
    lentTo: 'lent to',
    movedExpl: 'Ownership transferred. This variable can no longer be used',
    ptrHeap: 'heap data',
    quizTitle: 'Error Prediction Quiz',
    quizSub: 'Does this code compile... or not?',
    compilesOk: 'Compiles OK',
    compileError: 'Compile Error',
    correct: 'Correct!',
    incorrect: 'Incorrect',
    nextQ: 'Next Question',
    score: 'Score',
    streak: 'Streak',
    diffTitle: 'Before / After Comparison',
    compare: 'Compare',
    noChanges: 'No ownership changes detected',
    concepts: {
      Bind: 'In Rust, every value has exactly one owner. A let binding makes the variable the owner of the value.',
      Move: 'Assigning a non-Copy type is a "move". The original variable becomes invalid, preventing double free.',
      BorrowStart: 'Borrowing lets you reference a value without taking ownership. &=shared ref (multiple OK), &mut=exclusive ref (one only).',
      BorrowEnd: 'When a reference goes out of scope, the borrow ends and the owner can be used freely again.',
      Clone: '.clone() creates a deep copy including heap data. It has a cost but avoids ownership issues.',
      Drop: 'Drop is called automatically at scope end. Variables are dropped in reverse declaration order (LIFO).',
      CompileError: 'The compiler detected an ownership rule violation. This error ensures memory safety.',
    },
  },
};

class ProgressTracker {
  constructor() { this.key = 'rustsensei_progress'; }
  load() { try { return JSON.parse(localStorage.getItem(this.key)) || {}; } catch { return {}; } }
  save(d) { localStorage.setItem(this.key, JSON.stringify(d)); }
  markComplete(id) { const d = this.load(); d[id] = {completed:true}; this.save(d); }
  isComplete(id) { return this.load()[id]?.completed === true; }
  completedCount() { return Object.values(this.load()).filter(v => v.completed).length; }
  quizStats() { return this.load()._quiz || {score:0,total:0,streak:0,bestStreak:0}; }
  saveQuizStats(s) { const d = this.load(); d._quiz = s; this.save(d); }
}

class RustSenseiApp {
  constructor() {
    this.editorEl = null; this.diffEditorBefore = null; this.diffEditorAfter = null;
    this.steps = []; this.currentStep = -1; this.challenges = [];
    this.currentChallenge = null; this.hintIndex = 0;
    this.playing = false; this.playTimer = null;
    this.progress = new ProgressTracker();
    this.quizQuestions = []; this.quizIndex = 0;
    this.quizStats = this.progress.quizStats();
    this.lang = localStorage.getItem('rustsensei_lang') || 'ja';
    this.t = I18N[this.lang];
  }

  async init() {
    this.initEditor();
    this.bindEvents();
    this.updateLangButton();
    await Promise.all([this.loadChallenges(), this.loadQuizQuestions()]);
    this.updateProgressBadge();
    this.renderQuizStats();
    this.applyLangToUI();
  }

  toggleLang() {
    this.lang = this.lang === 'ja' ? 'en' : 'ja';
    this.t = I18N[this.lang];
    localStorage.setItem('rustsensei_lang', this.lang);
    this.updateLangButton();
    this.applyLangToUI();
    // Re-analyze with new lang if we have steps
    if (this.steps.length > 0) this.analyze();
  }

  updateLangButton() {
    const btn = document.getElementById('btn-lang');
    btn.textContent = this.lang === 'ja' ? 'EN' : 'JA';
    btn.title = this.lang === 'ja' ? 'Switch to English' : '日本語に切替';
  }

  applyLangToUI() {
    document.getElementById('step-desc-text').textContent = this.t.pressAnalyze;
    document.querySelector('.quiz-header h2').textContent = this.t.quizTitle;
    document.querySelector('.quiz-header p').textContent = this.t.quizSub;
    document.getElementById('btn-quiz-ok').textContent = this.t.compilesOk;
    document.getElementById('btn-quiz-error').textContent = this.t.compileError;
    document.getElementById('btn-quiz-next').textContent = this.t.nextQ;
    document.querySelector('.diff-header h2').textContent = this.t.diffTitle;
    document.getElementById('btn-diff-analyze').textContent = this.t.compare;
  }

  initEditor() {
    const c = document.getElementById('editor');
    const ta = document.createElement('textarea');
    ta.id = 'code-editor'; ta.className = 'code-textarea';
    ta.value = DEFAULT_CODE; ta.spellcheck = false;
    c.appendChild(ta); this.editorEl = ta;
    ta.addEventListener('keydown', e => {
      if (e.key === 'Tab') { e.preventDefault(); const s=ta.selectionStart; ta.value=ta.value.substring(0,s)+'    '+ta.value.substring(ta.selectionEnd); ta.selectionStart=ta.selectionEnd=s+4; }
    });
  }

  initDiffEditors() {
    if (this.diffEditorBefore) return;
    const mk = (parent, val) => { const ta = document.createElement('textarea'); ta.className='code-textarea'; ta.value=val; ta.spellcheck=false; parent.appendChild(ta); return ta; };
    this.diffEditorBefore = mk(document.getElementById('diff-editor-before'), DIFF_BEFORE);
    this.diffEditorAfter = mk(document.getElementById('diff-editor-after'), DIFF_AFTER);
  }

  bindEvents() {
    document.querySelectorAll('.tab').forEach(t => t.addEventListener('click', () => this.switchMode(t.dataset.mode)));
    const on = (id, fn) => document.getElementById(id).addEventListener('click', fn);
    on('btn-analyze', () => this.analyze()); on('btn-compile', () => this.compile()); on('btn-suggest', () => this.suggest());
    on('btn-first', () => this.goToStep(0)); on('btn-prev', () => this.goToStep(this.currentStep-1));
    on('btn-next', () => this.goToStep(this.currentStep+1)); on('btn-last', () => this.goToStep(this.steps.length-1));
    on('btn-play', () => this.togglePlay());
    document.getElementById('challenge-select').addEventListener('change', e => this.selectChallenge(e.target.value));
    on('btn-hint', () => this.showHint()); on('btn-solution', () => this.showSolution()); on('btn-mark-done', () => this.markChallengeComplete());
    on('btn-quiz-ok', () => this.submitQuiz(true)); on('btn-quiz-error', () => this.submitQuiz(false)); on('btn-quiz-next', () => this.nextQuizQuestion());
    on('btn-diff-analyze', () => this.analyzeDiff());
    on('btn-lang', () => this.toggleLang());
    document.addEventListener('keydown', e => {
      if (e.target.tagName === 'TEXTAREA' || e.target.tagName === 'INPUT' || e.target.tagName === 'SELECT') return;
      if (e.key==='ArrowRight'||e.key==='n') this.goToStep(this.currentStep+1);
      if (e.key==='ArrowLeft'||e.key==='p') this.goToStep(this.currentStep-1);
      if (e.key===' ') { e.preventDefault(); this.togglePlay(); }
    });
  }

  switchMode(m) {
    document.querySelectorAll('.tab').forEach(t => t.classList.toggle('active', t.dataset.mode===m));
    document.querySelectorAll('.mode-content').forEach(c => c.classList.toggle('active', c.id===`mode-${m}`));
    if (m==='diff') this.initDiffEditors();
    if (m==='quiz' && this.quizQuestions.length>0) this.showQuizQuestion();
  }

  getSource() { return this.editorEl.value; }
  setSource(c) { this.editorEl.value = c; }

  // --- API ---
  async analyze() {
    const r = await fetch('/api/analyze', {method:'POST', headers:{'Content-Type':'application/json'}, body:JSON.stringify({source:this.getSource(), lang:this.lang})});
    const d = await r.json();
    if (d.error) { this.showStatus(d.error, false); return; }
    this.steps = d.steps||[]; this.currentStep = -1;
    this.buildTimeline(); this.updateEventLog();
    if (d.has_error) this.showStatus(d.error_message||'Analysis error', false); else this.hideStatus();
    if (this.steps.length>0) this.goToStep(0);
    else { this.updateStepIndicator(); this.renderMemory({variables:[],memory:[]}); document.getElementById('step-desc-text').textContent=this.t.noEvents; }
  }

  async compile() {
    const r = await fetch('/api/compile', {method:'POST', headers:{'Content-Type':'application/json'}, body:JSON.stringify({source:this.getSource()})});
    const d = await r.json();
    if (d.error) { this.showStatus(d.error, false); return; }
    if (d.success) this.showStatus(this.t.compileOk, true);
    else this.showStatus((d.diagnostics||[]).filter(x=>x.level==='error').map(x=>`${x.message}${x.line?` (line ${x.line})`:''}`).join('\n')||this.t.compileFail, false);
  }

  async suggest() {
    const r = await fetch('/api/suggest', {method:'POST', headers:{'Content-Type':'application/json'}, body:JSON.stringify({source:this.getSource()})});
    const d = await r.json(); this.renderSuggestions(d);
  }

  async loadChallenges() {
    try {
      const r = await fetch('/api/challenges'); this.challenges = await r.json();
      const sel = document.getElementById('challenge-select');
      for (const c of this.challenges) { const o=document.createElement('option'); o.value=c.id; o.textContent=`Lv.${c.level} - ${c.title}${this.progress.isComplete(c.id)?' [done]':''}`; sel.appendChild(o); }
    } catch {}
  }

  async loadQuizQuestions() { try { this.quizQuestions = await (await fetch('/api/quiz/questions')).json(); } catch {} }

  async selectChallenge(id) {
    const info = document.getElementById('challenge-info');
    if (!id) { info.classList.add('hidden'); this.currentChallenge=null; this.setSource(DEFAULT_CODE); return; }
    try {
      const c = await (await fetch(`/api/challenges/${id}`)).json();
      this.currentChallenge=c; this.hintIndex=0;
      document.getElementById('challenge-title').textContent=c.title;
      document.getElementById('challenge-level').textContent=`Lv.${c.level}`;
      document.getElementById('challenge-description').textContent=c.description;
      const hc=document.getElementById('hints-container'); hc.classList.add('hidden'); hc.innerHTML='';
      const cb=document.getElementById('challenge-complete-badge'); cb.classList.toggle('hidden', !this.progress.isComplete(id));
      info.classList.remove('hidden'); this.setSource(c.initial_code.trim());
    } catch { info.classList.add('hidden'); }
  }

  showHint() {
    if (!this.currentChallenge) return;
    const c=document.getElementById('hints-container'); c.classList.remove('hidden');
    if (this.hintIndex<this.currentChallenge.hints.length) {
      const d=document.createElement('div'); d.className='hint-item'; d.textContent=`Hint ${this.hintIndex+1}: ${this.currentChallenge.hints[this.hintIndex]}`;
      c.appendChild(d); this.hintIndex++;
    }
  }

  showSolution() { if (this.currentChallenge) this.setSource(this.currentChallenge.solution_code.trim()); }

  markChallengeComplete() {
    if (!this.currentChallenge) return;
    this.progress.markComplete(this.currentChallenge.id);
    document.getElementById('challenge-complete-badge').classList.remove('hidden');
    this.updateProgressBadge();
    for (const o of document.getElementById('challenge-select').options)
      if (o.value===this.currentChallenge.id && !o.textContent.includes('[done]')) o.textContent+=' [done]';
  }

  updateProgressBadge() { document.getElementById('progress-badge').textContent=`${this.progress.completedCount()}/${this.challenges.length}`; }

  // --- Suggestions ---
  renderSuggestions(data) {
    const p=document.getElementById('suggestions-panel'), l=document.getElementById('suggestions-list'); l.innerHTML='';
    if (!data.suggestions?.length) { p.classList.add('hidden'); this.showStatus(this.t.noIssues, true); return; }
    for (const s of data.suggestions) {
      const c=document.createElement('div'); c.className='suggestion-card';
      c.innerHTML=`<span class="suggestion-strategy strategy-${s.strategy}">${s.strategy}</span><div class="suggestion-title">${s.title}</div><div class="suggestion-desc">${s.description}</div><div class="suggestion-tradeoff">${s.trade_off}</div>`;
      if (s.fixed_code) { c.addEventListener('click', () => { this.setSource(s.fixed_code); this.analyze(); }); c.title='Click to apply'; }
      l.appendChild(c);
    }
    p.classList.remove('hidden');
  }

  // --- Quiz ---
  showQuizQuestion() {
    if (this.quizIndex>=this.quizQuestions.length) this.quizIndex=0;
    const q=this.quizQuestions[this.quizIndex];
    document.getElementById('quiz-code').textContent=q.code;
    document.getElementById('quiz-buttons').classList.remove('hidden');
    for (const id of ['quiz-result','quiz-explanation','btn-quiz-next']) document.getElementById(id).classList.add('hidden');
    this.renderQuizStats();
  }

  async submitQuiz(pred) {
    const q=this.quizQuestions[this.quizIndex];
    const res = await (await fetch('/api/quiz/check', {method:'POST', headers:{'Content-Type':'application/json'}, body:JSON.stringify({question_id:q.id, prediction:pred})})).json();
    this.quizStats.total++;
    if (res.correct) { this.quizStats.score++; this.quizStats.streak++; if(this.quizStats.streak>this.quizStats.bestStreak) this.quizStats.bestStreak=this.quizStats.streak; } else this.quizStats.streak=0;
    this.progress.saveQuizStats(this.quizStats);
    const re=document.getElementById('quiz-result'); re.textContent=res.correct?this.t.correct:`${this.t.incorrect} (${res.expected?this.t.compilesOk:this.t.compileError})`; re.className=`quiz-result ${res.correct?'correct':'incorrect'}`; re.classList.remove('hidden');
    const ex=document.getElementById('quiz-explanation'); ex.textContent=res.explanation; ex.classList.remove('hidden');
    document.getElementById('quiz-buttons').classList.add('hidden'); document.getElementById('btn-quiz-next').classList.remove('hidden');
    this.renderQuizStats();
  }

  nextQuizQuestion() { this.quizIndex++; this.showQuizQuestion(); }
  renderQuizStats() { document.getElementById('quiz-score').textContent=this.quizStats.score; document.getElementById('quiz-total').textContent=this.quizStats.total; document.getElementById('quiz-streak').textContent=this.quizStats.streak; }

  // --- Diff ---
  async analyzeDiff() {
    const d = await (await fetch('/api/diff', {method:'POST', headers:{'Content-Type':'application/json'}, body:JSON.stringify({before:this.diffEditorBefore.value, after:this.diffEditorAfter.value})})).json();
    this.renderDiffResult(d);
  }

  renderDiffResult(d) {
    const ch=document.getElementById('diff-changes'); ch.innerHTML='';
    if (d.changes?.length) {
      for (const c of d.changes) { const i=document.createElement('div'); i.className='diff-change-item'; i.innerHTML=`<span class="diff-change-icon diff-change-${c.change_type}">${c.change_type.replace(/([A-Z])/g,' $1').trim()}</span><span>${c.description}</span>`; ch.appendChild(i); }
    } else { ch.innerHTML='<div style="padding:0.5rem;color:var(--text-muted);">No changes</div>'; }
    ch.classList.remove('hidden');
    this.renderDiffLog('diff-log-before', d.before); this.renderDiffLog('diff-log-after', d.after);
  }

  renderDiffLog(id, a) {
    const el=document.getElementById(id); el.innerHTML='';
    if (!a?.steps) return;
    for (const s of a.steps) { const e=document.createElement('div'); e.className='log-entry'; e.innerHTML=`<span class="log-icon">${EVENT_ICONS[s.event.type]||''}</span><span class="log-step-num">#${s.index+1}</span><span class="${this.eventLogClass(s.event.type)}">${s.description}</span>`; el.appendChild(e); }
  }

  // --- Timeline ---
  buildTimeline() {
    const dots=document.getElementById('step-timeline-dots'); dots.innerHTML='';
    for (let i=0; i<this.steps.length; i++) {
      const d=document.createElement('div'); d.className='timeline-dot'; d.title=`Step ${i+1}: ${this.steps[i].description.substring(0,40)}`;
      d.style.background = EVENT_COLORS[this.steps[i].event.type]||'#3b4261';
      d.addEventListener('click', () => this.goToStep(i)); dots.appendChild(d);
    }
  }

  updateTimeline() {
    const pct = this.steps.length>0 ? ((this.currentStep+1)/this.steps.length)*100 : 0;
    document.getElementById('step-timeline-bar').style.width=`${pct}%`;
    document.querySelectorAll('.timeline-dot').forEach((d,i) => {
      d.classList.toggle('passed', i<this.currentStep);
      d.classList.toggle('current', i===this.currentStep);
    });
  }

  // --- Step Navigation ---
  goToStep(i) {
    if (!this.steps.length) return;
    i = Math.max(0, Math.min(i, this.steps.length-1));
    this.currentStep = i;
    const step = this.steps[i];
    this.updateStepIndicator();
    this.updateTimeline();

    // Event icon + description + concept tip
    const icon = document.getElementById('step-event-icon');
    icon.textContent = EVENT_ICONS[step.event.type]||'';
    icon.className = `step-event-icon event-${step.event.type}`;
    document.getElementById('step-desc-text').textContent = step.description;
    document.getElementById('step-desc-line').textContent = step.source_line>0 ? `L${step.source_line}` : '';

    const concept = document.getElementById('step-concept');
    const tip = this.t.concepts[step.event.type];
    if (tip) { concept.textContent = tip; concept.classList.remove('hidden'); }
    else { concept.classList.add('hidden'); }

    this.highlightLine(step.source_line);
    this.renderMemory(step);
    this.drawArrows(step);
    this.highlightLogEntry(i);
  }

  togglePlay() {
    const b=document.getElementById('btn-play');
    if (this.playing) { this.playing=false; clearInterval(this.playTimer); b.textContent='Play'; b.classList.remove('playing'); }
    else { if(this.currentStep>=this.steps.length-1) this.goToStep(0); this.playing=true; b.textContent='Stop'; b.classList.add('playing');
      this.playTimer=setInterval(() => { if(this.currentStep>=this.steps.length-1){this.togglePlay();return;} this.goToStep(this.currentStep+1); }, 1200); }
  }

  updateStepIndicator() { const t=this.steps.length; document.getElementById('step-indicator').textContent=`Step ${t>0?this.currentStep+1:0} / ${t}`; }

  highlightLine(n) {
    if (!this.editorEl||n<=0) return;
    const lines=this.editorEl.value.split('\n'); if(n>lines.length) return;
    let s=0; for(let i=0;i<n-1;i++) s+=lines[i].length+1;
    // Only change selection, don't steal focus from other elements
    if (document.activeElement !== this.editorEl) return;
    this.editorEl.setSelectionRange(s, s+lines[n-1].length);
  }

  // --- Memory Visualization ---
  renderMemory(step) {
    const sv=document.getElementById('stack-view'), hv=document.getElementById('heap-view');
    sv.innerHTML=''; hv.innerHTML='';
    const empty = '<div style="color:var(--text-muted);font-size:0.75rem;padding:0.5rem;text-align:center;">Empty</div>';
    if (!step.variables?.length) { sv.innerHTML=empty; hv.innerHTML=empty; return; }

    const stackVars = step.variables.filter(v=>v.memory==='Stack');
    const heapVars = step.variables.filter(v=>v.memory==='Heap');

    for (const v of stackVars) sv.appendChild(this.createVarCard(v, step));
    for (const v of heapVars) hv.appendChild(this.createVarCard(v, step));
    if (!sv.hasChildNodes()) sv.innerHTML=empty;
    if (!hv.hasChildNodes()) hv.innerHTML=empty;
  }

  createVarCard(v, step) {
    const card = document.createElement('div');
    card.className = `var-card ${this.statusClass(v.status)}`;
    card.dataset.varName = v.name;

    // Check if this variable is the subject of the current event
    const evt = step?.event;
    if (evt) {
      const isSubject = (evt.variable===v.name || evt.from===v.name || evt.to===v.name);
      if (isSubject) card.classList.add('highlight');
    }

    const nameEl = document.createElement('span'); nameEl.className='var-name'; nameEl.textContent=v.name;
    const typeEl = document.createElement('span'); typeEl.className='var-type'; typeEl.textContent=v.type_name;
    const badge = document.createElement('span'); badge.className=`var-status-badge ${this.badgeClass(v.status)}`; badge.textContent=this.statusLabel(v.status);
    const valEl = document.createElement('span'); valEl.className='var-value'; valEl.textContent=v.value_hint;

    card.append(nameEl, typeEl, badge, valEl);

    // Pointer diagram for heap types
    if (v.memory === 'Heap' && v.status !== 'Moved' && v.status !== 'Dropped') {
      const ptr = document.createElement('div'); ptr.className='ptr-diagram';
      ptr.innerHTML = `<span class="ptr-field">ptr</span><span class="ptr-arrow">\u2192</span><span class="ptr-target">${this.t.ptrHeap}</span>`;
      card.appendChild(ptr);
    }

    if (v.borrows_from) {
      const ind = document.createElement('div'); ind.className='borrow-indicator';
      ind.innerHTML=`<span class="arrow-icon">\u{1F517}</span> <strong>${v.borrows_from}</strong> ${this.t.borrowsFrom}`;
      card.appendChild(ind);
    }
    if (v.borrowed_by?.length) {
      const ind = document.createElement('div'); ind.className='borrow-indicator';
      ind.innerHTML=`<span class="arrow-icon">\u{1F512}</span> <strong>${v.borrowed_by.join(', ')}</strong> ${this.t.lentTo}`;
      card.appendChild(ind);
    }
    // Moved explanation
    if (v.status === 'Moved') {
      const exp = document.createElement('div'); exp.className='moved-explanation';
      exp.textContent = this.t.movedExpl;
      card.appendChild(exp);
    }
    return card;
  }

  // --- SVG Arrows ---
  drawArrows(step) {
    const svg = document.getElementById('arrow-svg');
    svg.innerHTML = '';
    if (!step.variables?.length) return;

    const container = document.getElementById('memory-arrows');
    const rect = container.getBoundingClientRect();
    svg.setAttribute('width', rect.width);
    svg.setAttribute('height', rect.height);

    const stackView = document.getElementById('stack-view');
    const heapView = document.getElementById('heap-view');

    // Draw arrows from stack vars that own heap data
    for (const v of step.variables) {
      if (v.memory==='Heap' && v.status!=='Moved' && v.status!=='Dropped') {
        const heapCard = heapView.querySelector(`[data-var-name="${v.name}"]`);
        // Find a stack var that references this heap var (via borrows_from)
        const stackRefs = step.variables.filter(sv => sv.memory==='Stack' && sv.borrows_from===v.name);
        for (const ref of stackRefs) {
          const stackCard = stackView.querySelector(`[data-var-name="${ref.name}"]`);
          if (stackCard && heapCard) {
            this.drawArrow(svg, stackCard, heapCard, container, '#7aa2f7');
          }
        }
      }
    }

    // Also draw arrow from heap owner (conceptual: stack pointer -> heap data)
    for (const v of step.variables) {
      if (v.memory==='Heap' && v.status!=='Moved' && v.status!=='Dropped') {
        const heapCard = heapView.querySelector(`[data-var-name="${v.name}"]`);
        if (heapCard && heapCard.getBoundingClientRect().height > 0) {
          // Draw a simple line from left edge to right edge
          const hRect = heapCard.getBoundingClientRect();
          const cRect = container.getBoundingClientRect();
          const y = hRect.top - cRect.top + hRect.height/2;
          const line = document.createElementNS('http://www.w3.org/2000/svg', 'line');
          line.setAttribute('x1', '5'); line.setAttribute('y1', y);
          line.setAttribute('x2', rect.width-5); line.setAttribute('y2', y);
          line.setAttribute('stroke', '#ff9e64'); line.setAttribute('stroke-width', '2');
          line.setAttribute('stroke-dasharray', '4,3'); line.setAttribute('opacity', '0.5');
          svg.appendChild(line);
          // Arrow head
          const tri = document.createElementNS('http://www.w3.org/2000/svg', 'polygon');
          tri.setAttribute('points', `${rect.width-5},${y} ${rect.width-12},${y-4} ${rect.width-12},${y+4}`);
          tri.setAttribute('fill', '#ff9e64'); tri.setAttribute('opacity', '0.7');
          svg.appendChild(tri);
        }
      }
    }
  }

  drawArrow(svg, fromEl, toEl, container, color) {
    const cRect = container.getBoundingClientRect();
    const fRect = fromEl.getBoundingClientRect();
    const tRect = toEl.getBoundingClientRect();
    const x1 = 5, y1 = fRect.top - cRect.top + fRect.height/2;
    const x2 = cRect.width-5, y2 = tRect.top - cRect.top + tRect.height/2;
    const path = document.createElementNS('http://www.w3.org/2000/svg', 'path');
    const mx = cRect.width/2;
    path.setAttribute('d', `M${x1},${y1} C${mx},${y1} ${mx},${y2} ${x2},${y2}`);
    path.setAttribute('stroke', color); path.setAttribute('stroke-width', '2');
    path.setAttribute('fill', 'none'); path.setAttribute('opacity', '0.6');
    svg.appendChild(path);
  }

  statusClass(s) { return {Owned:'status-owned',Moved:'status-moved',BorrowedShared:'status-borrowed-shared',BorrowedMut:'status-borrowed-mut',LiveRef:'status-live-ref',LiveMutRef:'status-live-mut-ref',Dropped:'status-dropped'}[s]||''; }
  badgeClass(s) { return {Owned:'badge-owned',Moved:'badge-moved',BorrowedShared:'badge-borrowed',BorrowedMut:'badge-borrowed',LiveRef:'badge-ref',LiveMutRef:'badge-mut-ref',Dropped:'badge-moved'}[s]||''; }
  statusLabel(s) { return {Owned:'owned',Moved:'moved',BorrowedShared:'borrowed',BorrowedMut:'mut borrowed',LiveRef:'&ref',LiveMutRef:'&mut ref',Dropped:'dropped'}[s]||s; }

  // --- Event Log ---
  updateEventLog() {
    const log=document.getElementById('event-log'); log.innerHTML='';
    for (const s of this.steps) {
      const e=document.createElement('div'); e.className='log-entry'; e.dataset.step=s.index;
      e.innerHTML=`<span class="log-icon">${EVENT_ICONS[s.event.type]||''}</span><span class="log-step-num">#${s.index+1}</span><span class="${this.eventLogClass(s.event.type)}">${s.description}</span>`;
      e.addEventListener('click', () => this.goToStep(s.index)); log.appendChild(e);
    }
  }

  highlightLogEntry(i) {
    document.querySelectorAll('#event-log .log-entry').forEach(e => e.classList.toggle('active', parseInt(e.dataset.step)===i));
    const a=document.querySelector(`#event-log [data-step="${i}"]`); if(a) a.scrollIntoView({block:'nearest'});
  }

  eventLogClass(t) { return {Bind:'log-event-bind',Move:'log-event-move',BorrowStart:'log-event-borrow',BorrowEnd:'log-event-borrow',Clone:'log-event-clone',Drop:'log-event-drop',CompileError:'log-event-error'}[t]||''; }

  showStatus(m,ok) { const e=document.getElementById('compile-status'); e.textContent=m; e.className=`compile-status ${ok?'success':'error'}`; e.classList.remove('hidden'); }
  hideStatus() { document.getElementById('compile-status').classList.add('hidden'); }
}

document.addEventListener('DOMContentLoaded', () => new RustSenseiApp().init());
