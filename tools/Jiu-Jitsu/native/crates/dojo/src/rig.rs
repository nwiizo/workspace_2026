//! 解剖モデル → Avian 物理ジョイントへの変換と、articulated 人体の組み立て。
//!
//! 各体節は円柱 (Collider::capsule) の剛体。親体節の遠位端と子体節の近位端を
//! 解剖学的関節で連結する。関節の可動域リミットは `anatomy::PhysicsJoint` から来る。
//! これにより「腕十字 = 肘 revolute の伸展下端を超えさせる」を物理で表現できる。
//!
//! アンカー規約: 各体節は自身の中心を原点とし、円柱は Y 軸に沿う。
//!   近位端 (proximal, 上) = (0, +length/2, 0) / 遠位端 (distal, 下) = (0, -length/2, 0)。

use anatomy::{JointId, PhysicsJoint};
use avian3d::prelude::*;
use bevy::prelude::*;

const DEG: f32 = std::f32::consts::PI / 180.0;

/// 体節の寸法 (メートル)。
pub struct SegmentDims {
    pub radius: f32,
    pub length: f32,
}

impl SegmentDims {
    const fn new(radius: f32, length: f32) -> Self {
        Self { radius, length }
    }
    /// 中心原点ローカルでの近位端 (上)。
    fn proximal_local(&self) -> Vec3 {
        Vec3::new(0.0, self.length / 2.0, 0.0)
    }
    /// 中心原点ローカルでの遠位端 (下)。
    fn distal_local(&self) -> Vec3 {
        Vec3::new(0.0, -self.length / 2.0, 0.0)
    }
}

/// 円柱体節の剛体を、体節中心が `center` に来るよう spawn する。
fn spawn_segment(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    center: Vec3,
    dims: &SegmentDims,
    color: Color,
    dynamic: bool,
) -> Entity {
    let body = if dynamic {
        RigidBody::Dynamic
    } else {
        RigidBody::Static
    };
    commands
        .spawn((
            body,
            Collider::capsule(dims.radius, dims.length),
            // 自己衝突を無効化: 幅広の胴カプセルに四肢が押し出されるのを防ぐ。
            // (体節同士の接触は関節リミットで表現する。床接触も現段階では不要)
            CollisionLayers::new(LayerMask(0b1), LayerMask(0b0)),
            Mesh3d(meshes.add(Capsule3d::new(dims.radius, dims.length))),
            MeshMaterial3d(materials.add(StandardMaterial {
                base_color: color,
                perceptual_roughness: 0.8,
                ..default()
            })),
            Transform::from_translation(center),
        ))
        .id()
}

/// 解剖モデルの関節記述を Avian のジョイント component として `parent`↔`child` 間に spawn する。
/// `anchor_parent` / `anchor_child` は各体節ローカル (中心原点) の接続点。
fn connect_joint(
    commands: &mut Commands,
    id: JointId,
    parent: Entity,
    child: Entity,
    anchor_parent: Vec3,
    anchor_child: Vec3,
) {
    let spec = anatomy::joint_by_id(id);
    match spec.physics {
        PhysicsJoint::Revolute {
            hinge_axis,
            angle_limit_deg,
        } => {
            commands.spawn(
                RevoluteJoint::new(parent, child)
                    .with_hinge_axis(Vec3::from_array(hinge_axis))
                    .with_angle_limits(angle_limit_deg.0 * DEG, angle_limit_deg.1 * DEG)
                    .with_local_anchor1(anchor_parent)
                    .with_local_anchor2(anchor_child),
            );
        }
        PhysicsJoint::Spherical {
            swing_limit_deg,
            twist_limit_deg,
            ..
        } => {
            commands.spawn(
                SphericalJoint::new(parent, child)
                    .with_swing_limits(swing_limit_deg.0 * DEG, swing_limit_deg.1 * DEG)
                    .with_twist_limits(twist_limit_deg.0 * DEG, twist_limit_deg.1 * DEG)
                    .with_local_anchor1(anchor_parent)
                    .with_local_anchor2(anchor_child),
            );
        }
    }
}

