use std::{cmp::Reverse, collections::HashMap, str::FromStr};

use serde::Deserialize;
use time::{Date, OffsetDateTime, macros::offset};
use toasty::Db;
use topcoat::{
    Result,
    asset::{AssetBundle, RouterBuilderAssetExt},
    context::{Cx, app_context, memoize},
    cookie::RouterBuilderCookieExt,
    router::{
        Router, RouterBuilderDiscoverExt,
        content::Form,
        error::{RouterErrorExt, SeeOther, bad_request, forbidden, see_other, unauthorized},
        layout, page, route,
    },
    runtime::{Event, procedure, shard},
    session::{self, RouterBuilderSessionExt, SessionConfig, TokenHash},
    view::{component, view},
};

use crate::{
    auth::{hash_password, normalize_email, validate_password, verify_password},
    domain::{ApplicationResult, RecallRating, Uniform, parse_iso_date, schedule_review},
    models::{AuthSession, Review, TechniqueCard, TrainingSession, User},
};

// A valid hash keeps unknown-account logins on the normal verification path.
const DUMMY_PASSWORD_HASH: &str = "$argon2id$v=19$m=19456,t=2,p=1$pTdtu+F0E2va+D2TbJzITA$hFSfUGQD63aPAcTUPZqe2cdmsSfHxQUsnzMQvQfGXdc";

