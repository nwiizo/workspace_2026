import type { KeyValueStore } from "./storage";
import type { RouteFrame } from "./route";
import { bodyFrame, type BodyTargets } from "../render/mannequin";
import { Quaternion, Vector3 } from "three";

export interface SavedRoute { name: string; frames: RouteFrame[] }
export const ROUTE_KEY = "jiu-jitsu-dojo-v2/route";
export function saveRoute(store: KeyValueStore, route: SavedRoute): void {
  const body = (p: BodyTargets) => ({ pelvis: p.pelvis.toArray(), rotation: bodyFrame(p.pelvis, p.up, p.front).rotation.toArray(),
    arms: [p.arms.L, p.arms.R].map((limb) => [limb.end.toArray(), limb.pole.toArray()]),
    legs: [p.legs.L, p.legs.R].map((limb) => [limb.end.toArray(), limb.pole.toArray()]) });
  const data = JSON.stringify({ version: 1, name: route.name, frames: route.frames.map((f) => ({ label: f.label, seconds: f.seconds, blue: body(f.pose.blue), red: body(f.pose.red) })) });
  decode(data); // 非有限値などを保存前にも拒否する。既存の保存内容は上書きしない。
  store.setItem(ROUTE_KEY, data);
}
export function loadRoute(store: KeyValueStore): SavedRoute | null {
  const data = store.getItem(ROUTE_KEY);
  return data === null ? null : decode(data);
}

function decode(data: string): SavedRoute {
  if (data.length > 200_000) throw new Error("Route is too large");
  const root = object(JSON.parse(data));
  if (root.version !== 1) throw new Error("Unsupported route version");
  const frames = array(root.frames);
  if (frames.length < 1 || frames.length > 40) throw new Error("Invalid frame count");
  return { name: title(root.name), frames: frames.map((value) => {
    const frame = object(value), seconds = frame.seconds;
    if (typeof seconds !== "number" || !Number.isFinite(seconds) || seconds < 0.5 || seconds > 10) throw new Error("Invalid duration");
    return { label: title(frame.label), seconds, pose: { blue: body(frame.blue), red: body(frame.red) } };
  }) };
}
function object(value: unknown): Record<string, unknown> {
  if (!value || typeof value !== "object" || Array.isArray(value)) throw new Error("Invalid route object");
  return value as Record<string, unknown>;
}
function array(value: unknown, length?: number): unknown[] {
  if (!Array.isArray(value) || (length !== undefined && value.length !== length)) throw new Error("Invalid route array");
  return value;
}
function numbers(value: unknown, length: number): number[] {
  const values = array(value, length);
  if (!values.every((n): n is number => typeof n === "number" && Number.isFinite(n) && Math.abs(n) <= 3)) throw new Error("Invalid coordinate");
  return values;
}
function vector(value: unknown): Vector3 { return new Vector3().fromArray(numbers(value, 3)); }
function title(value: unknown): string {
  if (typeof value !== "string" || !value.trim() || value.length > 80) throw new Error("Invalid route name");
  return value;
}
function body(value: unknown): BodyTargets {
  const p = object(value), q = new Quaternion().fromArray(numbers(p.rotation, 4));
  if (Math.abs(q.lengthSq() - 1) > 1e-5) throw new Error("Invalid body orientation");
  const limbs = (value: unknown) => {
    const items = array(value, 2).map((item) => { const a = array(item, 2); return { end: vector(a[0]), pole: vector(a[1]) }; });
    return { L: items[0]!, R: items[1]! };
  };
  return { pelvis: vector(p.pelvis), up: new Vector3(0, 1, 0).applyQuaternion(q), front: new Vector3(0, 0, 1).applyQuaternion(q), arms: limbs(p.arms), legs: limbs(p.legs) };
}
