//! Single-run orchestration: load eval + tasks + skill, run trials,
//! collect graders, write JSONL.

use crate::config::{find_repo_root, load_project_config};
use crate::executor::{ExecutorOptions, run_claude};
use crate::graders::{self, GradingContext, output_contains_check};
use crate::jsonl;
use crate::self_report;
use crate::skill::{Skill, load_skill};
use crate::types::{
    EvalConfig, Grader, GraderResult, PhaseStatus, ProjectConfig, SelfReport, Task, TaskResult,
    TaskTrial,
};
use anyhow::{Context, Result, anyhow};
use futures::stream::{FuturesUnordered, StreamExt};
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Semaphore;

pub struct LoadedEval {
    pub repo_root: PathBuf,
    pub project: ProjectConfig,
    pub eval: EvalConfig,
    pub eval_dir: PathBuf,
    pub tasks: Vec<Task>,
    pub skill: Skill,
}

pub fn load(eval_path: &Path) -> Result<LoadedEval> {
    let eval_path = std::fs::canonicalize(eval_path)
        .with_context(|| format!("canonicalizing {}", eval_path.display()))?;
    let eval_dir = eval_path
        .parent()
        .ok_or_else(|| anyhow!("eval has no parent dir"))?
        .to_path_buf();
    let repo_root = find_repo_root(&eval_dir);
    let project = load_project_config(&repo_root)?;

    let eval_text = std::fs::read_to_string(&eval_path)
        .with_context(|| format!("reading {}", eval_path.display()))?;
    let eval: EvalConfig = serde_yaml::from_str(&eval_text)
        .with_context(|| format!("parsing {}", eval_path.display()))?;

    let tasks = load_tasks(&eval_dir, &eval.tasks)?;
    if tasks.is_empty() {
        // A glob like `tasks/*.yaml` matching zero files used to produce
        // a silent 100% pass rate. Fail loud instead.
        return Err(anyhow!(
            "no tasks matched any of these patterns (relative to {}): {:?}",
            eval_dir.display(),
            eval.tasks
        ));
    }

    let skills_root = repo_root.join(&project.paths.skills);
    let skill = load_skill(&skills_root, &eval.skill)?;

    Ok(LoadedEval {
        repo_root,
        project,
        eval,
        eval_dir,
        tasks,
        skill,
    })
}

fn load_tasks(eval_dir: &Path, patterns: &[String]) -> Result<Vec<Task>> {
    let mut paths: Vec<PathBuf> = Vec::new();
    for pat in patterns {
        let full = if Path::new(pat).is_absolute() {
            pat.to_string()
        } else {
            eval_dir.join(pat).to_string_lossy().into_owned()
        };
        let mut hits: Vec<PathBuf> = glob::glob(&full)
            .with_context(|| format!("invalid glob {full}"))?
            .filter_map(Result::ok)
            .collect();
        hits.sort();
        if hits.is_empty() && Path::new(&full).is_file() {
            hits.push(PathBuf::from(&full));
        }
        paths.extend(hits);
    }
    paths.sort();
    paths.dedup();

    let mut out = Vec::with_capacity(paths.len());
    for p in paths {
        let text =
            std::fs::read_to_string(&p).with_context(|| format!("reading task {}", p.display()))?;
        let task: Task =
            serde_yaml::from_str(&text).with_context(|| format!("parsing {}", p.display()))?;
        out.push(task);
    }
    Ok(out)
}

#[derive(Debug, Clone, Copy)]
pub struct RunOptions<'a> {
    /// When set, only run the task with this id.
    pub only_task: Option<&'a str>,
    /// Override the model from CLI; takes precedence over eval/project.
    pub model_override: Option<&'a str>,
    /// Iteration number — passed through to JSONL filename.
    pub iter: Option<u32>,
    /// Override the skill name. Used by `variant`.
    pub skill_override: Option<&'a str>,
    /// When false, don't write JSONL (used by iterate which writes its own).
    pub write_jsonl: bool,
}

