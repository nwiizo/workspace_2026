use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use clap::Parser;
use maskcam::{FaceBox, FaceSmoother, Visibility, overlay_placement, visibility_for_face_count};
use num_traits::ToPrimitive;
use opencv::{
    core::{self, Mat, Point2f, Scalar, Size},
    dnn, highgui, imgcodecs, imgproc,
    objdetect::{FaceDetectorYN, FaceDetectorYNTrait},
    prelude::*,
    videoio,
};

const WINDOW_NAME: &str = "Maskcam Output";

#[derive(Debug, Parser)]
#[command(version, about)]
struct Args {
    /// Image placed over the detected face.
    #[arg(long, default_value = "~/Downloads/icon.jpg", value_name = "PATH")]
    overlay: PathBuf,

    /// `YuNet` ONNX model path.
    #[arg(
        long,
        default_value = "models/face_detection_yunet_2026may.onnx",
        value_name = "PATH"
    )]
    model: PathBuf,

    /// `AVFoundation` camera index.
    #[arg(long, default_value_t = 0)]
    camera: i32,

    /// Requested capture width.
    #[arg(long, default_value_t = 1280)]
    width: i32,

    /// Requested capture height.
    #[arg(long, default_value_t = 720)]
    height: i32,

    /// Requested capture frame rate.
    #[arg(long, default_value_t = 30.0)]
    fps: f64,

    /// Width used for face detection; output remains at capture resolution.
    #[arg(long, default_value_t = 640)]
    detection_width: i32,

    /// Overlay size relative to the larger face-box dimension.
    #[arg(long, default_value_t = 1.9)]
    overlay_scale: f32,

    /// Minimum `YuNet` face confidence.
    #[arg(long, default_value_t = 0.85)]
    confidence: f32,

    /// Mirror the output horizontally.
    #[arg(long)]
    mirror: bool,

    /// Validate the overlay and face model without opening the camera.
    #[arg(long)]
    check: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();
    validate_args(&args)?;

    let overlay_path = expand_tilde(&args.overlay);
    let model_path = expand_tilde(&args.model);
    let overlay = load_image(&overlay_path, "overlay image")?;
    let mut detector = create_detector(&model_path, args.confidence)?;

    if args.check {
        check_detector(&mut detector)?;
        println!(
            "Maskcam check passed: overlay={}, model={}",
            overlay_path.display(),
            model_path.display()
        );
        return Ok(());
    }

    let mut camera = open_camera(&args)?;
    let mut smoother = FaceSmoother::new(0.45);

    highgui::named_window(
        WINDOW_NAME,
        highgui::WINDOW_NORMAL | highgui::WINDOW_KEEPRATIO,
    )
    .context("failed to create output window")?;
    highgui::resize_window(WINDOW_NAME, args.width, args.height)
        .context("failed to size output window")?;

    eprintln!(
        "Maskcam is running. Capture the '{WINDOW_NAME}' window in OBS. Press q or Esc to quit, f for fullscreen."
    );

    let mut fullscreen = false;
    loop {
        let mut frame = Mat::default();
        if !camera
            .read(&mut frame)
            .context("failed to read camera frame")?
            || frame.empty()
        {
            bail!("camera stopped producing frames");
        }

        if args.mirror {
            let mut mirrored = Mat::default();
            core::flip(&frame, &mut mirrored, 1).context("failed to mirror frame")?;
            frame = mirrored;
        }

        let output = render_frame(
            &frame,
            &overlay,
            &mut detector,
            &mut smoother,
            args.overlay_scale,
            args.detection_width,
        )?;
        highgui::imshow(WINDOW_NAME, &output).context("failed to display output frame")?;

        let key = highgui::wait_key(1).context("failed to poll keyboard")? & 0xff;
        match key {
            27 | 113 => break,
            102 => {
                fullscreen = !fullscreen;
                let mode = if fullscreen {
                    highgui::WINDOW_FULLSCREEN
                } else {
                    highgui::WINDOW_NORMAL
                };
                highgui::set_window_property(
                    WINDOW_NAME,
                    highgui::WND_PROP_FULLSCREEN,
                    f64::from(mode),
                )
                .context("failed to toggle fullscreen")?;
            }
            _ => {}
        }
    }

    Ok(())
}

fn validate_args(args: &Args) -> Result<()> {
    if args.width <= 0 || args.height <= 0 {
        bail!("width and height must be positive");
    }
    if args.fps <= 0.0 {
        bail!("fps must be positive");
    }
    if args.detection_width <= 0 {
        bail!("detection width must be positive");
    }
    if !(1.0..=4.0).contains(&args.overlay_scale) {
        bail!("overlay scale must be between 1.0 and 4.0");
    }
    if !(0.0..=1.0).contains(&args.confidence) {
        bail!("confidence must be between 0.0 and 1.0");
    }
    Ok(())
}

