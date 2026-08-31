use std::num::{NonZeroU32, NonZeroU64};

use crate::Locator;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CaptureTarget {
    Viewport,
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

pub struct OnDemandRasterProcess<F>
where
    F: RasterProcessFactory,
{
    factory: F,
    process: Option<F::Process>,
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

#[cfg(test)]
mod tests {
    use super::{
        CaptureRect, CaptureRectError, CaptureTarget, OnDemandRasterProcess, PaintScene,
        PreparedScreenshot, RasterImage, RasterImageError, RasterProcess, RasterProcessError,
        RasterProcessFactory,
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
}
