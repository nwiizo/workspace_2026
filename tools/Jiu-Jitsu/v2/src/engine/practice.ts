import { PRACTICE_LESSONS, practiceLesson, type PracticeLesson, type PracticeNode } from "../content/practice";
import type { SrsState } from "./srs";

export type PracticeMode = "learn" | "check";
export interface PracticeTurn { action: string; before: string; after: string; feedback: string; progressed: boolean }

export function practiceKey(lessonId: string, mode: PracticeMode): string {
  return `practice:${lessonId}:${mode}`;
}

export class PracticeRun {
  readonly lesson: PracticeLesson;
  readonly history: PracticeTurn[] = [];
  readonly mode: PracticeMode;
  nodeId: string;
  phase: "acting" | "feedback" | "complete" | "stopped" = "acting";
  hintUsed = false;
  private recorded = false;

  constructor(lessonId: string, mode: PracticeMode, variant = 0) {
    this.lesson = practiceLesson(lessonId);
    this.mode = mode;
    this.nodeId = this.lesson.starts[variant % 2 === 0 ? 0 : 1];
  }

  get node(): PracticeNode { return this.lesson.nodes[this.nodeId]!; }
  get mistakes(): number { return this.history.filter((turn) => !turn.progressed).length; }

  act(actionId: string): PracticeTurn | null {
    if (this.phase === "complete" || this.phase === "stopped") return null;
    if (actionId === "tap" && !this.node.moves.tap) {
      this.phase = "stopped";
      return null;
    }
    if (this.phase !== "acting") return null;
    const action = this.lesson.actions.find((item) => item.id === actionId);
    if (!action && actionId !== "tap") return null;
    const edge = this.node.moves[actionId];
    const setback = this.node.setbacks?.[actionId];
    const turn: PracticeTurn = {
      action: action?.label ?? "タップして止める",
      before: this.nodeId,
      after: edge?.to ?? setback?.to ?? this.nodeId,
      progressed: !!edge,
      feedback: edge?.feedback ?? setback?.feedback ?? action!.blocked,
    };
    this.history.push(turn);
    this.nodeId = turn.after;
    this.phase = "feedback";
    return turn;
  }

  continue(): void {
    if (this.phase !== "feedback") return;
    this.phase = this.node.end ? "complete" : "acting";
  }
  hint(): string { this.hintUsed = true; return this.node.hint; }

  takeResult(): { key: string; correct: boolean } | null {
    if (this.recorded || this.phase !== "complete") return null;
    this.recorded = true;
    return { key: practiceKey(this.lesson.id, this.mode), correct: this.mode === "learn" || (this.mistakes === 0 && !this.hintUsed) };
  }
}

export function nextPractice(srs: SrsState, now: number): { lessonId: string; mode: PracticeMode } {
  const due = PRACTICE_LESSONS.filter((lesson) => {
    const item = srs[practiceKey(lesson.id, "check")];
    return item && item.dueAt <= now;
  }).sort((a, b) => srs[practiceKey(a.id, "check")]!.dueAt - srs[practiceKey(b.id, "check")]!.dueAt)[0];
  if (due) return { lessonId: due.id, mode: "check" };
  for (const lesson of PRACTICE_LESSONS) {
    if (srs[practiceKey(lesson.id, "check")]?.box > 0) continue;
    return { lessonId: lesson.id, mode: srs[practiceKey(lesson.id, "learn")] ? "check" : "learn" };
  }
  const earliest = [...PRACTICE_LESSONS].sort((a, b) => srs[practiceKey(a.id, "check")]!.dueAt - srs[practiceKey(b.id, "check")]!.dueAt)[0]!;
  return { lessonId: earliest.id, mode: "check" };
}

export function checkedPracticeCount(srs: SrsState): number {
  return PRACTICE_LESSONS.filter((lesson) => srs[practiceKey(lesson.id, "check")]?.box > 0).length;
}
