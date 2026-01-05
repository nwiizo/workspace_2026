-- DONADONA Tenant Schema Template
-- This SQL is executed when creating a new tenant
-- Replace {{schema_name}} with the actual schema name (e.g., tenant_abc123)

CREATE SCHEMA IF NOT EXISTS {{schema_name}};

-- ============================================
-- CORE TABLES
-- ============================================

-- Specialties (Engineer skills/domains)
CREATE TABLE {{schema_name}}.specialties (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(100) NOT NULL,
    description TEXT,
    color VARCHAR(7) DEFAULT '#6B7280',
    is_default BOOLEAN DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE UNIQUE INDEX idx_{{schema_name}}_specialties_name ON {{schema_name}}.specialties(name);

-- Engineer specialties (many-to-many with proficiency)
CREATE TABLE {{schema_name}}.engineer_specialties (
    engineer_id UUID NOT NULL REFERENCES public.users(id) ON DELETE CASCADE,
    specialty_id UUID NOT NULL REFERENCES {{schema_name}}.specialties(id) ON DELETE CASCADE,
    proficiency VARCHAR(20) NOT NULL DEFAULT 'intermediate',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (engineer_id, specialty_id),
    CONSTRAINT chk_proficiency CHECK (proficiency IN ('beginner', 'intermediate', 'expert'))
);

CREATE INDEX idx_{{schema_name}}_eng_spec_engineer ON {{schema_name}}.engineer_specialties(engineer_id);
CREATE INDEX idx_{{schema_name}}_eng_spec_specialty ON {{schema_name}}.engineer_specialties(specialty_id);

-- Workflow statuses (custom per entity type)
CREATE TABLE {{schema_name}}.workflow_statuses (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    entity_type VARCHAR(20) NOT NULL,
    name VARCHAR(100) NOT NULL,
    color VARCHAR(7) DEFAULT '#6B7280',
    display_order INT DEFAULT 0,
    is_initial BOOLEAN DEFAULT false,
    is_terminal BOOLEAN DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT chk_entity_type CHECK (entity_type IN ('incident', 'project'))
);

CREATE INDEX idx_{{schema_name}}_wf_entity_type ON {{schema_name}}.workflow_statuses(entity_type);

-- Incidents
CREATE TABLE {{schema_name}}.incidents (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    title VARCHAR(255) NOT NULL,
    description TEXT,
    severity VARCHAR(20) NOT NULL DEFAULT 'medium',
    difficulty VARCHAR(20) NOT NULL DEFAULT 'normal',
    reward BIGINT NOT NULL DEFAULT 0,
    status_id UUID NOT NULL REFERENCES {{schema_name}}.workflow_statuses(id),
    assigned_engineer_id UUID REFERENCES public.users(id) ON DELETE SET NULL,
    reporter_id UUID NOT NULL REFERENCES public.users(id),
    required_specialty_id UUID REFERENCES {{schema_name}}.specialties(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    resolved_at TIMESTAMPTZ,
    closed_at TIMESTAMPTZ,
    CONSTRAINT chk_severity CHECK (severity IN ('critical', 'high', 'medium', 'low')),
    CONSTRAINT chk_difficulty CHECK (difficulty IN ('easy', 'normal', 'hard', 'expert', 'extreme'))
);

CREATE INDEX idx_{{schema_name}}_incidents_status ON {{schema_name}}.incidents(status_id);
CREATE INDEX idx_{{schema_name}}_incidents_assigned ON {{schema_name}}.incidents(assigned_engineer_id);
CREATE INDEX idx_{{schema_name}}_incidents_reporter ON {{schema_name}}.incidents(reporter_id);
CREATE INDEX idx_{{schema_name}}_incidents_severity ON {{schema_name}}.incidents(severity);
CREATE INDEX idx_{{schema_name}}_incidents_created ON {{schema_name}}.incidents(created_at DESC);

-- Projects
CREATE TABLE {{schema_name}}.projects (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    title VARCHAR(255) NOT NULL,
    description TEXT,
    status_id UUID NOT NULL REFERENCES {{schema_name}}.workflow_statuses(id),
    priority VARCHAR(20) NOT NULL DEFAULT 'medium',
    difficulty VARCHAR(20) NOT NULL DEFAULT 'normal',
    reward BIGINT NOT NULL DEFAULT 0,
    deadline TIMESTAMPTZ,
    estimated_hours INT,
    actual_hours INT DEFAULT 0,
    required_specialty_id UUID REFERENCES {{schema_name}}.specialties(id),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at TIMESTAMPTZ,
    CONSTRAINT chk_priority CHECK (priority IN ('high', 'medium', 'low')),
    CONSTRAINT chk_project_difficulty CHECK (difficulty IN ('easy', 'normal', 'hard', 'expert', 'extreme'))
);

CREATE INDEX idx_{{schema_name}}_projects_status ON {{schema_name}}.projects(status_id);
CREATE INDEX idx_{{schema_name}}_projects_priority ON {{schema_name}}.projects(priority);
CREATE INDEX idx_{{schema_name}}_projects_deadline ON {{schema_name}}.projects(deadline);
CREATE INDEX idx_{{schema_name}}_projects_created ON {{schema_name}}.projects(created_at DESC);

-- Assignments (engineers to incidents/projects)
CREATE TABLE {{schema_name}}.assignments (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    assignable_type VARCHAR(20) NOT NULL,
    assignable_id UUID NOT NULL,
    engineer_id UUID NOT NULL REFERENCES public.users(id) ON DELETE CASCADE,
    role_in_assignment VARCHAR(50) NOT NULL DEFAULT 'assignee',
    assigned_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    assigned_by UUID NOT NULL REFERENCES public.users(id),
    CONSTRAINT chk_assignable_type CHECK (assignable_type IN ('incident', 'project'))
);

CREATE INDEX idx_{{schema_name}}_assignments_engineer ON {{schema_name}}.assignments(engineer_id);
CREATE INDEX idx_{{schema_name}}_assignments_entity ON {{schema_name}}.assignments(assignable_type, assignable_id);
CREATE UNIQUE INDEX idx_{{schema_name}}_assignments_unique ON {{schema_name}}.assignments(assignable_type, assignable_id, engineer_id);

-- Comments/Activity log
CREATE TABLE {{schema_name}}.comments (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    entity_type VARCHAR(20) NOT NULL,
    entity_id UUID NOT NULL,
    author_id UUID NOT NULL REFERENCES public.users(id),
    content TEXT NOT NULL,
    is_internal BOOLEAN DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ,
    CONSTRAINT chk_comment_entity_type CHECK (entity_type IN ('incident', 'project'))
);

CREATE INDEX idx_{{schema_name}}_comments_entity ON {{schema_name}}.comments(entity_type, entity_id);
CREATE INDEX idx_{{schema_name}}_comments_author ON {{schema_name}}.comments(author_id);
CREATE INDEX idx_{{schema_name}}_comments_created ON {{schema_name}}.comments(created_at DESC);

-- ============================================
-- GAME SYSTEM TABLES
-- ============================================

-- Engineers (extended user data for game mechanics)
CREATE TABLE {{schema_name}}.engineers (
    id UUID PRIMARY KEY REFERENCES public.users(id) ON DELETE CASCADE,
    level INT NOT NULL DEFAULT 1,
    xp BIGINT NOT NULL DEFAULT 0,
    xp_to_next_level BIGINT NOT NULL DEFAULT 100,
    satisfaction INT NOT NULL DEFAULT 100,
    salary BIGINT NOT NULL DEFAULT 50000,
    total_revenue BIGINT NOT NULL DEFAULT 0,
    completed_projects INT NOT NULL DEFAULT 0,
    resolved_incidents INT NOT NULL DEFAULT 0,
    is_active BOOLEAN NOT NULL DEFAULT true,
    hired_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    fired_at TIMESTAMPTZ,
    CONSTRAINT chk_level CHECK (level >= 1 AND level <= 100),
    CONSTRAINT chk_satisfaction CHECK (satisfaction >= 0 AND satisfaction <= 100)
);

CREATE INDEX idx_{{schema_name}}_engineers_level ON {{schema_name}}.engineers(level);
CREATE INDEX idx_{{schema_name}}_engineers_active ON {{schema_name}}.engineers(is_active);

-- Achievements (badge definitions)
CREATE TABLE {{schema_name}}.achievements (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(100) NOT NULL,
    description TEXT NOT NULL,
    icon VARCHAR(50) DEFAULT 'trophy',
    category VARCHAR(20) NOT NULL DEFAULT 'special',
    condition_type VARCHAR(50) NOT NULL,
    condition_value INT NOT NULL,
    xp_reward BIGINT NOT NULL DEFAULT 0,
    is_hidden BOOLEAN DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT chk_category CHECK (category IN ('incidents', 'projects', 'skills', 'special'))
);

CREATE UNIQUE INDEX idx_{{schema_name}}_achievements_name ON {{schema_name}}.achievements(name);

-- Engineer achievements (unlocked badges)
CREATE TABLE {{schema_name}}.engineer_achievements (
    engineer_id UUID NOT NULL REFERENCES {{schema_name}}.engineers(id) ON DELETE CASCADE,
    achievement_id UUID NOT NULL REFERENCES {{schema_name}}.achievements(id) ON DELETE CASCADE,
    unlocked_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (engineer_id, achievement_id)
);

CREATE INDEX idx_{{schema_name}}_eng_achv_engineer ON {{schema_name}}.engineer_achievements(engineer_id);

-- Skill tree nodes
CREATE TABLE {{schema_name}}.skill_nodes (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    specialty_id UUID NOT NULL REFERENCES {{schema_name}}.specialties(id) ON DELETE CASCADE,
    name VARCHAR(100) NOT NULL,
    description TEXT NOT NULL,
    tier INT NOT NULL DEFAULT 1,
    required_level INT NOT NULL DEFAULT 1,
    required_xp BIGINT NOT NULL DEFAULT 0,
    parent_node_id UUID REFERENCES {{schema_name}}.skill_nodes(id) ON DELETE SET NULL,
    bonus_type VARCHAR(50) NOT NULL,
    bonus_value INT NOT NULL DEFAULT 10,
    icon VARCHAR(50) DEFAULT 'star',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT chk_tier CHECK (tier >= 1 AND tier <= 5)
);

CREATE INDEX idx_{{schema_name}}_skill_nodes_specialty ON {{schema_name}}.skill_nodes(specialty_id);
CREATE INDEX idx_{{schema_name}}_skill_nodes_tier ON {{schema_name}}.skill_nodes(tier);

-- Engineer skill nodes (unlocked skills)
CREATE TABLE {{schema_name}}.engineer_skill_nodes (
    engineer_id UUID NOT NULL REFERENCES {{schema_name}}.engineers(id) ON DELETE CASCADE,
    skill_node_id UUID NOT NULL REFERENCES {{schema_name}}.skill_nodes(id) ON DELETE CASCADE,
    unlocked_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (engineer_id, skill_node_id)
);

CREATE INDEX idx_{{schema_name}}_eng_skills_engineer ON {{schema_name}}.engineer_skill_nodes(engineer_id);

-- Tenant finance
CREATE TABLE {{schema_name}}.tenant_finance (
    tenant_id UUID PRIMARY KEY,
    balance BIGINT NOT NULL DEFAULT 1000000,
    monthly_revenue BIGINT NOT NULL DEFAULT 0,
    monthly_expenses BIGINT NOT NULL DEFAULT 0,
    revenue_target BIGINT NOT NULL DEFAULT 500000,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Transactions (financial log)
CREATE TABLE {{schema_name}}.transactions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    tenant_id UUID NOT NULL,
    transaction_type VARCHAR(50) NOT NULL,
    amount BIGINT NOT NULL,
    description TEXT NOT NULL,
    engineer_id UUID REFERENCES {{schema_name}}.engineers(id) ON DELETE SET NULL,
    incident_id UUID REFERENCES {{schema_name}}.incidents(id) ON DELETE SET NULL,
    project_id UUID REFERENCES {{schema_name}}.projects(id) ON DELETE SET NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_{{schema_name}}_transactions_type ON {{schema_name}}.transactions(transaction_type);
CREATE INDEX idx_{{schema_name}}_transactions_created ON {{schema_name}}.transactions(created_at DESC);
CREATE INDEX idx_{{schema_name}}_transactions_engineer ON {{schema_name}}.transactions(engineer_id);

-- Training definitions
CREATE TABLE {{schema_name}}.trainings (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(100) NOT NULL,
    description TEXT NOT NULL,
    specialty_id UUID NOT NULL REFERENCES {{schema_name}}.specialties(id) ON DELETE CASCADE,
    duration_hours INT NOT NULL DEFAULT 8,
    cost BIGINT NOT NULL DEFAULT 0,
    xp_gain BIGINT NOT NULL DEFAULT 100,
    proficiency_boost INT NOT NULL DEFAULT 1,
    required_level INT NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_{{schema_name}}_trainings_specialty ON {{schema_name}}.trainings(specialty_id);

-- Engineer training sessions
CREATE TABLE {{schema_name}}.engineer_trainings (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    engineer_id UUID NOT NULL REFERENCES {{schema_name}}.engineers(id) ON DELETE CASCADE,
    training_id UUID NOT NULL REFERENCES {{schema_name}}.trainings(id) ON DELETE CASCADE,
    started_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expected_completion_at TIMESTAMPTZ NOT NULL,
    completed_at TIMESTAMPTZ,
    status VARCHAR(20) NOT NULL DEFAULT 'in_progress',
    CONSTRAINT chk_training_status CHECK (status IN ('in_progress', 'completed', 'cancelled'))
);

CREATE INDEX idx_{{schema_name}}_eng_train_engineer ON {{schema_name}}.engineer_trainings(engineer_id);
CREATE INDEX idx_{{schema_name}}_eng_train_status ON {{schema_name}}.engineer_trainings(status);

-- ============================================
-- RECRUITMENT SYSTEM TABLES
-- ============================================

-- Candidates (hiring pool)
CREATE TABLE {{schema_name}}.candidates (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(100) NOT NULL,
    avatar VARCHAR(100) DEFAULT 'default',
    rarity VARCHAR(20) NOT NULL DEFAULT 'common',
    level INT NOT NULL DEFAULT 1,
    primary_specialty_id UUID NOT NULL REFERENCES {{schema_name}}.specialties(id),
    primary_proficiency VARCHAR(20) NOT NULL DEFAULT 'intermediate',
    secondary_specialty_id UUID REFERENCES {{schema_name}}.specialties(id),
    secondary_proficiency VARCHAR(20),
    expected_salary BIGINT NOT NULL DEFAULT 50000,
    hiring_cost BIGINT NOT NULL DEFAULT 10000,
    base_satisfaction INT NOT NULL DEFAULT 80,
    trait_name VARCHAR(100),
    trait_description TEXT,
    status VARCHAR(20) NOT NULL DEFAULT 'available',
    expires_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    CONSTRAINT chk_rarity CHECK (rarity IN ('common', 'uncommon', 'rare', 'epic', 'legendary')),
    CONSTRAINT chk_candidate_status CHECK (status IN ('available', 'interviewing', 'offer_pending', 'hired', 'unavailable')),
    CONSTRAINT chk_level CHECK (level >= 1 AND level <= 100),
    CONSTRAINT chk_satisfaction CHECK (base_satisfaction >= 0 AND base_satisfaction <= 100)
);

CREATE INDEX idx_{{schema_name}}_candidates_status ON {{schema_name}}.candidates(status);
CREATE INDEX idx_{{schema_name}}_candidates_rarity ON {{schema_name}}.candidates(rarity);
CREATE INDEX idx_{{schema_name}}_candidates_specialty ON {{schema_name}}.candidates(primary_specialty_id);
CREATE INDEX idx_{{schema_name}}_candidates_expires ON {{schema_name}}.candidates(expires_at);

-- Recruitment events (activity log)
CREATE TABLE {{schema_name}}.recruitment_events (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    candidate_id UUID NOT NULL REFERENCES {{schema_name}}.candidates(id) ON DELETE CASCADE,
    recruiter_id UUID NOT NULL REFERENCES public.users(id),
    event_type VARCHAR(50) NOT NULL,
    notes TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_{{schema_name}}_recruit_events_candidate ON {{schema_name}}.recruitment_events(candidate_id);
CREATE INDEX idx_{{schema_name}}_recruit_events_recruiter ON {{schema_name}}.recruitment_events(recruiter_id);
CREATE INDEX idx_{{schema_name}}_recruit_events_created ON {{schema_name}}.recruitment_events(created_at DESC);

-- Recruitment pool settings
CREATE TABLE {{schema_name}}.recruitment_settings (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    last_free_refresh_at TIMESTAMPTZ,
    free_refresh_interval_hours INT NOT NULL DEFAULT 24,
    refresh_cost BIGINT NOT NULL DEFAULT 5000,
    pool_size INT NOT NULL DEFAULT 5,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- ============================================
-- DEFAULT DATA
-- ============================================

-- Default specialties
INSERT INTO {{schema_name}}.specialties (name, description, color, is_default) VALUES
    ('SRE', 'Site Reliability Engineering', '#EF4444', true),
    ('Frontend', 'Frontend Development', '#3B82F6', true),
    ('Backend', 'Backend Development', '#10B981', true),
    ('Infrastructure', 'Infrastructure & DevOps', '#8B5CF6', true),
    ('Mobile', 'Mobile Development', '#F59E0B', true),
    ('QA', 'Quality Assurance', '#EC4899', true),
    ('Security', 'Security Engineering', '#6366F1', true);

-- Default workflow statuses for incidents
INSERT INTO {{schema_name}}.workflow_statuses (entity_type, name, color, display_order, is_initial, is_terminal) VALUES
    ('incident', 'Open', '#EF4444', 1, true, false),
    ('incident', 'Assigned', '#F59E0B', 2, false, false),
    ('incident', 'Investigating', '#3B82F6', 3, false, false),
    ('incident', 'Mitigating', '#8B5CF6', 4, false, false),
    ('incident', 'Resolved', '#10B981', 5, false, false),
    ('incident', 'Closed', '#6B7280', 6, false, true);

-- Default workflow statuses for projects
INSERT INTO {{schema_name}}.workflow_statuses (entity_type, name, color, display_order, is_initial, is_terminal) VALUES
    ('project', 'Backlog', '#6B7280', 1, true, false),
    ('project', 'Planning', '#8B5CF6', 2, false, false),
    ('project', 'In Progress', '#3B82F6', 3, false, false),
    ('project', 'Review', '#F59E0B', 4, false, false),
    ('project', 'Completed', '#10B981', 5, false, true),
    ('project', 'Cancelled', '#EF4444', 6, false, true);

-- Default achievements
INSERT INTO {{schema_name}}.achievements (name, description, icon, category, condition_type, condition_value, xp_reward, is_hidden) VALUES
    ('First Blood', 'Resolve your first incident', 'fire', 'incidents', 'incident_count', 1, 50, false),
    ('Incident Hunter', 'Resolve 10 incidents', 'target', 'incidents', 'incident_count', 10, 200, false),
    ('Incident Master', 'Resolve 100 incidents', 'crown', 'incidents', 'incident_count', 100, 1000, false),
    ('Project Starter', 'Complete your first project', 'rocket', 'projects', 'project_count', 1, 100, false),
    ('Project Pro', 'Complete 10 projects', 'star', 'projects', 'project_count', 10, 500, false),
    ('Level 10', 'Reach level 10', 'badge', 'skills', 'reach_level', 10, 300, false),
    ('Level 25', 'Reach level 25', 'medal', 'skills', 'reach_level', 25, 750, false),
    ('Level 50', 'Reach level 50', 'trophy', 'skills', 'reach_level', 50, 2000, false),
    ('Extreme Challenger', 'Complete an extreme difficulty task', 'skull', 'special', 'extreme_difficulty', 1, 500, true),
    ('Speed Demon', 'Resolve an incident in under 30 minutes', 'lightning', 'special', 'fast_incident_resolve', 30, 200, true),
    ('Revenue King', 'Generate 1 million in revenue', 'dollar', 'special', 'total_revenue', 1000000, 1500, false);
