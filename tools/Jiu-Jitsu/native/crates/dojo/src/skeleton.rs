//! キネマティック FK 骨格。表示専用 (物理なし)。
//!
//! 物理ラグドール (rig.rs) は関節ラボで可動域の限界を体感する用途。
//! 綺麗なグラップリング姿勢の表示にはこちらを使う: 親子 Transform 階層を関節角で駆動する
//! (v2 の fighter.ts と同じ考え方)。ポーズは関節ごとのオイラー角 (度) で与える。

use bevy::prelude::*;
use std::collections::HashMap;

const DEG: f32 = std::f32::consts::PI / 180.0;

/// 体格 (v2 と同じ寸法)。
mod dims {
    pub const SPINE: f32 = 0.2;
    pub const CHEST: f32 = 0.26;
    pub const NECK: f32 = 0.09;
    pub const HEAD_R: f32 = 0.125;
    pub const SHOULDER_HALF: f32 = 0.185;
    pub const UPPER_ARM: f32 = 0.27;
    pub const FOREARM: f32 = 0.25;
    pub const HAND: f32 = 0.1;
    pub const HIP_HALF: f32 = 0.11;
    pub const THIGH: f32 = 0.42;
    pub const SHIN: f32 = 0.4;
    pub const FOOT: f32 = 0.22;
    pub const LIMB_R: f32 = 0.066;
    pub const TORSO_R: f32 = 0.135;
}

/// 骨格トポロジー: (関節名, 親, 親原点からのローカル位置)。
const SKELETON: &[(&str, Option<&str>, [f32; 3])] = &[
    ("hips", None, [0.0, 0.0, 0.0]),
    ("spine", Some("hips"), [0.0, 0.0, 0.0]),
    ("chest", Some("spine"), [0.0, dims::SPINE, 0.0]),
    ("neck", Some("chest"), [0.0, dims::CHEST, 0.0]),
    ("head", Some("neck"), [0.0, dims::NECK, 0.0]),
    ("upperArmL", Some("chest"), [dims::SHOULDER_HALF, dims::CHEST - 0.03, 0.0]),
    ("forearmL", Some("upperArmL"), [0.0, -dims::UPPER_ARM, 0.0]),
    ("handL", Some("forearmL"), [0.0, -dims::FOREARM, 0.0]),
    ("upperArmR", Some("chest"), [-dims::SHOULDER_HALF, dims::CHEST - 0.03, 0.0]),
    ("forearmR", Some("upperArmR"), [0.0, -dims::UPPER_ARM, 0.0]),
    ("handR", Some("forearmR"), [0.0, -dims::FOREARM, 0.0]),
    ("thighL", Some("hips"), [dims::HIP_HALF, 0.0, 0.0]),
    ("shinL", Some("thighL"), [0.0, -dims::THIGH, 0.0]),
    ("footL", Some("shinL"), [0.0, -dims::SHIN, 0.0]),
    ("thighR", Some("hips"), [-dims::HIP_HALF, 0.0, 0.0]),
    ("shinR", Some("thighR"), [0.0, -dims::THIGH, 0.0]),
    ("footR", Some("shinR"), [0.0, -dims::SHIN, 0.0]),
];

/// 見た目プリミティブの形。
enum Shape {
    /// カプセル (半径, 長さ)。長さ方向はローカル Y。
    Capsule(f32, f32),
    /// 球 (半径)。scale で楕円体化して筋腹を表す。
    Sphere(f32),
}

/// 骨に重ねる 1 パーツ (骨の芯 + 筋腹の重ね合わせで筋肉質に見せる)。
struct Part {
    shape: Shape,
    offset: Vec3,
    scale: Vec3,
    skin: bool,
}

impl Part {
    fn limb(r: f32, len: f32) -> Self {
        Part { shape: Shape::Capsule(r, len), offset: Vec3::new(0.0, -len / 2.0, 0.0), scale: Vec3::ONE, skin: false }
    }
    fn up(r: f32, len: f32) -> Self {
        Part { shape: Shape::Capsule(r, len), offset: Vec3::new(0.0, len / 2.0, 0.0), scale: Vec3::ONE, skin: false }
    }
    /// 筋腹などの楕円体。
    fn belly(r: f32, offset: Vec3, scale: Vec3) -> Self {
        Part { shape: Shape::Sphere(r), offset, scale, skin: false }
    }
    fn skinned(mut self) -> Self {
        self.skin = true;
        self
    }
}

