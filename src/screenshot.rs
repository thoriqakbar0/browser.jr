use std::num::{NonZeroU32, NonZeroU64};

use crate::Locator;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CaptureTarget {
    Viewport,
    FullPage,
    Element(Locator),
    Rect(CaptureRect),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CaptureRect {
    x: i64,
    y: i64,
    width: NonZeroU64,
    height: NonZeroU64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CaptureRectError {
    EmptyWidth,
    EmptyHeight,
    HorizontalOverflow,
    VerticalOverflow,
}

impl CaptureRect {
    pub fn new(x: i64, y: i64, width: u64, height: u64) -> Result<Self, CaptureRectError> {
        let width = NonZeroU64::new(width).ok_or(CaptureRectError::EmptyWidth)?;
        let height = NonZeroU64::new(height).ok_or(CaptureRectError::EmptyHeight)?;
        checked_end(x, width).ok_or(CaptureRectError::HorizontalOverflow)?;
        checked_end(y, height).ok_or(CaptureRectError::VerticalOverflow)?;
        Ok(Self {
            x,
            y,
            width,
            height,
        })
    }

    pub const fn x(self) -> i64 {
        self.x
    }

    pub const fn y(self) -> i64 {
        self.y
    }

    pub const fn width(self) -> u64 {
        self.width.get()
    }

    pub const fn height(self) -> u64 {
        self.height.get()
    }

    pub fn right(self) -> i64 {
        checked_end(self.x, self.width).expect("capture rectangle validates its right edge")
    }

    pub fn bottom(self) -> i64 {
        checked_end(self.y, self.height).expect("capture rectangle validates its bottom edge")
    }
}

fn checked_end(origin: i64, length: NonZeroU64) -> Option<i64> {
    let length = i64::try_from(length.get()).ok()?;
    origin.checked_add(length)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Rgba8 {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
    pub alpha: u8,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PaintCommand {
    FillRect {
        source: String,
        bounds: CaptureRect,
        color: Rgba8,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PaintScene {
    pub capture_bounds: CaptureRect,
    pub commands: Vec<PaintCommand>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PreparedScreenshot {
    pub target: CaptureTarget,
    pub scene: PaintScene,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RasterImage {
    width: NonZeroU32,
    height: NonZeroU32,
    rgba: Vec<u8>,
}

pub const MAX_SCREENSHOT_PIXELS: u64 = 16_777_216;
pub const MAX_SCREENSHOT_PAINT_PIXELS: u64 = 67_108_864;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RasterImageError {
    EmptyWidth,
    EmptyHeight,
    ByteLengthOverflow,
    WrongByteLength { expected: usize, actual: usize },
}

impl RasterImage {
    pub fn new(width: u32, height: u32, rgba: Vec<u8>) -> Result<Self, RasterImageError> {
        let width = NonZeroU32::new(width).ok_or(RasterImageError::EmptyWidth)?;
        let height = NonZeroU32::new(height).ok_or(RasterImageError::EmptyHeight)?;
        let expected = usize::try_from(
            u64::from(width.get())
                .checked_mul(u64::from(height.get()))
                .and_then(|pixels| pixels.checked_mul(4))
                .ok_or(RasterImageError::ByteLengthOverflow)?,
        )
        .map_err(|_| RasterImageError::ByteLengthOverflow)?;
        if rgba.len() != expected {
            return Err(RasterImageError::WrongByteLength {
                expected,
                actual: rgba.len(),
            });
        }
        Ok(Self {
            width,
            height,
            rgba,
        })
    }

    pub const fn width(&self) -> u32 {
        self.width.get()
    }

    pub const fn height(&self) -> u32 {
        self.height.get()
    }

    pub fn rgba(&self) -> &[u8] {
        &self.rgba
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RasterProcessError {
    Start { reason: String },
    Protocol { reason: String },
    Render { reason: String },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PngEncodingError {
    reason: String,
}

impl PngEncodingError {
    pub fn reason(&self) -> &str {
        &self.reason
    }
}

pub trait RasterProcess {
    fn render(
        &mut self,
        screenshot: &PreparedScreenshot,
    ) -> Result<RasterImage, RasterProcessError>;
}

pub trait RasterProcessFactory {
    type Process: RasterProcess;

    fn start(&mut self) -> Result<Self::Process, RasterProcessError>;
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SoftwareRasterProcessFactory;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SoftwareRasterProcess;

impl RasterProcessFactory for SoftwareRasterProcessFactory {
    type Process = SoftwareRasterProcess;

    fn start(&mut self) -> Result<Self::Process, RasterProcessError> {
        Ok(SoftwareRasterProcess)
    }
}

impl RasterProcess for SoftwareRasterProcess {
    fn render(
        &mut self,
        screenshot: &PreparedScreenshot,
    ) -> Result<RasterImage, RasterProcessError> {
        rasterize_scene(&screenshot.scene)
    }
}

#[derive(Debug)]
pub struct OnDemandRasterProcess<F>
where
    F: RasterProcessFactory,
{
    factory: F,
    process: Option<F::Process>,
}

pub fn encode_png(image: &RasterImage) -> Result<Vec<u8>, PngEncodingError> {
    let mut bytes = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut bytes, image.width(), image.height());
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().map_err(png_error)?;
        writer.write_image_data(image.rgba()).map_err(png_error)?;
        writer.finish().map_err(png_error)?;
    }
    Ok(bytes)
}

fn png_error(error: png::EncodingError) -> PngEncodingError {
    PngEncodingError {
        reason: error.to_string(),
    }
}

fn rasterize_scene(scene: &PaintScene) -> Result<RasterImage, RasterProcessError> {
    let width = u32::try_from(scene.capture_bounds.width())
        .map_err(|_| render_error("capture width exceeds the software rasterizer limit"))?;
    let height = u32::try_from(scene.capture_bounds.height())
        .map_err(|_| render_error("capture height exceeds the software rasterizer limit"))?;
    let pixel_count = u64::from(width)
        .checked_mul(u64::from(height))
        .ok_or_else(|| render_error("capture pixel count overflows"))?;
    if pixel_count > MAX_SCREENSHOT_PIXELS {
        return Err(render_error(format!(
            "capture has {pixel_count} pixels; the limit is {MAX_SCREENSHOT_PIXELS}"
        )));
    }
    let mut paint_pixels = 0_u64;
    for command in &scene.commands {
        let PaintCommand::FillRect { bounds, .. } = command;
        paint_pixels = paint_pixels
            .checked_add(clipped_pixel_count(scene.capture_bounds, *bounds))
            .ok_or_else(|| render_error("capture paint work overflows"))?;
        if paint_pixels > MAX_SCREENSHOT_PAINT_PIXELS {
            return Err(render_error(format!(
                "capture paints {paint_pixels} clipped pixels; the limit is {MAX_SCREENSHOT_PAINT_PIXELS}"
            )));
        }
    }
    let byte_count = usize::try_from(
        pixel_count
            .checked_mul(4)
            .ok_or_else(|| render_error("capture byte length overflows"))?,
    )
    .map_err(|_| render_error("capture byte length exceeds platform limits"))?;
    let mut rgba = vec![0; byte_count];
    for command in &scene.commands {
        match command {
            PaintCommand::FillRect { bounds, color, .. } => {
                rasterize_fill_rect(&mut rgba, width, scene.capture_bounds, *bounds, *color);
            }
        }
    }
    RasterImage::new(width, height, rgba).map_err(|error| RasterProcessError::Protocol {
        reason: error.to_string(),
    })
}

fn clipped_pixel_count(capture: CaptureRect, bounds: CaptureRect) -> u64 {
    let left = bounds.x().max(capture.x());
    let top = bounds.y().max(capture.y());
    let right = bounds.right().min(capture.right());
    let bottom = bounds.bottom().min(capture.bottom());
    if left >= right || top >= bottom {
        return 0;
    }
    let width = u64::try_from(i128::from(right) - i128::from(left))
        .expect("clipped width is non-negative and bounded by a validated capture");
    let height = u64::try_from(i128::from(bottom) - i128::from(top))
        .expect("clipped height is non-negative and bounded by a validated capture");
    width
        .checked_mul(height)
        .expect("clipped area cannot exceed the validated capture area")
}

fn rasterize_fill_rect(
    rgba: &mut [u8],
    image_width: u32,
    capture: CaptureRect,
    bounds: CaptureRect,
    color: Rgba8,
) {
    let left = bounds.x().max(capture.x());
    let top = bounds.y().max(capture.y());
    let right = bounds.right().min(capture.right());
    let bottom = bounds.bottom().min(capture.bottom());
    if left >= right || top >= bottom || color.alpha == 0 {
        return;
    }
    let local_left = usize::try_from(i128::from(left) - i128::from(capture.x()))
        .expect("clipped left edge is inside the capture");
    let local_top = usize::try_from(i128::from(top) - i128::from(capture.y()))
        .expect("clipped top edge is inside the capture");
    let local_right = usize::try_from(i128::from(right) - i128::from(capture.x()))
        .expect("clipped right edge is inside the capture");
    let local_bottom = usize::try_from(i128::from(bottom) - i128::from(capture.y()))
        .expect("clipped bottom edge is inside the capture");
    let row_width = usize::try_from(image_width).expect("u32 image width fits usize");
    for y in local_top..local_bottom {
        for x in local_left..local_right {
            let index = (y * row_width + x) * 4;
            blend_source_over(&mut rgba[index..index + 4], color);
        }
    }
}

fn blend_source_over(destination: &mut [u8], source: Rgba8) {
    let source_alpha = u32::from(source.alpha);
    if source_alpha == 255 {
        destination.copy_from_slice(&[source.red, source.green, source.blue, source.alpha]);
        return;
    }
    let destination_alpha = u32::from(destination[3]);
    let inverse = 255 - source_alpha;
    let output_alpha = source_alpha + (destination_alpha * inverse + 127) / 255;
    if output_alpha == 0 {
        destination.fill(0);
        return;
    }
    for (channel, source_channel) in [source.red, source.green, source.blue]
        .into_iter()
        .enumerate()
    {
        let premultiplied = u32::from(source_channel) * source_alpha
            + (u32::from(destination[channel]) * destination_alpha * inverse + 127) / 255;
        destination[channel] = ((premultiplied + output_alpha / 2) / output_alpha) as u8;
    }
    destination[3] = output_alpha as u8;
}

fn render_error(reason: impl Into<String>) -> RasterProcessError {
    RasterProcessError::Render {
        reason: reason.into(),
    }
}

impl<F> OnDemandRasterProcess<F>
where
    F: RasterProcessFactory,
{
    pub const fn new(factory: F) -> Self {
        Self {
            factory,
            process: None,
        }
    }

    pub const fn is_started(&self) -> bool {
        self.process.is_some()
    }

    pub fn render(
        &mut self,
        screenshot: &PreparedScreenshot,
    ) -> Result<RasterImage, RasterProcessError> {
        if self.process.is_none() {
            self.process = Some(self.factory.start()?);
        }
        self.process
            .as_mut()
            .expect("raster process was installed before rendering")
            .render(screenshot)
    }
}

impl std::fmt::Display for CaptureRectError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::EmptyWidth => "capture width must be greater than zero",
            Self::EmptyHeight => "capture height must be greater than zero",
            Self::HorizontalOverflow => "capture rectangle exceeds horizontal coordinate limits",
            Self::VerticalOverflow => "capture rectangle exceeds vertical coordinate limits",
        })
    }
}

impl std::error::Error for CaptureRectError {}

impl std::fmt::Display for RasterImageError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyWidth => formatter.write_str("raster image width must be greater than zero"),
            Self::EmptyHeight => {
                formatter.write_str("raster image height must be greater than zero")
            }
            Self::ByteLengthOverflow => {
                formatter.write_str("raster image byte length exceeds platform limits")
            }
            Self::WrongByteLength { expected, actual } => write!(
                formatter,
                "raster image needs {expected} RGBA bytes but received {actual}"
            ),
        }
    }
}

impl std::error::Error for RasterImageError {}

impl std::fmt::Display for RasterProcessError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Start { reason } => write!(formatter, "raster process could not start: {reason}"),
            Self::Protocol { reason } => {
                write!(formatter, "raster process returned invalid data: {reason}")
            }
            Self::Render { reason } => write!(formatter, "raster process failed: {reason}"),
        }
    }
}

