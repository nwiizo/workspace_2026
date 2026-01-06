"use client";

import { useEffect, useState } from "react";
import Link from "next/link";
import api, { Engineer } from "@/lib/api";

export default function EngineersPage() {
  const [engineers, setEngineers] = useState<Engineer[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [salaryInfo, setSalaryInfo] = useState<{ total_monthly_salary: number; engineer_count: number } | null>(null);

  useEffect(() => {
    const cookieRow = document.cookie
      .split("; ")
      .find((row) => row.startsWith("auth_token="));
    const token = cookieRow ? cookieRow.substring("auth_token=".length) : null;
    if (token) api.setToken(token);

    loadEngineers();
  }, []);

  async function loadEngineers() {
    try {
      const [engineerData, salaryData] = await Promise.all([
        api.getEngineers(),
        api.getTotalSalary().catch(() => null),
      ]);
      setEngineers(engineerData);
      setSalaryInfo(salaryData);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to load engineers");
    } finally {
      setLoading(false);
    }
  }

  const getProficiencyColor = (proficiency: string) => {
    switch (proficiency) {
      case "expert": return "bg-purple-100 text-purple-800";
      case "intermediate": return "bg-blue-100 text-blue-800";
      case "beginner": return "bg-gray-100 text-gray-800";
      default: return "bg-gray-100 text-gray-800";
    }
  };

  const getSatisfactionColor = (satisfaction: number) => {
    if (satisfaction >= 70) return "text-green-600";
    if (satisfaction >= 40) return "text-yellow-600";
    return "text-red-600";
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
          <h1 className="text-3xl font-bold text-gray-900">Engineers</h1>
          <p className="text-gray-600 mt-1">Manage your engineering team</p>
        </div>
        <div className="flex items-center gap-4">
          {salaryInfo && (
            <div className="text-right">
              <p className="text-sm text-gray-500">Monthly Total</p>
              <p className="text-xl font-bold text-indigo-600">${salaryInfo.total_monthly_salary.toLocaleString()}</p>
            </div>
          )}
          <Link
            href="/recruitment"
            className="px-4 py-2 bg-indigo-600 text-white rounded-lg hover:bg-indigo-700 transition"
          >
            Hire New
          </Link>
        </div>
      </div>

      {error && (
        <div className="bg-red-50 border border-red-200 rounded-lg p-4">
          <p className="text-red-800">{error}</p>
        </div>
      )}

      {engineers.length === 0 ? (
        <div className="bg-white rounded-xl shadow-md p-8 text-center">
          <p className="text-gray-500 mb-4">No engineers yet.</p>
          <Link href="/recruitment" className="text-indigo-600 hover:text-indigo-800 font-medium">
            Go to Recruitment to hire engineers
          </Link>
        </div>
      ) : (
        <div className="grid md:grid-cols-2 lg:grid-cols-3 gap-6">
          {engineers.map((engineer) => (
            <div key={engineer.id} className="bg-white rounded-xl shadow-md p-6 hover:shadow-lg transition">
              <div className="flex items-center mb-4">
                <div className="w-12 h-12 rounded-full bg-gradient-to-br from-indigo-500 to-purple-600 flex items-center justify-center text-white font-bold text-lg mr-4">
                  {engineer.email.charAt(0).toUpperCase()}
                </div>
                <div className="flex-1 min-w-0">
                  <h3 className="font-semibold text-gray-900 truncate">{engineer.email}</h3>
                  <div className="flex items-center gap-2 mt-1">
                    <span className="text-sm font-medium text-indigo-600">Lv. {engineer.level}</span>
                    <div className="flex-1 bg-gray-200 rounded-full h-1.5">
                      <div
                        className="bg-indigo-600 h-1.5 rounded-full transition-all"
                        style={{ width: `${Math.min(100, (engineer.xp / engineer.xp_to_next_level) * 100)}%` }}
                      />
                    </div>
                  </div>
                </div>
              </div>

              <div className="grid grid-cols-2 gap-4 mb-4">
                <div className="text-center p-2 bg-gray-50 rounded-lg">
                  <p className="text-xs text-gray-500">Satisfaction</p>
                  <p className={`text-lg font-bold ${getSatisfactionColor(engineer.satisfaction)}`}>
                    {engineer.satisfaction}%
                  </p>
                </div>
                <div className="text-center p-2 bg-gray-50 rounded-lg">
                  <p className="text-xs text-gray-500">Salary</p>
                  <p className="text-lg font-bold text-gray-900">${engineer.salary.toLocaleString()}</p>
                </div>
              </div>

              <div className="flex justify-between text-sm text-gray-600 mb-4">
                <span>Incidents: {engineer.resolved_incidents}</span>
                <span>Projects: {engineer.completed_projects}</span>
              </div>

              {engineer.specialties && engineer.specialties.length > 0 && (
                <div className="flex flex-wrap gap-1">
                  {engineer.specialties.map((spec) => (
                    <span
                      key={spec.specialty_id}
                      className={`px-2 py-0.5 rounded-full text-xs font-medium ${getProficiencyColor(spec.proficiency)}`}
                      style={{ borderLeft: `3px solid ${spec.specialty_color}` }}
                    >
                      {spec.specialty_name}
                    </span>
                  ))}
                </div>
              )}
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
