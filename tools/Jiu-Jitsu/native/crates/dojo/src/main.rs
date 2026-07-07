//! 柔術道場 (ネイティブ版) — Bevy + Avian。
//! 関節を物理ジョイントで表現する articulated 人体の最小起動確認。
//!
//! Web 版 (v2) がプリミティブ人体を手打ち角度で並べていたのに対し、こちらは
//! `anatomy` クレートの関節可動域を Avian のジョイントリミットへ写す。これが本移行の要。

mod poses;
mod rig;
mod skeleton;

use avian3d::prelude::*;
use bevy::prelude::*;
use bevy::render::view::screenshot::{save_to_disk, Screenshot};

const RED: Color = Color::srgb(0.76, 0.23, 0.23);
const BLUE: Color = Color::srgb(0.18, 0.37, 0.82);
const SKIN: Color = Color::srgb(0.85, 0.66, 0.47);

/// DOJO_SCENE で表示を切替: "standing"(既定) / "mount" / "physics"。
fn scene_mode() -> String {
    std::env::var("DOJO_SCENE").unwrap_or_else(|_| "standing".to_string())
}

fn main() {
    let mut app = App::new();
    app.add_plugins((DefaultPlugins, PhysicsPlugins::default()))
        .insert_resource(ClearColor(Color::srgb(0.08, 0.09, 0.11)))
        .add_systems(Startup, setup);

    // DOJO_CAPTURE=<path> のとき、物理が落ち着いた頃に 1 枚撮って自動終了する (検証用)。
    if let Ok(path) = std::env::var("DOJO_CAPTURE") {
        app.insert_resource(Capture { path, frame: 0 })
            .add_systems(Update, capture_then_exit);
    }
    app.run();
}

#[derive(Resource)]
struct Capture {
    path: String,
    frame: u32,
}

fn capture_then_exit(
    mut commands: Commands,
    mut cap: ResMut<Capture>,
    mut exit: MessageWriter<AppExit>,
) {
    cap.frame += 1;
    if cap.frame == 90 {
        commands
            .spawn(Screenshot::primary_window())
            .observe(save_to_disk(cap.path.clone()));
    }
    if cap.frame >= 110 {
        exit.write(AppExit::Success);
    }
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // 畳 (静的な床)
    commands.spawn((
        RigidBody::Static,
        Collider::cuboid(8.0, 0.1, 8.0),
        Mesh3d(meshes.add(Cuboid::new(8.0, 0.1, 8.0))),
        MeshMaterial3d(materials.add(Color::srgb(0.25, 0.43, 0.33))),
        Transform::from_xyz(0.0, -0.05, 0.0),
    ));

    match scene_mode().as_str() {
        "physics" => {
            rig::spawn_demo_human(&mut commands, &mut meshes, &mut materials, BLUE);
        }
        "mount" => {
            let (red, blue) = (poses::red_mount_top(), poses::blue_under_mount());
            // 接地は下の青だけで決め、同じ lift を赤にも適用 (赤は青に対する相対高さで乗る)。
            let lift = skeleton::ground_lift(&[&blue]);
            skeleton::spawn_fighter(&mut commands, &mut meshes, &mut materials, &red, lift, RED, SKIN);
            skeleton::spawn_fighter(&mut commands, &mut meshes, &mut materials, &blue, lift, BLUE, SKIN);
        }
        _ => {
            let (red, blue) = (poses::standing_red(), poses::standing_blue());
            let lift = skeleton::ground_lift(&[&red, &blue]);
            skeleton::spawn_fighter(&mut commands, &mut meshes, &mut materials, &red, lift, RED, SKIN);
            skeleton::spawn_fighter(&mut commands, &mut meshes, &mut materials, &blue, lift, BLUE, SKIN);
        }
    }

    commands.spawn((
        DirectionalLight {
            shadows_enabled: true,
            illuminance: 8000.0,
            ..default()
        },
        Transform::from_xyz(3.0, 6.0, 3.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
    commands.spawn((
        PointLight {
            intensity: 400_000.0,
            ..default()
        },
        Transform::from_xyz(-3.0, 4.0, -2.0),
    ));

    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(2.4, 1.4, 2.8).looking_at(Vec3::new(0.0, 1.0, 0.0), Vec3::Y),
    ));
}
