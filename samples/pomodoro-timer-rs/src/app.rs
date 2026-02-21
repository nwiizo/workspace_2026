use eframe::egui;
use egui::{
    Align2, Color32, CornerRadius, FontId, Pos2, Rect, Shape, Stroke, Vec2, ViewportCommand,
};
use std::f32::consts::TAU;
use std::sync::Arc;
use std::time::Duration;

use yasume::history::{CompletedTask, History};
use yasume::i18n::{Lang, strings};
use yasume::ipc::{
    self, Command, CommandReceiver, CommandSender, DailyBreakdown, ReportInfo, Response, StatusInfo,
};
use yasume::notification::{
    is_late_night, notify_late_night, notify_overwork, notify_phase_complete,
};
use yasume::timer::{Timer, TimerPhase, TimerStatus};

#[derive(Clone, Copy)]
enum ButtonAction {
    Start,
    Pause,
    Reset,
    Skip,
    Done,
    Close,
}

// -- Colors --
// Non-hovered: whisper-quiet, barely there
const PANEL_BG: Color32 = Color32::from_rgba_premultiplied(10, 10, 14, 100);
// Hovered: solid enough to read and interact
const PANEL_BG_HOVER: Color32 = Color32::from_rgba_premultiplied(14, 14, 18, 210);
const WORK_COLOR: Color32 = Color32::from_rgb(255, 99, 71);
const WORK_COLOR_LATE: Color32 = Color32::from_rgb(255, 183, 77);
const SHORT_BREAK_COLOR: Color32 = Color32::from_rgb(76, 175, 80);
const LONG_BREAK_COLOR: Color32 = Color32::from_rgb(33, 150, 243);
// Two text tiers: hover vs idle
const TEXT_COLOR: Color32 = Color32::from_rgba_premultiplied(255, 255, 255, 220);
const TEXT_COLOR_IDLE: Color32 = Color32::from_rgba_premultiplied(255, 255, 255, 130);
const DIM_TEXT_COLOR: Color32 = Color32::from_rgba_premultiplied(255, 255, 255, 100);
const DIM_TEXT_COLOR_IDLE: Color32 = Color32::from_rgba_premultiplied(255, 255, 255, 55);
const RING_BG_COLOR: Color32 = Color32::from_rgba_premultiplied(255, 255, 255, 18);
const RING_BG_COLOR_IDLE: Color32 = Color32::from_rgba_premultiplied(255, 255, 255, 10);
const INPUT_BG: Color32 = Color32::from_rgba_premultiplied(30, 30, 35, 180);
const DIVIDER_COLOR: Color32 = Color32::from_rgba_premultiplied(255, 255, 255, 20);

// -- Layout --
const PANEL_WIDTH: f32 = 196.0;
const RING_RADIUS: f32 = 50.0;
const RING_THICKNESS: f32 = 3.5;
const RING_CENTER_Y: f32 = 70.0;
const CORNER_RADIUS: u8 = 14;
const MAX_HISTORY_ITEMS: usize = 5;

pub struct PomodoroApp {
    timer: Timer,
    history: History,
    is_hovered: bool,
    task_input: String,
    cmd_rx: CommandReceiver,
    _cmd_tx: CommandSender,
    should_quit: bool,
    lang: Lang,
    editing_time: bool,
    time_edit_buf: String,
    time_edit_needs_focus: bool,
    recent_task_names: Vec<String>,
}