impl<'a> Default for RunOptions<'a> {
    fn default() -> Self {
        Self {
            only_task: None,
            model_override: None,
            iter: None,
            skill_override: None,
            write_jsonl: true,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct EvalSummary {
    pub eval_name: String,
    pub skill: String,
    pub model: String,
    pub tasks: Vec<TaskResult>,
    pub mean_pass_rate: f64,
    pub total_unclear: usize,
    pub duration_ms: u128,
}

pub async fn run_eval(loaded: &LoadedEval, opts: &RunOptions<'_>) -> Result<EvalSummary> {
    let started = Instant::now();
    let model = resolve_model(loaded, opts);
    let run_opts = loaded.eval.options.as_ref();
    let trials = run_opts.and_then(|c| c.trials_per_task).unwrap_or(1).max(1);
    let timeout_s = run_opts
        .and_then(|c| c.timeout_seconds)
        .or(loaded.project.defaults.timeout)
        .unwrap_or(300);
    let parallel = run_opts.and_then(|c| c.parallel).unwrap_or(false);
    let workers = run_opts.and_then(|c| c.workers).unwrap_or(2).max(1);

    let skill = if let Some(name) = opts.skill_override {
        let skills_root = loaded.repo_root.join(&loaded.project.paths.skills);
        load_skill(&skills_root, name)?
    } else {
        loaded.skill.clone()
    };

    let executor_opts = ExecutorOptions {
        model: Some(model.clone()),
        timeout: Duration::from_secs(timeout_s),
        require_self_report: true,
        system_prompt: None,
    };

    let mut tasks: Vec<&Task> = loaded.tasks.iter().collect();
    if let Some(id) = opts.only_task {
        tasks.retain(|t| t.id == id);
        if tasks.is_empty() {
            return Err(anyhow!("no task with id {id}"));
        }
    }

    let jsonl_path = if opts.write_jsonl {
        Some(jsonl::results_path(
            &loaded.repo_root,
            &loaded.project.paths.results,
            &loaded.eval.name,
            opts.iter,
        ))
    } else {
        None
    };

    let global_graders = Arc::new(loaded.eval.graders.clone());
    let skill = Arc::new(skill);
    let exec_opts = Arc::new(executor_opts);

    let results = if parallel {
        run_parallel(tasks, trials, workers, &skill, &global_graders, &exec_opts).await?
    } else {
        run_sequential(tasks, trials, &skill, &global_graders, &exec_opts).await?
    };

    let mean_pass_rate = if results.is_empty() {
        1.0
    } else {
        results.iter().map(|r| r.pass_rate).sum::<f64>() / results.len() as f64
    };
    let total_unclear = results
        .iter()
        .flat_map(|r| r.trials.iter())
        .map(|t| {
            t.self_report
                .as_ref()
                .map(|sr| sr.unclear_points.len())
                .unwrap_or(0)
        })
        .sum();

    let summary = EvalSummary {
        eval_name: loaded.eval.name.clone(),
        skill: skill.name.clone(),
        model,
        tasks: results,
        mean_pass_rate,
        total_unclear,
        duration_ms: started.elapsed().as_millis(),
    };

    if let Some(path) = jsonl_path {
        for tr in &summary.tasks {
            jsonl::append(&path, tr)?;
        }
        jsonl::append(&path, &SummaryRecord::from(&summary))?;
    }

    print_summary(&summary);
    Ok(summary)
}

#[derive(Serialize)]
struct SummaryRecord<'a> {
    record: &'static str,
    eval_name: &'a str,
    skill: &'a str,
    model: &'a str,
    mean_pass_rate: f64,
    total_unclear: usize,
    duration_ms: u128,
}

impl<'a> From<&'a EvalSummary> for SummaryRecord<'a> {
    fn from(s: &'a EvalSummary) -> Self {
        Self {
            record: "summary",
            eval_name: &s.eval_name,
            skill: &s.skill,
            model: &s.model,
            mean_pass_rate: s.mean_pass_rate,
            total_unclear: s.total_unclear,
            duration_ms: s.duration_ms,
        }
    }
}

async fn run_sequential(
    tasks: Vec<&Task>,
    trials: u32,
    skill: &Arc<Skill>,
    global_graders: &Arc<Vec<Grader>>,
    exec_opts: &Arc<ExecutorOptions>,
) -> Result<Vec<TaskResult>> {
    let mut out = Vec::with_capacity(tasks.len());
    let total = tasks.len();
    for (idx, task) in tasks.into_iter().enumerate() {
        println!("[{}/{}] {}", idx + 1, total, task.name);
        let r = run_task(task, trials, skill, global_graders, exec_opts).await?;
        out.push(r);
    }
    Ok(out)
}

async fn run_parallel(
    tasks: Vec<&Task>,
    trials: u32,
    workers: usize,
    skill: &Arc<Skill>,
    global_graders: &Arc<Vec<Grader>>,
    exec_opts: &Arc<ExecutorOptions>,
) -> Result<Vec<TaskResult>> {
    let sem = Arc::new(Semaphore::new(workers));
    let mut futs = FuturesUnordered::new();
    for task in tasks.into_iter() {
        let sem = Arc::clone(&sem);
        let task = task.clone();
        let skill = Arc::clone(skill);
        let global_graders = Arc::clone(global_graders);
        let exec_opts = Arc::clone(exec_opts);
        futs.push(tokio::spawn(async move {
            // Semaphore is owned via Arc for the duration of this task, so
            // `acquire()` can only error if someone closes it — nothing in
            // this crate does.
            let _permit = sem.acquire().await.expect("semaphore never closed");
            println!("→ start: {}", task.name);
            let r = run_task(&task, trials, &skill, &global_graders, &exec_opts).await;
            if r.is_ok() {
                println!("← done : {}", task.name);
            }
            r
        }));
    }
    let mut out = Vec::new();
    while let Some(joined) = futs.next().await {
        out.push(joined.map_err(|e| anyhow!("task join error: {e}"))??);
    }
    Ok(out)
}

async fn run_task(
    task: &Task,
    trials: u32,
    skill: &Arc<Skill>,
    global_graders: &Arc<Vec<Grader>>,
    exec_opts: &Arc<ExecutorOptions>,
) -> Result<TaskResult> {
    let task_started = Instant::now();
    let prompt = compose_prompt(skill, task);
    // Apply the task-level `expected.require_self_report` override. Default
    // is true; setting `false` in YAML now actually suppresses the tail.
    let task_exec_opts = {
        let mut o = (**exec_opts).clone();
        if let Some(false) = task.expected.as_ref().and_then(|e| e.require_self_report) {
            o.require_self_report = false;
        }
        o
    };
    let mut trial_results: Vec<TaskTrial> = Vec::with_capacity(trials as usize);
    for n in 1..=trials {
        let trial_started = Instant::now();
        let output = match run_claude(&prompt, &task_exec_opts).await {
            Ok(s) => s,
            Err(e) => {
                let elapsed = trial_started.elapsed().as_millis();
                println!("  trial {n}/{trials}: executor error: {e}");
                trial_results.push(TaskTrial {
                    trial: n,
                    output: String::new(),
                    self_report: None,
                    graders: vec![crate::types::GraderResult {
                        name: "_executor".to_string(),
                        pass: false,
                        score: 0.0,
                        message: Some(format!("executor: {e}")),
                        duration_ms: elapsed,
                    }],
                    pass_rate: 0.0,
                    duration_ms: elapsed,
                });
                continue;
            }
        };
        let report = self_report::parse(&output);
        let ctx = GradingContext {
            output: &output,
            self_report: report.as_ref(),
            executor_opts: &task_exec_opts,
            prompt: &prompt,
        };

        let mut grader_results = Vec::new();
        let expected = task.expected.as_ref();
        let needles: Vec<String> = expected
            .map(|e| e.output_contains.clone())
            .unwrap_or_default();
        if !needles.is_empty() {
            grader_results.push(output_contains_check(&output, &needles));
        }

        for g in global_graders.iter().chain(task.graders.iter()) {
            grader_results.push(graders::run(g, &ctx).await);
        }

        let passes = grader_results.iter().filter(|r| r.pass).count();
        let pass_rate = if grader_results.is_empty() {
            1.0
        } else {
            passes as f64 / grader_results.len() as f64
        };
        print_trial(n, trials, &grader_results, report.as_ref(), pass_rate);
        trial_results.push(TaskTrial {
            trial: n,
            output,
            self_report: report,
            graders: grader_results,
            pass_rate,
            duration_ms: trial_started.elapsed().as_millis(),
        });
    }

    let pass_rate = if trial_results.is_empty() {
        0.0
    } else {
        trial_results.iter().map(|t| t.pass_rate).sum::<f64>() / trial_results.len() as f64
    };

    Ok(TaskResult {
        task_id: task.id.clone(),
        task_name: task.name.clone(),
        trials: trial_results,
        pass_rate,
        duration_ms: task_started.elapsed().as_millis(),
    })
}

fn compose_prompt(skill: &Skill, task: &Task) -> String {
    format!(
        "You are evaluating the skill named `{name}`. Do NOT auto-load other skills or \
         CLAUDE.md context. Operate strictly on the skill body and the task input below.\n\n\
         --- SKILL: {name} ---\n{description}\n\n{body}\n--- END SKILL ---\n\n\
         --- TASK: {task_name} ---\n{prompt}\n--- END TASK ---\n",
        name = skill.name,
        description = skill.description,
        body = skill.body,
        task_name = task.name,
        prompt = task.inputs.prompt,
    )
}

fn resolve_model(loaded: &LoadedEval, opts: &RunOptions<'_>) -> String {
    if let Some(m) = opts.model_override {
        return m.to_string();
    }
    if let Some(m) = loaded.eval.options.as_ref().and_then(|c| c.model.as_ref()) {
        return m.clone();
    }
    if let Some(m) = loaded.project.defaults.model.as_ref() {
        return m.clone();
    }
    "claude-sonnet-4-6".to_string()
}

fn print_trial(
    n: u32,
    total: u32,
    graders: &[GraderResult],
    report: Option<&SelfReport>,
    pass_rate: f64,
) {
    println!("  -- trial {n}/{total} --");
    let line: String = graders
        .iter()
        .map(|g| {
            let mark = if g.pass { "✓" } else { "✗" };
            format!("{mark} {}", g.name)
        })
        .collect::<Vec<_>>()
        .join("   ");
    println!("    {line}");
    if let Some(r) = report {
        let phases_ok = r.phase_trace.iter().all(|p| p.status == PhaseStatus::Ok);
        println!(
            "    self-report: phases={}, unclear={}, retries={}",
            if phases_ok { "all OK" } else { "not all OK" },
            r.unclear_points.len(),
            r.retries
        );
    }
    println!("    trial pass_rate={:.0}%", pass_rate * 100.0);
}

fn print_summary(s: &EvalSummary) {
    println!(
        "\nAGGREGATE: mean_pass_rate={:.0}%, total_unclear={}, duration={}ms",
        s.mean_pass_rate * 100.0,
        s.total_unclear,
        s.duration_ms,
    );
}