fn expand_tilde(path: &Path) -> PathBuf {
    let Some(value) = path.to_str() else {
        return path.to_owned();
    };
    if value == "~" {
        return std::env::var_os("HOME").map_or_else(|| path.to_owned(), PathBuf::from);
    }
    if let Some(rest) = value.strip_prefix("~/")
        && let Some(home) = std::env::var_os("HOME")
    {
        return PathBuf::from(home).join(rest);
    }
    path.to_owned()
}

fn load_image(path: &Path, label: &str) -> Result<Mat> {
    let path_text = path
        .to_str()
        .with_context(|| format!("{label} path is not valid UTF-8: {}", path.display()))?;
    let image = imgcodecs::imread(path_text, imgcodecs::IMREAD_COLOR)
        .with_context(|| format!("failed to decode {label}: {}", path.display()))?;
    if image.empty() {
        bail!("{label} is empty or unreadable: {}", path.display());
    }
    Ok(image)
}

fn create_detector(model_path: &Path, confidence: f32) -> Result<core::Ptr<FaceDetectorYN>> {
    if !model_path.is_file() {
        bail!(
            "YuNet model not found at {}. Run ./scripts/download-model.sh first",
            model_path.display()
        );
    }
    let model = model_path
        .to_str()
        .with_context(|| format!("model path is not valid UTF-8: {}", model_path.display()))?;

    FaceDetectorYN::create(
        model,
        "",
        Size::new(320, 320),
        confidence,
        0.3,
        5_000,
        dnn::DNN_BACKEND_OPENCV,
        dnn::DNN_TARGET_CPU,
    )
    .with_context(|| format!("failed to load YuNet model: {}", model_path.display()))
}

fn check_detector(detector: &mut core::Ptr<FaceDetectorYN>) -> Result<()> {
    let size = Size::new(640, 360);
    let black =
        Mat::new_rows_cols_with_default(size.height, size.width, core::CV_8UC3, Scalar::all(0.0))
            .context("failed to create detector test image")?;
    detector
        .set_input_size(size)
        .context("failed to set detector test size")?;
    let mut detections = Mat::default();
    detector
        .detect(&black, &mut detections)
        .context("YuNet model is incompatible with this OpenCV build")?;
    Ok(())
}

fn open_camera(args: &Args) -> Result<videoio::VideoCapture> {
    let mut camera = videoio::VideoCapture::new(args.camera, videoio::CAP_AVFOUNDATION)
        .with_context(|| format!("failed to open camera index {}", args.camera))?;
    if !camera.is_opened().context("failed to query camera state")? {
        bail!("camera index {} is unavailable", args.camera);
    }

    let _width_applied = camera.set(videoio::CAP_PROP_FRAME_WIDTH, f64::from(args.width))?;
    let _height_applied = camera.set(videoio::CAP_PROP_FRAME_HEIGHT, f64::from(args.height))?;
    let _fps_applied = camera.set(videoio::CAP_PROP_FPS, args.fps)?;
    Ok(camera)
}

fn render_frame(
    frame: &Mat,
    overlay: &Mat,
    detector: &mut core::Ptr<FaceDetectorYN>,
    smoother: &mut FaceSmoother,
    overlay_scale: f32,
    detection_width: i32,
) -> Result<Mat> {
    let frame_size = frame.size()?;
    let detection_size = if frame_size.width > detection_width {
        let scaled_height =
            i64::from(frame_size.height) * i64::from(detection_width) / i64::from(frame_size.width);
        Size::new(
            detection_width,
            i32::try_from(scaled_height)
                .context("scaled detection height exceeds the supported range")?
                .max(1),
        )
    } else {
        frame_size
    };
    let detection_frame = if detection_size == frame_size {
        frame
            .try_clone()
            .context("failed to clone detection frame")?
    } else {
        let mut resized = Mat::default();
        imgproc::resize(
            frame,
            &mut resized,
            detection_size,
            0.0,
            0.0,
            imgproc::INTER_AREA,
        )
        .context("failed to resize detection frame")?;
        resized
    };

    detector
        .set_input_size(detection_size)
        .context("failed to update detector input size")?;
    let mut detections = Mat::default();
    detector
        .detect(&detection_frame, &mut detections)
        .context("face detection failed")?;

    let coordinate_scale_x = frame_size
        .width
        .to_f32()
        .context("frame width cannot be represented for face coordinates")?
        / detection_size
            .width
            .to_f32()
            .context("detection width cannot be represented for face coordinates")?;
    let coordinate_scale_y = frame_size
        .height
        .to_f32()
        .context("frame height cannot be represented for face coordinates")?
        / detection_size
            .height
            .to_f32()
            .context("detection height cannot be represented for face coordinates")?;
    let faces = parse_faces(&detections, coordinate_scale_x, coordinate_scale_y)?;
    match visibility_for_face_count(faces.len()) {
        Visibility::ShowMaskedFace => {
            let mut output = frame.try_clone().context("failed to clone camera frame")?;
            let smoothed_face = smoother.update(faces[0]);
            apply_overlay(&mut output, overlay, smoothed_face, overlay_scale)?;
            Ok(output)
        }
        Visibility::HideEntireFrame => {
            smoother.reset();
            render_hidden_frame(frame_size, overlay)
        }
    }
}