const CSS: &str = r#"
:root{color-scheme:light;--ink:#17251f;--muted:#66736d;--paper:#f4f0e7;--card:#fffdf7;--line:#d9d3c7;--moss:#2f5c46;--moss-dark:#214333;--verm:#d35432;--sun:#e9bd63;--shadow:0 20px 55px rgba(31,51,41,.1)}
*{box-sizing:border-box}html{scroll-behavior:smooth}body{margin:0;background:var(--paper);color:var(--ink);font-family:"Hiragino Kaku Gothic ProN","Yu Gothic",system-ui,sans-serif;line-height:1.65;background-image:radial-gradient(circle at 12% 5%,rgba(233,189,99,.16),transparent 26rem),linear-gradient(rgba(47,92,70,.035) 1px,transparent 1px);background-size:auto,100% 32px}
a{color:inherit}.shell{width:min(1120px,calc(100% - 32px));margin:0 auto}.site-header{position:sticky;top:0;z-index:20;border-bottom:1px solid rgba(47,92,70,.14);background:rgba(244,240,231,.9);backdrop-filter:blur(14px)}.nav{min-height:68px;display:flex;align-items:center;justify-content:space-between;gap:18px}.brand{display:flex;align-items:center;gap:11px;text-decoration:none;font-family:Georgia,"Yu Mincho",serif;font-weight:800;letter-spacing:.04em}.brand-mark{display:grid;place-items:center;width:36px;height:36px;border:2px solid var(--ink);border-radius:50%;font-size:.74rem}.nav-links{display:flex;align-items:center;gap:16px;font-size:.88rem}.nav-links a{text-decoration:none}.email{color:var(--muted);max-width:220px;overflow:hidden;text-overflow:ellipsis}.link-button{border:0;background:none;color:var(--verm);font:inherit;cursor:pointer;padding:6px}
main{padding:42px 0 80px}.hero{position:relative;overflow:hidden;padding:clamp(32px,6vw,72px);border-radius:28px;background:var(--ink);color:#f7f1e5;box-shadow:var(--shadow)}.hero:after{content:"";position:absolute;width:310px;height:310px;border:1px solid rgba(255,255,255,.13);border-radius:50%;right:-100px;top:-140px;box-shadow:0 0 0 38px rgba(255,255,255,.035),0 0 0 76px rgba(255,255,255,.025)}.eyebrow{margin:0 0 10px;color:var(--sun);font-size:.72rem;font-weight:800;letter-spacing:.18em;text-transform:uppercase}.hero h1{position:relative;z-index:1;margin:0;max-width:760px;font:700 clamp(2.25rem,6vw,5rem)/1.08 Georgia,"Yu Mincho",serif;letter-spacing:-.03em}.hero-copy{position:relative;z-index:1;max-width:620px;margin:20px 0 0;color:#cfd8d1;font-size:1.04rem}.hero-actions{position:relative;z-index:1;display:flex;flex-wrap:wrap;gap:12px;margin-top:28px}
.button,button{min-height:44px;border:0;border-radius:999px;padding:11px 20px;background:var(--moss);color:white;font:700 .9rem/1 inherit;cursor:pointer;text-decoration:none;display:inline-flex;align-items:center;justify-content:center;transition:transform .15s ease,background .15s ease}.button:hover,button:hover{background:var(--moss-dark);transform:translateY(-1px)}.button.light{background:#f8f1e4;color:var(--ink)}.button.ghost,.review-actions button{background:transparent;color:var(--moss);border:1px solid var(--line)}.button.danger{background:var(--verm)}
.stats{display:grid;grid-template-columns:repeat(3,1fr);gap:1px;margin-top:28px;overflow:hidden;border:1px solid var(--line);border-radius:18px;background:var(--line)}.stat{padding:20px;background:var(--card)}.stat strong{display:block;font:700 1.9rem Georgia,serif}.stat span{color:var(--muted);font-size:.78rem}.grid{display:grid;grid-template-columns:1.1fr .9fr;gap:24px;margin-top:24px}.stack{display:grid;gap:24px}.panel{padding:26px;border:1px solid var(--line);border-radius:22px;background:rgba(255,253,247,.9);box-shadow:0 12px 35px rgba(31,51,41,.06)}.panel-head{display:flex;align-items:end;justify-content:space-between;gap:16px;margin-bottom:20px}.panel h2,.panel h3{margin:0;font-family:Georgia,"Yu Mincho",serif;line-height:1.3}.panel-sub{margin:4px 0 0;color:var(--muted);font-size:.88rem}.badge{display:inline-flex;border-radius:999px;padding:5px 10px;background:#e5eee8;color:var(--moss);font-size:.72rem;font-weight:800;text-transform:uppercase}
form.fields{display:grid;gap:15px}.two{display:grid;grid-template-columns:1fr 1fr;gap:14px}label{display:grid;gap:6px;font-size:.78rem;font-weight:800;letter-spacing:.02em}input,select,textarea{width:100%;border:1px solid var(--line);border-radius:12px;padding:11px 13px;background:#fffefb;color:var(--ink);font:inherit;outline:none}textarea{min-height:90px;resize:vertical}input:focus,select:focus,textarea:focus,button:focus-visible,a:focus-visible{outline:3px solid rgba(233,189,99,.55);outline-offset:2px;border-color:var(--moss)}.hint{color:var(--muted);font-size:.76rem;font-weight:400}.full{width:100%}
.review-card{padding:20px;border:1px solid var(--line);border-radius:17px;background:#fff}.review-card+.review-card{margin-top:12px}.review-title{display:flex;justify-content:space-between;gap:14px}.review-title h3{font-size:1.1rem}.position{color:var(--verm);font-size:.75rem;font-weight:800}.prompt{margin:16px 0;padding:15px;border-left:3px solid var(--sun);background:#faf5e8}.answer{margin:12px 0;padding:15px;border-radius:12px;background:#eaf0ec}.review-actions{display:flex;flex-wrap:wrap;gap:8px;margin-top:14px}.review-actions button{min-height:38px;padding:8px 12px;font-size:.75rem}.review-actions button:last-of-type{border-color:var(--moss);background:var(--moss);color:white}.status{min-height:1.5em;color:var(--moss);font-size:.8rem;font-weight:700}
.search-box{position:relative}.search-box input{padding-left:42px}.search-box:before{content:"⌕";position:absolute;left:15px;top:7px;color:var(--muted);font-size:1.3rem}.result-list{display:grid;grid-template-columns:repeat(2,1fr);gap:12px;margin-top:15px}.result{padding:15px;border:1px solid var(--line);border-radius:14px;background:white}.result strong{display:block}.result p{margin:5px 0 0;color:var(--muted);font-size:.82rem}.session-list{display:grid;gap:10px}.session-row{display:grid;grid-template-columns:100px 70px 1fr;gap:14px;align-items:start;padding:14px 0;border-bottom:1px solid var(--line);font-size:.86rem}.session-row:last-child{border-bottom:0}.session-row p{margin:0}.empty{padding:24px;border:1px dashed var(--line);border-radius:14px;text-align:center;color:var(--muted)}
.auth-wrap{min-height:calc(100vh - 190px);display:grid;place-items:center}.auth-card{width:min(470px,100%);padding:34px;border:1px solid var(--line);border-radius:26px;background:var(--card);box-shadow:var(--shadow)}.auth-card h1{margin:0;font:700 2.2rem Georgia,"Yu Mincho",serif}.auth-card>p{color:var(--muted)}.auth-switch{text-align:center;font-size:.85rem}.landing-grid{display:grid;grid-template-columns:repeat(3,1fr);gap:16px;margin-top:24px}.feature{padding:24px;border:1px solid var(--line);border-radius:18px;background:var(--card)}.feature b{display:block;margin-bottom:8px;font-family:Georgia,"Yu Mincho",serif}.feature p{margin:0;color:var(--muted);font-size:.88rem}
@media(max-width:800px){.grid{grid-template-columns:1fr}.landing-grid{grid-template-columns:1fr}.result-list{grid-template-columns:1fr}.email{display:none}.hero{border-radius:20px}.session-row{grid-template-columns:86px 58px 1fr}}@media(max-width:520px){.shell{width:min(100% - 20px,1120px)}main{padding-top:22px}.nav-links{gap:8px}.nav-links>a:not(.button){display:none}.stats{grid-template-columns:1fr}.two{grid-template-columns:1fr}.panel{padding:20px}.auth-card{padding:24px}.hero-actions{display:grid}.hero-actions .button{width:100%}}
"#;

#[derive(Debug, Clone, Copy)]
struct RuntimeAvailable(bool);

#[must_use]
pub fn router(db: Db, assets: Option<AssetBundle>) -> Router {
    let runtime_available = assets.is_some();
    let builder = Router::builder()
        .cookies()
        .sessions(SessionConfig::default())
        .app_context(db)
        .app_context(RuntimeAvailable(runtime_available));

    let builder = match assets {
        Some(bundle) => builder.assets(bundle),
        None => builder,
    };

    builder.discover().build()
}

fn db(cx: &Cx) -> Db {
    app_context::<Db>(cx).clone()
}

#[memoize]
async fn query_current_user(cx: &Cx) -> Result<Option<User>> {
    let Some(hash) = session::token_hash(cx).await? else {
        return Ok(None);
    };
    let key = token_hash_key(&hash);
    let mut database = db(cx);
    let record = AuthSession::filter_by_token_hash(&key)
        .first()
        .exec(&mut database)
        .await?;

    let Some(record) = record else {
        return Ok(None);
    };
    if record.expires_at <= OffsetDateTime::now_utc().unix_timestamp() {
        AuthSession::filter_by_token_hash(&key)
            .delete()
            .exec(&mut database)
            .await?;
        return Ok(None);
    }

    Ok(User::filter_by_id(record.user_id)
        .first()
        .exec(&mut database)
        .await?)
}

async fn current_user(cx: &Cx) -> Result<Option<User>> {
    query_current_user(cx)
        .await
        .cloned()
        .map_err(|error| std::io::Error::other(error.to_string()).into())
}

async fn require_user(cx: &Cx) -> Result<User> {
    Ok(current_user(cx).await?.ok_or_unauthorized()?)
}

fn token_hash_key(hash: &TokenHash) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut encoded = String::with_capacity(64);
    for byte in hash.iter().copied() {
        encoded.push(char::from(HEX[usize::from(byte >> 4)]));
        encoded.push(char::from(HEX[usize::from(byte & 0x0f)]));
    }
    encoded
}

fn today_jst() -> Date {
    OffsetDateTime::now_utc().to_offset(offset!(+9)).date()
}

fn uniform_label(value: &str) -> Result<&'static str> {
    match Uniform::from_str(value) {
        Ok(Uniform::Gi) => Ok("Gi"),
        Ok(Uniform::NoGi) => Ok("No-Gi"),
        Err(error) => Err(std::io::Error::other(error.to_string()).into()),
    }
}

#[allow(clippy::cast_precision_loss)]
fn browser_id(id: u64) -> f64 {
    id as f64
}

fn bounded_required(value: &str, max: usize, field: &str) -> Result<String> {
    let value = value.trim();
    if value.is_empty() || value.chars().count() > max {
        return Err(bad_request(format!("{field}は1文字以上{max}文字以下にしてください")).into());
    }
    Ok(value.to_owned())
}

fn bounded_optional(value: &str, max: usize, field: &str) -> Result<String> {
    let value = value.trim();
    if value.chars().count() > max {
        return Err(bad_request(format!("{field}は{max}文字以下にしてください")).into());
    }
    Ok(value.to_owned())
}

#[layout("/")]
async fn root_layout(cx: &Cx, slot: Result) -> Result {
    let user = current_user(cx).await?;
    let runtime_available = app_context::<RuntimeAvailable>(cx).0;
    view! {
        <!DOCTYPE html>
        <html lang="ja">
            <head>
                <meta charset="utf-8">
                <meta name="viewport" content="width=device-width, initial-scale=1">
                <meta name="description" content="稽古後に1分で残し、次の稽古前に3分で思い出すBJJトレーニングノート">
                <title>"Tatami Log — 次の稽古につながる柔術ノート"</title>
                <style>(CSS)</style>
                topcoat::dev::script()
                if runtime_available { topcoat::runtime::script() }
            </head>
            <body>
                <header class="site-header">
                    <div class="shell nav">
                        <a class="brand" href="/"><span class="brand-mark">"TL"</span>"Tatami Log"</a>
                        <nav class="nav-links" aria-label="メインナビゲーション">
                            if let Some(user) = user {
                                <a href="/app">"マイ稽古"</a>
                                <span class="email">(&user.email)</span>
                                <form method="post" action="/logout">
                                    <button class="link-button" type="submit">"ログアウト"</button>
                                </form>
                            } else {
                                <a href="/login">"ログイン"</a>
                                <a class="button" href="/register">"はじめる"</a>
                            }
                        </nav>
                    </div>
                </header>
                <main class="shell">(slot?)</main>
            </body>
        </html>
    }
}

#[page("/")]
async fn landing(cx: &Cx) -> Result {
    let destination = if current_user(cx).await?.is_some() {
        "/app"
    } else {
        "/register"
    };
    view! {
        <section class="hero">
            <p class="eyebrow">"Brazilian Jiu-Jitsu · Active Recall"</p>
            <h1>"稽古で習った技を、次の稽古まで忘れない。"</h1>
            <p class="hero-copy">"Tatami Logは、練習記録を復習カードへ変えます。稽古後に1分で残し、マットへ上がる前に3分で思い出す。"</p>
            <div class="hero-actions">
                <a class="button light" href=(destination)>"今日の復習をはじめる"</a>
                <a class="button ghost" href="#how">"使い方を見る"</a>
            </div>
        </section>
        <section id="how" class="landing-grid" aria-label="Tatami Logの使い方">
            <article class="feature"><span class="badge">"01 · Log"</span><b>"稽古後に1分"</b><p>"Gi / No-Gi、ラウンド数、技の合図と次に試すことだけを残します。"</p></article>
            <article class="feature"><span class="badge">"02 · Recall"</span><b>"答えを見ずに思い出す"</b><p>"復習期限になった技を最大3件。忘れた技ほど早く戻ってきます。"</p></article>
            <article class="feature"><span class="badge">"03 · Roll"</span><b>"スパーで確かめる"</b><p>"思い出せたかと、実際に試せたかを分けて記録します。"</p></article>
        </section>
    }
}

#[page("/login")]
async fn login_page(cx: &Cx) -> Result {
    if current_user(cx).await?.is_some() {
        return Err(topcoat::router::error::redirect("/app").into());
    }
    view! {
        <div class="auth-wrap">
            <section class="auth-card">
                <p class="eyebrow">"Welcome back"</p>
                <h1>"ログイン"</h1>
                <p>"前回の気づきを、次の一本につなげましょう。"</p>
                <form class="fields" method="post" action="/login">
                    <label>"メールアドレス"<input type="email" name="email" autocomplete="email" maxlength="254" required=""></label>
                    <label>"パスワード"<input type="password" name="password" autocomplete="current-password" minlength="12" maxlength="128" required=""></label>
                    <button class="full" type="submit">"ログインする"</button>
                </form>
                <p class="auth-switch">"初めてですか？ "<a href="/register">"アカウントを作成"</a></p>
            </section>
        </div>
    }
}

#[page("/register")]
async fn register_page(cx: &Cx) -> Result {
    if current_user(cx).await?.is_some() {
        return Err(topcoat::router::error::redirect("/app").into());
    }
    view! {
        <div class="auth-wrap">
            <section class="auth-card">
                <p class="eyebrow">"Start your practice loop"</p>
                <h1>"Tatami Logを始める"</h1>
                <p>"記録はユーザーごとに分離されます。パスワードは復元できないArgon2idハッシュで保存します。"</p>
                <form class="fields" method="post" action="/register">
                    <label>"メールアドレス"<input type="email" name="email" autocomplete="email" maxlength="254" required=""></label>
                    <label>"パスワード"<input type="password" name="password" autocomplete="new-password" minlength="12" maxlength="128" required=""><span class="hint">"12〜128文字"</span></label>
                    <button class="full" type="submit">"アカウントを作成"</button>
                </form>
                <p class="auth-switch">"アカウントをお持ちですか？ "<a href="/login">"ログイン"</a></p>
            </section>
        </div>
    }
}

#[derive(Deserialize)]
struct CredentialForm {
    email: String,
    password: String,
}

#[route(POST "/register")]
async fn register(cx: &Cx, Form(form): Form<CredentialForm>) -> Result<SeeOther> {
    let email = normalize_email(&form.email).map_err(|error| bad_request(error.to_string()))?;
    validate_password(&form.password).map_err(|error| bad_request(error.to_string()))?;

    let password_hash = hash_password(form.password).await?;
    let mut database = db(cx);
    let mut transaction = database.transaction().await?;
    let user = match toasty::create!(User {
        email: email.clone(),
        password_hash
    })
    .exec(&mut transaction)
    .await
    {
        Ok(user) => user,
        Err(error) => {
            drop(transaction);
            if User::filter_by_email(&email)
                .first()
                .exec(&mut database)
                .await?
                .is_some()
            {
                return Err(bad_request("このメールアドレスは使用できません").into());
            }
            return Err(error.into());
        }
    };
    let revoked = session::stop(cx).await?;
    let issued = session::start(cx).await?;
    if let Some(hash) = revoked {
        AuthSession::filter_by_token_hash(token_hash_key(&hash))
            .delete()
            .exec(&mut transaction)
            .await?;
    }
    let token_hash = token_hash_key(&issued.token_hash);
    let expires_at = OffsetDateTime::from(issued.expires_at).unix_timestamp();
    toasty::create!(AuthSession {
        token_hash,
        user_id: user.id,
        expires_at
    })
    .exec(&mut transaction)
    .await?;
    transaction.commit().await?;

    Ok(see_other("/app"))
}

#[route(POST "/login")]
async fn login(cx: &Cx, Form(form): Form<CredentialForm>) -> Result<SeeOther> {
    let email = normalize_email(&form.email).map_err(|_| unauthorized())?;
    validate_password(&form.password).map_err(|_| unauthorized())?;
    let mut database = db(cx);
    let user = User::filter_by_email(&email)
        .first()
        .exec(&mut database)
        .await?;
    let password_hash = user.as_ref().map_or_else(
        || DUMMY_PASSWORD_HASH.to_owned(),
        |user| user.password_hash.clone(),
    );

    let password_is_valid = verify_password(form.password, password_hash).await?;
    let Some(user) = user.filter(|_| password_is_valid) else {
        return Err(unauthorized().into());
    };

    let revoked = session::stop(cx).await?;
    let issued = session::start(cx).await?;
    let mut transaction = database.transaction().await?;
    if let Some(hash) = revoked {
        AuthSession::filter_by_token_hash(token_hash_key(&hash))
            .delete()
            .exec(&mut transaction)
            .await?;
    }
    let token_hash = token_hash_key(&issued.token_hash);
    let expires_at = OffsetDateTime::from(issued.expires_at).unix_timestamp();
    toasty::create!(AuthSession {
        token_hash,
        user_id: user.id,
        expires_at
    })
    .exec(&mut transaction)
    .await?;
    transaction.commit().await?;
    Ok(see_other("/app"))
}

#[route(POST "/logout")]
async fn logout(cx: &Cx) -> Result<SeeOther> {
    if let Some(hash) = session::stop(cx).await? {
        AuthSession::filter_by_token_hash(token_hash_key(&hash))
            .delete()
            .exec(&mut db(cx))
            .await?;
    }
    Ok(see_other("/"))
}

#[page("/app")]
async fn app_page(cx: &Cx) -> Result {
    let user = current_user(cx).await?.ok_or_redirect("/login")?;
    let mut database = db(cx);
    let sessions = TrainingSession::filter_by_user_id(user.id)
        .order_by(TrainingSession::fields().trained_on().desc())
        .exec(&mut database)
        .await?;
    let cards = TechniqueCard::filter_by_user_id(user.id)
        .order_by(TechniqueCard::fields().id().desc())
        .exec(&mut database)
        .await?;
    let reviews = Review::filter_by_user_id(user.id)
        .exec(&mut database)
        .await?;
    let today = today_jst().to_string();
    let mut latest_reviews = HashMap::new();
    for review in &reviews {
        latest_reviews
            .entry(review.card_id)
            .and_modify(|current: &mut &Review| {
                if (review.reviewed_on.as_str(), review.id)
                    > (current.reviewed_on.as_str(), current.id)
                {
                    *current = review;
                }
            })
            .or_insert(review);
    }
    let mut due_cards = Vec::new();
    for card in &cards {
        if latest_reviews
            .get(&card.id)
            .is_none_or(|review| review.next_review_on.as_str() <= today.as_str())
        {
            due_cards.push(card);
        }
        if due_cards.len() == 3 {
            break;
        }
    }
    let due_count = due_cards.len();
    let total_rounds: u64 = sessions.iter().map(|session| session.rounds).sum();
    let today_label = today.clone();
    let recent_sessions = sessions
        .iter()
        .take(6)
        .map(|session| uniform_label(&session.uniform).map(|label| (session, label)))
        .collect::<Result<Vec<_>>>()?;

    view! {
        <section class="hero">
            <p class="eyebrow">"Today's Tatami · "(today_label)</p>
            <h1>"次の一本で、何を試しますか。"</h1>
            <p class="hero-copy">"答えを見る前に身体の順番を思い出す。その数分が、稽古日誌を実戦の選択肢へ変えます。"</p>
        </section>
        <section class="stats" aria-label="練習統計">
            <div class="stat"><strong>(sessions.len())</strong><span>"記録した稽古"</span></div>
            <div class="stat"><strong>(total_rounds)</strong><span>"スパーリングラウンド"</span></div>
            <div class="stat"><strong>(due_count)</strong><span>"今日の復習（最大3件）"</span></div>
        </section>
        <div class="grid">
            <div class="stack">
                <section class="panel">
                    <div class="panel-head"><div><p class="eyebrow">"Active recall"</p><h2>"今日の復習"</h2><p class="panel-sub">"答えを開く前に、技の最初の動きを再生してください。"</p></div><span class="badge">(due_count)" cards"</span></div>
                    if due_cards.is_empty() {
                        <div class="empty">"今日の復習は完了です。次の稽古を記録しましょう。"</div>
                    } else {
                        for card in due_cards { review_card(card: card) }
                    }
                </section>
                <section class="panel">
                    <div class="panel-head"><div><p class="eyebrow">"Technique library"</p><h2>"自分の技を検索"</h2><p class="panel-sub">"技名・ポジション・合図からサーバー側で絞り込みます。"</p></div></div>
                    technique_search()
                </section>
            </div>
            <div class="stack">
                training_form(today: &today)
                <section class="panel">
                    <div class="panel-head"><div><p class="eyebrow">"Recent sessions"</p><h2>"最近の稽古"</h2></div></div>
                    if sessions.is_empty() {
                        <div class="empty">"まだ記録がありません。最初の稽古を1分で残してみましょう。"</div>
                    } else {
                        <div class="session-list">
                            for (session, uniform) in recent_sessions {
                                <article class="session-row"><strong>(&session.trained_on)</strong><span class="badge">(uniform)</span><p>(session.rounds)" rounds"<br><span class="panel-sub">(&session.reflection)</span></p></article>
                            }
                        </div>
                    }
                </section>
            </div>
        </div>
    }
}

#[component]
async fn training_form(today: &str) -> Result {
    view! {
        <section class="panel">
            <div class="panel-head"><div><p class="eyebrow">"One-minute log"</p><h2>"稽古を記録"</h2><p class="panel-sub">"技はまず1つ。次回に使う合図を短く残します。"</p></div></div>
            <form class="fields" method="post" action="/training-sessions">
                <div class="two">
                    <label>"稽古日"<input type="date" name="trained_on" value=(today) required=""></label>
                    <label>"スタイル"<select name="uniform" required=""><option value="gi">"Gi"</option><option value="no_gi">"No-Gi"</option></select></label>
                </div>
                <label>"スパーリングのラウンド数"<input type="number" name="rounds" value="0" min="0" max="100" required=""></label>
                <label>"今日の振り返り"<textarea name="reflection" maxlength="500" placeholder="例：相手の姿勢を崩し切れなかった"></textarea></label>
                <label>"技の名前"<input name="name" maxlength="100" placeholder="例：クローズドガードからの腕十字" required=""></label>
                <div class="two">
                    <label>"ポジション"<input name="position" maxlength="100" placeholder="クローズドガード" required=""></label>
                    <label>"最初の合図"<input name="cue" maxlength="160" placeholder="肩線を越える" required=""></label>
                </div>
                <label>"思い出す答え"<textarea name="answer" maxlength="800" placeholder="身体の順番や注意点を、自分の言葉で" required=""></textarea></label>
                <label>"次に試すこと"<input name="next_try" maxlength="300" placeholder="例：相手が肘を引いたら三角へつなぐ"></label>
                <button class="full" type="submit">"記録して復習カードを作る"</button>
            </form>
        </section>
    }
}

#[derive(Deserialize)]
struct TrainingForm {
    trained_on: String,
    uniform: String,
    rounds: u64,
    reflection: String,
    name: String,
    position: String,
    cue: String,
    answer: String,
    next_try: String,
}

#[route(POST "/training-sessions")]
async fn create_training(cx: &Cx, Form(form): Form<TrainingForm>) -> Result<SeeOther> {
    let user = require_user(cx).await?;
    let trained_on = parse_iso_date(&form.trained_on)
        .map_err(|_| bad_request("稽古日が正しくありません"))?
        .to_string();
    let uniform = Uniform::from_str(&form.uniform)
        .map_err(|_| bad_request("スタイルが正しくありません"))?
        .as_str()
        .to_owned();
    if form.rounds > 100 {
        return Err(bad_request("ラウンド数は100以下にしてください").into());
    }
    let reflection = bounded_optional(&form.reflection, 500, "振り返り")?;
    let name = bounded_required(&form.name, 100, "技の名前")?;
    let position = bounded_required(&form.position, 100, "ポジション")?;
    let cue = bounded_required(&form.cue, 160, "最初の合図")?;
    let answer = bounded_required(&form.answer, 800, "思い出す答え")?;
    let next_try = bounded_optional(&form.next_try, 300, "次に試すこと")?;

    let mut database = db(cx);
    let mut transaction = database.transaction().await?;
    let training = toasty::create!(TrainingSession {
        user_id: user.id,
        trained_on,
        uniform,
        rounds: form.rounds,
        reflection,
    })
    .exec(&mut transaction)
    .await?;
    toasty::create!(TechniqueCard {
        user_id: user.id,
        training_session_id: training.id,
        name,
        position,
        cue,
        answer,
        next_try,
    })
    .exec(&mut transaction)
    .await?;
    transaction.commit().await?;

    Ok(see_other("/app"))
}

#[component]
async fn review_card(card: &TechniqueCard) -> Result {
    let initial_card_id = browser_id(card.id);
    view! {
        signal revealed = false;
        signal status = String::new();
        signal card_id = initial_card_id;
        <article class="review-card">
            <div class="review-title"><div><span class="position">(&card.position)</span><h3>(&card.name)</h3></div><span class="badge">"Due"</span></div>
            <p class="prompt"><strong>"最初の合図："</strong>(&card.cue)</p>
            <button class="button ghost" @click=$(|_e: Event| revealed.toggle())>$(if revealed.get() { "答えを隠す" } else { "答えを見る" })</button>
            <div class="answer" :hidden=$(!revealed.get())><strong>"自分の答え"</strong><p>(&card.answer)</p>if !card.next_try.is_empty() { <p><strong>"次に試す："</strong>(&card.next_try)</p> }</div>
            <div class="review-actions" aria-label="復習結果">
                <button @click=$(async |_e: Event| { let message = record_review(card_id.get(), "forgot".to_owned(), "not_tried".to_owned()).await; status.set(message); })>"忘れた"</button>
                <button @click=$(async |_e: Event| { let message = record_review(card_id.get(), "fuzzy".to_owned(), "not_tried".to_owned()).await; status.set(message); })>"曖昧"</button>
                <button @click=$(async |_e: Event| { let message = record_review(card_id.get(), "clear".to_owned(), "not_tried".to_owned()).await; status.set(message); })>"説明できる"</button>
                <button @click=$(async |_e: Event| { let message = record_review(card_id.get(), "clear".to_owned(), "attempted".to_owned()).await; status.set(message); })>"スパーで試した"</button>
                <button @click=$(async |_e: Event| { let message = record_review(card_id.get(), "clear".to_owned(), "worked".to_owned()).await; status.set(message); })>"スパーで使えた"</button>
            </div>
            <p class="status" aria-live="polite">$(status.get())</p>
        </article>
    }
}

#[procedure]
async fn record_review(
    cx: &Cx,
    card_id: f64,
    recall: String,
    application: String,
) -> Result<String> {
    let user = require_user(cx).await?;
    if !card_id.is_finite()
        || card_id.fract() != 0.0
        || !(0.0..=9_007_199_254_740_991.0).contains(&card_id)
    {
        return Err(bad_request("カードIDが正しくありません").into());
    }
    let card_id = format!("{card_id:.0}")
        .parse::<u64>()
        .map_err(|_| bad_request("カードIDが正しくありません"))?;
    let recall =
        RecallRating::from_str(&recall).map_err(|_| bad_request("復習結果が正しくありません"))?;
    let application = ApplicationResult::from_str(&application)
        .map_err(|_| bad_request("実戦結果が正しくありません"))?;
    let mut database = db(cx);
    let card = TechniqueCard::filter_by_id(card_id)
        .first()
        .exec(&mut database)
        .await?
        .ok_or_not_found()?;
    if card.user_id != user.id {
        return Err(forbidden().into());
    }

    let reviewed_on = today_jst();
    let schedule = schedule_review(reviewed_on, recall, application);
    let reviewed_on = reviewed_on.to_string();
    let next_review_on = schedule.next_review_on.to_string();
    let recall_rating = recall.as_str().to_owned();
    let application_result = application.as_str().to_owned();
    toasty::create!(Review {
        user_id: user.id,
        card_id,
        reviewed_on,
        recall_rating,
        application_result,
        next_review_on: next_review_on.clone(),
    })
    .exec(&mut database)
    .await?;

    Ok(format!("記録しました。次回は {next_review_on}"))
}

#[component]
async fn technique_search() -> Result {
    view! {
        signal query = String::new();
        <div class="search-box"><input type="search" aria-label="技を検索" placeholder="腕十字、クローズドガード、肩線…" maxlength="80" @input=$(|event: Event| query.set(event.target.value))></div>
        technique_results(query: $(query.get()))
    }
}

#[shard]
async fn technique_results(cx: &Cx, query: String) -> Result {
    let user = require_user(cx).await?;
    if query.chars().count() > 80 {
        return Err(bad_request("検索語は80文字以下にしてください").into());
    }
    let needle = query.trim().to_lowercase();
    let mut cards = TechniqueCard::filter_by_user_id(user.id)
        .exec(&mut db(cx))
        .await?;
    cards.sort_by_key(|card| Reverse(card.id));
    let matches = cards
        .into_iter()
        .filter(|card| {
            needle.is_empty()
                || card.name.to_lowercase().contains(&needle)
                || card.position.to_lowercase().contains(&needle)
                || card.cue.to_lowercase().contains(&needle)
        })
        .take(8)
        .collect::<Vec<_>>();

    view! {
        if matches.is_empty() {
            <div class="empty">"一致する技はありません。表記を変えて試してください。"</div>
        } else {
            <div class="result-list">
                for card in matches {
                    <article class="result"><span class="position">(&card.position)</span><strong>(&card.name)</strong><p>"合図："(&card.cue)</p></article>
                }
            </div>
        }
    }
}
