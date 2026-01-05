const API_BASE_URL = process.env.NEXT_PUBLIC_API_BASE_URL || "http://localhost:3000";

// ============= Types =============

export interface Tenant {
  id: string;
  slug: string;
  name: string;
  schema_name: string;
  plan: string;
  status: string;
  created_at: string;
  updated_at: string;
}

// Incidents
export interface Incident {
  id: string;
  title: string;
  description?: string;
  severity: "critical" | "high" | "medium" | "low";
  status: string;
  status_id: string;
  assigned_engineer_id?: string;
  reporter_id: string;
  difficulty?: string;
  xp_reward?: number;
  revenue?: number;
  created_at: string;
  updated_at: string;
  resolved_at?: string;
  closed_at?: string;
}

export interface IncidentStatistics {
  total: number;
  open: number;
  in_progress: number;
  resolved: number;
  closed: number;
  by_severity: Record<string, number>;
}

// Projects
export interface Project {
  id: string;
  title: string;
  description?: string;
  status: string;
  status_id: string;
  priority: "high" | "medium" | "low";
  deadline?: string;
  estimated_hours?: number;
  actual_hours?: number;
  difficulty?: string;
  xp_reward?: number;
  revenue?: number;
  created_at: string;
  updated_at: string;
  completed_at?: string;
}

export interface ProjectStatistics {
  total: number;
  backlog: number;
  in_progress: number;
  completed: number;
  by_priority: Record<string, number>;
  total_estimated_hours: number;
  total_actual_hours: number;
}

// Engineers
export interface Engineer {
  id: string;
  email: string;
  level: number;
  xp: number;
  xp_to_next_level: number;
  satisfaction: number;
  salary: number;
  total_revenue: number;
  completed_projects: number;
  resolved_incidents: number;
  is_active: boolean;
  specialties?: EngineerSpecialty[];
}

export interface EngineerSpecialty {
  specialty_id: string;
  specialty_name: string;
  specialty_color: string;
  proficiency: "beginner" | "intermediate" | "expert";
}

// Recruitment
export interface Candidate {
  id: string;
  name: string;
  avatar: string;
  rarity: "common" | "uncommon" | "rare" | "epic" | "legendary";
  level: number;
  primary_specialty_id: string;
  primary_proficiency: string;
  secondary_specialty_id?: string;
  secondary_proficiency?: string;
  expected_salary: number;
  hiring_cost: number;
  base_satisfaction: number;
  trait_name?: string;
  trait_description?: string;
  status: string;
  expires_at?: string;
}

export interface CandidateWithDetails extends Candidate {
  can_afford: boolean;
  primary_specialty_name: string;
  primary_specialty_color: string;
  secondary_specialty_name?: string;
  secondary_specialty_color?: string;
}

export interface HireResult {
  engineer_id: string;
  candidate_id: string;
  hiring_cost: number;
  monthly_salary: number;
  new_balance: number;
}

// Leaderboard
export interface LeaderboardEntry {
  rank: number;
  engineer_id: string;
  engineer_email: string;
  value: number;
  level?: number;
}

// Requests
export interface CreateIncidentRequest {
  title: string;
  description?: string;
  severity: string;
}

export interface UpdateIncidentRequest {
  title?: string;
  description?: string;
  severity?: string;
}

export interface AssignRequest {
  engineer_id: string;
  role?: string;
}

export interface ChangeStatusRequest {
  status_id: string;
}

export interface CreateProjectRequest {
  title: string;
  description?: string;
  priority?: string;
  deadline?: string;
  estimated_hours?: number;
}

export interface UpdateProjectRequest {
  title?: string;
  description?: string;
  priority?: string;
  deadline?: string;
  estimated_hours?: number;
}

export interface UpdateHoursRequest {
  hours: number;
}

export interface HireCandidateRequest {
  candidate_id: string;
  negotiated_salary?: number;
  email?: string;
}

export interface AddSpecialtyRequest {
  specialty_id: string;
  proficiency: string;
}

// ============= API Client =============

class ApiClient {
  private baseUrl: string;
  private token?: string;

  constructor(baseUrl: string = API_BASE_URL) {
    this.baseUrl = baseUrl;
  }

  setToken(token: string) {
    this.token = token;
  }