/// 各関節の骨を、芯カプセル + 筋腹楕円体の重ね合わせで表現する。
/// muscle bellies は骨ローカル系 (骨は原点から -Y へ伸び、前面 = +Z)。
fn parts_for(name: &str) -> Vec<Part> {
    use dims::*;
    let r = LIMB_R;
    match name {
        "hips" => vec![
            Part::belly(TORSO_R * 0.92, Vec3::new(0.0, 0.0, 0.0), Vec3::new(1.15, 0.85, 0.9)),
            // 臀筋
            Part::belly(TORSO_R * 0.5, Vec3::new(0.07, -0.03, -0.06), Vec3::new(1.0, 0.9, 1.1)),
            Part::belly(TORSO_R * 0.5, Vec3::new(-0.07, -0.03, -0.06), Vec3::new(1.0, 0.9, 1.1)),
        ],
        "spine" => vec![
            // 腹部 (前後に薄い) + 腹直筋の膨らみ
            Part { shape: Shape::Capsule(TORSO_R * 0.82, SPINE), offset: Vec3::new(0.0, SPINE / 2.0, 0.0), scale: Vec3::new(1.0, 1.0, 0.82), skin: false },
            Part::belly(TORSO_R * 0.5, Vec3::new(0.0, SPINE * 0.5, 0.09), Vec3::new(1.1, 1.3, 0.5)),
        ],
        "chest" => vec![
            // 胸郭 (前後に薄い)
            Part { shape: Shape::Capsule(TORSO_R * 1.02, CHEST), offset: Vec3::new(0.0, CHEST / 2.0, 0.0), scale: Vec3::new(1.05, 1.0, 0.8), skin: false },
            // 大胸筋 左右
            Part::belly(TORSO_R * 0.55, Vec3::new(0.06, CHEST * 0.62, 0.09), Vec3::new(1.0, 0.7, 0.6)),
            Part::belly(TORSO_R * 0.55, Vec3::new(-0.06, CHEST * 0.62, 0.09), Vec3::new(1.0, 0.7, 0.6)),
            // 僧帽筋
            Part::belly(TORSO_R * 0.6, Vec3::new(0.0, CHEST * 0.95, -0.02), Vec3::new(1.6, 0.5, 0.8)),
        ],
        "neck" => vec![Part::up(0.052, NECK).skinned()],
        "head" => vec![
            Part { shape: Shape::Sphere(HEAD_R), offset: Vec3::new(0.0, HEAD_R * 0.82, 0.0), scale: Vec3::new(0.9, 1.05, 0.95), skin: true },
            // 顎
            Part { shape: Shape::Sphere(HEAD_R * 0.5), offset: Vec3::new(0.0, HEAD_R * 0.42, HEAD_R * 0.42), scale: Vec3::ONE, skin: true },
        ],
        "upperArmL" | "upperArmR" => vec![
            Part::limb(r * 0.8, UPPER_ARM),
            // 三角筋 (肩の丸み)
            Part::belly(r * 1.3, Vec3::new(0.0, -0.02, 0.0), Vec3::new(1.05, 1.0, 1.0)),
            // 上腕二頭筋 (前)
            Part::belly(r * 0.95, Vec3::new(0.0, -UPPER_ARM * 0.35, 0.03), Vec3::new(0.9, 1.5, 0.95)),
            // 上腕三頭筋 (後)
            Part::belly(r * 0.9, Vec3::new(0.0, -UPPER_ARM * 0.38, -0.03), Vec3::new(0.85, 1.5, 0.85)),
        ],
        "forearmL" | "forearmR" => vec![
            Part::limb(r * 0.66, FOREARM),
            // 前腕屈筋群 (肘寄りが太く手首へ細く)
            Part::belly(r * 0.85, Vec3::new(0.0, -FOREARM * 0.3, 0.02), Vec3::new(0.95, 1.4, 0.95)),
            Part::belly(r * 0.5, Vec3::new(0.0, -FOREARM * 0.95, 0.0), Vec3::ONE),
        ],
        "handL" | "handR" => vec![Part { shape: Shape::Sphere(r * 0.85), offset: Vec3::new(0.0, -HAND * 0.5, 0.0), scale: Vec3::new(1.0, 1.15, 0.6), skin: true }],
        "thighL" | "thighR" => vec![
            Part::limb(r * 1.2, THIGH),
            // 大腿四頭筋 (前)
            Part::belly(r * 1.5, Vec3::new(0.0, -THIGH * 0.4, 0.04), Vec3::new(1.0, 1.7, 1.1)),
            // ハムストリング (後)
            Part::belly(r * 1.4, Vec3::new(0.0, -THIGH * 0.42, -0.05), Vec3::new(0.95, 1.6, 0.9)),
            // 膝へ向けて細く
            Part::belly(r * 0.85, Vec3::new(0.0, -THIGH * 0.95, 0.0), Vec3::ONE),
        ],
        "shinL" | "shinR" => vec![
            Part::limb(r * 0.85, SHIN),
            // 腓腹筋 (後上部)
            Part::belly(r * 1.15, Vec3::new(0.0, -SHIN * 0.3, -0.04), Vec3::new(0.95, 1.35, 1.05)),
            // 足首へ細く
            Part::belly(r * 0.55, Vec3::new(0.0, -SHIN * 0.95, 0.0), Vec3::ONE),
        ],
        "footL" | "footR" => vec![Part { shape: Shape::Sphere(r * 0.8), offset: Vec3::new(0.0, -0.02, FOOT * 0.4), scale: Vec3::new(0.9, 0.55, 2.2), skin: true }],
        _ => vec![],
    }
}

