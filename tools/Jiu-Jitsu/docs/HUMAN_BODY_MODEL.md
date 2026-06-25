# Human Body Model

このゲームの人体モデルは、実技の代替になる解剖シミュレーションではなく、BJJ の局面を読める教育用リグとして扱う。人体らしさは、次の順序で上げる。

## 1. 現MVPの方針

現状は Three.js のプリミティブで作った簡易ヒューマノイドに、`poses.js` の関節角を当てている。人体らしさは物理ではなく、次の制約と軽量 kinematics で担保する。

- 関節ごとの可動域を `scripts/validate-data.mjs` で検証する。
- `fighter.js` は `anatomy.js` の同じ可動域でレンダリング時にも関節角を clamp する。
- `fighter.js` は床付近のポーズに対して floor probe を使い、身体がマットを明確に突き抜ける/浮き続ける場合に root 高さを小さく補正する。
- `positionCatalog.js` で許可された BJJ role ペアだけを scenario に出す。
- `poseSpecs.js` で支持基底、接触点、力の向きを定義する。
- `_audit.html` の重心投影、支持エリア、荷重線、接触点ラベルでスクショ確認する。

## 2. 関節の規約

### 膝

膝は簡易リグではヒンジ関節として扱う。BJJ では脚全体が大きく動くが、その大部分は股関節の屈曲・外転・外旋で表現し、膝そのものに逆方向の屈曲や大きなねじりを持たせない。

実装規約:

- `shinL` / `shinR` の `x` は屈曲方向の `0..150` 度だけを許す。
- `shinL` / `shinR` の `y` / `z` は見た目調整用の小さな値に限定する。
- 脚を横へ開く、相手の腰を囲む、三角で角度を切る表現は、原則として `thighL` / `thighR` と root 回転で作る。
- 負の `shin*.x` は逆関節に見えるため禁止する。

### 肘

肘もヒンジ寄りに扱う。腕十字などで「伸ばされている」表現は前腕を極端に反らすのではなく、上腕の位置、相手の脚/腰の位置、守る側の姿勢崩れで読ませる。

### 首

首は flexion / extension / rotation / lateral bending を持つが、寝技の圧や絞めの表現で過大に曲げない。首への危険な力は、角度を大きくするより、相手の胸・腕・脚の位置と read cue で示す。

### 肩・股関節

肩と股関節は多方向に動ける関節として扱う。ただし、上腕・大腿だけで極端な見た目を作らず、root 位置、胴体向き、支持点、相手との接触と組み合わせて局面を表す。

## 3. 技術選定

### Phase 1: 角度制約付きプリミティブ

現在の方式。実装が軽く、局面の読みやすさを早く改善できる。`anatomy.js` で共有する関節可動域と、`fighter.js` の floor probe による root 補正を持つ。一方で、手足を任意の目標位置へ自然に置く本格 IK ではないため、細かい接触や関節の向きはまだ破綻しうる。

継続条件:

- `invalidJointLimits === 0`
- `invalidRuntimeAnatomy === 0`
- `invalidPosePairGeometry === 0`
- `invalidPosePairOrientation === 0`
- `invalidPosePairSupport === 0`
- `_audit` スクショで逆関節・浮き・向き違いがない

### Phase 2: IK

手、足、膝、頭の目標位置を決め、肘/膝の曲がる向きを制約して中間関節を解く。これにより「足首を相手の腰へ置く」「膝をマットへ置く」「手でフレームを作る」のような接触が、角度の手打ちより安定する。現時点では floor probe による軽量 grounding のみ実装済みで、2-bone IK / CCD IK は次段階とする。

Three.js には `CCDIKSolver` があり、`SkinnedMesh` 向けに CCD の IK を解く。より細かいヒンジ/ボール制約が必要なら FABRIK/CCD の IK ライブラリを検討する。

### Phase 3: glTF のスキン付き人体リグ

プリミティブ人形ではなく、Blender などで作ったスキン付き glTF を `GLTFLoader` で読み、標準的な骨名へポーズをリターゲットする。Three.js の `SkinnedMesh` は Skeleton と skin weights により、骨の動きにメッシュを追従させる。

採用目安:

- 膝・肘・肩・股関節の見た目破綻が角度調整では繰り返し発生する。
- 三角、腕十字、バックフック、クローズドガードの接触を教育上はっきり見せたい。
- gi/no-gi の衣服表現を人体メッシュと一体で扱いたい。

### Phase 4: 物理拘束

接触や押し返しを本当に扱うなら、剛体とジョイント拘束が必要になる。肘・膝はヒンジ、肩・股関節・首は cone/ball 系の制約で近似する。ただし教育ゲームとしてはコストが大きいため、IK と pose spec で不足する場合に限る。

## 4. 参考ソース

- NCBI Bookshelf / StatPearls: Knee Anatomy. 膝は主に flexion / extension のヒンジとして扱われ、その他の動きは限定的。
  https://www.ncbi.nlm.nih.gov/books/NBK500017/
- NCBI Bookshelf / StatPearls: Hinge Joints. 肘・膝・足関節などは主に一平面の動きを許す関節として説明される。
  https://www.ncbi.nlm.nih.gov/books/NBK518967/
- NCBI Bookshelf / StatPearls: Neck Movements. 頸椎は屈曲、伸展、回旋、側屈を持つが、部位ごとに主運動が異なる。
  https://www.ncbi.nlm.nih.gov/books/NBK557555/
- CDC Stacks: Normal Joint Range of Motion dataset. 関節可動域は年齢・性別で変わるため、ゲーム内の角度は代表値ではなく安全側の表示制約として扱う。
  https://stacks.cdc.gov/view/cdc/153156/
- Three.js docs: SkinnedMesh.
  https://threejs.org/docs/#api/en/objects/SkinnedMesh
- Three.js docs: CCDIKSolver.
  https://threejs.org/docs/#examples/en/animations/CCDIKSolver
