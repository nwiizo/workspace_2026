"use client";

import { useEffect, useState } from "react";
import Link from "next/link";
import api, { IncidentStatistics, ProjectStatistics, LeaderboardEntry } from "@/lib/api";

export default function DashboardPage() {
  const [incidentStats, setIncidentStats] = useState<IncidentStatistics | null>(null);
  const [projectStats, setProjectStats] = useState<ProjectStatistics | null>(null);
  const [topEngineers, setTopEngineers] = useState<LeaderboardEntry[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const fetchData = async () => {
      try {
        const [incidents, projects, leaderboard] = await Promise.all([
          api.getIncidentStats().catch(() => null),
          api.getProjectStats().catch(() => null),
          api.getLevelLeaderboard(5).catch(() => []),
        ]);
        setIncidentStats(incidents);
        setProjectStats(projects);
        setTopEngineers(leaderboard);
      } catch (err) {
        setError(err instanceof Error ? err.message : "Failed to load dashboard");
      } finally {
        setLoading(false);
      }
    };

    fetchData();
  }, []);

  if (loading) {
    return (
      <div className="flex justify-center items-center h-64">
        <div className="animate-spin rounded-full h-12 w-12 border-b-2 border-indigo-600"></div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="bg-red-50 border border-red-200 rounded-lg p-4">
        <p className="text-red-800">{error}</p>
      </div>
    );
  }

  return (
    <div className="space-y-8">
      <div>
        <h1 className="text-3xl font-bold text-gray-900">Dashboard</h1>
        <p className="text-gray-600 mt-1">Overview of your team&apos;s performance</p>
      </div>

      {/* Stats Grid */}
      <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6">
        <StatCard
          title="Open Incidents"
          value={incidentStats?.open ?? 0}
          total={incidentStats?.total ?? 0}
          color="red"
          href="/incidents"
        />
        <StatCard
          title="Active Projects"
          value={projectStats?.in_progress ?? 0}
          total={projectStats?.total ?? 0}
          color="blue"
          href="/projects"
        />
        <StatCard
          title="Resolved Today"
          value={incidentStats?.resolved ?? 0}
          color="green"
        />
        <StatCard
          title="Hours Logged"
          value={projectStats?.total_actual_hours ?? 0}
          subtitle={`of ${projectStats?.total_estimated_hours ?? 0} estimated`}
          color="purple"
        />
      </div>

      {/* Quick Actions & Leaderboard */}
      <div className="grid lg:grid-cols-2 gap-8">
        {/* Quick Actions */}
        <div className="bg-white rounded-xl shadow-md p-6">
          <h2 className="text-lg font-semibold text-gray-900 mb-4">Quick Actions</h2>
          <div className="grid grid-cols-2 gap-4">
            <Link
              href="/incidents"
              className="flex items-center p-4 bg-red-50 rounded-lg hover:bg-red-100 transition"
            >
              <div className="w-10 h-10 bg-red-100 rounded-lg flex items-center justify-center mr-3">
                <svg className="w-5 h-5 text-red-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 4v16m8-8H4" />
                </svg>
              </div>
              <span className="font-medium text-gray-900">New Incident</span>
            </Link>
            <Link
              href="/projects"
              className="flex items-center p-4 bg-blue-50 rounded-lg hover:bg-blue-100 transition"
            >
              <div className="w-10 h-10 bg-blue-100 rounded-lg flex items-center justify-center mr-3">
                <svg className="w-5 h-5 text-blue-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 4v16m8-8H4" />
                </svg>
              </div>
              <span className="font-medium text-gray-900">New Project</span>
            </Link>
            <Link
              href="/engineers"
              className="flex items-center p-4 bg-green-50 rounded-lg hover:bg-green-100 transition"
            >
              <div className="w-10 h-10 bg-green-100 rounded-lg flex items-center justify-center mr-3">
                <svg className="w-5 h-5 text-green-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M17 20h5v-2a3 3 0 00-5.356-1.857M17 20H7m10 0v-2c0-.656-.126-1.283-.356-1.857M7 20H2v-2a3 3 0 015.356-1.857M7 20v-2c0-.656.126-1.283.356-1.857m0 0a5.002 5.002 0 019.288 0M15 7a3 3 0 11-6 0 3 3 0 016 0z" />
                </svg>
              </div>
              <span className="font-medium text-gray-900">View Team</span>
            </Link>
            <Link
              href="/recruitment"
              className="flex items-center p-4 bg-purple-50 rounded-lg hover:bg-purple-100 transition"
            >
              <div className="w-10 h-10 bg-purple-100 rounded-lg flex items-center justify-center mr-3">
                <svg className="w-5 h-5 text-purple-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M18 9v3m0 0v3m0-3h3m-3 0h-3m-2-5a4 4 0 11-8 0 4 4 0 018 0zM3 20a6 6 0 0112 0v1H3v-1z" />
                </svg>
              </div>
              <span className="font-medium text-gray-900">Hire Engineer</span>
            </Link>
          </div>
        </div>

        {/* Top Engineers */}
        <div className="bg-white rounded-xl shadow-md p-6">
          <div className="flex justify-between items-center mb-4">
            <h2 className="text-lg font-semibold text-gray-900">Top Engineers</h2>
            <Link href="/leaderboard" className="text-sm text-indigo-600 hover:text-indigo-800">
              View All
            </Link>
          </div>
          {topEngineers.length > 0 ? (
            <div className="space-y-3">
              {topEngineers.map((entry, index) => (
                <div
                  key={entry.engineer_id}
                  className="flex items-center justify-between p-3 bg-gray-50 rounded-lg"
                >
                  <div className="flex items-center">
                    <span className={`w-8 h-8 rounded-full flex items-center justify-center text-sm font-bold mr-3 ${
                      index === 0 ? "bg-yellow-100 text-yellow-700" :
                      index === 1 ? "bg-gray-200 text-gray-700" :
                      index === 2 ? "bg-orange-100 text-orange-700" :
                      "bg-gray-100 text-gray-600"
                    }`}>
                      {entry.rank}
                    </span>
                    <span className="font-medium text-gray-900">{entry.engineer_email}</span>
                  </div>
                  <span className="text-sm font-semibold text-indigo-600">
                    Lv. {entry.level ?? entry.value}
                  </span>
                </div>
              ))}
            </div>
          ) : (
            <p className="text-gray-500 text-center py-8">No engineers yet</p>
          )}
        </div>
      </div>
    </div>
  );
}

interface StatCardProps {
  title: string;
  value: number;
  total?: number;
  subtitle?: string;
  color: "red" | "blue" | "green" | "purple";
  href?: string;
}

function StatCard({ title, value, total, subtitle, color, href }: StatCardProps) {
  const colorClasses = {
    red: "bg-red-50 text-red-600",
    blue: "bg-blue-50 text-blue-600",
    green: "bg-green-50 text-green-600",
    purple: "bg-purple-50 text-purple-600",
  };

  const content = (
    <div className={`${colorClasses[color]} rounded-xl p-6 ${href ? "hover:opacity-80 transition" : ""}`}>
      <p className="text-sm font-medium opacity-80">{title}</p>
      <p className="text-3xl font-bold mt-2">{value}</p>
      {total !== undefined && (
        <p className="text-sm opacity-70 mt-1">of {total} total</p>
      )}
      {subtitle && (
        <p className="text-sm opacity-70 mt-1">{subtitle}</p>
      )}
    </div>
  );

  if (href) {
    return <Link href={href}>{content}</Link>;
  }
  return content;
}
