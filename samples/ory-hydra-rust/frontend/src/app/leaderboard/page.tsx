"use client";

import { useEffect, useState } from "react";
import api, { LeaderboardEntry } from "@/lib/api";

type LeaderboardType = "level" | "revenue" | "incidents" | "projects";

export default function LeaderboardPage() {
  const [leaderboard, setLeaderboard] = useState<LeaderboardEntry[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [activeType, setActiveType] = useState<LeaderboardType>("level");

  useEffect(() => {
    loadLeaderboard(activeType);
  }, [activeType]);

  async function loadLeaderboard(type: LeaderboardType) {
    setLoading(true);
    try {
      let data: LeaderboardEntry[];
      switch (type) {
        case "level":
          data = await api.getLevelLeaderboard();
          break;
        case "revenue":
          data = await api.getRevenueLeaderboard();
          break;
        case "incidents":
          data = await api.getIncidentsLeaderboard();
          break;
        case "projects":
          data = await api.getProjectsLeaderboard();
          break;
      }
      setLeaderboard(data);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to load leaderboard");
    } finally {
      setLoading(false);
    }
  }

  const tabs: { id: LeaderboardType; label: string }[] = [
    { id: "level", label: "Level" },
    { id: "revenue", label: "Revenue" },
    { id: "incidents", label: "Incidents" },
    { id: "projects", label: "Projects" },
  ];

  const getRankBadge = (rank: number) => {
    switch (rank) {
      case 1:
        return "bg-gradient-to-r from-yellow-400 to-yellow-600 text-white";
      case 2:
        return "bg-gradient-to-r from-gray-300 to-gray-500 text-white";
      case 3:
        return "bg-gradient-to-r from-orange-400 to-orange-600 text-white";
      default:
        return "bg-gray-100 text-gray-700";
    }
  };

  const getValueLabel = (type: LeaderboardType, value: number) => {
    switch (type) {
      case "level":
        return `Level ${value}`;
      case "revenue":
        return `$${value.toLocaleString()}`;
      case "incidents":
        return `${value} resolved`;
      case "projects":
        return `${value} completed`;
    }
  };

  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-3xl font-bold text-gray-900">Leaderboard</h1>
        <p className="text-gray-600 mt-1">Top performing engineers</p>
      </div>

      {/* Tabs */}
      <div className="flex space-x-1 bg-gray-100 p-1 rounded-lg w-fit">
        {tabs.map((tab) => (
          <button
            key={tab.id}
            onClick={() => setActiveType(tab.id)}
            className={`px-4 py-2 rounded-md text-sm font-medium transition ${
              activeType === tab.id
                ? "bg-white text-indigo-600 shadow"
                : "text-gray-600 hover:text-gray-900"
            }`}
          >
            {tab.label}
          </button>
        ))}
      </div>

      {error && (
        <div className="bg-red-50 border border-red-200 rounded-lg p-4">
          <p className="text-red-800">{error}</p>
        </div>
      )}

      {loading ? (
        <div className="flex justify-center items-center h-64">
          <div className="animate-spin rounded-full h-12 w-12 border-b-2 border-indigo-600"></div>
        </div>
      ) : leaderboard.length === 0 ? (
        <div className="bg-white rounded-xl shadow-md p-8 text-center text-gray-500">
          No data available
        </div>
      ) : (
        <div className="bg-white rounded-xl shadow-md overflow-hidden">
          <table className="w-full">
            <thead className="bg-gray-50">
              <tr>
                <th className="px-6 py-4 text-left text-xs font-medium text-gray-500 uppercase">Rank</th>
                <th className="px-6 py-4 text-left text-xs font-medium text-gray-500 uppercase">Engineer</th>
                <th className="px-6 py-4 text-right text-xs font-medium text-gray-500 uppercase">Score</th>
              </tr>
            </thead>
            <tbody className="divide-y">
              {leaderboard.map((entry) => (
                <tr key={entry.engineer_id} className="hover:bg-gray-50">
                  <td className="px-6 py-4">
                    <span className={`inline-flex items-center justify-center w-8 h-8 rounded-full text-sm font-bold ${getRankBadge(entry.rank)}`}>
                      {entry.rank}
                    </span>
                  </td>
                  <td className="px-6 py-4">
                    <div className="flex items-center">
                      <div className="w-10 h-10 rounded-full bg-gradient-to-br from-indigo-500 to-purple-600 flex items-center justify-center text-white font-bold mr-3">
                        {entry.engineer_email.charAt(0).toUpperCase()}
                      </div>
                      <div>
                        <p className="font-medium text-gray-900">{entry.engineer_email}</p>
                        {entry.level && activeType !== "level" && (
                          <p className="text-sm text-gray-500">Level {entry.level}</p>
                        )}
                      </div>
                    </div>
                  </td>
                  <td className="px-6 py-4 text-right">
                    <span className="text-lg font-bold text-indigo-600">
                      {getValueLabel(activeType, entry.value)}
                    </span>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </div>
  );
}