impl std::error::Error for RasterProcessError {}

impl std::fmt::Display for PngEncodingError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "PNG encoding failed: {}", self.reason)
    }
}

impl std::error::Error for PngEncodingError {}

#[cfg(test)]
mod tests {
    use super::{
        CaptureRect, CaptureRectError, CaptureTarget, OnDemandRasterProcess, PaintCommand,
        PaintScene, PreparedScreenshot, RasterImage, RasterImageError, RasterProcess,
        RasterProcessError, RasterProcessFactory, Rgba8, SoftwareRasterProcessFactory, encode_png,
    };
    use std::cell::Cell;
    use std::rc::Rc;

    struct CountingFactory {
        starts: Rc<Cell<usize>>,
        renders: Rc<Cell<usize>>,
    }

    struct CountingProcess {
        renders: Rc<Cell<usize>>,
    }

    struct FailingFactory {
        starts: Rc<Cell<usize>>,
    }

    impl RasterProcessFactory for CountingFactory {
        type Process = CountingProcess;

        fn start(&mut self) -> Result<Self::Process, RasterProcessError> {
            self.starts.set(self.starts.get() + 1);
            Ok(CountingProcess {
                renders: self.renders.clone(),
            })
        }
    }

    impl RasterProcess for CountingProcess {
        fn render(
            &mut self,
            _screenshot: &PreparedScreenshot,
        ) -> Result<RasterImage, RasterProcessError> {
            self.renders.set(self.renders.get() + 1);
            RasterImage::new(1, 1, vec![0, 0, 0, 0]).map_err(|error| RasterProcessError::Protocol {
                reason: error.to_string(),
            })
        }
    }

