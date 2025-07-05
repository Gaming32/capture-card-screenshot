use std::borrow::Cow;
use nokhwa::utils::{ApiBackend, RequestedFormat, RequestedFormatType};
use regex::RegexSet;
use std::sync::LazyLock;
use arboard::{Clipboard, ImageData};
use nokhwa::NokhwaError;
use nokhwa::pixel_format::{RgbAFormat, RgbFormat};
use thiserror::Error;

const DIALOG_TITLE: &str = "Capture Card Screenshot";

static CAPTURE_CARD_REGEX: LazyLock<RegexSet> = LazyLock::new(|| RegexSet::new([
    "^UGREEN HDMI Capture$",
    "^Live Gamer .*-Video$",
]).unwrap());

#[derive(Debug, Error)]
enum ScreenshotError {
    #[error("No supported cameras found")]
    NoCamerasFound,
    #[error("Failed to take screenshot: {0}")]
    ScreenshotError(#[from] NokhwaError),
    #[error("Failed to copy to clipboard: {0}")]
    ClipboardError(#[from] arboard::Error),
}

fn main() {
    nokhwa::nokhwa_initialize(|granted| {
        if !granted {
            msgbox::create(
                DIALOG_TITLE,
                "Camera permission is required to take a screenshot.",
                msgbox::IconType::Error,
            ).unwrap();
        }
    });
    if let Err(e) = perform_screenshot() {
        let mut message = e.to_string();
        if matches!(e, ScreenshotError::NoCamerasFound) {
            message.push_str("\nAvailable cameras:");
            for camera in nokhwa::query(ApiBackend::Auto).into_iter().flatten() {
                message.push_str("\n  -");
                message.push_str(&camera.human_name());
            }
        }
        msgbox::create(
            DIALOG_TITLE,
            &e.to_string(),
            msgbox::IconType::Error,
        ).unwrap();
    }
}

fn perform_screenshot() -> Result<(), ScreenshotError> {
    let camera = nokhwa::query(ApiBackend::Auto)?
        .into_iter()
        .find(|x| CAPTURE_CARD_REGEX.is_match(&x.human_name()))
        .ok_or(ScreenshotError::NoCamerasFound)?;
    println!("Found camera: {camera}");

    let mut camera = nokhwa::Camera::new(
        camera.index().clone(),
        RequestedFormat::new::<RgbFormat>(RequestedFormatType::AbsoluteHighestResolution)
    )?;
    println!("Created camera {} with backend {}", camera.index(), camera.backend());

    camera.open_stream()?;
    println!("Opened camera with format {}", camera.camera_format());

    let frame = camera.frame()?.decode_image::<RgbAFormat>()?;
    println!("Captured image {}x{}", frame.width(), frame.height());

    camera.stop_stream()?;
    drop(camera);
    println!("Closed camera");

    let mut clipboard = Clipboard::new()?;
    clipboard.set_image(ImageData {
        width: frame.width() as usize,
        height: frame.height() as usize,
        bytes: Cow::from(frame.as_raw())
    })?;

    Ok(())
}
