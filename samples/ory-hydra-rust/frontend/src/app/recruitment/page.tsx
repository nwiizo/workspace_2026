"use client";

import { useEffect, useState } from "react";
import api, { CandidateWithDetails } from "@/lib/api";

export default function RecruitmentPage() {
  const [candidates, setCandidates] = useState<CandidateWithDetails[]>([]);
  const [loading, setLoading] = useState(true);
  const [refreshing, setRefreshing] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [refreshStatus, setRefreshStatus] = useState<{ can_free_refresh: boolean; refresh_cost: number } | null>(null);

  useEffect(() => {
    const token = document.cookie
      .split("; ")
      .find((row) => row.startsWith("auth_token="))
      ?.split("=")[1];
    if (token) api.setToken(token);

    loadData();
  }, []);

  async function loadData() {
    try {
      const [candidatesData, statusData] = await Promise.all([
        api.getCandidates(),
        api.getRefreshStatus().catch(() => null),
      ]);
      setCandidates(candidatesData);
      setRefreshStatus(statusData);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to load candidates");
    } finally {
      setLoading(false);
    }
  }

  async function handleRefresh() {
    setRefreshing(true);
    try {
      await api.refreshPool();
      await loadData();
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to refresh pool");
    } finally {
      setRefreshing(false);
    }
  }

  async function handleHire(candidateId: string) {
    try {
      await api.hireCandidate({ candidate_id: candidateId });
      await loadData();
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to hire candidate");
    }
  }

  const getRarityColor = (rarity: string) => {
    switch (rarity) {
      case "legendary": return "bg-gradient-to-r from-yellow-400 to-orange-500 text-white";
      case "epic": return "bg-purple-600 text-white";
      case "rare": return "bg-blue-600 text-white";
      case "uncommon": return "bg-green-600 text-white";
      default: return "bg-gray-400 text-white";
    }
  };

  const getRarityBorder = (rarity: string) => {
    switch (rarity) {
      case "legendary": return "border-2 border-yellow-400";
      case "epic": return "border-2 border-purple-500";
      case "rare": return "border-2 border-blue-500";
      case "uncommon": return "border-2 border-green-500";
      default: return "border border-gray-200";
    }
  };

  if (loading) {
    return (
      <div className="flex justify-center items-center h-64">
        <div className="animate-spin rounded-full h-12 w-12 border-b-2 border-indigo-600"></div>
      </div>
    );
  }

  return (
    <div className="space-y-6">
      <div className="flex justify-between items-center">
        <div>
          <h1 className="text-3xl font-bold text-gray-900">Recruitment</h1>
          <p className="text-gray-600 mt-1">Hire new engineers for your team</p>
        </div>
        <button
          onClick={handleRefresh}
          disabled={refreshing}
          className="px-4 py-2 bg-indigo-600 text-white rounded-lg hover:bg-indigo-700 transition disabled:opacity-50"
        >
          {refreshing ? "Refreshing..." : refreshStatus?.can_free_refresh ? "Free Refresh" : `Refresh ($${refreshStatus?.refresh_cost ?? 5000})`}
        </button>
      </div>

      {error && (
        <div className="bg-red-50 border border-red-200 rounded-lg p-4">
          <p className="text-red-800">{error}</p>
          <button onClick={() => setError(null)} className="text-red-600 hover:text-red-800 text-sm mt-1">
            Dismiss
          </button>
        </div>
      )}

      {candidates.length === 0 ? (
        <div className="bg-white rounded-xl shadow-md p-8 text-center">
          <p className="text-gray-500 mb-4">No candidates available.</p>
          <button onClick={handleRefresh} className="text-indigo-600 hover:text-indigo-800 font-medium">
            Refresh the candidate pool
          </button>
        </div>
      ) : (
        <div className="grid md:grid-cols-2 lg:grid-cols-3 gap-6">
          {candidates.map((candidate) => (
            <div
              key={candidate.id}
              className={`bg-white rounded-xl shadow-md overflow-hidden ${getRarityBorder(candidate.rarity)}`}
            >
              <div className={`px-4 py-2 ${getRarityColor(candidate.rarity)} flex justify-between items-center`}>
                <span className="font-medium capitalize">{candidate.rarity}</span>
                <span className="text-sm">Lv. {candidate.level}</span>
              </div>
              <div className="p-4">
                <h3 className="font-bold text-lg text-gray-900 mb-2">{candidate.name}</h3>

                <div className="space-y-2 mb-4">
                  <div className="flex items-center gap-2">
                    <span
                      className="w-3 h-3 rounded-full"
                      style={{ backgroundColor: candidate.primary_specialty_color }}
                    />
                    <span className="text-sm text-gray-700">{candidate.primary_specialty_name}</span>
                    <span className="text-xs text-gray-500 capitalize">({candidate.primary_proficiency})</span>
                  </div>
                  {candidate.secondary_specialty_name && (
                    <div className="flex items-center gap-2">
                      <span
                        className="w-3 h-3 rounded-full"
                        style={{ backgroundColor: candidate.secondary_specialty_color }}
                      />
                      <span className="text-sm text-gray-700">{candidate.secondary_specialty_name}</span>
                      <span className="text-xs text-gray-500 capitalize">({candidate.secondary_proficiency})</span>
                    </div>
                  )}
                </div>

                {candidate.trait_name && (
                  <div className="mb-4 p-2 bg-indigo-50 rounded-lg">
                    <p className="text-xs font-medium text-indigo-700">{candidate.trait_name}</p>
                    <p className="text-xs text-indigo-600">{candidate.trait_description}</p>
                  </div>
                )}

                <div className="grid grid-cols-2 gap-2 text-sm mb-4">
                  <div>
                    <p className="text-gray-500">Hiring Cost</p>
                    <p className="font-semibold">${candidate.hiring_cost.toLocaleString()}</p>
                  </div>
                  <div>
                    <p className="text-gray-500">Salary</p>
                    <p className="font-semibold">${candidate.expected_salary.toLocaleString()}/mo</p>
                  </div>
                </div>

                <button
                  onClick={() => handleHire(candidate.id)}
                  disabled={!candidate.can_afford}
                  className={`w-full py-2 rounded-lg font-medium transition ${
                    candidate.can_afford
                      ? "bg-indigo-600 text-white hover:bg-indigo-700"
                      : "bg-gray-200 text-gray-500 cursor-not-allowed"
                  }`}
                >
                  {candidate.can_afford ? "Hire" : "Cannot Afford"}
                </button>
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
