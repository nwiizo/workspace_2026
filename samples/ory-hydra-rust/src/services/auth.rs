use crate::error::AppError;
use crate::models::{User, UserRole, UserStatus};
use crate::services::UserService;
use argon2::{
    Argon2,
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString, rand_core::OsRng},
};
use sqlx::PgPool;
use tracing::instrument;
use uuid::Uuid;

/// Authentication service for user management and password verification
pub struct AuthService {
    user_service: UserService,
}

impl AuthService {
    /// Create a new authentication service with database backend
    pub fn new(pool: PgPool) -> Self {
        Self {
            user_service: UserService::new(pool),
        }
    }

    /// Initialize the service (seed demo user)
    pub async fn init(&self) -> Result<(), AppError> {
        let password_hash = Self::hash_password_internal("password123")
            .map_err(|e| AppError::Internal(e.to_string()))?;
        self.user_service.seed_demo_user(&password_hash).await?;
        Ok(())
    }

    /// Hash a password using Argon2id
    fn hash_password_internal(password: &str) -> Result<String, password_hash::Error> {
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        let password_hash = argon2.hash_password(password.as_bytes(), &salt)?;
        Ok(password_hash.to_string())
    }

    /// Hash a password using Argon2id (OWASP recommended)
    #[instrument(skip(self, password))]
    pub fn hash_password(&self, password: &str) -> Result<String, AppError> {
        Self::hash_password_internal(password).map_err(AppError::from)
    }

    /// Verify a password against a hash
    #[instrument(skip(self, password, hash))]
    pub fn verify_password(&self, password: &str, hash: &str) -> Result<bool, AppError> {
        let parsed_hash = PasswordHash::new(hash).map_err(|e| AppError::Internal(e.to_string()))?;

        Ok(Argon2::default()
            .verify_password(password.as_bytes(), &parsed_hash)
            .is_ok())
    }

    /// Authenticate user with email and password
    #[instrument(skip(self, password))]
    pub async fn authenticate(&self, email: &str, password: &str) -> Result<User, AppError> {
        let user = self.user_service.get_by_email(email).await?;

        if user.status != UserStatus::Active {
            return Err(AppError::AuthenticationFailed(
                "Account is not active".to_string(),
            ));
        }

        let password_hash = user
            .password_hash
            .as_ref()
            .ok_or(AppError::InvalidCredentials)?;

        if !self.verify_password(password, password_hash)? {
            return Err(AppError::InvalidCredentials);
        }

        // Update last login time
        self.user_service.update_last_login(user.id).await?;

        // Re-fetch user with updated login time
        self.user_service.get_by_id(user.id).await
    }

    /// Get user by ID
    #[instrument(skip(self))]
    pub async fn get_user_by_id(&self, user_id: &Uuid) -> Result<User, AppError> {
        self.user_service.get_by_id(*user_id).await
    }

    /// Get user by email
    #[allow(unused)]
    #[instrument(skip(self))]
    pub async fn get_user_by_email(&self, email: &str) -> Result<User, AppError> {
        self.user_service.get_by_email(email).await
    }

    /// Register a new user (defaults to Reporter role)
    #[instrument(skip(self, password))]
    pub async fn register(&self, email: &str, password: &str) -> Result<User, AppError> {
        self.register_with_role(email, password, UserRole::Reporter, None)
            .await
    }

    /// Register a new user with a specific role and tenant
    #[instrument(skip(self, password))]
    pub async fn register_with_role(
        &self,
        email: &str,
        password: &str,
        role: UserRole,
        tenant_id: Option<Uuid>,
    ) -> Result<User, AppError> {
        let password_hash = self.hash_password(password)?;
        self.user_service
            .create(email, Some(password_hash), role, tenant_id)
            .await
    }

    /// Check if user has admin permission (for consent flow)
    #[allow(unused)]
    #[instrument(skip(self))]
    pub async fn check_admin_permission(&self, user_id: &str) -> Result<bool, AppError> {
        let user_uuid = Uuid::parse_str(user_id)
            .map_err(|e| AppError::BadRequest(format!("Invalid user ID: {}", e)))?;

        let user = self.user_service.get_by_id(user_uuid).await?;
        Ok(user.status == UserStatus::Active)
    }

    /// Get user service reference
    #[allow(unused)]
    pub fn user_service(&self) -> &UserService {
        &self.user_service
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_password_produces_valid_hash() {
        let hash = AuthService::hash_password_internal("test_password_123")
            .expect("Failed to hash password");

        // Argon2 hashes start with $argon2
        assert!(hash.starts_with("$argon2"), "Hash should be Argon2 format");
        assert!(hash.len() > 50, "Hash should be reasonably long");
    }

    #[test]
    fn test_hash_password_produces_different_hashes_for_same_password() {
        let password = "same_password";

        let hash1 = AuthService::hash_password_internal(password).expect("Failed to hash password");
        let hash2 = AuthService::hash_password_internal(password).expect("Failed to hash password");

        // Due to random salt, hashes should be different
        assert_ne!(hash1, hash2, "Hashes should differ due to random salt");
    }
}
