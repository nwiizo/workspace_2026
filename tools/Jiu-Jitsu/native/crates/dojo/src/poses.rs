//! 表示用ポーズのライブラリ (v2 poses.ts から移植)。角度は度。
//! 座標規約: 立位 root=(x,0.92,0)・顔 +Z・脊柱 +Y・手脚ローカル -Y。
//! 仰向け頭-Z: root.rot=[-90,0,0] / 頭+Z: [-90,0,180] / うつ伏せ頭+Z: [+90,0,0]。

use crate::skeleton::{JointAngle, Pose};
use bevy::prelude::Vec3;

macro_rules! angles {
    ($($j:literal => [$x:expr, $y:expr, $z:expr]),* $(,)?) => {
        &[$(JointAngle { joint: $j, euler_deg: [$x as f32, $y as f32, $z as f32] }),*]
    };
}

/// 立ち姿 (礼) — 赤。青と向き合う (root を -90° 回して顔を -X へ)。
pub fn standing_red() -> Pose {
    Pose {
        root_pos: Vec3::new(0.42, 0.92, 0.0),
        root_rot_deg: [0.0, -90.0, 0.0],
        joints: angles! {
            "upperArmL" => [4, 0, 6], "upperArmR" => [4, 0, -6],
        },
    }
}

/// 立ち姿 (礼) — 青。
pub fn standing_blue() -> Pose {
    Pose {
        root_pos: Vec3::new(-0.42, 0.92, 0.0),
        root_rot_deg: [0.0, 90.0, 0.0],
        joints: angles! {
            "upperArmL" => [4, 0, 6], "upperArmR" => [4, 0, -6],
        },
    }
}

/// マウント上 (赤) — 青の腹の上に跨り、膝はマットへ。顔は -Z で青の頭を見る。
/// 膝立ち: 腿を前へ倒して膝をマットへ下ろし、脛を後ろへ畳む。座はやや前傾。
pub fn red_mount_top() -> Pose {
    Pose {
        root_pos: Vec3::new(0.0, 0.34, 0.02),
        root_rot_deg: [14.0, 180.0, 0.0],
        joints: angles! {
            // 膝を左右に開いて青の胴を跨ぎ、マットへ下ろす
            "thighL" => [-84, 8, 26], "shinL" => [128, 0, 0],
            "thighR" => [-84, -8, -26], "shinR" => [128, 0, 0],
            "upperArmL" => [36, 0, 18], "forearmL" => [52, 0, 0],
            "upperArmR" => [36, 0, -18], "forearmR" => [52, 0, 0],
            "neck" => [10, 0, 0],
        },
    }
}

/// マウント下 (青) — マット上に平らな仰向け。頭 -Z、腹が上。膝は軽く立てる。
pub fn blue_under_mount() -> Pose {
    Pose {
        root_pos: Vec3::new(0.0, 0.13, 0.0),
        root_rot_deg: [-90.0, 0.0, 0.0],
        joints: angles! {
            "thighL" => [16, 0, 10], "shinL" => [46, 0, 0],
            "thighR" => [16, 0, -10], "shinR" => [46, 0, 0],
            "upperArmL" => [-44, 0, 26], "forearmL" => [-70, 0, 0],
            "upperArmR" => [-44, 0, -26], "forearmR" => [-70, 0, 0],
            "neck" => [-16, 0, 0],
        },
    }
}