  clearToken() {
    this.token = undefined;
  }

  private async fetch<T>(
    endpoint: string,
    options: RequestInit = {}
  ): Promise<T> {
    const headers: HeadersInit = {
      "Content-Type": "application/json",
      ...options.headers,
    };

    if (this.token) {
      (headers as Record<string, string>)["Authorization"] = `Bearer ${this.token}`;
    }

    const response = await fetch(`${this.baseUrl}${endpoint}`, {
      ...options,
      headers,
    });

    if (!response.ok) {
      const error = await response.json().catch(() => ({}));
      throw new Error(error.error || error.message || `API Error: ${response.status}`);
    }

    // Handle empty responses
    const text = await response.text();
    if (!text) return {} as T;
    return JSON.parse(text);
  }

  // ============= Tenants (Platform Admin) =============
  async getTenants(): Promise<Tenant[]> {
    return this.fetch<Tenant[]>("/api/v1/tenants");
  }

  async getTenant(tenantId: string): Promise<Tenant> {
    return this.fetch<Tenant>(`/api/v1/tenants/${tenantId}`);
  }

  async createTenant(tenant: { slug: string; name: string; plan?: string }): Promise<Tenant> {
    return this.fetch<Tenant>("/api/v1/tenants", {
      method: "POST",
      body: JSON.stringify(tenant),
    });
  }

  async updateTenant(tenantId: string, tenant: Partial<Tenant>): Promise<Tenant> {
    return this.fetch<Tenant>(`/api/v1/tenants/${tenantId}`, {
      method: "PUT",
      body: JSON.stringify(tenant),
    });
  }

  async deleteTenant(tenantId: string): Promise<void> {
    await this.fetch<void>(`/api/v1/tenants/${tenantId}`, {
      method: "DELETE",
    });
  }

  // ============= Incidents =============
  async getIncidents(): Promise<Incident[]> {
    return this.fetch<Incident[]>("/api/v1/tenant/incidents");
  }

  async getIncident(id: string): Promise<Incident> {
    return this.fetch<Incident>(`/api/v1/tenant/incidents/${id}`);
  }

  async createIncident(data: CreateIncidentRequest): Promise<Incident> {
    return this.fetch<Incident>("/api/v1/tenant/incidents", {
      method: "POST",
      body: JSON.stringify(data),
    });
  }

  async updateIncident(id: string, data: UpdateIncidentRequest): Promise<Incident> {
    return this.fetch<Incident>(`/api/v1/tenant/incidents/${id}`, {
      method: "PUT",
      body: JSON.stringify(data),
    });
  }

  async deleteIncident(id: string): Promise<void> {
    await this.fetch<void>(`/api/v1/tenant/incidents/${id}`, {
      method: "DELETE",
    });
  }

  async assignIncident(id: string, data: AssignRequest): Promise<Incident> {
    return this.fetch<Incident>(`/api/v1/tenant/incidents/${id}/assign`, {
      method: "POST",
      body: JSON.stringify(data),
    });
  }

  async changeIncidentStatus(id: string, data: ChangeStatusRequest): Promise<Incident> {
    return this.fetch<Incident>(`/api/v1/tenant/incidents/${id}/status`, {
      method: "PATCH",
      body: JSON.stringify(data),
    });
  }

  async getIncidentStats(): Promise<IncidentStatistics> {
    return this.fetch<IncidentStatistics>("/api/v1/tenant/incidents/stats");
  }

  // ============= Projects =============
  async getProjects(): Promise<Project[]> {
    return this.fetch<Project[]>("/api/v1/tenant/projects");
  }

  async getProject(id: string): Promise<Project> {
    return this.fetch<Project>(`/api/v1/tenant/projects/${id}`);
  }

  async createProject(data: CreateProjectRequest): Promise<Project> {
    return this.fetch<Project>("/api/v1/tenant/projects", {
      method: "POST",
      body: JSON.stringify(data),
    });
  }

  async updateProject(id: string, data: UpdateProjectRequest): Promise<Project> {
    return this.fetch<Project>(`/api/v1/tenant/projects/${id}`, {
      method: "PUT",
      body: JSON.stringify(data),
    });
  }

  async deleteProject(id: string): Promise<void> {
    await this.fetch<void>(`/api/v1/tenant/projects/${id}`, {
      method: "DELETE",
    });
  }

