//! 関節の解剖モデル。ツール全体の核。
//!
//! この 1 つのモデルが 3 役を駆動する:
//!   1. 物理シミュレーションの関節拘束 (Avian の Revolute/Spherical joint とリミット)
//!   2. ポーズ・可動域の検証テスト
//!   3. 関節ラボの教育コンテンツ (「なぜその技でタップするのか」)
//!
//! Web 版 (v2, TypeScript) の解剖モデルを Rust へ移植し、物理エンジン向けに
//! `PhysicsJoint` マッピングを追加した。glam/Bevy に依存させず、純データとして保つ。

mod joints;

pub use joints::JOINTS;

/// 教育対象としての解剖学的関節。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum JointId {
    Neck,
    Shoulder,
    Elbow,
    Wrist,
    Hip,
    Knee,
    Ankle,
}

/// 滑膜関節の構造分類。物理ジョイントの種類を決める。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JointKind {
    /// 蝶番: 1 自由度 (肘・膝・足首)
    Hinge,
    /// 球: 3 自由度 (肩・股)
    BallSocket,
    /// 車軸: 回旋主体 (首の連鎖近似)
    Pivot,
    /// 顆状: 2 自由度 (手首)
    Condyloid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Axis {
    X,
    Y,
    Z,
}

impl Axis {
    pub const fn index(self) -> usize {
        match self {
            Axis::X => 0,
            Axis::Y => 1,
            Axis::Z => 2,
        }
    }
}

/// 1 軸ぶんの運動仕様。角度は度。
#[derive(Debug, Clone, Copy)]
pub struct AxisSpec {
    pub axis: Axis,
    /// 運動名 (負方向, 正方向)。例: ("伸展", "屈曲")
    pub motion: (&'static str, &'static str),
    /// 表示リグ・物理ジョイントで許す角度レンジ (度)。
    pub rig_range_deg: (f32, f32),
    /// 解剖学的な参考可動域 (度)。関節ラボの安全域表示に使う。
    pub anatomical_range_deg: (f32, f32),
}

/// この関節を攻めるサブミッション。
#[derive(Debug, Clone, Copy)]
pub struct SubmissionLink {
    pub name: &'static str,
    pub how: &'static str,
}

/// 解剖モデルを物理ジョイントへ変換するための記述。
///
/// dojo クレートはこれを見て Avian の `RevoluteJoint` / `SphericalJoint` を生成する。
/// 角度・軸は関節ローカル系。
#[derive(Debug, Clone, Copy)]
pub enum PhysicsJoint {
    /// 蝶番。1 軸まわりの回転のみ許し、角度リミットで骨性ストップを表現する。
    Revolute {
        /// ヒンジ軸 (関節ローカル系の単位ベクトル)
        hinge_axis: [f32; 3],
        /// 許容回転角 (度)
        angle_limit_deg: (f32, f32),
    },
    /// 球。twist 軸まわりのねじりと、それに直交する swing (コーン) を別々に制限する。
    Spherical {
        twist_axis: [f32; 3],
        /// swing 半角リミット (度)
        swing_limit_deg: (f32, f32),
        /// twist リミット (度)
        twist_limit_deg: (f32, f32),
    },
}

/// 1 つの解剖学的関節の完全な仕様。
#[derive(Debug, Clone, Copy)]
pub struct JointSpec {
    pub id: JointId,
    pub jp: &'static str,
    pub en: &'static str,
    pub kind: JointKind,
    pub kind_jp: &'static str,
    pub axes: &'static [AxisSpec],
    /// 物理エンジン向けのジョイント記述。
    pub physics: PhysicsJoint,
    /// 可動域を制限している構造 (骨・靭帯・関節包)。
    pub limited_by: &'static str,
    /// 可動域を超えると何が壊れるか。
    pub failure_mode: &'static str,
    /// この関節を攻めるサブミッション。
    pub submissions: &'static [SubmissionLink],
    /// 補足 (例: 絞めは関節でなく血流の技)。
    pub note: Option<&'static str>,
}

impl JointSpec {
    /// 指定軸の運動仕様を返す。
    pub fn axis(&self, axis: Axis) -> Option<&AxisSpec> {
        self.axes.iter().find(|a| a.axis == axis)
    }

    /// 与えられたオイラー角 (度, [x,y,z]) を rig レンジへ clamp する。
    pub fn clamp_euler_deg(&self, mut euler_deg: [f32; 3]) -> [f32; 3] {
        for spec in self.axes {
            let i = spec.axis.index();
            let (min, max) = spec.rig_range_deg;
            euler_deg[i] = euler_deg[i].clamp(min, max);
        }
        euler_deg
    }

    /// 与えられたオイラー角のうち rig レンジ外の軸を列挙する (検証用)。
    pub fn range_violations(&self, euler_deg: [f32; 3]) -> Vec<String> {
        let mut out = Vec::new();
        for spec in self.axes {
            let i = spec.axis.index();
            let v = euler_deg[i];
            let (min, max) = spec.rig_range_deg;
            if v < min - 1e-3 || v > max + 1e-3 {
                out.push(format!(
                    "{:?}.{:?}={:.1} outside {}..{}",
                    self.id, spec.axis, v, min, max
                ));
            }
        }
        out
    }
}

/// 全関節から ID で 1 件引く。
pub fn joint_by_id(id: JointId) -> &'static JointSpec {
    JOINTS
        .iter()
        .find(|j| j.id == id)
        .expect("JointId は JOINTS に必ず存在する")
}
