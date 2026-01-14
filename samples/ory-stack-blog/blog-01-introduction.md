# Ory HydraでOAuth2認可サーバーを構築する

## はじめに

認可サーバーを構築するタスクがアサインされた。技術選定の裁量はある。仕事の合間にRFC 6749を読み始めた。

[https://datatracker.ietf.org/doc/html/rfc6749:embed:cite]

帰宅後の深夜、週末の空き時間。3日目の深夜2時、私は確信した。

これは自前で作るべきではない。

認可コードフロー、インプリシットフロー、リソースオーナーパスワードクレデンシャル、クライアントクレデンシャル。4つのグラントタイプ。それぞれにセキュリティ要件がある。PKCEも必要だ。OpenID Connectも。IDトークンのクレーム設計。JWKSエンドポイント。セッション管理。トークン失効。リフレッシュトークンのローテーション。

仕様を読めば理解できる。実装もできる。でも、これをプロダクション品質で検証し続けるのは、私たちの仕事ではない。3日間RFCを読んで分かったのは、「自前で作ることの非合理性」だった。

調べていく中で、OpenAIがOryを採用していることを知った。

[https://www.ory.com/case-studies/openai:embed:cite]

彼らは認可サーバーの実装に時間を使わないことを選んだ。彼らの本業はAIモデルの開発だ。認証認可は重要だが、「解くべき問題」ではなく「解決済みの問題を使う」領域として扱っている。妥当な判断だと思う。

Ory Hydraを採用することにした。

[https://www.ory.sh/hydra/:embed:cite]

[https://github.com/ory/hydra:embed:cite]

この記事では、Hydraのアーキテクチャを解説し、Docker Composeで実際に動かすところまでやる。OAuth2/OIDCの基本概念は知っている前提で進める。

## 「認証をしない認可サーバー」という逆説

[https://www.ory.com/docs/hydra/:embed:cite]

Hydraのドキュメントを読んでいて、ある一文で手が止まった。

「Hydraは認証をしません」

認可サーバーなのに認証しない。最初は設計の欠落かと思った。Auth0やKeycloakは全部やってくれるのに。

だが、ドキュメントを読み進めるうちに意図が見えてきた。

**これは欠陥ではない。これこそが設計の核心だ。**

考えてみてください。あなたの会社には、おそらく既にユーザーデータベースがある。10年使ってきた認証システムがある。LDAPで認証している。多要素認証は自前のものを使っている。パスキー対応も進めている。

一般的なIdP——Auth0やKeycloak——を導入すると、これらを全部IdP側に合わせなければなりません。データ移行。認証フローの再設計。既存システムとの複雑な連携。

Hydraは違うアプローチを取ります。

**「認証はあなたたちでやってください。終わったら教えてくれれば、あとはこちらでOAuth2/OIDCの面倒なことは全部やります」**

この瞬間、私の中で何かがカチッとはまりました。

既存の認証システムはそのまま。ユーザーDBもいじらない。ただ、OAuth2/OIDCのプロトコル層だけをHydraに任せる。認証と認可の責務が完全に分離される。

これが「ヘッドレス」な認可サーバーというコンセプトです。具体的には以下のメリットがあります。

- **既存システムはそのまま使える**: ユーザーDB・認証ロジックをいじらなくていい
- **認証方法は完全に自由**: パスワード、パスキー、生体認証、なんでも
- **Hydraが担保するのはプロトコル準拠**: OpenID Connect Certificationを取得済み

[https://openid.net/certification/:embed:cite]

## アーキテクチャの全体像

[https://www.ory.com/docs/oauth2-oidc/custom-login-consent/flow:embed:cite]

Hydraを使ったシステムは、3つのコンポーネントで構成されます。

**Hydra Public API**（ポート4444）はOAuth2/OIDCの「顔」です。クライアントアプリケーションが`/oauth2/auth`に認可リクエストを投げ、`/oauth2/token`でトークンを受け取る。ここはHydraが全部やってくれます。

**Login/Consent Provider**（ポート3000）が私たちの実装領域です。Hydraからリダイレクトされてきたユーザーに対して、`/login`で認証画面を、`/consent`で同意画面を表示します。「このユーザーは本人か？」「このスコープを許可するか？」という判断を担う。ここに既存の認証ロジックを組み込みます。

**Hydra Admin API**（ポート4445）は裏方です。Login/Consent Providerが認証・同意の結果をHydraに通知するために使います。チャレンジの検証、承認の通知、セッション管理を担当します。外部には公開せず、内部ネットワークからのみアクセスさせます。

この構成を理解したとき、肩の荷が下りた気がしました。OAuth2/OIDCの複雑な部分はHydraに任せて、自分たちは「認証」という本質的な部分だけに集中できる。これなら、やれそうだ。

## チャレンジベースのフロー

[https://www.ory.com/docs/hydra/reference/api:embed:cite]

HydraとProviderの連携には「チャレンジ」という仕組みが使われます。

最初は「なんで直接やり取りしないんだろう」と思いました。でも、この設計にはちゃんと理由があります。

1. クライアントがHydraの`/oauth2/auth`にリダイレクト
2. Hydraが`login_challenge`を生成し、Login Providerにリダイレクト
3. Login Providerは`login_challenge`を検証し、ユーザーを認証
4. 認証成功後、Admin APIで承認を通知し、Hydraに戻る
5. Hydraが`consent_challenge`を生成し、Consent Providerにリダイレクト
6. Consent Providerはスコープを確認し、Admin APIで承認
7. クライアントに認可コードが返される

チャレンジは一度きりの使い捨てトークンです。傍受されても再利用できない。リプレイ攻撃やセッションハイジャックを構造的に防ぎます。

この手のセキュリティ上の細かい配慮——正直、自前実装だと見落としがちだ。PKCEのcode_verifierの長さ制限（43-128文字）。stateパラメータに暗号学的に安全な乱数を使うべきこと。RFCを読んでいたあの3日間で、攻撃ベクトルをどれだけ考慮できていたか。

Hydraはこれらをすべて内包しています。OpenID Connect Certificationを取得しているということは、私が見落としていたであろう細部まで検証されているということです。

## Docker Compose環境の構築

[https://www.ory.com/docs/hydra/self-hosted/configure-deploy:embed:cite]

理論は十分。実際に動かしてみましょう。

OAuth2/OIDCの仕様は複雑です。RFC 6749を読んでも、認可コードフローの全体像が頭に入らなかった。実際にcurlでリクエストを投げ、リダイレクトを追いかけることで、初めて仕様書の抽象的な記述が腑に落ちました。

開発環境は4つのサービスで構成されます。HydraのDockerイメージは公式で提供されています。

[https://hub.docker.com/r/oryd/hydra/:embed:cite]

```yaml
services:
  postgres:
    image: postgres:16-alpine
    environment:
      POSTGRES_USER: hydra
      POSTGRES_PASSWORD: secret
      POSTGRES_DB: hydra
    volumes:
      - postgres_data:/var/lib/postgresql/data
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U hydra -d hydra"]
      interval: 5s
      timeout: 5s
      retries: 5

  hydra-migrate:
    image: oryd/hydra:v2.2
    environment:
      DSN: postgres://hydra:secret@postgres:5432/hydra?sslmode=disable
    command: migrate sql -e --yes
    depends_on:
      postgres:
        condition: service_healthy

  hydra:
    image: oryd/hydra:v2.2
    environment:
      DSN: postgres://hydra:secret@postgres:5432/hydra?sslmode=disable
      SECRETS_SYSTEM: super-secret-system-secret-at-least-32-chars
      URLS_SELF_ISSUER: http://localhost:4444
      URLS_CONSENT: http://localhost:3000/consent
      URLS_LOGIN: http://localhost:3000/login
      URLS_LOGOUT: http://localhost:3000/logout
      LOG_LEVEL: debug
    command: serve all --dev
    ports:
      - "4444:4444"
      - "4445:4445"
    depends_on:
      hydra-migrate:
        condition: service_completed_successfully
    healthcheck:
      test: ["CMD", "wget", "-q", "--spider", "http://localhost:4444/health/ready"]
      interval: 10s
      timeout: 5s
      retries: 5

  auth-provider:
    build: .
    environment:
      HOST: 0.0.0.0
      PORT: 3000
      HYDRA_ADMIN_URL: http://hydra:4445
      RUST_LOG: ory_hydra_rust=debug,tower_http=debug
    ports:
      - "3000:3000"
    depends_on:
      hydra:
        condition: service_healthy

volumes:
  postgres_data:
```

> **注意**: 上記の設定は開発環境用です。本番環境では`SECRETS_SYSTEM`に32文字以上の暗号学的に安全な値を設定し、`sslmode=disable`は`require`に変更してください。

[https://docs.docker.com/compose/:embed:cite]

`auth-provider`サービスの`build: .`は、Login/Consent ProviderのDockerfileを参照しています。このDockerfileとRust実装は次回の記事で解説します。今回はHydraのアーキテクチャ理解に集中しましょう。サンプルコードは以下のリポジトリで公開しています。

[https://github.com/nwiizo/workspace_2026/tree/main/samples/ory-hydra-rust:embed:cite]

`depends_on`と`healthcheck`の組み合わせがポイントです。PostgreSQL → マイグレーション → Hydra → auth-providerという起動順序が保証されます。私は最初これを書かずに「DBがない」エラーで30分悩みました。

## 環境の起動と動作確認

```sh
docker compose up -d --build
docker compose logs -f auth-provider
```

ヘルスチェック用エンドポイントにアクセスしてみます。

```sh
curl http://localhost:3000/health
# {"status":"healthy"}
```

`{"status":"healthy"}`が返ってきた。

たった数十行のdocker-compose.ymlで、OAuth2認可サーバーの基盤が動いている。RFCを読んでいたあの3日間で見えた複雑さが、Hydraの中に隠蔽されている。

## OAuth2クライアントの登録

OAuth2フローをテストするには、まずクライアントを登録します。

[https://www.ory.sh/docs/hydra/cli/hydra-create-oauth2-client:embed:cite]

```sh
docker compose exec hydra hydra create oauth2-client \
  --endpoint http://localhost:4445 \
  --grant-type authorization_code \
  --response-type code \
  --scope openid,offline_access,profile,email \
  --redirect-uri http://localhost:8080/callback \
  --name "Test Client"
```

クライアントIDとシークレットが出力されるので控えておきます。

## OAuth2フローのテスト

テストユーザーを作成します。

```sh
curl -X POST http://localhost:3000/api/auth/register \
  -H "Content-Type: application/json" \
  -d '{"email": "test@example.com", "password": "password123"}'
```

ブラウザで認可エンドポイントにアクセスします（`<CLIENT_ID>`は先ほど取得したもの）。

```
http://localhost:4444/oauth2/auth?client_id=<CLIENT_ID>&response_type=code&scope=openid+profile+email&redirect_uri=http://localhost:8080/callback&state=random_state
```

フローは以下のように進みます。

1. Hydraがログイン画面にリダイレクト
2. メールアドレスとパスワードを入力してログイン
3. Hydraが同意画面にリダイレクト
4. スコープを確認して同意
5. `http://localhost:8080/callback?code=...`にリダイレクト

リダイレクト先（8080）は存在しなくても構いません。URLから認可コードを取得できれば成功です。

## おわりに

この記事を書き終えて、時計を見た。深夜1時だ。

正直に言うと、書いている途中で何度かRFCのタブを開いてしまった。「この説明で合ってるかな」と不安になって。

私はこの記事を書いたからといって、OAuth2/OIDCを完全に理解したわけではない。たぶん来週も、仕様書の細部で「あれ？」となる瞬間がある。

でも、少しだけ違うことがある。

3日目の深夜2時、RFCのタブを20個開いて、私は判断した。これは自前で作るべきではない、と。仕様は理解できる。実装もできる。でも、プロダクション品質で検証し続けることは、私たちの仕事ではない。

Hydraのアーキテクチャを理解して、Docker Composeで動かしてみて、その判断が正しかったと確信した。認証と認可は分離できる。複雑なプロトコル層は、検証済みの実装に任せていい。私が書くべきコードは、真ん中の「Login/Consent Provider」だけだ。

「認可サーバーを自前で作ってくれ」

もしあなたが今、この言葉を受けてRFCを読んでいるなら。3日読めば分かる。作れるかどうかではない。作るべきかどうかだ。

RFCを読むことには意味がある。私もあの3日間があったから、Hydraの設計思想が腑に落ちた。でも、プロダクション品質の認可サーバーを一人で検証し続ける必要はない。検証済みの実装がある。

明日の朝、目覚ましが鳴る。また仕事が始まる。

おい、RFCのタブを閉じろ。Hydraのドキュメントを開け。

何度でも思い出せることの方が大事だ。次の記事では、RustでLogin/Consent Providerを実装する。一緒に認証画面を作ろう。