impl PomodoroApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        Self::setup_fonts(&cc.egui_ctx);

        let (cmd_tx, cmd_rx) = ipc::create_channel();
        ipc::start_listener(cmd_tx.clone());
        let history = History::load();
        let recent_task_names = history.recent_task_names(7);
        Self {
            timer: Timer::new(),
            history,
            is_hovered: false,
            task_input: String::new(),
            cmd_rx,
            _cmd_tx: cmd_tx,
            should_quit: false,
            lang: Lang::default(),
            editing_time: false,
            time_edit_buf: String::new(),
            time_edit_needs_focus: false,
            recent_task_names,
        }
    }

    fn setup_fonts(ctx: &egui::Context) {
        const CJK_FONT_PATHS: &[&str] = &[
            "/System/Library/Fonts/ヒラギノ角ゴシック W3.ttc",
            "/System/Library/Fonts/Hiragino Sans GB.ttc",
            "/System/Library/Fonts/PingFang.ttc",
        ];

        let mut fonts = egui::FontDefinitions::default();

        for path in CJK_FONT_PATHS {
            if let Ok(font_data) = std::fs::read(path) {
                fonts.font_data.insert(
                    "cjk".to_owned(),
                    Arc::new(egui::FontData::from_owned(font_data)),
                );
                if let Some(family) = fonts.families.get_mut(&egui::FontFamily::Proportional) {
                    family.push("cjk".to_owned());
                }
                if let Some(family) = fonts.families.get_mut(&egui::FontFamily::Monospace) {
                    family.push("cjk".to_owned());
                }
                break;
            }
        }

        ctx.set_fonts(fonts);
    }

    fn phase_color(&self) -> Color32 {
        match self.timer.phase {
            TimerPhase::Work => {
                if is_late_night() {
                    WORK_COLOR_LATE
                } else {
                    WORK_COLOR
                }
            }
            TimerPhase::ShortBreak => SHORT_BREAK_COLOR,
            TimerPhase::LongBreak => LONG_BREAK_COLOR,
        }
    }

    fn accent_border_color(&self) -> Color32 {
        let c = self.phase_color();
        Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), 35)
    }

    fn draw_ring(&self, painter: &egui::Painter, center: Pos2, idle: bool) {
        let ring_bg = if idle {
            RING_BG_COLOR_IDLE
        } else {
            RING_BG_COLOR
        };
        draw_arc(
            painter,
            center,
            RING_RADIUS,
            0.0,
            TAU,
            Stroke::new(RING_THICKNESS, ring_bg),
        );

        let progress = self.timer.progress();
        if progress > 0.0 {
            let sweep = progress * TAU;
            let color = self.phase_color();

            if idle {
                // Subtle progress arc only (no glow) when not hovered
                let muted = Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 120);
                draw_arc(
                    painter,
                    center,
                    RING_RADIUS,
                    -TAU / 4.0,
                    sweep,
                    Stroke::new(RING_THICKNESS, muted),
                );
            } else {
                // Glow layer + full-color arc when hovered
                let glow = Color32::from_rgba_unmultiplied(color.r(), color.g(), color.b(), 22);
                draw_arc(
                    painter,
                    center,
                    RING_RADIUS,
                    -TAU / 4.0,
                    sweep,
                    Stroke::new(RING_THICKNESS + 10.0, glow),
                );
                draw_arc(
                    painter,
                    center,
                    RING_RADIUS,
                    -TAU / 4.0,
                    sweep,
                    Stroke::new(RING_THICKNESS, color),
                );
            }
        }
    }

    fn draw_controls(&mut self, ui: &mut egui::Ui, center_x: f32, y: f32, ctx: &egui::Context) {
        let button_size = Vec2::new(30.0, 26.0);
        let spacing = 10.0;

        let labels: &[(&str, ButtonAction)] = match self.timer.status {
            TimerStatus::Idle | TimerStatus::Finished => &[
                ("\u{25B6}", ButtonAction::Start),
                ("\u{27F2}", ButtonAction::Reset),
                ("\u{23ED}", ButtonAction::Skip),
                ("\u{00D7}", ButtonAction::Close),
            ],
            TimerStatus::Running => &[
                ("\u{23F8}", ButtonAction::Pause),
                ("\u{2713}", ButtonAction::Done),
                ("\u{23ED}", ButtonAction::Skip),
                ("\u{00D7}", ButtonAction::Close),
            ],
            TimerStatus::Paused => &[
                ("\u{25B6}", ButtonAction::Start),
                ("\u{2713}", ButtonAction::Done),
                ("\u{23ED}", ButtonAction::Skip),
                ("\u{00D7}", ButtonAction::Close),
            ],
        };

        let count = labels.len() as f32;
        let total_width = count * button_size.x + (count - 1.0) * spacing;
        let start_x = center_x - total_width / 2.0;

        let mut clicked_action = None;

        for (i, &(label, action)) in labels.iter().enumerate() {
            let x = start_x + i as f32 * (button_size.x + spacing);
            let rect = Rect::from_min_size(Pos2::new(x, y), button_size);

            let response = ui.allocate_rect(rect, egui::Sense::click());
            let bg = if response.hovered() {
                Color32::from_rgba_premultiplied(255, 255, 255, 35)
            } else {
                Color32::TRANSPARENT
            };

            ui.painter().rect_filled(rect, CornerRadius::same(6), bg);
            ui.painter().text(
                rect.center(),
                Align2::CENTER_CENTER,
                label,
                FontId::proportional(14.0),
                TEXT_COLOR,
            );

            if response.clicked() {
                clicked_action = Some(action);
            }
        }

        if let Some(action) = clicked_action {
            match action {
                ButtonAction::Start => self.timer.start(),
                ButtonAction::Pause => self.timer.pause(),
                ButtonAction::Reset => self.timer.reset(),
                ButtonAction::Skip => self.timer.skip(),
                ButtonAction::Done => {
                    let started_at = self.timer.phase_started_at;
                    let task = self.timer.current_task.clone();
                    let lang = self.lang;
                    if let Some((phase, actual_minutes)) = self.timer.done() {
                        self.history
                            .record(phase, task.clone(), started_at, actual_minutes);
                        self.recent_task_names = self.history.recent_task_names(7);
                        notify_phase_complete(phase, task.as_deref(), lang);
                        if self.timer.should_auto_start(phase) {
                            self.timer.start();
                        }
                    }
                }
                ButtonAction::Close => {
                    ctx.send_viewport_cmd(ViewportCommand::Close);
                }
            }
        }
    }

    fn draw_task_input(&mut self, ui: &mut egui::Ui, center_x: f32, y: f32) {
        let s = strings(self.lang);
        let input_width = PANEL_WIDTH - 40.0;
        let input_height = 22.0;
        let input_rect = Rect::from_min_size(
            Pos2::new(center_x - input_width / 2.0, y),
            Vec2::new(input_width, input_height),
        );

        let text_edit_has_focus = ui.memory(|m| {
            m.focused()
                .is_some_and(|id| id == egui::Id::new("task_input"))
        });
        if !text_edit_has_focus {
            let current = self.timer.current_task.as_deref().unwrap_or("");
            if self.task_input != current {
                self.task_input.clone_from(&current.to_string());
            }
        }

        ui.scope_builder(egui::UiBuilder::new().max_rect(input_rect), |ui| {
            let te = egui::TextEdit::singleline(&mut self.task_input)
                .id(egui::Id::new("task_input"))
                .hint_text(s.task_hint)
                .font(FontId::proportional(11.0))
                .text_color(TEXT_COLOR)
                .horizontal_align(egui::Align::Center)
                .desired_width(input_width)
                .margin(egui::Margin::symmetric(6, 3))
                .background_color(INPUT_BG);
            let response = ui.add(te);

            if response.lost_focus() {
                let trimmed = self.task_input.trim().to_string();
                if trimmed.is_empty() {
                    self.timer.current_task = None;
                    self.task_input.clear();
                } else {
                    self.timer.current_task = Some(trimmed);
                }
            }
        });
    }

    fn draw_task_suggestions(
        &self,
        ui: &mut egui::Ui,
        center_x: f32,
        y: f32,
        panel_left: f32,
        panel_right: f32,
    ) -> Option<String> {
        let current_task = self.timer.current_task.as_deref().unwrap_or("");
        let suggestions: Vec<&String> = self
            .recent_task_names
            .iter()
            .filter(|n| n.as_str() != current_task)
            .take(3)
            .collect();

        if suggestions.is_empty() {
            return None;
        }

        let font = FontId::proportional(9.0);
        let avail_width = panel_right - panel_left - 40.0;
        let mut clicked = None;

        // Layout: measure all labels first, then center them
        let galley_data: Vec<_> = suggestions
            .iter()
            .map(|name| {
                let display: String = if name.chars().count() > 10 {
                    let truncated: String = name.chars().take(9).collect();
                    format!("{truncated}…")
                } else {
                    (*name).clone()
                };
                let galley =
                    ui.painter()
                        .layout_no_wrap(display.clone(), font.clone(), DIM_TEXT_COLOR);
                let w = galley.size().x + 12.0; // padding
                (display, (*name).clone(), w)
            })
            .collect();

        let total_width: f32 = galley_data.iter().map(|(_, _, w)| w + 4.0).sum::<f32>() - 4.0;
        let total_width = total_width.min(avail_width);
        let mut x = center_x - total_width / 2.0;

        for (display, full_name, w) in &galley_data {
            let rect = Rect::from_min_size(Pos2::new(x, y), Vec2::new(*w, 16.0));
            let response = ui.allocate_rect(rect, egui::Sense::click());

            let bg = if response.hovered() {
                Color32::from_rgba_premultiplied(255, 255, 255, 20)
            } else {
                Color32::TRANSPARENT
            };
            ui.painter().rect_filled(rect, CornerRadius::same(4), bg);
            ui.painter().text(
                rect.center(),
                Align2::CENTER_CENTER,
                display,
                font.clone(),
                if response.hovered() {
                    TEXT_COLOR
                } else {
                    DIM_TEXT_COLOR
                },
            );

            if response.clicked() {
                clicked = Some(full_name.clone());
            }
            x += w + 4.0;
        }

        clicked
    }

    fn draw_history_section(
        &self,
        painter: &egui::Painter,
        center_x: f32,
        top_y: f32,
        panel_left: f32,
        panel_right: f32,
    ) {
        let s = strings(self.lang);

        // Divider line
        painter.line_segment(
            [
                Pos2::new(panel_left + 20.0, top_y),
                Pos2::new(panel_right - 20.0, top_y),
            ],
            Stroke::new(1.0, DIVIDER_COLOR),
        );

        // Section header
        let header_y = top_y + 14.0;
        painter.text(
            Pos2::new(center_x, header_y),
            Align2::CENTER_CENTER,
            s.today_log,
            FontId::proportional(10.0),
            DIM_TEXT_COLOR,
        );

        // Get today's tasks (newest first)
        let today_tasks: Vec<&CompletedTask> = {
            let mut tasks = self.history.today();
            tasks.reverse();
            tasks.into_iter().take(MAX_HISTORY_ITEMS).collect()
        };

        if today_tasks.is_empty() {
            painter.text(
                Pos2::new(center_x, header_y + 20.0),
                Align2::CENTER_CENTER,
                s.no_records,
                FontId::proportional(9.0),
                DIM_TEXT_COLOR,
            );
            return;
        }

        let item_start_y = header_y + 18.0;

        for (i, task) in today_tasks.iter().enumerate() {
            let y = item_start_y + i as f32 * 18.0;
            self.draw_history_item(painter, y, panel_left, panel_right, task);
        }
    }

    fn draw_history_item(
        &self,
        painter: &egui::Painter,
        y: f32,
        panel_left: f32,
        panel_right: f32,
        task: &CompletedTask,
    ) {
        let s = strings(self.lang);
        let font = FontId::proportional(10.0);

        let time_str = task.started_at.format("%H:%M").to_string();
        let phase_str = task.phase.label_with(self.lang);
        let dur_str = format!("{}{}", task.duration_minutes, s.min_short);

        let item_color = if task.phase == TimerPhase::Work {
            Color32::from_rgba_premultiplied(255, 255, 255, 180)
        } else {
            DIM_TEXT_COLOR
        };

        // Small colored dot indicating phase
        let dot_color = match task.phase {
            TimerPhase::Work => {
                if is_late_night() {
                    WORK_COLOR_LATE
                } else {
                    WORK_COLOR
                }
            }
            TimerPhase::ShortBreak => SHORT_BREAK_COLOR,
            TimerPhase::LongBreak => LONG_BREAK_COLOR,
        };
        painter.circle_filled(Pos2::new(panel_left + 18.0, y), 2.5, dot_color);

        // Time
        painter.text(
            Pos2::new(panel_left + 28.0, y),
            Align2::LEFT_CENTER,
            &time_str,
            font.clone(),
            Color32::from_rgba_premultiplied(255, 255, 255, 100),
        );

        // Phase + duration
        painter.text(
            Pos2::new(panel_left + 66.0, y),
            Align2::LEFT_CENTER,
            format!("{} {}", phase_str, dur_str),
            font.clone(),
            item_color,
        );

        // Task name (right-aligned, truncated)
        if let Some(name) = &task.task_name {
            let display: String = if name.chars().count() > 8 {
                let truncated: String = name.chars().take(7).collect();
                format!("{truncated}…")
            } else {
                name.clone()
            };
            painter.text(
                Pos2::new(panel_right - 16.0, y),
                Align2::RIGHT_CENTER,
                &display,
                font,
                DIM_TEXT_COLOR,
            );
        }
    }

    fn apply_time_edit(&mut self) {
        let input = self.time_edit_buf.trim();
        if let Ok(minutes) = input.parse::<u64>()
            && minutes > 0
            && minutes <= 120
        {
            let mut config = self.timer.config.clone();
            match self.timer.phase {
                TimerPhase::Work => config.work_minutes = minutes,
                TimerPhase::ShortBreak => config.short_break_minutes = minutes,
                TimerPhase::LongBreak => config.long_break_minutes = minutes,
            }
            self.timer.set_config(config);
        }
    }

    fn process_ipc_commands(&mut self, ctx: &egui::Context) {
        while let Ok((cmd, resp_tx)) = self.cmd_rx.try_recv() {
            let resp = self.handle_command(cmd, ctx);
            let _ = resp_tx.send(resp);
        }
    }

    fn handle_command(&mut self, cmd: Command, ctx: &egui::Context) -> Response {
        let s = strings(self.lang);
        match cmd {
            Command::Start => {
                self.timer.start();
                Response::ok(s.resp_start)
            }
            Command::Pause => {
                self.timer.pause();
                Response::ok(s.resp_pause)
            }
            Command::Reset => {
                self.timer.reset();
                Response::ok(s.resp_reset)
            }
            Command::Skip => {
                self.timer.skip();
                Response::ok(s.resp_skip)
            }
            Command::Status => {
                let stats = self.history.today_stats();
                let info = StatusInfo {
                    phase: self.timer.phase,
                    status: self.timer.status,
                    remaining: self.timer.remaining_display(),
                    task: self.timer.current_task.clone(),
                    completed_sessions: self.timer.completed_sessions,
                    config: self.timer.config.clone(),
                    today_pomodoros: stats.pomodoros,
                    today_focus_minutes: stats.focus_minutes,
                    lang: self.lang,
                };
                Response {
                    ok: true,
                    message: None,
                    status: Some(info),
                    history: None,
                    report: None,
                    recent_tasks: None,
                }
            }
            Command::SetTask { name } => {
                self.timer.current_task = Some(name.clone());
                Response::ok(format!("Task: {name}"))
            }
            Command::ClearTask => {
                self.timer.current_task = None;
                Response::ok(s.resp_task_clear)
            }
            Command::SetTimes {
                work,
                short_break,
                long_break,
                auto_start_work,
                auto_start_break,
            } => {
                let mut config = self.timer.config.clone();
                if let Some(w) = work {
                    config.work_minutes = w;
                }
                if let Some(sb) = short_break {
                    config.short_break_minutes = sb;
                }
                if let Some(l) = long_break {
                    config.long_break_minutes = l;
                }
                if let Some(asw) = auto_start_work {
                    config.auto_start_work = asw;
                }
                if let Some(asb) = auto_start_break {
                    config.auto_start_break = asb;
                }
                self.timer.set_config(config);
                Response::ok(format!(
                    "{}={}m {}={}m {}={}m",
                    strings(self.lang).work,
                    self.timer.config.work_minutes,
                    strings(self.lang).short_break,
                    self.timer.config.short_break_minutes,
                    strings(self.lang).long_break,
                    self.timer.config.long_break_minutes,
                ))
            }
            Command::History { today_only } => {
                let tasks = if today_only.unwrap_or(false) {
                    self.history.today().into_iter().cloned().collect()
                } else {
                    self.history.tasks.clone()
                };
                Response {
                    ok: true,
                    message: None,
                    status: None,
                    history: Some(tasks),
                    report: None,
                    recent_tasks: None,
                }
            }
            Command::Report { week } => {
                let report = self.build_report(week.unwrap_or(false));
                Response {
                    ok: true,
                    message: None,
                    status: None,
                    history: None,
                    report: Some(report),
                    recent_tasks: None,
                }
            }
            Command::Done => {
                let started_at = self.timer.phase_started_at;
                let task = self.timer.current_task.clone();
                if let Some((phase, actual_minutes)) = self.timer.done() {
                    self.history.record(phase, task, started_at, actual_minutes);
                    self.recent_task_names = self.history.recent_task_names(7);
                    Response::ok(format!(
                        "{} ({}{})",
                        s.resp_done, actual_minutes, s.min_short
                    ))
                } else {
                    Response::ok(s.resp_done)
                }
            }
            Command::Log { name, duration, at } => {
                let now = chrono::Local::now();
                let started_at = if let Some(time_str) = &at {
                    chrono::NaiveTime::parse_from_str(time_str, "%H:%M")
                        .ok()
                        .and_then(|t| {
                            now.date_naive()
                                .and_time(t)
                                .and_local_timezone(chrono::Local)
                                .single()
                        })
                        .unwrap_or(now - chrono::Duration::minutes(duration as i64))
                } else {
                    now - chrono::Duration::minutes(duration as i64)
                };
                self.history.record(
                    yasume::timer::TimerPhase::Work,
                    Some(name.clone()),
                    Some(started_at),
                    duration,
                );
                self.recent_task_names = self.history.recent_task_names(7);
                Response::ok(format!(
                    "{}: {} ({}{})",
                    s.resp_log, name, duration, s.min_short
                ))
            }
            Command::Quit => {
                self.should_quit = true;
                ctx.send_viewport_cmd(ViewportCommand::Close);
                Response::ok(s.resp_quit)
            }
            Command::List { from, to } => {
                let from_date = from
                    .as_deref()
                    .and_then(|d| chrono::NaiveDate::parse_from_str(d, "%Y-%m-%d").ok());
                let to_date = to
                    .as_deref()
                    .and_then(|d| chrono::NaiveDate::parse_from_str(d, "%Y-%m-%d").ok());
                let tasks: Vec<_> = self
                    .history
                    .filter_by_date_range(from_date, to_date)
                    .into_iter()
                    .cloned()
                    .collect();
                Response {
                    ok: true,
                    message: None,
                    status: None,
                    history: Some(tasks),
                    report: None,
                    recent_tasks: None,
                }
            }
            Command::SetLang { lang } => {
                self.lang = lang;
                let label = match lang {
                    Lang::Ja => "日本語",
                    Lang::En => "English",
                };
                Response::ok(label)
            }
            Command::RecentTasks => {
                let names = self.history.recent_task_names(7);
                Response {
                    ok: true,
                    message: None,
                    status: None,
                    history: None,
                    report: None,
                    recent_tasks: Some(names),
                }
            }
        }
    }

    fn build_report(&self, week: bool) -> ReportInfo {
        use std::collections::{BTreeMap, HashSet};

        let s = strings(self.lang);
        let tasks = if week {
            self.history.this_week()
        } else {
            self.history.today()
        };

        let work_tasks: Vec<_> = tasks
            .iter()
            .filter(|t| t.phase == TimerPhase::Work)
            .collect();

        let total_pomodoros = work_tasks.len();
        let total_focus_minutes: u64 = work_tasks.iter().map(|t| t.duration_minutes).sum();
        let unique_tasks: Vec<String> = work_tasks
            .iter()
            .filter_map(|t| t.task_name.as_ref())
            .collect::<HashSet<_>>()
            .into_iter()
            .cloned()
            .collect();

        let mut daily: BTreeMap<String, (usize, u64)> = BTreeMap::new();
        for t in &work_tasks {
            let date = t.completed_at.format("%Y-%m-%d").to_string();
            let entry = daily.entry(date).or_insert((0, 0));
            entry.0 += 1;
            entry.1 += t.duration_minutes;
        }

        let daily_breakdown = daily
            .into_iter()
            .map(|(date, (pomodoros, focus_minutes))| DailyBreakdown {
                date,
                pomodoros,
                focus_minutes,
            })
            .collect();

        ReportInfo {
            period: if week {
                s.report_week.to_string()
            } else {
                s.report_today.to_string()
            },
            total_pomodoros,
            total_focus_minutes,
            unique_tasks,
            daily_breakdown,
        }
    }

    fn record_completion(&mut self, phase: TimerPhase) {
        let duration_minutes = match phase {
            TimerPhase::Work => self.timer.config.work_minutes,
            TimerPhase::ShortBreak => self.timer.config.short_break_minutes,
            TimerPhase::LongBreak => self.timer.config.long_break_minutes,
        };
        self.history.record(
            phase,
            self.timer.current_task.clone(),
            self.timer.phase_started_at,
            duration_minutes,
        );
        self.recent_task_names = self.history.recent_task_names(7);
    }
}

