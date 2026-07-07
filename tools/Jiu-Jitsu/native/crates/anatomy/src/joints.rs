//! 関節データ本体。v2 (TypeScript) の joints.ts を移植し、各関節に物理ジョイント記述を追加。
//! rig_range は保守的な表示・物理リミット、anatomical_range は標準的なキネシオロジー参考値。

use crate::{Axis, AxisSpec, JointId, JointKind, JointSpec, PhysicsJoint, SubmissionLink};

use Axis::{X, Y, Z};

pub static JOINTS: &[JointSpec] = &[
    JointSpec {
        id: JointId::Neck,
        jp: "頸椎 (首)",
        en: "Cervical spine",
        kind: JointKind::Pivot,
        kind_jp: "車軸関節 + 椎間関節の連鎖",
        axes: &[
            AxisSpec { axis: X, motion: ("伸展 (上を向く)", "屈曲 (顎を引く)"), rig_range_deg: (-35.0, 35.0), anatomical_range_deg: (-60.0, 50.0) },
            AxisSpec { axis: Y, motion: ("左回旋", "右回旋"), rig_range_deg: (-30.0, 30.0), anatomical_range_deg: (-80.0, 80.0) },
            AxisSpec { axis: Z, motion: ("左側屈", "右側屈"), rig_range_deg: (-30.0, 30.0), anatomical_range_deg: (-45.0, 45.0) },
        ],
        // 首は多軸だが、簡易人体では swing/twist を持つ球で近似する。
        physics: PhysicsJoint::Spherical { twist_axis: [0.0, 1.0, 0.0], swing_limit_deg: (-35.0, 35.0), twist_limit_deg: (-30.0, 30.0) },
        limited_by: "椎骨の形状、項靭帯・翼状靭帯、椎間板",
        failure_mode: "過屈曲・過伸展で靭帯損傷や椎間板・神経の圧迫。ただし絞め技の主対象は関節ではなく頸動脈の血流",
        submissions: &[
            SubmissionLink { name: "裸絞め / mata-leão", how: "関節技ではなく血流の技。前腕で両側の頸動脈を圧迫し、数秒で脳血流を落とす。顎を引いて腕の挿入を防ぐのが第一防御" },
            SubmissionLink { name: "三角絞め / triângulo", how: "自分の脚と相手自身の肩で首の両側を挟む。片腕が中に残る構造が絞めの条件" },
        ],
        note: Some("「首を守る」は柔術防御の最優先。絞めは関節破壊より速く意識を落とすため、タップは早めに。"),
    },
    JointSpec {
        id: JointId::Shoulder,
        jp: "肩関節 (肩甲上腕関節)",
        en: "Glenohumeral joint",
        kind: JointKind::BallSocket,
        kind_jp: "球関節 — 人体で最も可動域が広く、最も脱臼しやすい",
        axes: &[
            AxisSpec { axis: X, motion: ("屈曲 (腕を前へ上げる)", "伸展 (腕を後ろへ)"), rig_range_deg: (-125.0, 125.0), anatomical_range_deg: (-180.0, 60.0) },
            AxisSpec { axis: Y, motion: ("外旋", "内旋"), rig_range_deg: (-70.0, 70.0), anatomical_range_deg: (-90.0, 70.0) },
            AxisSpec { axis: Z, motion: ("内転 (体側へ)", "外転 (横へ上げる)"), rig_range_deg: (-80.0, 80.0), anatomical_range_deg: (-45.0, 180.0) },
        ],
        physics: PhysicsJoint::Spherical { twist_axis: [0.0, -1.0, 0.0], swing_limit_deg: (-125.0, 125.0), twist_limit_deg: (-70.0, 70.0) },
        limited_by: "関節唇、関節包、回旋筋腱板 (ローテーターカフ)",
        failure_mode: "外旋・伸展の限界を超えると関節包前面と腱板が損傷し、亜脱臼・脱臼に至る",
        submissions: &[
            SubmissionLink { name: "キムラ / 腕緘 (うでがらみ)", how: "肘を 90° に固定したまま前腕をテコに肩を強制内旋→背面へ回す。サイド下で腕を相手の下へ差すと入口を渡す" },
            SubmissionLink { name: "アメリカーナ", how: "キムラの逆方向。腕を頭側へ倒して肩を外旋・伸展させる" },
        ],
        note: None,
    },
    JointSpec {
        id: JointId::Elbow,
        jp: "肘関節 (腕尺関節)",
        en: "Elbow (humeroulnar joint)",
        kind: JointKind::Hinge,
        kind_jp: "蝶番 (ヒンジ) 関節 — 曲げ伸ばしの 1 自由度が主",
        axes: &[
            AxisSpec { axis: X, motion: ("伸展 (伸ばす)", "屈曲 (曲げる)"), rig_range_deg: (-120.0, 120.0), anatomical_range_deg: (0.0, 145.0) },
            AxisSpec { axis: Y, motion: ("回外", "回内"), rig_range_deg: (-55.0, 55.0), anatomical_range_deg: (-85.0, 80.0) },
            AxisSpec { axis: Z, motion: ("内反", "外反"), rig_range_deg: (-35.0, 35.0), anatomical_range_deg: (-5.0, 5.0) },
        ],
        // 蝶番。伸展 0° の骨性ストップ = angle_limit の下端。腕十字はこの下端を超えさせる技。
        physics: PhysicsJoint::Revolute { hinge_axis: [1.0, 0.0, 0.0], angle_limit_deg: (0.0, 145.0) },
        limited_by: "肘頭が肘頭窩に骨性に当たって伸展 0° で止まる。内外反は側副靭帯が抑える",
        failure_mode: "伸展 0° を超える過伸展で内側側副靭帯・関節包が損傷し、肘頭の骨性衝突に至る。競技傷害の最多部位 (整形外科的傷害の 38.9% が肘、その大半が腕十字)",
        submissions: &[
            SubmissionLink { name: "腕十字 / 十字固め / juji-gatame", how: "腰を支点に相手の腕全体をテコで伸ばし、伸展 0° の骨性ロックを越えさせる。「肘を体から離さない」が防御の核" },
        ],
        note: Some("ヒンジ関節は自由度が少ないぶん、限界方向への力に構造的な逃げ場がない。"),
    },
    JointSpec {
        id: JointId::Wrist,
        jp: "手関節 (手首)",
        en: "Wrist (radiocarpal joint)",
        kind: JointKind::Condyloid,
        kind_jp: "顆状関節 — 2 自由度 (掌背屈・橈尺屈)",
        axes: &[
            AxisSpec { axis: X, motion: ("背屈 (甲側へ)", "掌屈 (手のひら側へ)"), rig_range_deg: (-55.0, 55.0), anatomical_range_deg: (-70.0, 80.0) },
            AxisSpec { axis: Y, motion: ("回外方向", "回内方向"), rig_range_deg: (-45.0, 45.0), anatomical_range_deg: (-10.0, 10.0) },
            AxisSpec { axis: Z, motion: ("橈屈 (親指側へ)", "尺屈 (小指側へ)"), rig_range_deg: (-45.0, 45.0), anatomical_range_deg: (-20.0, 30.0) },
        ],
        physics: PhysicsJoint::Spherical { twist_axis: [0.0, 1.0, 0.0], swing_limit_deg: (-55.0, 55.0), twist_limit_deg: (-10.0, 10.0) },
        limited_by: "手根骨の配列と掌側・背側の靭帯群",
        failure_mode: "過度の掌背屈で靭帯損傷。リストロックは小さい力で速く極まる",
        submissions: &[
            SubmissionLink { name: "リストロック", how: "掌屈または背屈方向へ手首を折り込む。ノーギのハンドファイトで露出しやすい" },
        ],
        note: Some("回内・回外は本来、前腕の橈尺関節の運動。簡易リグでは手首の y 軸にまとめて表現している。手関節自体の自由度は掌背屈・橈尺屈の 2 つ。"),
    },
    JointSpec {
        id: JointId::Hip,
        jp: "股関節",
        en: "Hip joint",
        kind: JointKind::BallSocket,
        kind_jp: "球関節 — 肩より深い臼蓋で安定性重視",
        axes: &[
            AxisSpec { axis: X, motion: ("伸展 (脚を後ろへ)", "屈曲 (膝を胸へ)"), rig_range_deg: (-130.0, 145.0), anatomical_range_deg: (-30.0, 120.0) },
            AxisSpec { axis: Y, motion: ("外旋", "内旋"), rig_range_deg: (-55.0, 55.0), anatomical_range_deg: (-45.0, 45.0) },
            AxisSpec { axis: Z, motion: ("内転", "外転"), rig_range_deg: (-80.0, 80.0), anatomical_range_deg: (-30.0, 45.0) },
        ],
        physics: PhysicsJoint::Spherical { twist_axis: [0.0, -1.0, 0.0], swing_limit_deg: (-130.0, 145.0), twist_limit_deg: (-55.0, 55.0) },
        limited_by: "深い臼蓋 (寛骨臼)、腸骨大腿靭帯 — 人体最強の靭帯",
        failure_mode: "単体で極められることは少ないが、腰の角度が消えると上の関節が全て守れなくなる",
        submissions: &[
            SubmissionLink { name: "(直接のサブミッションは少ない)", how: "股関節は「攻められる関節」ではなく「技の動力源」。ブリッジ・海老・角度切りはすべて股関節の可動域で作る" },
        ],
        note: Some("ガードワークとは股関節の可動域の使い方のこと。柔術で最初に柔らかくすべき関節。"),
    },
    JointSpec {
        id: JointId::Knee,
        jp: "膝関節",
        en: "Knee joint",
        kind: JointKind::Hinge,
        kind_jp: "蝶番 (ヒンジ) 関節 — 屈曲時のみわずかに回旋",
        axes: &[
            AxisSpec { axis: X, motion: ("伸展 (伸ばす)", "屈曲 (曲げる)"), rig_range_deg: (0.0, 150.0), anatomical_range_deg: (0.0, 135.0) },
            AxisSpec { axis: Y, motion: ("内旋", "外旋"), rig_range_deg: (-30.0, 30.0), anatomical_range_deg: (-10.0, 30.0) },
            AxisSpec { axis: Z, motion: ("内反", "外反"), rig_range_deg: (-25.0, 25.0), anatomical_range_deg: (-5.0, 5.0) },
        ],
        physics: PhysicsJoint::Revolute { hinge_axis: [1.0, 0.0, 0.0], angle_limit_deg: (0.0, 135.0) },
        limited_by: "前後十字靭帯 (ACL/PCL)、内外側側副靭帯 (MCL/LCL)、半月板",
        failure_mode: "過伸展や捻りで靭帯・半月板が損傷する。痛みより先に構造が壊れることがあり「痛くなってからタップ」では遅い",
        submissions: &[
            SubmissionLink { name: "膝十字 / ヒールフック (本道場では扱わない)", how: "下半身関節技は位置支配がなくても極まりやすく (Spanias 2022)、損傷リスクが高いため上級者の領域。本道場は上半身の攻防に限定" },
        ],
        note: Some("膝は x=0..150 のみ許可 — 逆関節に見える負方向屈曲を構造で禁止している。"),
    },
    JointSpec {
        id: JointId::Ankle,
        jp: "足関節 (足首)",
        en: "Ankle joint",
        kind: JointKind::Hinge,
        kind_jp: "蝶番 (ヒンジ) 関節 (距腿関節)",
        axes: &[
            AxisSpec { axis: X, motion: ("背屈 (つま先を上げる)", "底屈 (つま先を伸ばす)"), rig_range_deg: (-55.0, 55.0), anatomical_range_deg: (-20.0, 50.0) },
            AxisSpec { axis: Y, motion: ("内転", "外転"), rig_range_deg: (-55.0, 55.0), anatomical_range_deg: (-35.0, 25.0) },
            AxisSpec { axis: Z, motion: ("内がえし", "外がえし"), rig_range_deg: (-55.0, 55.0), anatomical_range_deg: (-35.0, 20.0) },
        ],
        physics: PhysicsJoint::Revolute { hinge_axis: [1.0, 0.0, 0.0], angle_limit_deg: (-20.0, 50.0) },
        limited_by: "距骨のほぞ穴構造 (脛骨・腓骨に挟まれる)、三角靭帯・外側靭帯群",
        failure_mode: "過底屈 + 内がえしでアキレス腱と靭帯にストレス (アンクルロックの原理)",
        submissions: &[
            SubmissionLink { name: "アキレス固め (本道場では扱わない)", how: "底屈方向へ足首を伸ばし込みつつアキレス腱を前腕で圧迫する入門的足関節技" },
        ],
        note: Some("内がえし・外がえしは距骨下関節の運動。簡易リグでは距腿関節の z 軸にまとめて表現している。"),
    },
];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{joint_by_id, JointKind, PhysicsJoint};

    #[test]
    fn all_joints_present_and_unique() {
        assert_eq!(JOINTS.len(), 7);
        let mut ids: Vec<_> = JOINTS.iter().map(|j| j.id).collect();
        ids.sort_by_key(|id| format!("{id:?}"));
        ids.dedup();
        assert_eq!(ids.len(), 7, "JointId が重複している");
    }

    #[test]
    fn hinge_joints_map_to_revolute_ball_to_spherical() {
        for j in JOINTS {
            match (j.kind, j.physics) {
                (JointKind::Hinge, PhysicsJoint::Revolute { .. }) => {}
                (JointKind::BallSocket, PhysicsJoint::Spherical { .. }) => {}
                (JointKind::Pivot | JointKind::Condyloid, PhysicsJoint::Spherical { .. }) => {}
                other => panic!("{:?} の kind/physics 対応が不正: {other:?}", j.id),
            }
        }
    }

    #[test]
    fn elbow_extension_stop_matches_anatomy() {
        // 腕十字の教育的核: 肘の伸展下端 0° が骨性ストップであること。
        let elbow = joint_by_id(JointId::Elbow);
        if let PhysicsJoint::Revolute {
            angle_limit_deg, ..
        } = elbow.physics
        {
            assert_eq!(angle_limit_deg.0, 0.0, "肘の伸展ストップは 0°");
        } else {
            panic!("肘は Revolute のはず");
        }
    }

    #[test]
    fn physics_axes_are_unit_length() {
        for j in JOINTS {
            let axis = match j.physics {
                PhysicsJoint::Revolute { hinge_axis, .. } => hinge_axis,
                PhysicsJoint::Spherical { twist_axis, .. } => twist_axis,
            };
            let len = (axis[0] * axis[0] + axis[1] * axis[1] + axis[2] * axis[2]).sqrt();
            assert!(
                (len - 1.0).abs() < 1e-4,
                "{:?} の軸が単位ベクトルでない",
                j.id
            );
        }
    }

    #[test]
    fn clamp_and_violations_agree() {
        for j in JOINTS {
            // 各軸の rig 上限 +50° を与えると、clamp 後は範囲内・違反ゼロ。
            let over = [200.0, 200.0, 200.0];
            let clamped = j.clamp_euler_deg(over);
            assert!(
                j.range_violations(clamped).is_empty(),
                "{:?}: clamp 後に違反",
                j.id
            );
            // clamp 前は少なくとも 1 軸が違反する。
            assert!(
                !j.range_violations(over).is_empty(),
                "{:?}: 極端値で違反ゼロはおかしい",
                j.id
            );
        }
    }

    #[test]
    fn hinge_physics_limit_matches_anatomical_x_range() {
        // 蝶番の物理リミットは主軸 (X) の解剖学的可動域と一致すること。
        // (rig_range は表示 clamp 用で目的が異なるため一致しない — それは正しい設計)
        for j in JOINTS {
            if let PhysicsJoint::Revolute {
                angle_limit_deg, ..
            } = j.physics
            {
                let x = j.axis(Axis::X).expect("hinge に X 軸");
                assert!(
                    (angle_limit_deg.0 - x.anatomical_range_deg.0).abs() < 1e-3
                        && (angle_limit_deg.1 - x.anatomical_range_deg.1).abs() < 1e-3,
                    "{:?}: 物理リミット {:?} が解剖 X レンジ {:?} と不一致",
                    j.id,
                    angle_limit_deg,
                    x.anatomical_range_deg
                );
            }
        }
    }
}