    impl RasterProcessFactory for FailingFactory {
        type Process = CountingProcess;

        fn start(&mut self) -> Result<Self::Process, RasterProcessError> {
            self.starts.set(self.starts.get() + 1);
            Err(RasterProcessError::Start {
                reason: "fixture unavailable".into(),
            })
        }
    }

    fn screenshot() -> PreparedScreenshot {
        let capture_bounds = CaptureRect::new(0, 0, 1, 1).unwrap();
        PreparedScreenshot {
            target: CaptureTarget::Viewport,
            scene: PaintScene {
                capture_bounds,
                commands: vec![],
            },
        }
    }

    #[test]
    fn raster_process_starts_only_for_the_first_render() {
        let starts = Rc::new(Cell::new(0));
        let renders = Rc::new(Cell::new(0));
        let factory = CountingFactory {
            starts: starts.clone(),
            renders: renders.clone(),
        };
        let mut raster = OnDemandRasterProcess::new(factory);

        assert!(!raster.is_started());
        assert_eq!(starts.get(), 0);

        raster.render(&screenshot()).unwrap();
        raster.render(&screenshot()).unwrap();

        assert!(raster.is_started());
        assert_eq!(starts.get(), 1);
        assert_eq!(renders.get(), 2);
    }

    #[test]
    fn capture_rect_rejects_empty_and_overflowing_regions() {
        assert_eq!(
            CaptureRect::new(0, 0, 0, 1),
            Err(CaptureRectError::EmptyWidth)
        );
        assert_eq!(
            CaptureRect::new(0, 0, 1, 0),
            Err(CaptureRectError::EmptyHeight)
        );
        assert_eq!(
            CaptureRect::new(i64::MAX, 0, 1, 1),
            Err(CaptureRectError::HorizontalOverflow)
        );
    }

