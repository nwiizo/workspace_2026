//! 共通型定義
//!
//! ブログ記事で推奨されるRust型のパターン

use std::fmt;
use std::str::FromStr;
use uuid::Uuid;

/// タグごとの投稿数
#[derive(Debug)]
#[allow(dead_code)]
pub struct TagCount {
    pub name: String,
    pub count: i64,
}

/// 投稿
#[derive(Debug)]
#[allow(dead_code)]
pub struct Post {
    pub post_id: Uuid,
    pub title: String,
}

/// アプリケーションエラー型
///
/// 外部キー違反などのDBエラーをアプリケーション固有のエラーに変換
#[derive(Debug)]
pub enum AppError {
    PostNotFound,
    Database(sqlx::Error),
    InvalidStatus(String),
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::PostNotFound => write!(f, "Post not found"),
            AppError::Database(e) => write!(f, "Database error: {}", e),
            AppError::InvalidStatus(s) => write!(f, "Invalid status: {}", s),
        }
    }
}

impl std::error::Error for AppError {}

/// 投稿ステータス（参照テーブル版）
///
/// FromStr/Display トレイトを実装してDBの文字列と相互変換
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PostStatus {
    Draft,
    PendingReview,
    Published,
    Archived,
}

impl PostStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::PendingReview => "pending_review",
            Self::Published => "published",
            Self::Archived => "archived",
        }
    }
}

impl FromStr for PostStatus {
    type Err = AppError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "draft" => Ok(Self::Draft),
            "pending_review" => Ok(Self::PendingReview),
            "published" => Ok(Self::Published),
            "archived" => Ok(Self::Archived),
            _ => Err(AppError::InvalidStatus(s.to_string())),
        }
    }
}

impl fmt::Display for PostStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// PostgreSQL ENUM連携用
///
/// sqlx::Type を derive することでPostgreSQL ENUMと直接マッピング
#[derive(Debug, Clone, Copy, PartialEq, Eq, sqlx::Type)]
#[sqlx(type_name = "priority_level", rename_all = "lowercase")]
pub enum Priority {
    Low,
    Medium,
    High,
    Critical,
}
