"use client";

import Link from "next/link";
import { useSearchParams } from "next/navigation";
import { useEffect, useState, Suspense } from "react";

function ErrorToast({ message, onClose }: { message: string; onClose: () => void }) {
  useEffect(() => {
    const timer = setTimeout(onClose, 5000);
    return () => clearTimeout(timer);
  }, [onClose]);

  return (
    <div className="fixed top-4 right-4 z-50 animate-slide-in">
      <div className="bg-red-500 text-white px-6 py-4 rounded-lg shadow-lg flex items-center gap-3">
        <svg className="w-6 h-6 flex-shrink-0" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" />
        </svg>
        <div>
          <p className="font-semibold">Access Denied</p>
          <p className="text-sm text-red-100">{message}</p>
        </div>
        <button onClick={onClose} className="ml-4 text-red-200 hover:text-white">
          <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
          </svg>
        </button>
      </div>
    </div>
  );
}

function HomeContent() {
  const searchParams = useSearchParams();
  const [showError, setShowError] = useState(false);
  const [errorMessage, setErrorMessage] = useState("");

  useEffect(() => {
    const error = searchParams.get("error");
    if (error === "unauthorized") {
      setErrorMessage("You don't have permission to access that page. Please log in with an authorized account.");
      setShowError(true);
      window.history.replaceState({}, "", "/");
    }
  }, [searchParams]);

  return (
    <>
      {showError && <ErrorToast message={errorMessage} onClose={() => setShowError(false)} />}

      <div className="text-center py-12">
        <h1 className="text-5xl font-bold text-gray-900 mb-4">
          Welcome to <span className="text-indigo-600">DONADONA</span>
        </h1>
        <p className="text-xl text-gray-600 mb-8 max-w-2xl mx-auto">
          A gamified engineer assignment platform for managing incidents and projects
          with skill-based matching and progression systems.
        </p>

        <div className="grid md:grid-cols-3 gap-8 max-w-4xl mx-auto mt-12">
          <div className="bg-white p-6 rounded-xl shadow-md hover:shadow-lg transition">
            <div className="w-12 h-12 bg-red-100 rounded-lg flex items-center justify-center mx-auto mb-4">
              <svg className="w-6 h-6 text-red-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" />
              </svg>
            </div>
            <h3 className="text-lg font-semibold text-gray-900 mb-2">Incident Management</h3>
            <p className="text-gray-600 text-sm">
              Track and resolve incidents with severity-based prioritization and XP rewards.
            </p>
          </div>

          <div className="bg-white p-6 rounded-xl shadow-md hover:shadow-lg transition">
            <div className="w-12 h-12 bg-blue-100 rounded-lg flex items-center justify-center mx-auto mb-4">
              <svg className="w-6 h-6 text-blue-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 5H7a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2V7a2 2 0 00-2-2h-2M9 5a2 2 0 002 2h2a2 2 0 002-2M9 5a2 2 0 012-2h2a2 2 0 012 2" />
              </svg>
            </div>
            <h3 className="text-lg font-semibold text-gray-900 mb-2">Project Tracking</h3>
            <p className="text-gray-600 text-sm">
              Manage projects with deadlines, hour tracking, and team assignments.
            </p>
          </div>

          <div className="bg-white p-6 rounded-xl shadow-md hover:shadow-lg transition">
            <div className="w-12 h-12 bg-green-100 rounded-lg flex items-center justify-center mx-auto mb-4">
              <svg className="w-6 h-6 text-green-600" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M17 20h5v-2a3 3 0 00-5.356-1.857M17 20H7m10 0v-2c0-.656-.126-1.283-.356-1.857M7 20H2v-2a3 3 0 015.356-1.857M7 20v-2c0-.656.126-1.283.356-1.857m0 0a5.002 5.002 0 019.288 0M15 7a3 3 0 11-6 0 3 3 0 016 0zm6 3a2 2 0 11-4 0 2 2 0 014 0zM7 10a2 2 0 11-4 0 2 2 0 014 0z" />
              </svg>
            </div>
            <h3 className="text-lg font-semibold text-gray-900 mb-2">Engineer Growth</h3>
            <p className="text-gray-600 text-sm">
              Level up engineers through assignments, training, and skill development.
            </p>
          </div>
        </div>

        {/* Test Accounts */}
        <div className="mt-16 p-6 bg-white border rounded-xl shadow-sm max-w-3xl mx-auto">
          <h3 className="text-xl font-semibold mb-4 text-gray-900">Test Accounts</h3>
          <p className="text-gray-600 text-sm mb-4">
            All accounts use password: <code className="px-2 py-1 bg-gray-100 rounded font-mono">password123</code>
          </p>
          <div className="overflow-x-auto">
            <table className="w-full text-left text-sm">
              <thead className="bg-gray-50">
                <tr>
                  <th className="px-4 py-3 font-semibold text-gray-700">Role</th>
                  <th className="px-4 py-3 font-semibold text-gray-700">Email</th>
                  <th className="px-4 py-3 font-semibold text-gray-700">Permissions</th>
                </tr>
              </thead>
              <tbody className="divide-y">
                <tr className="hover:bg-gray-50">
                  <td className="px-4 py-3">
                    <span className="px-2 py-1 bg-red-100 text-red-800 rounded-full text-xs font-medium">
                      Platform Admin
                    </span>
                  </td>
                  <td className="px-4 py-3 font-mono text-gray-900">demo@example.com</td>
                  <td className="px-4 py-3 text-gray-600">Full platform access, tenant management</td>
                </tr>
                <tr className="hover:bg-gray-50">
                  <td className="px-4 py-3">
                    <span className="px-2 py-1 bg-purple-100 text-purple-800 rounded-full text-xs font-medium">
                      Manager
                    </span>
                  </td>
                  <td className="px-4 py-3 font-mono text-gray-900">manager@example.com</td>
                  <td className="px-4 py-3 text-gray-600">Manage engineers, assign tasks, hire/fire</td>
                </tr>
                <tr className="hover:bg-gray-50">
                  <td className="px-4 py-3">
                    <span className="px-2 py-1 bg-blue-100 text-blue-800 rounded-full text-xs font-medium">
                      Engineer
                    </span>
                  </td>
                  <td className="px-4 py-3 font-mono text-gray-900">sato@example.com</td>
                  <td className="px-4 py-3 text-gray-600">Work on assigned tasks, update status</td>
                </tr>
                <tr className="hover:bg-gray-50">
                  <td className="px-4 py-3">
                    <span className="px-2 py-1 bg-green-100 text-green-800 rounded-full text-xs font-medium">
                      Reporter
                    </span>
                  </td>
                  <td className="px-4 py-3 font-mono text-gray-900">reporter@example.com</td>
                  <td className="px-4 py-3 text-gray-600">Report incidents only</td>
                </tr>
              </tbody>
            </table>
          </div>
        </div>

        <div className="mt-12">
          <Link
            href="/api/auth/login"
            className="inline-flex items-center px-6 py-3 border border-transparent text-base font-medium rounded-md text-white bg-indigo-600 hover:bg-indigo-700 transition"
          >
            Get Started
          </Link>
        </div>
      </div>
    </>
  );
}

export default function Home() {
  return (
    <Suspense fallback={<div className="min-h-screen flex items-center justify-center">Loading...</div>}>
      <HomeContent />
    </Suspense>
  );
}
