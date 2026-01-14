# RustでOry Hydra用認証プロバイダーを実装する
## はじめに

年が明けた。月曜日。エディタを開いている。

認証プロバイダーを自分で実装できるか、と聞かれたら、たぶん「できる」と答える。OAuth2のRFCは読んだ。フローも理解している、と思う。ただ、「じゃあ書いて」と言われたとき、キーボードに手を置いたまま止まってしまうことがある。頭では分かっている。手が動かない。

10年近くインフラやプラットフォームを触ってきた。認可の仕組みは何度も設計した。Kubernetesの認証、サービスメッシュの認可、アクセストークンの検証。それでも「Login Providerをゼロから書け」と言われると、急に自信がなくなる。分かっているはずなのに、分かっていない気がする。

Ory Hydraのドキュメントを開く。Login ProviderとConsent Providerを自分で実装しろ、と書いてある。Node.jsのサンプルがある。Goのサンプルもある。どちらも動く。でも私はRustで書きたかった。

年末年始、ぼんやり考えていて気づいたことがある。止まっているのは、技術的に難しいからではない気がする。「何をどの順番で実装すればいいのか」が見えていないのだ。全体像が掴めないまま、最初の一歩が踏み出せずにいる。

だからこの記事を書くことにした。過去の自分に向けて。最初の一歩を、順番に。