impl Drop for PomodoroApp {
    fn drop(&mut self) {
        ipc::cleanup_socket();
    }
}

impl eframe::App for PomodoroApp {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        egui::Rgba::TRANSPARENT.to_array()
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.should_quit {
            ctx.send_viewport_cmd(ViewportCommand::Close);
            return;
        }

        self.process_ipc_commands(ctx);

        if let Some(completed_phase) = self.timer.tick() {
            self.record_completion(completed_phase);
            notify_phase_complete(
                completed_phase,
                self.timer.current_task.as_deref(),
                self.lang,
            );

            if self.timer.should_auto_start(completed_phase) {
                self.timer.start();
            }

            if completed_phase == TimerPhase::Work && self.timer.skipped_breaks >= 2 {
                notify_overwork(self.timer.skipped_breaks, self.lang);
            }

            notify_late_night(self.lang);
        }

        if self.timer.status == TimerStatus::Running {
            ctx.request_repaint();
        } else {
            ctx.request_repaint_after(Duration::from_millis(100));
        }

        let pointer_in_window = ctx.input(|i| {
            i.pointer
                .hover_pos()
                .is_some_and(|pos| pos.x >= 0.0 && pos.y >= 0.0)
        });

        // Keep panel active while typing in any text input
        let text_input_active = self.editing_time
            || ctx.memory(|m| {
                m.focused().is_some_and(|id| {
                    id == egui::Id::new("task_input") || id == egui::Id::new("time_edit")
                })
            });