/// 1 関節ぶんのポーズ角 (度)。
pub struct JointAngle {
    pub joint: &'static str,
    pub euler_deg: [f32; 3],
}

/// ポーズ = ルート変換 + 関節角のリスト。列挙にない関節は identity。
pub struct Pose {
    pub root_pos: Vec3,
    pub root_rot_deg: [f32; 3],
    pub joints: &'static [JointAngle],
}

fn quat_deg(e: [f32; 3]) -> Quat {
    Quat::from_euler(EulerRot::XYZ, e[0] * DEG, e[1] * DEG, e[2] * DEG)
}

/// FK 骨格を 1 体 spawn する。関節角を初期 Transform に焼き込む (表示専用)。
/// `lift` は接地補正の鉛直オフセット。寝技ペアでは相対位置を保つため両者に同じ値を渡す。
pub fn spawn_fighter(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    pose: &Pose,
    lift: f32,
    body_color: Color,
    skin_color: Color,
) {
    let body_mat = materials.add(StandardMaterial {
        base_color: body_color,
        perceptual_roughness: 0.85,
        ..default()
    });
    let skin_mat = materials.add(StandardMaterial {
        base_color: skin_color,
        perceptual_roughness: 0.6,
        ..default()
    });

    let angle_of: HashMap<&str, [f32; 3]> =
        pose.joints.iter().map(|j| (j.joint, j.euler_deg)).collect();

    let root_rot = quat_deg(pose.root_rot_deg);
    let root_pos = pose.root_pos + Vec3::new(0.0, lift, 0.0);

    let mut entities: HashMap<&str, Entity> = HashMap::new();

    for (name, parent, local_pos) in SKELETON {
        let rot = quat_deg(angle_of.get(name).copied().unwrap_or([0.0, 0.0, 0.0]));
        let transform = match parent {
            Some(_) => Transform::from_translation(Vec3::from_array(*local_pos)).with_rotation(rot),
            None => Transform::from_translation(root_pos).with_rotation(root_rot),
        };
        let joint = commands.spawn((transform, Visibility::default())).id();
        entities.insert(name, joint);
        if let Some(p) = parent {
            commands.entity(entities[p]).add_child(joint);
        }

        for part in parts_for(name) {
            let mat = if part.skin { skin_mat.clone() } else { body_mat.clone() };
            let mesh = match part.shape {
                Shape::Capsule(r, len) => meshes.add(Capsule3d::new(r, len)),
                Shape::Sphere(r) => meshes.add(Sphere::new(r)),
            };
            let bone = commands
                .spawn((
                    Mesh3d(mesh),
                    MeshMaterial3d(mat),
                    Transform::from_translation(part.offset).with_scale(part.scale),
                ))
                .id();
            commands.entity(joint).add_child(bone);
        }
    }
}

const FLOOR_MARGIN: f32 = 0.01;

/// シーン内の全ポーズをまとめて接地させる鉛直オフセットを返す。
/// ペア (寝技) では両者に同じ値を渡し、相対位置を保ったまま床に乗せる。
pub fn ground_lift(poses: &[&Pose]) -> f32 {
    let min_y = poses
        .iter()
        .map(|p| {
            let angle_of: HashMap<&str, [f32; 3]> =
                p.joints.iter().map(|j| (j.joint, j.euler_deg)).collect();
            lowest_world_y(&angle_of, p.root_pos, quat_deg(p.root_rot_deg))
        })
        .fold(f32::INFINITY, f32::min);
    FLOOR_MARGIN - min_y
}

/// ポーズを CPU 側 FK で解き、全パーツの最下点のワールド Y を返す (接地補正用)。
fn lowest_world_y(angle_of: &HashMap<&str, [f32; 3]>, root_pos: Vec3, root_rot: Quat) -> f32 {
    let mut world: HashMap<&str, Transform> = HashMap::new();
    let mut min_y = f32::INFINITY;

    for (name, parent, local_pos) in SKELETON {
        let rot = quat_deg(angle_of.get(name).copied().unwrap_or([0.0, 0.0, 0.0]));
        let t = match parent {
            Some(p) => {
                let local = Transform::from_translation(Vec3::from_array(*local_pos)).with_rotation(rot);
                world[p].mul_transform(local)
            }
            None => Transform::from_translation(root_pos).with_rotation(root_rot),
        };
        world.insert(name, t);

        for part in parts_for(name) {
            // パーツ中心のワールド座標から、鉛直方向の最大半径を引いた点を下端とみなす。
            let center = t.transform_point(part.offset);
            let radius = match part.shape {
                Shape::Sphere(r) => r * part.scale.max_element(),
                // カプセルは両端 (ローカル ±len/2) も評価する。
                Shape::Capsule(r, len) => {
                    for end in [part.offset.y + len / 2.0, part.offset.y - len / 2.0] {
                        let p = t.transform_point(Vec3::new(part.offset.x, end, part.offset.z));
                        min_y = min_y.min(p.y - r);
                    }
                    r
                }
            };
            min_y = min_y.min(center.y - radius);
        }
    }
    min_y
}
