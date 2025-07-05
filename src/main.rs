use clipboard_rs::common::RustImage;
use clipboard_rs::{Clipboard, ClipboardContext, RustImageData};
use nokhwa::NokhwaError;
use nokhwa::pixel_format::RgbFormat;
use nokhwa::utils::{ApiBackend, RequestedFormat, RequestedFormatType};
use regex::RegexSet;
use std::sync::LazyLock;
use thiserror::Error;

const DIALOG_TITLE: &str = "Capture Card Screenshot";

static CAPTURE_CARD_REGEX: LazyLock<RegexSet> =
    LazyLock::new(|| RegexSet::new(["^UGREEN HDMI Capture$", "^Live Gamer .*-Video$"]).unwrap());

#[derive(Debug, Error)]
enum ScreenshotError {
    #[error("No supported cameras found")]
    NoCamerasFound,
    #[error("Failed to take screenshot: {0}")]
    Screenshot(#[from] NokhwaError),
    #[error("Failed to copy to clipboard: {0}")]
    Clipboard(#[from] Box<dyn std::error::Error + Send + Sync>),
}

fn main() {
    nokhwa::nokhwa_initialize(|granted| {
        if !granted {
            msgbox::create(
                DIALOG_TITLE,
                "Camera permission is required to take a screenshot.",
                msgbox::IconType::Error,
            )
            .unwrap();
        }
    });
    if let Err(e) = perform_screenshot() {
        msgbox::create(DIALOG_TITLE, &e.to_string(), msgbox::IconType::Error).unwrap();
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
        RequestedFormat::new::<RgbFormat>(RequestedFormatType::AbsoluteHighestResolution),
    )?;
    println!(
        "Created camera {} with backend {}",
        camera.index(),
        camera.backend()
    );

    camera.open_stream()?;
    println!("Opened camera with format {}", camera.camera_format());

    // Use a loop, as some capture cards can take a moment to start up
    let frame = loop {
        let frame = camera.frame()?.decode_image::<RgbFormat>()?;
        println!("Captured image {}x{}", frame.width(), frame.height());
        if frame.iter().enumerate().any(|(i, p)| i & 3 != 3 && *p > 0) {
            break frame;
        }
    };

    camera.stop_stream()?;
    drop(camera);
    println!("Closed camera");

    let clipboard = ClipboardContext::new()?;
    println!("Opened clipboard");

    let image = RustImageData::from_dynamic_image(frame.into());
    println!("Converted image");

    clipboard.set_image(image)?;
    println!("Copied to clipboard");

    Ok(())
}
