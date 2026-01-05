-- Create users table in public schema
-- Users are stored centrally to ensure unique email addresses across all tenants
CREATE TABLE IF NOT EXISTS public.users (
    id UUID PRIMARY KEY,
    email VARCHAR(255) UNIQUE NOT NULL,
    email_verified BOOLEAN NOT NULL DEFAULT FALSE,
    password_hash VARCHAR(255),
    role VARCHAR(50) NOT NULL DEFAULT 'customer',
    tenant_id UUID REFERENCES public.tenants(id) ON DELETE SET NULL,
    status VARCHAR(50) NOT NULL DEFAULT 'active',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_login_at TIMESTAMPTZ
);

-- Indexes for common queries
CREATE INDEX IF NOT EXISTS idx_users_email ON public.users(email);
CREATE INDEX IF NOT EXISTS idx_users_tenant_id ON public.users(tenant_id);
CREATE INDEX IF NOT EXISTS idx_users_role ON public.users(role);
CREATE INDEX IF NOT EXISTS idx_users_status ON public.users(status);

-- Constraint: Platform admins must not have a tenant_id
-- Tenant-specific roles must have a tenant_id
ALTER TABLE public.users ADD CONSTRAINT chk_users_role_tenant
    CHECK (
        (role = 'platform_admin' AND tenant_id IS NULL) OR
        (role != 'platform_admin')
    );