fn parse_faces(
    detections: &Mat,
    coordinate_scale_x: f32,
    coordinate_scale_y: f32,
) -> Result<Vec<FaceBox>> {
    let capacity = usize::try_from(detections.rows().max(0)).unwrap_or_default();
    let mut faces = Vec::with_capacity(capacity);
    for row in 0..detections.rows() {
        let left_eye = Point2f::new(
            *detections.at_2d::<f32>(row, 4)?,
            *detections.at_2d::<f32>(row, 5)?,
        );
        let right_eye = Point2f::new(
            *detections.at_2d::<f32>(row, 6)?,
            *detections.at_2d::<f32>(row, 7)?,
        );
        let angle_degrees = f64::from(right_eye.y - left_eye.y)
            .atan2(f64::from(right_eye.x - left_eye.x))
            .to_degrees();

        faces.push(FaceBox {
            x: *detections.at_2d::<f32>(row, 0)? * coordinate_scale_x,
            y: *detections.at_2d::<f32>(row, 1)? * coordinate_scale_y,
            width: *detections.at_2d::<f32>(row, 2)? * coordinate_scale_x,
            height: *detections.at_2d::<f32>(row, 3)? * coordinate_scale_y,
            angle_degrees,
        });
    }
    faces.sort_by(|left, right| right.area().total_cmp(&left.area()));
    Ok(faces)
}

fn apply_overlay(frame: &mut Mat, overlay: &Mat, face: FaceBox, scale: f32) -> Result<()> {
    let placement = overlay_placement(face, scale, frame.size()?)
        .context("face is outside the camera frame")?;

    let mut resized = Mat::default();
    imgproc::resize(
        overlay,
        &mut resized,
        placement.canvas_size,
        0.0,
        0.0,
        imgproc::INTER_AREA,
    )
    .context("failed to resize overlay")?;

    let center = Point2f::new(
        placement
            .canvas_size
            .width
            .to_f32()
            .context("overlay width cannot be represented for rotation")?
            / 2.0,
        placement
            .canvas_size
            .height
            .to_f32()
            .context("overlay height cannot be represented for rotation")?
            / 2.0,
    );
    let radians = face.angle_degrees.to_radians();
    let alpha = radians.cos();
    let beta = radians.sin();
    let center_x = f64::from(center.x);
    let center_y = f64::from(center.y);
    let rotation = Mat::from_slice_2d(&[
        [
            alpha,
            beta,
            (1.0 - alpha).mul_add(center_x, -beta * center_y),
        ],
        [
            -beta,
            alpha,
            beta.mul_add(center_x, (1.0 - alpha) * center_y),
        ],
    ])
    .context("failed to calculate overlay rotation")?;
    let mut rotated = Mat::default();
    imgproc::warp_affine(
        &resized,
        &mut rotated,
        &rotation,
        placement.canvas_size,
        imgproc::INTER_LINEAR,
        core::BORDER_CONSTANT,
        Scalar::all(255.0),
        core::AlgorithmHint::ALGO_HINT_DEFAULT,
    )
    .context("failed to rotate overlay")?;

    let source = Mat::roi(&rotated, placement.source).context("failed to crop overlay")?;
    let mut destination =
        Mat::roi_mut(frame, placement.destination).context("failed to select output region")?;
    source
        .copy_to(&mut destination)
        .context("failed to composite overlay")?;
    Ok(())
}

fn render_hidden_frame(frame_size: Size, overlay: &Mat) -> Result<Mat> {
    let mut hidden = Mat::new_rows_cols_with_default(
        frame_size.height,
        frame_size.width,
        core::CV_8UC3,
        Scalar::all(0.0),
    )
    .context("failed to create hidden frame")?;

    let side = (frame_size.width.min(frame_size.height) / 3).max(1);
    let mut icon = Mat::default();
    imgproc::resize(
        overlay,
        &mut icon,
        Size::new(side, side),
        0.0,
        0.0,
        imgproc::INTER_AREA,
    )
    .context("failed to resize hidden-frame icon")?;
    let target = core::Rect::new(
        (frame_size.width - side) / 2,
        (frame_size.height - side) / 2,
        side,
        side,
    );
    let mut destination =
        Mat::roi_mut(&mut hidden, target).context("failed to select hidden-frame region")?;
    icon.copy_to(&mut destination)
        .context("failed to render hidden-frame icon")?;
    Ok(hidden)
}