    #[test]
    fn failed_start_does_not_install_a_process() {
        let starts = Rc::new(Cell::new(0));
        let mut raster = OnDemandRasterProcess::new(FailingFactory {
            starts: starts.clone(),
        });

        let first = raster.render(&screenshot());
        let second = raster.render(&screenshot());

        assert!(matches!(first, Err(RasterProcessError::Start { .. })));
        assert!(matches!(second, Err(RasterProcessError::Start { .. })));
        assert!(!raster.is_started());
        assert_eq!(starts.get(), 2);
    }

    #[test]
    fn raster_image_requires_one_rgba_value_per_channel() {
        assert_eq!(
            RasterImage::new(2, 1, vec![0; 7]),
            Err(RasterImageError::WrongByteLength {
                expected: 8,
                actual: 7,
            })
        );
        let image = RasterImage::new(2, 1, vec![0; 8]).unwrap();
        assert_eq!(image.width(), 2);
        assert_eq!(image.height(), 1);
        assert_eq!(image.rgba().len(), 8);
    }

    #[test]
    fn software_rasterizer_composites_and_encodes_rgba_pixels() {
        let capture = CaptureRect::new(0, 0, 2, 1).unwrap();
        let left = CaptureRect::new(0, 0, 1, 1).unwrap();
        let screenshot = PreparedScreenshot {
            target: CaptureTarget::Viewport,
            scene: PaintScene {
                capture_bounds: capture,
                commands: vec![
                    PaintCommand::FillRect {
                        source: "canvas".into(),
                        bounds: capture,
                        color: Rgba8 {
                            red: 0,
                            green: 0,
                            blue: 255,
                            alpha: 255,
                        },
                    },
                    PaintCommand::FillRect {
                        source: "overlay".into(),
                        bounds: left,
                        color: Rgba8 {
                            red: 255,
                            green: 0,
                            blue: 0,
                            alpha: 128,
                        },
                    },
                ],
            },
        };
        let mut raster = OnDemandRasterProcess::new(SoftwareRasterProcessFactory);

        let image = raster.render(&screenshot).unwrap();
        let png = encode_png(&image).unwrap();

        assert_eq!(image.rgba(), &[128, 0, 127, 255, 0, 0, 255, 255]);
        assert_eq!(&png[..8], b"\x89PNG\r\n\x1a\n");
    }