  async assignProject(id: string, data: AssignRequest): Promise<Project> {
    return this.fetch<Project>(`/api/v1/tenant/projects/${id}/assign`, {
      method: "POST",
      body: JSON.stringify(data),
    });
  }

  async changeProjectStatus(id: string, data: ChangeStatusRequest): Promise<Project> {
    return this.fetch<Project>(`/api/v1/tenant/projects/${id}/status`, {
      method: "PATCH",
      body: JSON.stringify(data),
    });
  }

  async updateProjectHours(id: string, data: UpdateHoursRequest): Promise<Project> {
    return this.fetch<Project>(`/api/v1/tenant/projects/${id}/hours`, {
      method: "PATCH",
      body: JSON.stringify(data),
    });
  }

  async getProjectStats(): Promise<ProjectStatistics> {
    return this.fetch<ProjectStatistics>("/api/v1/tenant/projects/stats");
  }

  // ============= Engineers =============
  async getEngineers(): Promise<Engineer[]> {
    return this.fetch<Engineer[]>("/api/v1/tenant/engineers");
  }

  async getEngineer(id: string): Promise<Engineer> {
    return this.fetch<Engineer>(`/api/v1/tenant/engineers/${id}`);
  }

  async addEngineerSpecialty(id: string, data: AddSpecialtyRequest): Promise<void> {
    await this.fetch<void>(`/api/v1/tenant/engineers/${id}/specialties`, {
      method: "POST",
      body: JSON.stringify(data),
    });
  }

  async fireEngineer(id: string, reason: string): Promise<void> {
    await this.fetch<void>(`/api/v1/tenant/engineers/${id}/fire`, {
      method: "POST",
      body: JSON.stringify({ reason }),
    });
  }

  async getTotalSalary(): Promise<{ total_monthly_salary: number; engineer_count: number }> {
    return this.fetch("/api/v1/tenant/engineers/salary-total");
  }

  // ============= Recruitment =============
  async getCandidates(): Promise<CandidateWithDetails[]> {
    return this.fetch<CandidateWithDetails[]>("/api/v1/tenant/recruitment/candidates");
  }

  async getCandidate(id: string): Promise<Candidate> {
    return this.fetch<Candidate>(`/api/v1/tenant/recruitment/candidates/${id}`);
  }

  async refreshPool(): Promise<Candidate[]> {
    return this.fetch<Candidate[]>("/api/v1/tenant/recruitment/refresh", {
      method: "POST",
    });
  }

  async hireCandidate(data: HireCandidateRequest): Promise<HireResult> {
    return this.fetch<HireResult>("/api/v1/tenant/recruitment/hire", {
      method: "POST",
      body: JSON.stringify(data),
    });
  }

  async getRefreshStatus(): Promise<{ can_free_refresh: boolean; refresh_cost: number }> {
    return this.fetch("/api/v1/tenant/recruitment/status");
  }

  // ============= Leaderboard =============
  async getLeaderboard(type: string = "level", limit: number = 10): Promise<LeaderboardEntry[]> {
    return this.fetch<LeaderboardEntry[]>(`/api/v1/tenant/leaderboard?leaderboard_type=${type}&limit=${limit}`);
  }

  async getLevelLeaderboard(limit: number = 10): Promise<LeaderboardEntry[]> {
    return this.fetch<LeaderboardEntry[]>(`/api/v1/tenant/leaderboard/level?limit=${limit}`);
  }

  async getRevenueLeaderboard(limit: number = 10): Promise<LeaderboardEntry[]> {
    return this.fetch<LeaderboardEntry[]>(`/api/v1/tenant/leaderboard/revenue?limit=${limit}`);
  }

  async getIncidentsLeaderboard(limit: number = 10): Promise<LeaderboardEntry[]> {
    return this.fetch<LeaderboardEntry[]>(`/api/v1/tenant/leaderboard/incidents?limit=${limit}`);
  }

  async getProjectsLeaderboard(limit: number = 10): Promise<LeaderboardEntry[]> {
    return this.fetch<LeaderboardEntry[]>(`/api/v1/tenant/leaderboard/projects?limit=${limit}`);
  }
}

export const api = new ApiClient();
export default api;
