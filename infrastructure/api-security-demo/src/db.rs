//! Database utilities using SQLite

use crate::error::AppError;
use crate::models::{Coupon, Order, Payment, Referral, Ticket, User};
use rusqlite::{Connection, params};
use std::sync::{Arc, Mutex};

/// Database connection wrapper
#[derive(Clone)]
pub struct Database {
    conn: Arc<Mutex<Connection>>,
}

impl Database {
    /// Create a new in-memory database
    pub fn new_in_memory() -> Result<Self, AppError> {
        let conn = Connection::open_in_memory()?;
        let db = Self {
            conn: Arc::new(Mutex::new(conn)),
        };
        db.init_schema()?;
        Ok(db)
    }

    /// Create a database from file
    pub fn new_from_file(path: &str) -> Result<Self, AppError> {
        let conn = Connection::open(path)?;
        let db = Self {
            conn: Arc::new(Mutex::new(conn)),
        };
        db.init_schema()?;
        Ok(db)
    }

    /// Initialize database schema
    fn init_schema(&self) -> Result<(), AppError> {
        let conn = self.conn.lock().unwrap();

        conn.execute(
            "CREATE TABLE IF NOT EXISTS orders (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                user TEXT NOT NULL,
                product TEXT NOT NULL,
                quantity INTEGER NOT NULL
            )",
            [],
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS payments (
                id TEXT PRIMARY KEY,
                amount REAL NOT NULL,
                currency TEXT NOT NULL,
                status TEXT NOT NULL DEFAULT 'pending',
                created_at TEXT NOT NULL
            )",
            [],
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS users (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                email TEXT UNIQUE NOT NULL,
                password_hash TEXT NOT NULL,
                role TEXT NOT NULL DEFAULT 'user',
                created_at TEXT NOT NULL
            )",
            [],
        )?;

