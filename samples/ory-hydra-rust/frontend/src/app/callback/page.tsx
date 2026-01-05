"use client";

import { Suspense, useEffect, useState } from "react";
import { useSearchParams } from "next/navigation";

function CallbackContent() {
  const searchParams = useSearchParams();
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    async function handleCallback() {
      const code = searchParams.get("code");
      const state = searchParams.get("state");
      const errorParam = searchParams.get("error");

      if (errorParam) {
        setError(searchParams.get("error_description") || errorParam);
        setLoading(false);
        return;
      }

      if (!code) {
        setError("No authorization code received");
        setLoading(false);
        return;
      }

      try {
        // Exchange code for tokens via our API
        const response = await fetch("/api/auth/callback", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ code }),
        });

        if (!response.ok) {
          const data = await response.json();
          throw new Error(data.error || "Failed to exchange code");
        }

        // Decode redirect URL from state (format: randomBytes:base64EncodedUrl)
        let redirectTo = "/";
        if (state) {
          try {
            const parts = state.split(":");
            if (parts.length >= 2) {
              redirectTo = atob(parts.slice(1).join(":")) || "/";
            }
          } catch {
            // If decoding fails, redirect to home
            redirectTo = "/";
          }
        }

        // Use full page reload to ensure cookies are read by Header component
        window.location.href = redirectTo;
      } catch (err) {
        setError(err instanceof Error ? err.message : "Authentication failed");
        setLoading(false);
      }
    }

    handleCallback();
  }, [searchParams]);

  if (loading) {
    return (
      <div className="min-h-screen flex items-center justify-center bg-gray-50">
        <div className="text-center">
          <div className="animate-spin rounded-full h-12 w-12 border-b-2 border-indigo-600 mx-auto mb-4"></div>
          <p className="text-gray-600">Completing authentication...</p>
        </div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="min-h-screen flex items-center justify-center bg-gray-50">
        <div className="bg-white p-8 rounded-lg shadow-md max-w-md w-full">
          <div className="text-center">
            <div className="text-red-500 text-5xl mb-4">!</div>
            <h1 className="text-xl font-semibold text-gray-900 mb-2">
              Authentication Failed
            </h1>
            <p className="text-gray-600 mb-6">{error}</p>
            <a
              href="/"
              className="inline-block bg-indigo-600 text-white px-6 py-2 rounded-md hover:bg-indigo-700"
            >
              Return Home
            </a>
          </div>
        </div>
      </div>
    );
  }

  return null;
}

export default function CallbackPage() {
  return (
    <Suspense
      fallback={
        <div className="min-h-screen flex items-center justify-center bg-gray-50">
          <div className="animate-spin rounded-full h-12 w-12 border-b-2 border-indigo-600"></div>
        </div>
      }
    >
      <CallbackContent />
    </Suspense>
  );
}
