import { inspectTransition, type RouteFrame, type MotionIssue } from "./route";

export class RoutePlayback {
  private position: number;
  private readonly problems: ReturnType<typeof inspectTransition>[];
  constructor(private readonly frames: RouteFrame[], position = 0) {
    this.position = Math.max(0, Math.min(frames.length - 1, position));
    this.problems = frames.slice(1).map((frame, i) => inspectTransition(frames[i]!.pose, frame.pose));
  }
  advance(seconds: number): { position: number; done: boolean; problem: MotionIssue | null } {
    while (this.position < this.frames.length - 1) {
      const index = Math.floor(this.position), fraction = this.position - index;
      const duration = this.frames[index + 1]!.seconds;
      const next = Math.min(1, fraction + Math.max(0, seconds) / duration);
      const problem = this.problems[index];
      if (problem && problem.fraction >= fraction && problem.fraction <= next) {
        this.position = index + problem.fraction;
        return { position: this.position, done: false, problem: problem.issues[0]! };
      }
      seconds -= (next - fraction) * duration;
      this.position = index + next;
      if (next < 1 || seconds <= 0) break;
    }
    return { position: this.position, done: this.position >= this.frames.length - 1, problem: null };
  }
}
