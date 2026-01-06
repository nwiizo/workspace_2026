"use client";

import { useEffect, useState } from "react";
import Link from "next/link";
import { api, Tenant } from "@/lib/api";

export default function TenantsPage() {
  const [tenants, setTenants] = useState<Tenant[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [showCreateForm, setShowCreateForm] = useState(false);
  const [creating, setCreating] = useState(false);

  // Form state
  const [newTenant, setNewTenant] = useState({
    slug: "",
    name: "",
    plan: "free",
  });

  useEffect(() => {
    loadTenants();
  }, []);

  function setupApi() {
    const cookieRow = document.cookie
      .split("; ")
      .find((row) => row.startsWith("auth_token="));
    const token = cookieRow ? cookieRow.substring("auth_token=".length) : null;
    if (token) api.setToken(token);
  }

  async function loadTenants() {
    try {
      setupApi();
      const data = await api.getTenants();
      setTenants(data);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to load tenants");
    } finally {
      setLoading(false);
    }
  }

  async function handleCreateTenant(e: React.FormEvent) {
    e.preventDefault();
    setCreating(true);
    setError(null);

    try {
      setupApi();
      await api.createTenant(newTenant);
      setNewTenant({ slug: "", name: "", plan: "free" });
      setShowCreateForm(false);
      loadTenants();
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to create tenant");
    } finally {
      setCreating(false);
    }
  }

  async function handleDeleteTenant(tenantId: string) {
    if (!confirm("Are you sure you want to delete this tenant?")) return;

    try {
      setupApi();
      await api.deleteTenant(tenantId);
      loadTenants();
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to delete tenant");
    }
  }

  if (loading) {
    return (
      <div className="min-h-screen bg-gray-50 flex items-center justify-center">
        <div className="animate-spin rounded-full h-12 w-12 border-b-2 border-indigo-600"></div>
      </div>
    );
  }

  return (
    <div className="min-h-screen bg-gray-50">
      {/* Header */}
      <header className="bg-gradient-to-r from-gray-800 to-gray-900 text-white">
        <div className="max-w-7xl mx-auto px-4 py-6">
          <div className="flex justify-between items-center">
            <div>
              <h1 className="text-2xl font-bold">Platform Administration</h1>
              <p className="text-gray-300">Manage tenants and platform settings</p>
            </div>
            <Link
              href="/"
              className="px-4 py-2 bg-white/20 rounded-md hover:bg-white/30"
            >
              Back to Home
            </Link>
          </div>
        </div>
      </header>

      <div className="max-w-7xl mx-auto px-4 py-8">
        {error && (
          <div className="bg-red-50 text-red-700 p-4 rounded-md mb-6">
            {error}
          </div>
        )}

        {/* Actions */}
        <div className="flex justify-between items-center mb-6">
          <h2 className="text-xl font-semibold text-gray-900">Tenants</h2>
          <button
            onClick={() => setShowCreateForm(!showCreateForm)}
            className="px-4 py-2 bg-indigo-600 text-white rounded-md hover:bg-indigo-700"
          >
            {showCreateForm ? "Cancel" : "Create Tenant"}
          </button>
        </div>

        {/* Create Form */}
        {showCreateForm && (
          <div className="bg-white rounded-lg shadow p-6 mb-6">
            <h3 className="text-lg font-semibold mb-4">Create New Tenant</h3>
            <form onSubmit={handleCreateTenant} className="space-y-4">
              <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
                <div>
                  <label className="block text-sm font-medium text-gray-700 mb-1">
                    Slug (URL-friendly)
                  </label>
                  <input
                    type="text"
                    value={newTenant.slug}
                    onChange={(e) =>
                      setNewTenant({ ...newTenant, slug: e.target.value })
                    }
                    placeholder="my-shop"
                    pattern="[a-z0-9-]+"
                    required
                    className="w-full border border-gray-300 rounded-md p-2 focus:ring-2 focus:ring-indigo-500"
                  />
                </div>
                <div>
                  <label className="block text-sm font-medium text-gray-700 mb-1">
                    Name
                  </label>
                  <input
                    type="text"
                    value={newTenant.name}
                    onChange={(e) =>
                      setNewTenant({ ...newTenant, name: e.target.value })
                    }
                    placeholder="My Shop"
                    required
                    className="w-full border border-gray-300 rounded-md p-2 focus:ring-2 focus:ring-indigo-500"
                  />
                </div>
                <div>
                  <label className="block text-sm font-medium text-gray-700 mb-1">
                    Plan
                  </label>
                  <select
                    value={newTenant.plan}
                    onChange={(e) =>
                      setNewTenant({ ...newTenant, plan: e.target.value })
                    }
                    className="w-full border border-gray-300 rounded-md p-2 focus:ring-2 focus:ring-indigo-500"
                  >
                    <option value="free">Free</option>
                    <option value="basic">Basic</option>
                    <option value="premium">Premium</option>
                    <option value="enterprise">Enterprise</option>
                  </select>
                </div>
              </div>
              <div className="flex justify-end">
                <button
                  type="submit"
                  disabled={creating}
                  className="px-4 py-2 bg-indigo-600 text-white rounded-md hover:bg-indigo-700 disabled:opacity-50"
                >
                  {creating ? "Creating..." : "Create Tenant"}
                </button>
              </div>
            </form>
          </div>
        )}

        {/* Tenants List */}
        {tenants.length === 0 ? (
          <div className="bg-white rounded-lg shadow p-8 text-center">
            <div className="text-6xl mb-4">🏪</div>
            <h3 className="text-xl font-semibold text-gray-900 mb-2">
              No tenants yet
            </h3>
            <p className="text-gray-600">
              Create your first tenant to get started.
            </p>
          </div>
        ) : (
          <div className="bg-white rounded-lg shadow overflow-hidden">
            <table className="w-full">
              <thead className="bg-gray-50">
                <tr>
                  <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase">
                    Name
                  </th>
                  <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase">
                    Slug
                  </th>
                  <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase">
                    Plan
                  </th>
                  <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase">
                    Status
                  </th>
                  <th className="px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase">
                    Created
                  </th>
                  <th className="px-6 py-3 text-right text-xs font-medium text-gray-500 uppercase">
                    Actions
                  </th>
                </tr>
              </thead>
              <tbody className="divide-y">
                {tenants.map((tenant) => (
                  <tr key={tenant.id} className="hover:bg-gray-50">
                    <td className="px-6 py-4 font-medium text-gray-900">
                      {tenant.name}
                    </td>
                    <td className="px-6 py-4 font-mono text-sm text-gray-600">
                      {tenant.slug}
                    </td>
                    <td className="px-6 py-4 capitalize">{tenant.plan}</td>
                    <td className="px-6 py-4">
                      <span
                        className={`px-2 py-1 rounded-full text-xs font-medium ${
                          tenant.status === "active"
                            ? "bg-green-100 text-green-800"
                            : "bg-gray-100 text-gray-800"
                        }`}
                      >
                        {tenant.status}
                      </span>
                    </td>
                    <td className="px-6 py-4 text-gray-500 text-sm">
                      {new Date(tenant.created_at).toLocaleDateString()}
                    </td>
                    <td className="px-6 py-4 text-right">
                      <button
                        onClick={() => handleDeleteTenant(tenant.id)}
                        className="text-red-600 hover:text-red-800"
                      >
                        Delete
                      </button>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </div>
    </div>
  );
}