> **前提知識**: この記事は[前回の記事](https://syu-m-5151.hatenablog.com/entry/2026/01/04/133007)の続編です。OAuth2認可コードフローの基礎知識と、Ory Hydraのアーキテクチャ（Login/Consent Providerの役割）を理解している前提で進めます。

[https://syu-m-5151.hatenablog.com/entry/2026/01/04/133007:embed:cite]

## 作るもの

Login/Consent Providerとは、Ory Hydraと連携してOAuth2認証フローを処理するWebアプリケーションだ。以下の5つのエンドポイントを実装する。

| エンドポイント | 役割 |
|---------------|------|
| `GET /login` | ログインフォームを表示する |
| `POST /login` | 認証処理を行い、Hydraに結果を通知する |
| `GET /consent` | スコープ承認画面を表示する |
| `POST /consent` | 承認結果をHydraに通知し、トークン発行へ進む |
| `GET /logout` | ログアウト処理を行い、セッションを破棄する |

## 全体の流れ

OAuth2認可コードフローの中で、Login/Consent Providerがどう動くかを示す。

```
1. ユーザーがクライアントアプリで「ログイン」をクリック
2. クライアントがHydraの /oauth2/auth にリダイレクト
3. Hydra が Login Provider の GET /login にリダイレクト（login_challenge付き）
4. Login Provider がログインフォームを表示
5. ユーザーがメール・パスワードを入力して送信
6. Login Provider が認証し、Hydra に accept_login を送信
7. Hydra が Consent Provider の GET /consent にリダイレクト（consent_challenge付き）
8. Consent Provider がスコープ承認画面を表示
9. ユーザーが承認
10. Consent Provider が Hydra に accept_consent を送信
11. Hydra がクライアントにリダイレクト（認可コード付き）
12. クライアントが認可コードをトークンに交換
```

Login/Consent Providerが担当するのは3〜10だ。Hydraとの通信には6つのAPIを使う。

| API | 役割 |
|-----|------|
| `GET /admin/oauth2/auth/requests/login` | login_challengeからリクエスト情報を取得 |
| `PUT /admin/oauth2/auth/requests/login/accept` | 認証成功をHydraに通知 |
| `GET /admin/oauth2/auth/requests/consent` | consent_challengeからリクエスト情報を取得 |
| `PUT /admin/oauth2/auth/requests/consent/accept` | 承認結果をHydraに通知 |
| `GET /admin/oauth2/auth/requests/logout` | logout_challengeからリクエスト情報を取得 |
| `PUT /admin/oauth2/auth/requests/logout/accept` | ログアウトをHydraに通知 |

[https://www.ory.com/docs/hydra/reference/api:embed:cite]

## Login Handler

Login Handlerは2つのエンドポイントで構成される。

### GET /login

1. クエリパラメータから`login_challenge`を取得する
2. Hydra APIで`login_challenge`を検証し、リクエスト情報を取得する
3. `skip`フラグが立っていれば（既にセッションがあれば）、フォームを表示せず即座に`accept_login`
4. そうでなければログインフォームを表示する

### POST /login

1. フォームから`email`、`password`、`login_challenge`を受け取る
2. 認証サービスでパスワードを検証する
3. 認証成功なら、ユーザー情報を`context`に詰めて`accept_login`を呼ぶ
4. Hydraが返すリダイレクトURLへ転送する

```rust
pub async fn login_submit(
    State(state): State<AppState>,
    Form(form): Form<LoginForm>,
) -> Result<Redirect, AppError> {
    // 1. 認証処理
    let user = state.auth.authenticate(&form.email, &form.password).await?;

    // 2. ユーザー情報をcontextに保存（Consent時にDBルックアップ不要）
    let user_context = UserContext {
        email: user.email.clone(),
        role: "customer".to_string(),
        tenant_id: None,
    };

    // 3. Hydraに認証成功を通知
    let completed = state
        .hydra
        .accept_login(
            &form.login_challenge,
            &user.id.to_string(),
            false,
            Some(serde_json::to_value(&user_context)?),
        )
        .await?;

    // 4. Consent画面へリダイレクト
    Ok(Redirect::to(&completed.redirect_to))
}
```

ポイントは`context`だ。Login時に認証したユーザー情報（email、role、tenant_id）をJSON形式で保存し、Consent Providerへ受け渡す。これにより、Consent処理でDBルックアップが不要になる。

## Consent Handler

Consent Handlerも2つのエンドポイントで構成される。

### GET /consent

1. クエリパラメータから`consent_challenge`を取得する
2. Hydra APIでリクエスト情報（要求されたスコープ、クライアント情報）を取得する
3. `skip`フラグが立っていれば（既に承認済みなら）、即座に`accept_consent`
4. そうでなければスコープ承認画面を表示する

### POST /consent

1. フォームから`consent_challenge`と承認するスコープを受け取る
2. Login時に保存した`context`からユーザー情報を取得する
3. IDトークンにカスタムクレーム（`email`、`role`、`tenant_id`）を追加する
4. `accept_consent`を呼び、Hydraが返すリダイレクトURLへ転送する

IDトークンにクレームを追加することで、クライアントアプリケーションはトークンをデコードするだけでユーザー情報を取得できる。

## Logout Handler

Logout Handlerは1つのエンドポイントで構成される。Login/Consentと比べてシンプルだ。

### GET /logout

1. クエリパラメータから`logout_challenge`を取得する
2. Hydra APIで`accept_logout`を呼び出す
3. Hydraが返すリダイレクトURLへ転送する

```rust
pub async fn logout_handler(
    State(state): State<AppState>,
    Query(query): Query<LogoutQuery>,
) -> Result<Redirect, AppError> {
    let completed = state.hydra.accept_logout(&query.logout_challenge).await?;
    Ok(Redirect::to(&completed.redirect_to))
}
```

ログアウトフローはLogin/Consentと異なり、確認画面を表示せずに即座に`accept_logout`を呼んでいる。本番環境では「本当にログアウトしますか？」という確認画面を挟むことを検討してもよい。

## 動作確認

```sh
docker compose up -d
./scripts/e2e-test.sh
```

IDトークンに`email`、`role`、`sub`が含まれていれば成功だ。

ここまでが「何を作るか」「どう動くか」の説明だ。以降は実装の詳細に入る。

## 認証サービスの実装

Login Handlerから呼び出される認証サービスの実装に入る。パスワード認証にはOWASPのガイドラインに従い、Argon2idを採用した。

[https://cheatsheetseries.owasp.org/cheatsheets/Password_Storage_Cheat_Sheet.html:embed:cite]

`Argon2::default()`を使っているが、これは意図的だ。`argon2`クレートのデフォルト値はOWASP推奨設定に準拠している。「専門家が作ったものを信頼する方が合理的」という前回の記事と同じ論理だ。

認証部分で見落としがちなのが次の点だ。

```rust
pub async fn authenticate(&self, email: &str, password: &str) -> Result<User, AppError> {
    let users = self.users.read().await;
    let user = users.get(email).ok_or(AppError::InvalidCredentials)?;

    Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .map_err(|_| AppError::InvalidCredentials)?;

    Ok(user.clone())
}
```

ユーザーが存在しない場合も、パスワードが間違っている場合も、返すエラーは同じ`InvalidCredentials`だ。「ユーザーが見つかりません」というエラーを返したくなるが、それは攻撃者に情報を与えてしまう。

これはユーザー列挙攻撃（User Enumeration Attack）への対策だ。攻撃者はまず有効なメールアドレスを特定しようとする。エラーメッセージが違えば、登録済みかどうかが分かってしまう。

なお、完全な対策にはタイミング攻撃への考慮も必要だ。ユーザーが存在しない場合はArgon2の検証が走らないため、レスポンス時間の差で存在を推測される可能性がある。本番環境では、ユーザー不在時もダミーハッシュを検証することを検討してほしい。

[https://owasp.org/www-project-web-security-testing-guide/latest/4-Web_Application_Security_Testing/03-Identity_Management_Testing/04-Testing_for_Account_Enumeration_and_Guessable_User_Account:embed:cite]

## テスト設計

認証システムのバグは「静かに」起きる。だからテストの考え方も変わる。普通の機能開発では「この操作をしたらこうなる」というテストを書く。でも認証システムでは**「この操作をしてもこうならない」**というテストの方に価値がある。

```rust
#[tokio::test]
async fn test_login_does_not_reveal_user_existence() {
    let service = AuthService::new();
    service.register("exists@example.com", "password").await.unwrap();

    let err1 = service.authenticate("exists@example.com", "wrong").await.unwrap_err();
    let err2 = service.authenticate("nobody@example.com", "password").await.unwrap_err();

    assert_eq!(err1.to_string(), err2.to_string());
}
```

このテストは「エラーメッセージが同じ」という実装の意図を明示化している。将来誰かが「親切なエラーメッセージにしよう」と思って変更しても、このテストが警告を出す。

### 責任分界点

全ての攻撃をアプリケーション層で防ぐ必要はない。何を守り、何をインフラに任せるかを明確にする。

- **ブルートフォース対策**: Nginxのrate limitで弾く
- **セッション固定化攻撃**: フレームワーク（Axum + tower-sessions）に委譲
- **HTTPS強制**: インフラ設定の問題

## プロジェクト構成

今回はAxumを使った。

[https://github.com/tokio-rs/axum:embed:cite]

```
src/
├── main.rs          # サーバーエントリーポイント
├── auth.rs          # 認証サービス
├── handlers.rs      # Login/Consent/Logoutハンドラー
├── hydra.rs         # Hydra Admin APIクライアント
├── models.rs        # Hydra API型定義
└── error.rs         # エラー型定義
```

ハンドラー層とサービス層を分離している。認証ロジックは`auth.rs`に置き、ハンドラーはHTTPリクエストの受け取りとレスポンスの返却だけを担う。

フルコードはGitHubリポジトリを参照してほしい。

[https://github.com/nwiizo/workspace_2026/tree/main/samples/ory-hydra-verification:embed:cite]

## 実装チェックリスト

### 必須の実装

- [ ] **Hydra APIクライアント** - 6つのAPI呼び出し
- [ ] **GET /login** - `login_challenge`検証、`skip`フラグ確認、フォーム表示
- [ ] **POST /login** - 認証、`context`にユーザー情報、`accept_login`
- [ ] **GET /consent** - `consent_challenge`検証、`skip`フラグ確認、承認画面表示
- [ ] **POST /consent** - `context`取得、IDトークンにクレーム追加、`accept_consent`
- [ ] **GET /logout** - `logout_challenge`取得、`accept_logout`
- [ ] **認証サービス** - Argon2id、ユーザー列挙攻撃対策

### 忘れがちなポイント

- `login_challenge`と`consent_challenge`はhiddenフィールドでフォームに埋め込む
- `skip`フラグが立っている場合は画面を表示せず即座にacceptする
- `context`でLogin→Consent間のユーザー情報受け渡し
- エラーメッセージはユーザーの存在を漏らさない

## おわりに

この文章を書き終えて、ターミナルに戻った。`docker compose up -d`を叩く。コンテナが立ち上がる。E2Eテストを走らせる。グリーン。IDトークンに`email`と`role`が入っている。動いた。

正直に言うと、書いている途中で何度か不安になった。これで説明になっているのか。Login HandlerとConsent Handlerの違いが曖昧になっていないか。`context`の使い方は2回書き直した。

それでも、動いた。

冒頭で書いた「キーボードに手を置いたまま止まってしまう」感覚は、たぶん、また来る。次に認証システムを書くときも、OAuth2のフローを思い出すところから始めるだろう。`login_challenge`って何だっけ、と調べ直すかもしれない。

それでいいのだと思う。認証は「一度理解したら終わり」という領域ではない気がする。毎回、RFCを確認しながら、慎重に実装する。ユーザー列挙攻撃のテストを書いたのも、将来の自分が「親切なエラーメッセージ」を入れようとしたときに止めるためだ。

年が明けて、また仕事が始まる。本番の認証システムはOry Hydraに任せる。Login Providerは自分で書く。その境界線が、今の私には見えている気がする。