    #[test]
    fn software_rasterizer_rejects_oversized_captures_before_allocation() {
        let capture = CaptureRect::new(0, 0, 4097, 4097).unwrap();
        let screenshot = PreparedScreenshot {
            target: CaptureTarget::FullPage,
            scene: PaintScene {
                capture_bounds: capture,
                commands: vec![],
            },
        };
        let mut raster = OnDemandRasterProcess::new(SoftwareRasterProcessFactory);

        let result = raster.render(&screenshot);

        assert!(matches!(
            result,
            Err(RasterProcessError::Render { reason }) if reason.contains("the limit is")
        ));
    }

    #[test]
    fn software_rasterizer_rejects_excessive_overdraw_before_allocation() {
        let capture = CaptureRect::new(0, 0, 4096, 4096).unwrap();
        let screenshot = PreparedScreenshot {
            target: CaptureTarget::FullPage,
            scene: PaintScene {
                capture_bounds: capture,
                commands: (0..5)
                    .map(|index| PaintCommand::FillRect {
                        source: format!("layer-{index}"),
                        bounds: capture,
                        color: Rgba8 {
                            red: 0,
                            green: 0,
                            blue: 0,
                            alpha: 255,
                        },
                    })
                    .collect(),
            },
        };
        let mut raster = OnDemandRasterProcess::new(SoftwareRasterProcessFactory);

        let result = raster.render(&screenshot);

        assert!(matches!(
            result,
            Err(RasterProcessError::Render { reason }) if reason.contains("clipped pixels")
        ));
    }
}