        let was_hovered = self.is_hovered;
        self.is_hovered = pointer_in_window || text_input_active;

        if self.is_hovered != was_hovered {
            ctx.send_viewport_cmd(ViewportCommand::MousePassthrough(!self.is_hovered));
        }

        let panel_frame = egui::Frame::new()
            .fill(Color32::TRANSPARENT)
            .inner_margin(0.0);

        egui::CentralPanel::default()
            .frame(panel_frame)
            .show(ctx, |ui| {
                let available = ui.available_rect_before_wrap();
                let center_x = available.center().x;
                let ring_center = Pos2::new(center_x, RING_CENTER_Y);
                let idle = !self.is_hovered;

                // Dynamic panel height
                let panel_height = if self.is_hovered {
                    let item_count = self.history.today().len().min(MAX_HISTORY_ITEMS);
                    let history_h = if item_count == 0 {
                        36.0
                    } else {
                        34.0 + item_count as f32 * 18.0
                    };
                    240.0 + history_h
                } else {
                    150.0
                };

                let panel_rect = Rect::from_min_size(
                    Pos2::new(center_x - PANEL_WIDTH / 2.0, 5.0),
                    Vec2::new(PANEL_WIDTH, panel_height),
                );

                // -- Painter-only pass (before widget calls) --
                {
                    let p = ui.painter();
                    let bg = if self.is_hovered {
                        PANEL_BG_HOVER
                    } else {
                        PANEL_BG
                    };
                    p.rect_filled(panel_rect, CornerRadius::same(CORNER_RADIUS), bg);

                    if !idle {
                        p.rect_stroke(
                            panel_rect,
                            CornerRadius::same(CORNER_RADIUS),
                            Stroke::new(1.0, self.accent_border_color()),
                            egui::StrokeKind::Outside,
                        );
                    }

                    self.draw_ring(p, ring_center, idle);
                }

                // -- Time display / edit (may use widgets) --
                let time_center = Pos2::new(center_x, ring_center.y - 8.0);
                let time_font = if idle { 20.0 } else { 26.0 };

                if self.editing_time {
                    let edit_rect = Rect::from_center_size(time_center, Vec2::new(70.0, 24.0));
                    ui.scope_builder(egui::UiBuilder::new().max_rect(edit_rect), |ui| {
                        let te = egui::TextEdit::singleline(&mut self.time_edit_buf)
                            .id(egui::Id::new("time_edit"))
                            .font(FontId::monospace(16.0))
                            .text_color(TEXT_COLOR)
                            .horizontal_align(egui::Align::Center)
                            .desired_width(edit_rect.width())
                            .margin(egui::Margin::symmetric(4, 2))
                            .background_color(INPUT_BG);
                        let response = ui.add(te);

                        if self.time_edit_needs_focus {
                            response.request_focus();
                            self.time_edit_needs_focus = false;
                        }

                        if response.lost_focus() {
                            let escaped = ui.input(|i| i.key_pressed(egui::Key::Escape));
                            self.editing_time = false;
                            if !escaped {
                                self.apply_time_edit();
                            }
                        }
                    });
                } else {
                    ui.painter().text(
                        time_center,
                        Align2::CENTER_CENTER,
                        self.timer.remaining_display(),
                        FontId::monospace(time_font),
                        if idle { TEXT_COLOR_IDLE } else { TEXT_COLOR },
                    );

                    // Double-click to edit (only when hovered)
                    if !idle {
                        let click_rect = Rect::from_center_size(time_center, Vec2::new(80.0, 28.0));
                        let response = ui.allocate_rect(click_rect, egui::Sense::click());
                        if response.double_clicked() {
                            self.editing_time = true;
                            self.time_edit_needs_focus = true;
                            let current_minutes = match self.timer.phase {
                                TimerPhase::Work => self.timer.config.work_minutes,
                                TimerPhase::ShortBreak => self.timer.config.short_break_minutes,
                                TimerPhase::LongBreak => self.timer.config.long_break_minutes,
                            };
                            self.time_edit_buf = current_minutes.to_string();
                        }
                    }
                }

                // -- Painter pass for labels (after widget calls) --
                {
                    let p = ui.painter();

                    // Task name inside ring (compact mode only)
                    if idle && let Some(task) = &self.timer.current_task {
                        let display_name: String = if task.chars().count() > 14 {
                            let truncated: String = task.chars().take(12).collect();
                            format!("{truncated}…")
                        } else {
                            task.clone()
                        };
                        p.text(
                            Pos2::new(center_x, ring_center.y + 6.0),
                            Align2::CENTER_CENTER,
                            &display_name,
                            FontId::proportional(8.0),
                            DIM_TEXT_COLOR_IDLE,
                        );
                    }

                    // Phase label + session counter
                    let label_y = if idle && self.timer.current_task.is_some() {
                        ring_center.y + 18.0
                    } else {
                        ring_center.y + 12.0
                    };
                    let phase_label = self.timer.phase.label_with(self.lang);
                    let session_text =
                        format!("{} · {}/4", phase_label, self.timer.completed_sessions);
                    let label_color = if idle {
                        DIM_TEXT_COLOR_IDLE
                    } else {
                        DIM_TEXT_COLOR
                    };
                    let label_font = if idle { 9.0 } else { 10.0 };
                    p.text(
                        Pos2::new(center_x, label_y),
                        Align2::CENTER_CENTER,
                        &session_text,
                        FontId::proportional(label_font),
                        label_color,
                    );
                }

                // -- Expanded UI (hovered) --
                if self.is_hovered {
                    let mut y = ring_center.y + RING_RADIUS + 24.0;

                    // Control buttons
                    self.draw_controls(ui, center_x, y, ctx);
                    y += 30.0;

                    // Task input
                    self.draw_task_input(ui, center_x, y);
                    y += 26.0;

                    // Recent task suggestions (clickable)
                    let suggestion_clicked = self.draw_task_suggestions(
                        ui,
                        center_x,
                        y,
                        panel_rect.left(),
                        panel_rect.right(),
                    );
                    if let Some(name) = suggestion_clicked {
                        self.timer.current_task = Some(name.clone());
                        self.task_input = name;
                    }
                    if !self.recent_task_names.is_empty() {
                        y += 20.0;
                    }

                    // Stats
                    let stats = self.history.today_stats();
                    let s = strings(self.lang);
                    let stats_text = s.format_stats(stats.pomodoros, stats.focus_minutes);
                    ui.painter().text(
                        Pos2::new(center_x, y),
                        Align2::CENTER_CENTER,
                        &stats_text,
                        FontId::proportional(11.0),
                        TEXT_COLOR,
                    );
                    y += 18.0;

                    // History section
                    self.draw_history_section(
                        ui.painter(),
                        center_x,
                        y,
                        panel_rect.left(),
                        panel_rect.right(),
                    );

                    // Drag area (ring region)
                    let drag_rect = Rect::from_min_size(
                        panel_rect.min,
                        Vec2::new(PANEL_WIDTH, ring_center.y + RING_RADIUS),
                    );
                    let drag_response = ui.allocate_rect(drag_rect, egui::Sense::click_and_drag());
                    if drag_response.dragged() {
                        ctx.send_viewport_cmd(ViewportCommand::StartDrag);
                    }
                }
            });
    }
}

fn draw_arc(
    painter: &egui::Painter,
    center: Pos2,
    radius: f32,
    start_angle: f32,
    sweep: f32,
    stroke: Stroke,
) {
    let segments = (sweep.abs() / TAU * 64.0).max(1.0) as usize;
    let points: Vec<Pos2> = (0..=segments)
        .map(|i| {
            let t = i as f32 / segments as f32;
            let angle = start_angle + sweep * t;
            Pos2::new(
                center.x + radius * angle.cos(),
                center.y + radius * angle.sin(),
            )
        })
        .collect();

    if points.len() >= 2 {
        painter.add(Shape::line(points, stroke));
    }
}
