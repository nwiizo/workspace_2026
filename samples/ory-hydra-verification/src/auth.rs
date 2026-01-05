//! Authentication Service
//!
//! Argon2idを使用したパスワード認証の実装
//! OWASPガイドラインに準拠

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

use argon2::{
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use rand_core::OsRng;

use crate::error::AppError;

#[derive(Debug, Clone)]
pub struct User {
    pub id: Uuid,
    pub email: String,
    pub password_hash: String,
}

#[derive(Clone)]
pub struct AuthService {
    users: Arc<RwLock<HashMap<String, User>>>,
}

impl AuthService {
    pub fn new() -> Self {
        Self {
            users: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// ユーザー登録
    ///
    /// Argon2idでパスワードをハッシュ化して保存
    pub async fn register(&self, email: &str, password: &str) -> Result<User, AppError> {
        if email.is_empty() || password.is_empty() {
            return Err(AppError::BadRequest(
                "Email and password are required".to_string(),
            ));
        }

        let salt = SaltString::generate(&mut OsRng);
        let password_hash = Argon2::default()
            .hash_password(password.as_bytes(), &salt)?
            .to_string();

        let user = User {
            id: Uuid::new_v4(),
            email: email.to_string(),
            password_hash,
        };

        let mut users = self.users.write().await;
        if users.contains_key(email) {
            return Err(AppError::UserAlreadyExists);
        }
        users.insert(email.to_string(), user.clone());

        Ok(user)
    }

    /// 認証
    ///
    /// 重要: ユーザー列挙攻撃対策のため、
    /// ユーザーが存在しない場合もパスワード不正の場合も同じエラーを返す
    pub async fn authenticate(&self, email: &str, password: &str) -> Result<User, AppError> {
        // 空パスワードは早期リターン
        if password.is_empty() {
            return Err(AppError::InvalidCredentials);
        }

        let users = self.users.read().await;
        let user = users.get(email).ok_or(AppError::InvalidCredentials)?;

        let parsed_hash =
            PasswordHash::new(&user.password_hash).map_err(|e| AppError::Internal(e.to_string()))?;

        Argon2::default()
            .verify_password(password.as_bytes(), &parsed_hash)
            .map_err(|_| AppError::InvalidCredentials)?;

        Ok(user.clone())
    }
}

impl Default for AuthService {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ===========================================
    // 正常系テスト
    // ===========================================

    #[tokio::test]
    async fn test_register_and_authenticate() {
        let service = AuthService::new();
        let email = "test@example.com";
        let password = "secure_password123";

        let user = service
            .register(email, password)
            .await
            .expect("Registration should succeed");
        assert_eq!(user.email, email);

        let authenticated = service
            .authenticate(email, password)
            .await
            .expect("Authentication should succeed");
        assert_eq!(authenticated.id, user.id);
    }

    // ===========================================
    // 異常系テスト: できないことの確認
    // ===========================================

    #[tokio::test]
    async fn test_cannot_authenticate_with_wrong_password() {
        let service = AuthService::new();
        service
            .register("user@example.com", "correct_password")
            .await
            .expect("Registration should succeed");

        let result = service
            .authenticate("user@example.com", "wrong_password")
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_cannot_authenticate_with_empty_password() {
        let service = AuthService::new();
        service
            .register("user@example.com", "valid_password")
            .await
            .expect("Registration should succeed");

        let result = service.authenticate("user@example.com", "").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_cannot_register_with_empty_email() {
        let service = AuthService::new();
        let result = service.register("", "password").await;
        assert!(matches!(result, Err(AppError::BadRequest(_))));
    }

    #[tokio::test]
    async fn test_cannot_register_with_empty_password() {
        let service = AuthService::new();
        let result = service.register("user@example.com", "").await;
        assert!(matches!(result, Err(AppError::BadRequest(_))));
    }

    #[tokio::test]
    async fn test_cannot_register_duplicate_email() {
        let service = AuthService::new();
        service
            .register("dup@example.com", "password1")
            .await
            .expect("First registration should succeed");

        let result = service.register("dup@example.com", "password2").await;
        assert!(matches!(result, Err(AppError::UserAlreadyExists)));
    }

    // ===========================================
    // セキュリティテスト: 攻撃者の視点
    // ===========================================

    /// ユーザー列挙攻撃対策テスト
    ///
    /// 存在するユーザーと存在しないユーザーで
    /// 同じエラーメッセージを返すことを確認
    #[tokio::test]
    async fn test_login_does_not_reveal_user_existence() {
        let service = AuthService::new();
        service
            .register("exists@example.com", "password")
            .await
            .expect("Registration should succeed");

        let err1 = service
            .authenticate("exists@example.com", "wrong")
            .await
            .unwrap_err();
        let err2 = service
            .authenticate("nobody@example.com", "password")
            .await
            .unwrap_err();

        // エラーメッセージが同じであることを確認
        assert_eq!(err1.to_string(), err2.to_string());
    }

    // ===========================================
    // 並行処理テスト: 競合状態の検出
    // ===========================================

    #[tokio::test]
    async fn test_concurrent_registration_same_email() {
        let service = AuthService::new();
        let email = "race@example.com";

        let service1 = service.clone();
        let service2 = service.clone();
        let email1 = email.to_string();
        let email2 = email.to_string();

        let handle1 = tokio::spawn(async move { service1.register(&email1, "password1").await });
        let handle2 = tokio::spawn(async move { service2.register(&email2, "password2").await });

        let result1 = handle1.await.expect("Task should complete");
        let result2 = handle2.await.expect("Task should complete");

        let success_count = [result1.is_ok(), result2.is_ok()]
            .iter()
            .filter(|&&x| x)
            .count();

        assert_eq!(success_count, 1, "Exactly one registration should succeed");
    }

    // ===========================================
    // エッジケース: 特殊文字とUnicode
    // ===========================================

    #[tokio::test]
    async fn test_special_characters_in_password() {
        let service = AuthService::new();
        let password = r#"p@$$w0rd!@#$%^&*()_+-=[]{}|;':",.<>?/`~"#;

        service
            .register("special@example.com", password)
            .await
            .expect("Registration should succeed");
        let result = service.authenticate("special@example.com", password).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_unicode_password() {
        let service = AuthService::new();
        let password = "パスワード123🔐";

        service
            .register("unicode@example.com", password)
            .await
            .expect("Registration should succeed");
        let result = service.authenticate("unicode@example.com", password).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_long_password() {
        let service = AuthService::new();
        let password = "a".repeat(1000);

        service
            .register("long@example.com", &password)
            .await
            .expect("Registration should succeed");
        let result = service.authenticate("long@example.com", &password).await;
        assert!(result.is_ok());
    }
}
