#!/usr/bin/env node

import { readFileSync } from "node:fs";
import { basename } from "node:path";

const reportPath = process.argv[2] ?? "reports/zap-full.json";
const raw = readFileSync(reportPath, "utf8");
const report = JSON.parse(raw);

const riskOrder = new Map([
  ["High", 0],
  ["Medium", 1],
  ["Low", 2],
  ["Informational", 3],
]);

function riskName(riskdesc = "") {
  return String(riskdesc).split(" ")[0] || "Unknown";
}

function severityRank(alert) {
  return riskOrder.get(riskName(alert.riskdesc)) ?? 99;
}

function text(value) {
  return String(value ?? "").replace(/\s+/g, " ").trim();
}

function truncate(value, max = 180) {
  const normalized = text(value);
  if (normalized.length <= max) {
    return normalized;
  }
  return `${normalized.slice(0, max - 1)}...`;
}

function code(value) {
  const normalized = text(value);
  return normalized ? `\`${normalized.replaceAll("`", "'")}\`` : "";
}

function tableCell(value) {
  return text(value).replaceAll("|", "\\|");
}

const alerts = [];

for (const site of report.site ?? []) {
  for (const alert of site.alerts ?? []) {
    alerts.push({
      site: site["@name"] ?? site.name ?? "",
      alert: alert.alert ?? "Unknown alert",
      riskdesc: alert.riskdesc ?? "Unknown",
      confidence: alert.confidence ?? "",
      pluginid: alert.pluginid ?? "",
      cweid: alert.cweid ?? "",
      wascid: alert.wascid ?? "",
      desc: alert.desc ?? "",
      solution: alert.solution ?? "",
      instances: alert.instances ?? [],
    });
  }
}

alerts.sort((a, b) => {
  const byRisk = severityRank(a) - severityRank(b);
  if (byRisk !== 0) {
    return byRisk;
  }
  return a.alert.localeCompare(b.alert);
});

const totals = new Map();
for (const alert of alerts) {
  const risk = riskName(alert.riskdesc);
  const current = totals.get(risk) ?? { alerts: 0, instances: 0 };
  current.alerts += 1;
  current.instances += alert.instances.length;
  totals.set(risk, current);
}

console.log("# ZAP findings summary");
console.log("");
console.log(`Source: ${reportPath}`);
console.log(`Generated from: ${basename(reportPath)}`);
console.log("");
console.log("## Totals");
console.log("");
console.log("| Risk | Alerts | Instances |");
console.log("| --- | ---: | ---: |");

for (const [risk, total] of [...totals.entries()].sort(
  ([a], [b]) => (riskOrder.get(a) ?? 99) - (riskOrder.get(b) ?? 99),
)) {
  console.log(`| ${tableCell(risk)} | ${total.alerts} | ${total.instances} |`);
}

console.log("");
console.log("## Findings");

for (const alert of alerts) {
  console.log("");
  console.log(`### ${riskName(alert.riskdesc)}: ${alert.alert}`);
  console.log("");
  console.log(`- Risk: ${alert.riskdesc}`);
  if (alert.confidence) {
    console.log(`- Confidence: ${alert.confidence}`);
  }
  if (alert.pluginid) {
    console.log(`- Plugin ID: ${alert.pluginid}`);
  }
  if (alert.cweid && alert.cweid !== "-1") {
    console.log(`- CWE: ${alert.cweid}`);
  }
  if (alert.wascid && alert.wascid !== "-1") {
    console.log(`- WASC: ${alert.wascid}`);
  }
  console.log(`- Instances: ${alert.instances.length}`);

  const shownInstances = alert.instances.slice(0, 5);
  if (shownInstances.length > 0) {
    console.log("- Example locations:");
    for (const instance of shownInstances) {
      const method = text(instance.method) || "GET";
      const uri = text(instance.uri);
      const param = text(instance.param);
      const evidence = truncate(instance.evidence || instance.attack || instance.otherinfo);
      const parts = [`${method} ${uri}`];
      if (param) {
        parts.push(`param ${code(param)}`);
      }
      if (evidence) {
        parts.push(`evidence ${code(evidence)}`);
      }
      console.log(`  - ${parts.join("; ")}`);
    }
    if (alert.instances.length > shownInstances.length) {
      console.log(`  - ... ${alert.instances.length - shownInstances.length} more instance(s) omitted`);
    }
  }

  if (alert.desc) {
    console.log(`- Description: ${truncate(alert.desc)}`);
  }
  if (alert.solution) {
    console.log(`- Suggested fix: ${truncate(alert.solution)}`);
  }
}