/// 体格 (メートル)。おおよそ身長 1.7 相当。
struct BodyDims {
    trunk: SegmentDims,
    upper_arm: SegmentDims,
    forearm: SegmentDims,
    thigh: SegmentDims,
    shin: SegmentDims,
    head_radius: f32,
}

impl BodyDims {
    fn standing() -> Self {
        Self {
            trunk: SegmentDims::new(0.13, 0.55),
            upper_arm: SegmentDims::new(0.05, 0.27),
            forearm: SegmentDims::new(0.045, 0.25),
            thigh: SegmentDims::new(0.08, 0.42),
            shin: SegmentDims::new(0.06, 0.4),
            head_radius: 0.12,
        }
    }
}

/// 直立フルボディ (頭・胴・両腕・両脚) を spawn する。
/// 胴 (trunk) と頭は静的、四肢は動的で解剖学的関節で吊るす。
/// 重力で腕は体側へ、脚は下へ垂れ、関節リミットで可動域が効く様子を目視できる。
pub fn spawn_demo_human(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    skin: Color,
) {
    let d = BodyDims::standing();
    let hip_y = 0.9;
    let shoulder_y = hip_y + d.trunk.length; // 1.45
    let trunk_center = Vec3::new(0.0, hip_y + d.trunk.length / 2.0, 0.0);

    // 胴 (静的) — この 1 本に肩と股のアンカーを持たせる。
    let trunk = spawn_segment(
        commands,
        meshes,
        materials,
        trunk_center,
        &d.trunk,
        skin,
        false,
    );

    // 頭 (静的・見た目用の球)。首の高さに置く。
    let head_center = Vec3::new(0.0, shoulder_y + d.head_radius + 0.04, 0.0);
    commands.spawn((
        RigidBody::Static,
        Mesh3d(meshes.add(Sphere::new(d.head_radius))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: skin,
            perceptual_roughness: 0.8,
            ..default()
        })),
        Transform::from_translation(head_center),
    ));

    // 左右の腕・脚。sx = +1 が向かって右、-1 が左。
    for sx in [-1.0_f32, 1.0] {
        // --- 腕: 肩 (球) → 上腕 → 肘 (蝶番) → 前腕 ---
        let shoulder = Vec3::new(sx * 0.18, shoulder_y - 0.04, 0.0);
        let upper_center = shoulder + d.upper_arm.distal_local(); // 近位端を肩に合わせる
        let upper = spawn_segment(
            commands,
            meshes,
            materials,
            upper_center,
            &d.upper_arm,
            skin,
            true,
        );
        connect_joint(
            commands,
            JointId::Shoulder,
            trunk,
            upper,
            shoulder - trunk_center,      // 胴ローカルの肩位置
            d.upper_arm.proximal_local(), // 上腕の近位端
        );

        let elbow = upper_center + d.upper_arm.distal_local();
        let fore_center = elbow + d.forearm.distal_local();
        let fore = spawn_segment(
            commands,
            meshes,
            materials,
            fore_center,
            &d.forearm,
            skin,
            true,
        );
        connect_joint(
            commands,
            JointId::Elbow,
            upper,
            fore,
            d.upper_arm.distal_local(),
            d.forearm.proximal_local(),
        );

        // --- 脚: 股 (球) → 腿 → 膝 (蝶番) → 脛 ---
        let hip = Vec3::new(sx * 0.1, hip_y, 0.0);
        let thigh_center = hip + d.thigh.distal_local();
        let thigh = spawn_segment(
            commands,
            meshes,
            materials,
            thigh_center,
            &d.thigh,
            skin,
            true,
        );
        connect_joint(
            commands,
            JointId::Hip,
            trunk,
            thigh,
            hip - trunk_center,
            d.thigh.proximal_local(),
        );

        let knee = thigh_center + d.thigh.distal_local();
        let shin_center = knee + d.shin.distal_local();
        let shin = spawn_segment(
            commands,
            meshes,
            materials,
            shin_center,
            &d.shin,
            skin,
            true,
        );
        connect_joint(
            commands,
            JointId::Knee,
            thigh,
            shin,
            d.thigh.distal_local(),
            d.shin.proximal_local(),
        );
    }
}