        // API6: Business flow tables
        conn.execute(
            "CREATE TABLE IF NOT EXISTS coupons (
                code TEXT PRIMARY KEY,
                discount_percent INTEGER NOT NULL,
                max_uses INTEGER NOT NULL,
                current_uses INTEGER NOT NULL DEFAULT 0,
                expires_at TEXT NOT NULL,
                single_use_per_user INTEGER NOT NULL DEFAULT 1
            )",
            [],
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS coupon_usages (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                coupon_code TEXT NOT NULL,
                user_id TEXT NOT NULL,
                used_at TEXT NOT NULL,
                UNIQUE(coupon_code, user_id)
            )",
            [],
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS tickets (
                id TEXT PRIMARY KEY,
                event_name TEXT NOT NULL,
                price REAL NOT NULL,
                available_quantity INTEGER NOT NULL,
                max_per_user INTEGER NOT NULL DEFAULT 4
            )",
            [],
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS ticket_purchases (
                id TEXT PRIMARY KEY,
                ticket_id TEXT NOT NULL,
                user_id TEXT NOT NULL,
                quantity INTEGER NOT NULL,
                purchased_at TEXT NOT NULL
            )",
            [],
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS referrals (
                id TEXT PRIMARY KEY,
                referrer_id TEXT NOT NULL,
                referred_id TEXT NOT NULL,
                bonus_amount REAL NOT NULL,
                created_at TEXT NOT NULL,
                UNIQUE(referred_id)
            )",
            [],
        )?;

        Ok(())
    }

    /// Seed sample data for orders
    pub fn seed_orders(&self) -> Result<(), AppError> {
        let conn = self.conn.lock().unwrap();

        // Check if already seeded
        let count: i64 = conn.query_row("SELECT COUNT(*) FROM orders", [], |row| row.get(0))?;
        if count > 0 {
            return Ok(());
        }

        conn.execute(
            "INSERT INTO orders (user, product, quantity) VALUES (?1, ?2, ?3)",
            params!["alice", "Widget A", 5],
        )?;
        conn.execute(
            "INSERT INTO orders (user, product, quantity) VALUES (?1, ?2, ?3)",
            params!["bob", "Widget B", 3],
        )?;
        conn.execute(
            "INSERT INTO orders (user, product, quantity) VALUES (?1, ?2, ?3)",
            params!["alice", "Widget C", 10],
        )?;

        Ok(())
    }

    /// Get order by ID (no authorization check - vulnerable)
    pub fn get_order_by_id(&self, id: i64) -> Result<Option<Order>, AppError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt =
            conn.prepare("SELECT id, user, product, quantity FROM orders WHERE id = ?1")?;

        let order = stmt
            .query_row(params![id], |row| {
                Ok(Order {
                    id: row.get(0)?,
                    user: row.get(1)?,
                    product: row.get(2)?,
                    quantity: row.get(3)?,
                })
            })
            .ok();

        Ok(order)
    }

    /// Get order by ID with user check (secure)
    pub fn get_order_by_id_for_user(&self, id: i64, user: &str) -> Result<Option<Order>, AppError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, user, product, quantity FROM orders WHERE id = ?1 AND user = ?2",
        )?;

        let order = stmt
            .query_row(params![id, user], |row| {
                Ok(Order {
                    id: row.get(0)?,
                    user: row.get(1)?,
                    product: row.get(2)?,
                    quantity: row.get(3)?,
                })
            })
            .ok();

        Ok(order)
    }

    /// Get all orders for a user
    pub fn get_orders_for_user(&self, user: &str) -> Result<Vec<Order>, AppError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt =
            conn.prepare("SELECT id, user, product, quantity FROM orders WHERE user = ?1")?;

        let orders = stmt
            .query_map(params![user], |row| {
                Ok(Order {
                    id: row.get(0)?,
                    user: row.get(1)?,
                    product: row.get(2)?,
                    quantity: row.get(3)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;

        Ok(orders)
    }

    /// Create a new order
    pub fn create_order(
        &self,
        user: &str,
        product: &str,
        quantity: i32,
    ) -> Result<Order, AppError> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO orders (user, product, quantity) VALUES (?1, ?2, ?3)",
            params![user, product, quantity],
        )?;
        let id = conn.last_insert_rowid();

        Ok(Order {
            id,
            user: user.to_string(),
            product: product.to_string(),
            quantity,
        })
    }

    /// Create a payment (safe version)
    pub fn create_payment(&self, payment: &Payment) -> Result<(), AppError> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO payments (id, amount, currency, status, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![payment.id, payment.amount, payment.currency, payment.status, payment.created_at],
        )?;
        Ok(())
    }

    /// Get payment by ID
    pub fn get_payment_by_id(&self, id: &str) -> Result<Option<Payment>, AppError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, amount, currency, status, created_at FROM payments WHERE id = ?1",
        )?;

        let payment = stmt
            .query_row(params![id], |row| {
                Ok(Payment {
                    id: row.get(0)?,
                    amount: row.get(1)?,
                    currency: row.get(2)?,
                    status: row.get(3)?,
                    created_at: row.get(4)?,
                })
            })
            .ok();

        Ok(payment)
    }

    /// Create a user
    pub fn create_user(
        &self,
        email: &str,
        password_hash: &str,
        role: &str,
    ) -> Result<User, AppError> {
        let conn = self.conn.lock().unwrap();
        let created_at = chrono::Utc::now().to_rfc3339();

        conn.execute(
            "INSERT INTO users (email, password_hash, role, created_at) VALUES (?1, ?2, ?3, ?4)",
            params![email, password_hash, role, created_at],
        )?;
        let id = conn.last_insert_rowid();

        Ok(User {
            id,
            email: email.to_string(),
            password_hash: password_hash.to_string(),
            role: role.to_string(),
            created_at,
        })
    }

    /// Get user by email
    pub fn get_user_by_email(&self, email: &str) -> Result<Option<User>, AppError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, email, password_hash, role, created_at FROM users WHERE email = ?1",
        )?;

        let user = stmt
            .query_row(params![email], |row| {
                Ok(User {
                    id: row.get(0)?,
                    email: row.get(1)?,
                    password_hash: row.get(2)?,
                    role: row.get(3)?,
                    created_at: row.get(4)?,
                })
            })
            .ok();

        Ok(user)
    }

    // ============================================
    // API6: Business Flow Methods
    // ============================================

    /// Seed sample coupons
    pub fn seed_coupons(&self) -> Result<(), AppError> {
        let conn = self.conn.lock().unwrap();

        let count: i64 = conn.query_row("SELECT COUNT(*) FROM coupons", [], |row| row.get(0))?;
        if count > 0 {
            return Ok(());
        }

        let future_date = (chrono::Utc::now() + chrono::Duration::days(30)).to_rfc3339();
        let past_date = (chrono::Utc::now() - chrono::Duration::days(1)).to_rfc3339();

        conn.execute(
            "INSERT INTO coupons (code, discount_percent, max_uses, current_uses, expires_at, single_use_per_user) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params!["SAVE20", 20, 100, 0, &future_date, 1],
        )?;
        conn.execute(
            "INSERT INTO coupons (code, discount_percent, max_uses, current_uses, expires_at, single_use_per_user) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params!["EXPIRED10", 10, 100, 0, &past_date, 1],
        )?;
        conn.execute(
            "INSERT INTO coupons (code, discount_percent, max_uses, current_uses, expires_at, single_use_per_user) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params!["MAXED50", 50, 5, 5, &future_date, 1],
        )?;

        Ok(())
    }

    /// Get coupon by code
    pub fn get_coupon(&self, code: &str) -> Result<Option<Coupon>, AppError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT code, discount_percent, max_uses, current_uses, expires_at, single_use_per_user FROM coupons WHERE code = ?1",
        )?;

        let coupon = stmt
            .query_row(params![code], |row| {
                Ok(Coupon {
                    code: row.get(0)?,
                    discount_percent: row.get(1)?,
                    max_uses: row.get(2)?,
                    current_uses: row.get(3)?,
                    expires_at: row.get(4)?,
                    single_use_per_user: row.get::<_, i32>(5)? != 0,
                })
            })
            .ok();

        Ok(coupon)
    }

    /// Check if user has already used coupon
    pub fn has_user_used_coupon(&self, code: &str, user_id: &str) -> Result<bool, AppError> {
        let conn = self.conn.lock().unwrap();
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM coupon_usages WHERE coupon_code = ?1 AND user_id = ?2",
            params![code, user_id],
            |row| row.get(0),
        )?;
        Ok(count > 0)
    }

    /// Record coupon usage (secure version)
    pub fn use_coupon(&self, code: &str, user_id: &str) -> Result<(), AppError> {
        let conn = self.conn.lock().unwrap();

        // Increment usage count
        conn.execute(
            "UPDATE coupons SET current_uses = current_uses + 1 WHERE code = ?1",
            params![code],
        )?;

        // Record user usage
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO coupon_usages (coupon_code, user_id, used_at) VALUES (?1, ?2, ?3)",
            params![code, user_id, now],
        )?;

        Ok(())
    }

    /// Increment coupon usage count only (vulnerable - no user tracking)
    pub fn increment_coupon_usage(&self, code: &str) -> Result<(), AppError> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "UPDATE coupons SET current_uses = current_uses + 1 WHERE code = ?1",
            params![code],
        )?;
        Ok(())
    }

    /// Seed sample tickets
    pub fn seed_tickets(&self) -> Result<(), AppError> {
        let conn = self.conn.lock().unwrap();

        let count: i64 = conn.query_row("SELECT COUNT(*) FROM tickets", [], |row| row.get(0))?;
        if count > 0 {
            return Ok(());
        }

        conn.execute(
            "INSERT INTO tickets (id, event_name, price, available_quantity, max_per_user) VALUES (?1, ?2, ?3, ?4, ?5)",
            params!["ticket-001", "Concert 2026", 150.0, 100, 4],
        )?;
        conn.execute(
            "INSERT INTO tickets (id, event_name, price, available_quantity, max_per_user) VALUES (?1, ?2, ?3, ?4, ?5)",
            params!["ticket-002", "Limited VIP Event", 500.0, 10, 2],
        )?;

        Ok(())
    }

    /// Get ticket by ID
    pub fn get_ticket(&self, id: &str) -> Result<Option<Ticket>, AppError> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            "SELECT id, event_name, price, available_quantity, max_per_user FROM tickets WHERE id = ?1",
        )?;

        let ticket = stmt
            .query_row(params![id], |row| {
                Ok(Ticket {
                    id: row.get(0)?,
                    event_name: row.get(1)?,
                    price: row.get(2)?,
                    available_quantity: row.get(3)?,
                    max_per_user: row.get(4)?,
                })
            })
            .ok();

        Ok(ticket)
    }

    /// Get user's total purchases for a ticket
    pub fn get_user_ticket_purchases(&self, ticket_id: &str, user_id: &str) -> Result<u32, AppError> {
        let conn = self.conn.lock().unwrap();
        let total: i64 = conn.query_row(
            "SELECT COALESCE(SUM(quantity), 0) FROM ticket_purchases WHERE ticket_id = ?1 AND user_id = ?2",
            params![ticket_id, user_id],
            |row| row.get(0),
        )?;
        Ok(total as u32)
    }

    /// Purchase ticket (secure version with inventory check)
    pub fn purchase_ticket_secure(
        &self,
        ticket_id: &str,
        user_id: &str,
        quantity: u32,
    ) -> Result<String, AppError> {
        let conn = self.conn.lock().unwrap();

        // Check available quantity
        let available: u32 = conn.query_row(
            "SELECT available_quantity FROM tickets WHERE id = ?1",
            params![ticket_id],
            |row| row.get(0),
        )?;

        if quantity > available {
            return Err(AppError::BadRequest("Not enough tickets available".to_string()));
        }

        // Decrement available quantity
        conn.execute(
            "UPDATE tickets SET available_quantity = available_quantity - ?1 WHERE id = ?2",
            params![quantity, ticket_id],
        )?;

        // Create purchase record
        let purchase_id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO ticket_purchases (id, ticket_id, user_id, quantity, purchased_at) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![purchase_id, ticket_id, user_id, quantity, now],
        )?;

        Ok(purchase_id)
    }

    /// Purchase ticket (vulnerable - no inventory management)
    pub fn purchase_ticket_vulnerable(
        &self,
        ticket_id: &str,
        user_id: &str,
        quantity: u32,
    ) -> Result<String, AppError> {
        let conn = self.conn.lock().unwrap();

        let purchase_id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();
        conn.execute(
            "INSERT INTO ticket_purchases (id, ticket_id, user_id, quantity, purchased_at) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![purchase_id, ticket_id, user_id, quantity, now],
        )?;

        Ok(purchase_id)
    }

    /// Check if referral already exists for user
    pub fn has_been_referred(&self, user_id: &str) -> Result<bool, AppError> {
        let conn = self.conn.lock().unwrap();
        let count: i64 = conn.query_row(
            "SELECT COUNT(*) FROM referrals WHERE referred_id = ?1",
            params![user_id],
            |row| row.get(0),
        )?;
        Ok(count > 0)
    }

    /// Create referral (secure version)
    pub fn create_referral(
        &self,
        referrer_id: &str,
        referred_id: &str,
        bonus_amount: f64,
    ) -> Result<Referral, AppError> {
        let conn = self.conn.lock().unwrap();

        let id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();

        conn.execute(
            "INSERT INTO referrals (id, referrer_id, referred_id, bonus_amount, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![id, referrer_id, referred_id, bonus_amount, now],
        )?;

        Ok(Referral {
            id,
            referrer_id: referrer_id.to_string(),
            referred_id: referred_id.to_string(),
            bonus_amount,
            created_at: now,
        })
    }

    /// Create referral (vulnerable - no duplicate check)
    pub fn create_referral_vulnerable(
        &self,
        referrer_id: &str,
        referred_id: &str,
        bonus_amount: f64,
    ) -> Result<Referral, AppError> {
        let conn = self.conn.lock().unwrap();

        let id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();

        // Note: This bypasses the UNIQUE constraint by using a different table or ignoring errors
        // For demo, we'll just create without checking
        conn.execute(
            "INSERT OR REPLACE INTO referrals (id, referrer_id, referred_id, bonus_amount, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![id, referrer_id, referred_id, bonus_amount, now],
        )?;

        Ok(Referral {
            id,
            referrer_id: referrer_id.to_string(),
            referred_id: referred_id.to_string(),
            bonus_amount,
            created_at: now,
        })
    }

    /// Seed sample users
    pub fn seed_users(&self) -> Result<(), AppError> {
        let conn = self.conn.lock().unwrap();

        // Check if already seeded
        let count: i64 = conn.query_row("SELECT COUNT(*) FROM users", [], |row| row.get(0))?;
        if count > 0 {
            return Ok(());
        }

        drop(conn); // Release lock before calling create_user

        // Using argon2 password hashing
        use argon2::{
            Argon2,
            password_hash::{PasswordHasher, SaltString, rand_core::OsRng},
        };

        let argon2 = Argon2::default();
        let salt = SaltString::generate(&mut OsRng);

        let admin_hash = argon2
            .hash_password(b"admin123", &salt)
            .map_err(|e| AppError::Internal(e.to_string()))?
            .to_string();

        let user_hash = argon2
            .hash_password(b"user123", &salt)
            .map_err(|e| AppError::Internal(e.to_string()))?
            .to_string();

        self.create_user("admin@example.com", &admin_hash, "admin")?;
        self.create_user("user@example.com", &user_hash, "user")?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_and_get_order() {
        let db = Database::new_in_memory().unwrap();
        let order = db.create_order("alice", "Test Product", 5).unwrap();

        assert_eq!(order.user, "alice");
        assert_eq!(order.product, "Test Product");
        assert_eq!(order.quantity, 5);

        let fetched = db.get_order_by_id(order.id).unwrap().unwrap();
        assert_eq!(fetched.id, order.id);
    }

    #[test]
    fn test_order_authorization() {
        let db = Database::new_in_memory().unwrap();
        let order = db.create_order("alice", "Test Product", 5).unwrap();

        // Alice can access her order
        let result = db.get_order_by_id_for_user(order.id, "alice").unwrap();
        assert!(result.is_some());

        // Bob cannot access Alice's order
        let result = db.get_order_by_id_for_user(order.id, "bob").unwrap();
        assert!(result.is_none());
    }
}
