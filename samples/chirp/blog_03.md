# Leptos 0.8 で Twitter クローンを作って学んだこと【後編】Rust の言語哲学とフロントエンドの交差点

[https://leptos.dev/:embed:cite]

[https://github.com/leptos-rs/leptos:embed:cite]

> 本記事は3部構成です。
> - [前編](blog_01.md): Leptos の基本機能 — プロジェクト構成、コンポーネント、Signal、サーバー関数、ルーティング
> - [中編](blog_02.md): バックエンドの設計パターン — DB 設計、認証、検索、通知
> - **後編（本記事）**: Rust の言語哲学とフロントエンドの交差点 — 所有権、クロージャ、型システムの衝突と折り合い

## 目次

1. [所有権 vs 共有可変状態 — 根本的な思想の衝突](#1-所有権-vs-共有可変状態--根本的な思想の衝突)
2. [`move ||` の氾濫 — クロージャと所有権の摩擦](#2-move--の氾濫--クロージャと所有権の摩擦)
3. [型の厳格さ vs HTML の寛容さ — `.into_any()` という翻訳](#3-型の厳格さ-vs-html-の寛容さ--intoany-という翻訳)
4. [明示性の文化 — 「なぜ再レンダリングされた？」が存在しない世界](#4-明示性の文化--なぜ再レンダリングされたが存在しない世界)
5. [エラーの扱い方 — 楽観と悲観の間](#5-エラーの扱い方--楽観と悲観の間)
6. [`#[server]` — 二つの世界をつなぐマクロの魔法](#6-server--二つの世界をつなぐマクロの魔法)
7. [`view!` マクロ — 二つの構文体系の結婚](#7-view-マクロ--二つの構文体系の結婚)
8. [コンパイル時間というコスト — 安全性の代償](#8-コンパイル時間というコスト--安全性の代償)
9. [楽観的更新 — 正確性と即時性の交渉をコードで読む](#9-楽観的更新--正確性と即時性の交渉をコードで読む)
10. [`Effect` — 副作用に名前を付ける](#10-effect--副作用に名前を付ける)
11. [蓄積する状態 — `Resource` の限界と `RwSignal` への移行](#11-蓄積する状態--resource-の限界と-rwsignal-への移行)
12. [言語の選択は思考の選択](#12-言語の選択は思考の選択)
13. [まとめ — Leptos の設計思想と実装から得た教訓](#13-まとめ--leptos-の設計思想と実装から得た教訓)

---



Chirp を実装する中で、Rust の言語特性とフロントエンド開発の要請が衝突する場面に何度も遭遇した。それぞれの衝突は、Leptos がどう設計されたかを理解する手がかりになる。そして言語の設計思想がコードの隅々にまで影響を及ぼす様子は、自然言語の文法が話者の思考を形作るのと似て、とても興味深い。

## 1. 所有権 vs 共有可変状態 — 根本的な思想の衝突

Rust の最も根源的な設計判断は「共有参照（`&T`）か、排他参照（`&mut T`）か、どちらか一方」というルールだ。これは C/C++ の「誰がこのメモリを所有しているのか分からない」という問題に対する、コンパイル時の解答として生まれた。

一方、フロントエンド開発は本質的に **共有された変更可能な状態** の世界だ。ユーザーがボタンをクリックすると、いいねカウンターが変わり、ボタンの色が変わり、場合によっては他のコンポーネントの表示も連動して変わる。すべてが共有されていて、すべてが変更される。

この衝突に対する Leptos の解答が **Signal** だ。Chirp の `ActionBar` を見てみよう:

```rust
let (is_liked, set_liked) = signal(liked);
let (local_like_count, set_like_count) = signal(like_count);

let on_like = move |_| {
    let was_liked = is_liked.get_untracked();
    set_liked.set(!was_liked);
    set_like_count.update(|c| {
        if was_liked { *c -= 1 } else { *c += 1 }
    });
};
```

ここで `is_liked` は `ReadSignal<bool>` であり、`set_liked` は `WriteSignal<bool>` だ。重要なのは、Leptos 0.7 以降で **Signal が `Copy` トレイトを実装している** ことだ。Rust の所有権モデルでは、値をクロージャに渡すと所有権が移動（move）する。しかし Signal が `Copy` なら、移動ではなくコピーされる。複数のクロージャが同じ Signal を自由に参照できるのだ。

これは偶然の設計ではない。内部的には Signal はアリーナアロケータ上のインデックス（実質的にはただの整数）であり、実データへの「チケット」にすぎない。実データはリアクティブランタイムが一元管理する。Rust の所有権システムをバイパスするのではなく、**所有権を持つのはランタイムであり、Signal はそのハンドルにすぎない** という設計で衝突を解消している。

これは React の `useState` とも、Vue の `ref` とも異なるアプローチだ。React はイミュータブルな値の再代入（と仮想 DOM の差分検出）で状態変更を表現する。Vue は JavaScript の `Proxy` で変更を検出する。Leptos は Rust の型システムの中で、`Copy` + 内部可変性（interior mutability）という言語機能を使って、同じ問題に異なる角度から答えている。

## 2. `move ||` の氾濫 — クロージャと所有権の摩擦

Chirp のコードで最も目を引くのは、`move ||` クロージャの多さだろう。`PostComposer` だけでも 5 つの `move` クロージャがある:

```rust
// テキストエリアの値をバインド
prop:value=move || content.get()
// 入力イベントをハンドル
on:input=move |ev| { set_content.set(event_target_value(&ev)); }
// 文字数カウンターの色を動的に変える
class=move || {
    if char_count() > 260 { "text-sm text-red-500" }
    else { "text-sm text-gray-500" }
}
// 文字数テキストを動的に更新
{move || format!("{}/280", char_count())}
// 送信ボタンの disabled 状態
disabled=move || { content.get().trim().is_empty() || char_count() > 280 }
```

なぜこれほど `move` が必要なのか？ Rust のクロージャはデフォルトで外部変数を **参照でキャプチャ** する。しかし `view!` マクロが生成するリアクティブノードは、コンポーネント関数が返った後も生き続ける。参照は借用元より長生きできない — これは Rust のライフタイムルールだ。だから `move` で所有権をクロージャに移す必要がある。

JavaScript では関数はクロージャのスコープチェーンを通じて外部変数を自動でキャプチャし、ガベージコレクタが生存期間を管理するから、こうした問題は存在しない。Rust はガベージコレクタを持たないからこそ所有権の明示が必要であり、その結果が `move ||` の氾濫となって表出する。

ここで Signal の `Copy` 実装が効いてくる。もし Signal が `Copy` でなければ、同一スコープ内で複数の `move` クロージャに Signal を渡すたびに `.clone()` が必要になる。かつての Leptos 0.6 以前では、実際に `let is_liked = is_liked.clone();` を何行も書く必要があった。Signal を `Copy` にしたことで、この冗長さが一掃されたのだ。

これは Rust の言語設計者たちが `Copy` トレイトに込めた意味 — 「ビットレベルのコピーで完結し、追加のセマンティクスを持たない型」 — を、リアクティブフレームワークの設計者が巧みに利用した例と言える。Signal は実質的にはただの `usize`（アリーナ上のスロットインデックス）であり、`Copy` の契約に忠実だ。

## 3. 型の厳格さ vs HTML の寛容さ — `.into_any()` という翻訳

ブラウザは「タグスープ」を受け入れる。閉じタグがなくても、属性が不正でも、何とかレンダリングする。HTML は「壊れない」ことを最優先にした言語だ。

Rust は対極にある。すべてが型チェックを通らなければコンパイルすらされない。`view!` マクロはこの二つの世界の翻訳層として機能するが、完全には隠し切れない場面がある。

`HomePage` の条件分岐を見てみよう:

```rust
match result {
    Ok(posts) => view! { <PostList posts=posts /> }.into_any(),
    Err(e) => view! {
        <div class="p-4 text-red-500">{format!("エラー: {e}")}</div>
    }.into_any(),
}
```

`.into_any()` は Rust の型システムとの折衝だ。`view! { <PostList .../> }` と `view! { <div>...</div> }` は **異なる型** を返す。Rust の `match` は全アームが同じ型を返す必要があるから、型消去（type erasure）で `AnyView` に統一する。

HTML の世界では「ここに div が来るか ul が来るかは実行時に決まる」のは当然だが、Rust の世界では「コンパイル時に型が確定しない」のは異常事態だ。`.into_any()` は、HTML の寛容さを Rust の型システムに通すための小さな妥協点として、Chirp のコードの至るところに現れる。

同様に `Option` の扱いも特徴的だ。プロフィールページのヘッダー画像:

```rust
{user.header_url.clone().map(|url| {
    view! { <img src=url class="w-full h-full object-cover" alt="header" /> }
})}
```

JavaScript なら `{headerUrl && <img src={headerUrl} />}` と書ける。Rust では `Option<T>` を明示的に `.map()` で展開する必要がある。冗長に見えるかもしれない。しかしこの冗長さは「値が存在しない可能性」をコンパイラが追跡していることの表れだ。JavaScript の `undefined` アクセスでアプリが静かに壊れることは、ここでは構造的にあり得ない。

## 4. 明示性の文化 — 「なぜ再レンダリングされた？」が存在しない世界

React を使ったことがある人なら、「なぜこのコンポーネントが再レンダリングされたのか」をデバッグした経験があるだろう。`React.memo` やら `useMemo` やら `useCallback` やらで依存関係を手動管理した経験があるだろう。React のリアクティビティモデルは暗黙的だ — 状態が変わると「どこまで再計算するか」をフレームワークが推測する。

Leptos は逆のアプローチを取る。依存関係は **すべて明示的** だ:

```rust
// Resource の依存は第一引数で宣言する
let profile = Resource::new(
    username,  // この Signal が変わったら再フェッチ
    |u| async move { get_user_profile(u).await },
);

// 表示の更新は Signal の .get() を呼んだ箇所だけ
class=move || {
    if is_liked.get() {   // ← ここで購読が発生
        "text-pink-600"
    } else {
        "hover:text-pink-600"
    }
}
```

`is_liked.get()` を呼んだクロージャだけが、`is_liked` の変更で再実行される。コンポーネント全体ではなく、**その DOM ノードだけ** が更新される。仮想 DOM の差分検出は不要だ。購読関係はコードから自明で、推測の余地がない。

この設計はまさに Rust の「明示性の文化」の延長にある。Rust は暗黙の型変換をしない。暗黙のメモリ割り当てをしない。暗黙のコピーをしない（`Copy` トレイトの明示実装が必要）。Leptos は暗黙の再レンダリングをしない。すべてのリアクティブな依存関係が、コード上で追跡可能だ。

## 5. エラーの扱い方 — 楽観と悲観の間

JavaScript のフロントエンド開発には「楽観主義」がある。API コールが失敗するかもしれない、JSON のパースが壊れるかもしれない — でもとりあえず `try/catch` で囲んでおけば、最悪でもアプリは動き続ける。エラーは例外として空中を飛び、キャッチされなければコンソールに落ちるだけだ。

Rust は徹底した「悲観主義」だ。すべての失敗可能な操作は `Result<T, E>` を返し、呼び出し側はそれを処理しなければコンパイルが通らない。Chirp のサーバー関数を見ると、その密度が分かる:

```rust
let user_id: Uuid = session
    .get("user_id")              // Result: セッションアクセスが失敗するかもしれない
    .await
    .map_err(|e| ServerFnError::new(format!("Session error: {e}")))?  // 変換して伝播
    .ok_or_else(|| ServerFnError::new("Not authenticated"))?;         // None を Error に変換

let post_id: Uuid = post_id
    .parse()                     // Result: UUID パースが失敗するかもしれない
    .map_err(|_| ServerFnError::new("Invalid post ID"))?;
```

4 行のコードに 3 つの `?` 演算子。セッションへのアクセスが失敗するかもしれない。ユーザーがログインしていないかもしれない。文字列が有効な UUID でないかもしれない。Rust はすべての可能性を列挙し、すべてに対処を要求する。

この「悲観主義」は冗長だが、`Suspense` と組み合わさるとフロントエンドとしての一貫した体験になる:

```rust
<Suspense fallback=move || {
    view! { <div class="p-4 text-gray-500">"読み込み中..."</div> }
}>
    {move || {
        timeline.get().map(|result| {
            match result {
                Ok(posts) => view! { <PostList posts=posts /> }.into_any(),
                Err(e) => view! {
                    <div class="p-4 text-red-500">{format!("エラー: {e}")}</div>
                }.into_any(),
            }
        })
    }}
</Suspense>
```

`Suspense` は「データがまだない状態」を表現し、`Result` は「データの取得に成功した/失敗した」を表現する。二つを組み合わせることで、ローディング → 成功表示 / エラー表示 という状態遷移が型レベルで保証される。「ローディング中にエラーメッセージが表示される」「データが来たのにスピナーが消えない」といったバグは、この設計では構造的に起こり得ない。

## 6. `#[server]` — 二つの世界をつなぐマクロの魔法

Leptos の `#[server]` マクロは、Rust の手続きマクロシステムを使った最も野心的な抽象化の一つだ。一つの関数定義から、サーバー側には実装本体を、クライアント側には HTTP リクエストを発行する RPC スタブを、それぞれ生成する。

```rust
#[server]
pub async fn create_post(content: String, reply_to_id: Option<String>)
    -> Result<PostWithMeta, ServerFnError>
{
    // ここはサーバーでのみ実行される
    use uuid::Uuid;  // サーバー専用の依存もここに書ける
    let pool = super::db::pool()?;
    // ...
}
```

クライアントから見ると `create_post("hello".into(), None).await` は普通の非同期関数呼び出しだ。しかし実際には HTTP POST が飛び、引数がシリアライズされ、サーバーで実行され、結果がデシリアライズされて返ってくる。**ネットワーク境界が関数シグネチャの中に隠蔽されている**。

これは TypeScript の tRPC やサーバーアクションと同じ発想だが、Rust のマクロシステムによって **コンパイル時に型チェックされる** 点が異なる。引数の型が `Serialize` を実装していなければコンパイルエラーになるし、戻り値の型がサーバーとクライアントで一致していなければそもそも一つのクレートからビルドされないから型の不整合が発生しない。

C/C++ ではマクロは単なるテキスト置換だった。Rust の手続きマクロは AST（抽象構文木）レベルの変換であり、型情報を保持したまま新しいコードを生成できる。この能力が `#[server]` を可能にしている。言語の進化が、フレームワークの表現力を直接的に拡張している好例だ。

## 7. `view!` マクロ — 二つの構文体系の結婚

`view!` マクロは、HTML の見た目を Rust の型安全性の中に持ち込む翻訳層だ。しかしこの翻訳は完全ではなく、両方の世界の「癖」が透けて見える:

```rust
view! {
    <A href=user_url.clone() attr:class="font-bold hover:underline truncate">
        {display_name}
    </A>
}
```

`<A>` は HTML の `<a>` ではなく、Leptos のコンポーネントだ。`href` は文字列属性ではなく Rust の式だ。`attr:class` の `attr:` プレフィックスは「これは HTML 属性であって props ではない」という指示だ。`{display_name}` の波括弧は JSX のそれに見えるが、実際は Rust の式評価だ。

一見 HTML に見えるものの、すべてが Rust のコンパイラを通過する。タグ名を間違えれば型エラーになり、存在しない props を渡せばコンパイルエラーになる。HTML の「壊れても何とかレンダリングする」寛容さは、ここでは意図的に排除されている。

これは自然言語における外来語の取り込みに似ている。日本語がカタカナで外来語を受け入れながら、日本語の文法規則に従わせるように、`view!` マクロは HTML の語彙を受け入れながら、Rust の型規則に従わせる。借用語は元の言語の響きを保ちつつ、借用先の文法で使われるのだ。

## 8. コンパイル時間というコスト — 安全性の代償

ここまで Rust の型安全性やコンパイル時保証の恩恵を語ってきたが、代償にも触れなければ公平ではない。正直に告白すると、Chirp の開発中に「Vite に戻りたい」と思った瞬間は一度や二度ではなかった。

JavaScript のフロントエンド開発では、ファイルを保存して数百ミリ秒でブラウザに反映される。Vite の HMR は文字通り「瞬き」の速度だ。一方、Rust + Leptos では `cargo leptos serve` がホットリロードを提供するものの、Rust のコンパイルは本質的に重い。CSS のクラス名を1つ変えただけで、SSR バイナリと WASM の両方がリビルドされる。型チェック、借用チェック、単相化（monomorphization）、LLVM バックエンドの最適化 — これらは安全性と引き換えのコストだ。

`cargo leptos serve` はインクリメンタルコンパイルとホットリロードでこの痛みを軽減するが、「型を一つ変えたらクレート全体がリビルド」という場面は避けられない。特に SSR バイナリと WASM の両方をビルドする Leptos では、変更のたびに 2 つのターゲットがコンパイルされる。

これは Rust が意図的に選んだトレードオフだ。Graydon Hoare が Mozilla Research で Rust を設計したとき、Firefox のメモリ安全性バグが動機にあった。「実行時に壊れるより、コンパイル時に止まれ」。このビジョンは正しいが、フロントエンド開発の「素早く試して素早く壊す」文化とは緊張関係にある。

Leptos がこの緊張にどう対処しているかは興味深い。`cargo leptos serve` のホットリロードは、Rust の型安全性を維持したまま開発サイクルを短縮する試みだ。完璧ではないが、「安全性を犠牲にして速度を得る」のではなく「安全性を保ったまま速度を改善する」という方向性は、Rust の設計哲学に忠実だ。

## 9. 楽観的更新 — 正確性と即時性の交渉をコードで読む

ここまでの議論を踏まえて、Chirp の `ActionBar` コンポーネントの実装を TypeScript（React）と並べて読んでみよう。同じ「いいねボタン」を両言語で書いたとき、言語の設計思想の違いがどう表出するかが見える。

**TypeScript + React の場合:**

```typescript
function ActionBar({ postId, likeCount, liked }: Props) {
  const [isLiked, setIsLiked] = useState(liked);
  const [count, setCount] = useState(likeCount);

  const onLike = async () => {
    const wasLiked = isLiked;
    setIsLiked(!wasLiked);                    // 楽観的更新
    setCount(c => wasLiked ? c - 1 : c + 1);
    try {
      await toggleLike(postId);               // サーバー呼び出し
    } catch {
      setIsLiked(wasLiked);                   // ロールバック
      setCount(c => wasLiked ? c + 1 : c - 1);
    }
  };

  return <button onClick={onLike}>...</button>;
}
```

シンプルだ。`async` クロージャの中で `await` し、`try/catch` でエラーを処理する。`postId` は何も考えずに参照できる。JavaScript ではすべての値がヒープ上にあり、ガベージコレクタが生存期間を管理するから、所有権という概念が存在しない。

**Rust + Leptos の場合:**

```rust
#[component]
pub fn ActionBar(
    post_id: String,     // String は Copy でない — ここから所有権の物語が始まる
    like_count: i32,
    liked: bool,
) -> impl IntoView {
    let (is_liked, set_liked) = signal(liked);          // Signal は Copy
    let (local_like_count, set_like_count) = signal(like_count);

    // post_id は String — Copy トレイトを実装していない。
    // この clone は「この文字列の所有権を、いいねクロージャに分配する」宣言だ。
    let post_id_for_like = post_id.clone();

    let on_like = move |_| {                             // move: 所有権をクロージャに移動
        let was_liked = is_liked.get_untracked();        // 購読せずに読む

        set_liked.set(!was_liked);                       // 楽観的更新
        set_like_count.update(|c| {
            if was_liked { *c -= 1 } else { *c += 1 }
        });

        // spawn_local: sync → async の橋
        let pid = post_id_for_like.clone();              // さらに clone — async ブロック用
        leptos::task::spawn_local(async move {
            if toggle_like(pid).await.is_err() {         // Result — 失敗は型で表現される
                set_liked.set(was_liked);                 // ロールバック
                set_like_count.update(|c| {
                    if was_liked { *c += 1 } else { *c -= 1 }
                });
            }
        });
    };
    // ...
}
```

同じ機能を実現するのに、Rust 版は TypeScript 版にはない 3 つの概念が登場する:

**1. `post_id.clone()` — 所有権の分配**

TypeScript では `postId` をいくつのクロージャからでも自由に参照できる。Rust ではクロージャが `move` で値の所有権を奪うため、同じ `String` を 2 つのクロージャ（いいね用・リチャープ用）で使うには、事前に所有権を分配（clone）する必要がある。

これは「コスト」に見えるが、同時に「ドキュメント」でもある。`post_id_for_like` と `post_id_for_rechirp` という変数名が、「この文字列はここで使われる」ことを明示している。JavaScript の暗黙的なクロージャキャプチャでは、どのクロージャがどの変数を参照しているか、コードを読まなければ分からない。

Signal は `Copy` だから clone 不要だが、`String` は `Copy` でない。この違いが Chirp のコード全体に通底する設計上の制約になる。Signal で管理できる値（数値、bool）は所有権を気にせず扱え、`String` や `Vec` は常に「誰が持つか」を考える必要がある。

**2. `spawn_local` — 二つの時間の流れの橋渡し**

React では `onClick={async () => { ... }}` と書ける。イベントハンドラが非同期関数であることを、フレームワークが暗黙的に受け入れる。

Rust の型システムはそれを許さない。`on:click` が期待するのは `Fn(MouseEvent)` — 同期関数だ。`async fn` は `Future` を返す型であり、`Fn(MouseEvent)` とは型が違う。同期の世界と非同期の世界は、Rust では **型レベルで区別されている**。

`spawn_local` は「この非同期タスクをブラウザのイベントループに投げる」ことで、二つの世界を接続する。これは Rust が非同期処理をランタイムではなく型システムで表現するという設計思想の帰結だ。JavaScript では `async/await` はシンタックスシュガーだが、Rust では型変換だ。

**3. `is_err()` vs `try/catch` — エラーの可視性**

TypeScript の `try/catch` はエラーを「例外」として扱う。例外はコードの通常のフローの外側を飛び、キャッチされなければ静かに消える。`catch` ブロックを書き忘れても、コンパイラは何も言わない。

Rust の `Result` はエラーを「値」として扱う。`toggle_like(pid).await` は `Result<bool, ServerFnError>` を返し、この値を処理しなければコンパイルが通らない。`.is_err()` と書いた瞬間、「失敗する可能性がある」ことがコードに刻まれ、ロールバックのロジックが必然的に要求される。

この違いは哲学的だ。例外ベースのエラーハンドリングは「ハッピーパスを書け、エラーは後で考えろ」と言う。Result ベースは「失敗を最初から型の一部として設計しろ」と言う。どちらが正しいかは場面による。しかし楽観的更新のような「失敗時にUIを巻き戻す」パターンでは、Result の方が自然にロールバックを導き出すことは確かだ。

## 10. `Effect` — 副作用に名前を付ける

投稿フォーム `PostComposer` では、送信成功後にテキストエリアをクリアする必要がある。React と Leptos でこれをどう書くか比べてみよう。

**React:**

```typescript
const [content, setContent] = useState('');
const result = useMutation(createPost);

useEffect(() => {
  if (result.isSuccess) setContent('');
}, [result.isSuccess]);     // ← 依存配列の手動指定
```

React の `useEffect` は依存配列（第二引数の `[result.isSuccess]`）を手動で指定する。ここに `result` を入れ忘れると stale closure バグになり、余計なものを入れると無限ループになる。React チームが `useEffect` の正しい使い方を何度もドキュメントで説明してきたのは、この設計の難しさの表れだ。

**Leptos:**

```rust
let create_post = ServerAction::<CreatePost>::new();
let (content, set_content) = signal(String::new());

let action_value = create_post.value();
Effect::new(move || {
    if let Some(Ok(_)) = action_value.get() {   // ← .get() で自動購読
        set_content.set(String::new());
    }
});
```

Leptos の `Effect` には依存配列がない。代わりに、`action_value.get()` を呼んだ瞬間に「この Effect は `action_value` に依存する」という関係が自動的に登録される。依存の指定漏れも、余計な依存の指定も、構造的に起こり得ない。

これは Leptos（そして SolidJS）の「きめ細かいリアクティビティ」の恩恵だ。依存関係の追跡をフレームワークに任せるのではなく（React）、開発者に手動で指定させるのでもなく（React の `useEffect`）、**コードの実行パスから自動的に推論する**。

この設計は Rust の「明示性の文化」と緊張関係にある。Rust は暗黙の動作を嫌う言語のはずだ。しかし Leptos は「依存関係の追跡」に限って暗黙の自動化を選んだ。なぜなら、ここでの暗黙性は **正確** だからだ。`.get()` を呼んだ Signal は必ず依存に含まれ、呼んでいない Signal は含まれない。React の依存配列のように「開発者が嘘をつける」余地がない。

Rust の設計哲学は「暗黙を排除する」ことではなく、「暗黙を正確にする」ことだと言える。所有権の推論、ライフタイムの省略記法（elision）、`?` 演算子によるエラー伝播 — いずれも暗黙の動作だが、その振る舞いは完全に予測可能だ。Leptos の自動依存追跡も同じ系譜にある。

## 11. 蓄積する状態 — `Resource` の限界と `RwSignal` への移行

`HomePage` のタイムラインに「もっと見る」ボタンを実装したとき、Leptos の `Resource` の設計思想の限界に突き当たった。

`Resource` は「一度の非同期取得」を表現するプリミティブだ:

```rust
// Resource: 「このデータを取ってきて」の宣言
let timeline = Resource::new(|| (), |_| async { get_home_timeline(None, None).await });
```

しかしページネーションは「前のデータを保持しつつ、新しいデータを追加する」パターンだ。`Resource` を再フェッチすると前のデータが消える。これは `Resource` が「現在の状態のスナップショット」を表現するように設計されているからだ。

ここで `RwSignal` が必要になる:

```rust
// RwSignal: 蓄積される状態
let posts = RwSignal::new(Vec::new());
let cursor = RwSignal::new(None::<String>);  // None = 初回、Some = 続きあり
let is_loading = RwSignal::new(false);
let has_more = RwSignal::new(true);
```

TypeScript なら `let posts: Post[] = []; let cursor: string | null = null;` で済む。Rust では 4 つの `RwSignal` が必要だ。なぜか？

- `Vec::new()` は空のベクタ。投稿が追加されるたびに `posts.update(|p| p.extend(new_posts))` で追記する
- `None::<String>` は「まだカーソルがない」状態。型注釈が必要なのは、Rust コンパイラが `None` だけでは `Option<何>` の `何` を推論できないから
- `RwSignal` は `ReadSignal + WriteSignal` を一つにまとめた型。コンポーネント内で読み書き両方必要な場合に使う

**「もっと見る」のハンドラ:**

```rust
let load_more = move |_| {
    if is_loading.get_untracked() || !has_more.get_untracked() {
        return;
    }
    is_loading.set(true);
    let current_cursor = cursor.get_untracked();  // Option<String> は Copy でない → clone が発生

    leptos::task::spawn_local(async move {
        match get_home_timeline(current_cursor, None).await {
            Ok(new_posts) => {
                if new_posts.len() < 20 { has_more.set(false); }
                if let Some(last) = new_posts.last() {
                    cursor.set(Some(last.id.to_string()));
                }
                posts.update(|p| p.extend(new_posts));  // 既存データに追記
            }
            Err(_) => { has_more.set(false); }
        }
        is_loading.set(false);
    });
};
```

ここに Rust の所有権モデルの影響が鮮明に表れている:

- `current_cursor` は `Option<String>`。`get_untracked()` は内部の値をクローンして返す。Signal が `Copy` でも、中身の `String` はクローンが必要だ
- `async move` ブロックは `current_cursor` の所有権を奪う。同期の世界から非同期の世界への所有権の移転だ
- `posts.update(|p| p.extend(new_posts))` — `extend` は `new_posts` の所有権を取り、各要素を `posts` の `Vec` に移動する。JavaScript の `posts.push(...newPosts)` と意味論的には同じだが、Rust では「new_posts の中身が posts に移動し、new_posts は空になる」ことが型レベルで保証される

React のページネーションでは `setPosts(prev => [...prev, ...newPosts])` と書く。配列のスプレッドは新しい配列を作り、古い配列はガベージコレクタが回収する。Rust では `extend` がインプレースで追記し、不要な中間配列は生まれない。これはパフォーマンスの違いでもあるが、それ以上に、二つの言語が「データの所有と移動」をどう考えているかの違いだ。

## 12. 言語の選択は思考の選択

ここまでの実装を通じて見えてきたのは、同じ「いいねボタン」「無限スクロール」「フォーム送信後のクリア」という機能が、言語によって全く異なるコードの「形」になるということだ。

日本語話者が「兄」と「弟」を区別し、英語話者が "brother" で括るように、言語が持つ語彙は話者が世界を切り取る粒度を変える。プログラミング言語も同じだ。言語が持つ概念装置が、開発者の問題の捉え方を規定する。

JavaScript で SNS を作るとき、開発者は「状態をどう共有するか」を考える。React の Context API、Redux、Zustand — 共有状態の管理が設計の中心になる。エラーは例外として `try/catch` で囲み、非同期は `async/await` で自然に書き、値はガベージコレクタが管理してくれる。

Rust で同じものを作るとき、開発者は「所有権をどう分配するか」を考える。`post_id.clone()` は所有権の分配。`spawn_local` は同期と非同期の型の橋渡し。`Result` はエラーを値として設計に組み込む。`Signal` の `Copy` は所有権問題の構造的解決。すべてのコードに、「この値は誰のものか」「この操作は失敗しうるか」「この参照はいつまで有効か」という問いへの回答が刻まれる。

TypeScript の型システムは「ヒントとして型を書く — 実行時には消える」という思想だ。gradual typing の世界では、型は開発者を助けるガイドレールであり、最悪の場合は `any` で逃げられる。Rust の型システムは「型が実行時の振る舞いを決定する — エスケープハッチはない」。`Copy` か `Clone` かで所有権の移転方法が変わり、`Send` か `!Send` かで並行処理の安全性が変わる。

この違いは「哲学」の違いだ。TypeScript は「人間は間違えるから、間違えても動くようにしよう」という思想。Rust は「人間は間違えるから、間違えられないようにしよう」という思想。どちらが「正しい」ということではない。それぞれの言語が、それぞれの歴史的文脈から生まれた概念装置で同じ問題に取り組んでいるのだ。

Chirp を Rust + Leptos で書くという経験は、JavaScript のフロントエンドでは意識すらしなかった問題 — この値の所有者は誰か？ この参照はいつまで有効か？ この型はクライアントに送って安全か？ この操作は同期か非同期か？ — を考えさせてくれた。そしてそれらの問いに答えることで、アプリケーションの構造はより堅牢になった。

言語が違えば、同じ目的地に至る道も違う。Rust で UI を書くとき、所有権の移動を追い、型の変換を設計し、`?` の連鎖でエラーパスを織り上げる — その過程で見える風景は、JavaScript で同じ道を歩いたときとは全く異なる。言語の歴史を紐解くことは、人間の思考の進化を追体験することだ。そしてその追体験は、コードを書くときに最も鮮烈な形で経験できる。

---

## 13. まとめ — Leptos の設計思想と実装から得た教訓

Chirp の実装を通じて見えた Leptos の特徴と、実践から得た教訓をまとめる。

### Leptos の強み

**フルスタック Rust**: フロントエンドとバックエンドが同じ言語・同じクレートで、データモデルを共有できる。型の不一致によるバグが構造的に起きない。`User` / `UserSummary` / `UserProfile` のようなモデルバリアントを定義すれば、型レベルで情報漏洩も防げる。

**きめ細かいリアクティビティ**: 仮想 DOM を使わず、Signal の変更が直接 DOM ノードの更新につながる。React のような再レンダリングの考慮が不要で、パフォーマンスがデフォルトで良い。

**サーバー関数による境界の抽象化**: `#[server]` 一つで RPC の生成、シリアライズ、HTTP 通信を隠蔽する。API の定義と実装が同じ場所にあるため、フロントとバックの乖離が起きにくい。関数内の `use` 文でサーバー依存を局所化でき、`#[cfg(feature = "ssr")]` の管理も最小限に抑えられる。

**プログレッシブエンハンスメント**: SSR → Hydration の流れにより、初回表示が速く、JavaScript が無効でもフォームが動作する。`ActionForm` + `leptos_axum::redirect` の組み合わせで、JS の有無を問わないフォーム処理が自然に実現できる。

**Rust エコシステムとの統合**: Axum の tower レイヤー、SQLx のコンパイル時クエリチェック、`thiserror` によるエラー型、`argon2` によるパスワードハッシュなど、Rust の既存エコシステムをそのまま活用できる。

### 実装から得た教訓

**条件付きコンパイルは計画的に**: SSR / Hydrate の feature flag は Leptos の基盤だが、`PostWithMetaRow` → `PostWithMeta` のような変換層が必要になる場面がある。モデルの設計段階で「DB の行構造」と「クライアントに送る構造」の差分を意識しておくと、後から手戻りが少ない。

**PostgreSQL の機能を活用する**: カウンターキャッシュのトリガー、部分インデックス、ENUM 型、pg_trgm — これらはアプリケーション層で実装するより PostgreSQL に任せた方がシンプルで信頼性が高い。特にトリガーによるカウンター更新は、アプリケーションコードの複雑さを大幅に減らしてくれた。

**カーソルベースページネーションは UUID v7 と好相性**: OFFSET ベースでは実現できないデータ整合性と一定のパフォーマンスが、UUID v7 + keyset pagination の組み合わせで自然に手に入る。SNS のように「末尾に追加され続ける」データ構造では特に有効だ。

**型でセキュリティを表現する**: `#[serde(skip_serializing)]` だけでなく、そもそもセンシティブなフィールドを含まない型（`UserSummary`）を定義する。「うっかりシリアライズ」が型システムにより不可能になる。

### 言語と思考

Chirp の実装を通じて最も印象に残ったのは、Rust で UI を書くという行為が、フロントエンド開発に対する認識を変えてくれたことだ。

JavaScript では当たり前すぎて意識しない概念 — ガベージコレクション、動的型付け、暗黙のクロージャキャプチャ — が、Rust では存在しないか、明示的な操作を要求される。`move ||` を書くたびに「この値の所有権は誰にあるのか」を考え、`.into_any()` を書くたびに「なぜ異なるビュー型を統一する必要があるのか」を考える。Signal が `Copy` であることの意味、`#[server]` マクロが生成するコードの構造、`Result` の連鎖が保証するエラーハンドリングの網羅性。

これらは Rust の「制約」ではなく「語彙」だ。新しい語彙を持つことで、これまで見えなかった問題の構造が見えるようになる。

——と書いて、立ち止まる。本当だろうか。Rust で UI を書くことが「新しい視点を得る」体験だったのは事実だ。しかしそれは、Rust でなければ得られない視点だったのか。それとも、普段と違うことをしたから新鮮に感じただけなのか。たぶん、両方だ。ただ、`move ||` を書くたびに「この値は誰のものか」を考えさせられたのは確かで、TypeScript に戻ったとき、クロージャが変数を暗黙にキャプチャすることの「気楽さと危うさ」を同時に感じたのも確かだ。

プログラミング言語も自然言語も、それぞれの誕生には独自の背景がある。設計者の哲学、時代の要請、技術の制約が構文や文法に刻まれている。その言語でコードを書くことは、設計者の思考を追体験することでもある。

## 参考資料

- [Leptos 公式サイト](https://leptos.dev/)
- [leptos-rs/leptos - GitHub](https://github.com/leptos-rs/leptos)
- [SolidJS](https://www.solidjs.com/) — Leptos が影響を受けたきめ細かいリアクティビティの先駆者
- [React ドキュメント: useEffect](https://react.dev/reference/react/useEffect)
- [Rust RFC: Copy trait](https://doc.rust-lang.org/std/marker/trait.Copy.html)
- [tRPC](https://trpc.io/) — TypeScript の型安全 RPC。`#[server]` との思想的な類似点
