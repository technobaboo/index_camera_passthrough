use nalgebra::{matrix, Affine3, Matrix4, TCategory};
use serde::{Deserialize, Serialize};

/// Because your eye and the camera is at different physical locations, it is impossible
/// to project camera view into VR space perfectly. There are trade offs approximating
/// this projection. (viewing range means things too close to you will give you double vision).
#[derive(Eq, PartialEq, Debug, Serialize, Deserialize, Clone, Copy, PartialOrd, Ord)]
pub enum ProjectionMode {
    /// in this mode, we assume your eyes are at the cameras' physical location. this mode
    /// has larger viewing range, but everything will smaller to you.
    FromCamera,
    /// in this mode, we assume your cameras are at your eyes' physical location. everything will
    /// have the right scale in this mode, but the viewing range is smaller.
    FromEye,
}

impl Default for ProjectionMode {
    fn default() -> Self {
        Self::FromCamera
    }
}
pub const fn default_overlay_distance() -> f32 {
    1.0
}
fn serialize_transform<S>(transform: &Affine3<f32>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    let m = transform.matrix();
    m.data.0.serialize(serializer)
}
fn deserialize_transform<'de, D>(deserializer: D) -> Result<Affine3<f32>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let m = <[[f32; 4]; 4]>::deserialize(deserializer)?;
    let m = nalgebra::Matrix4::from(m);
    if !nalgebra::geometry::TAffine::check_homogeneous_invariants(&m) {
        return Err(serde::de::Error::custom("transform not affine"));
    }
    Ok(Affine3::from_matrix_unchecked(m))
}
#[derive(PartialEq, Debug, Serialize, Deserialize, Clone, Copy)]
#[serde(tag = "mode")]
pub enum PositionMode {
    /// the overlay is shown right in front of your HMD
    Hmd {
        /// how far away should the overlay be
        #[serde(default = "default_overlay_distance")]
        distance: f32,
    },
    /// the overlay will stick to a fixed position in world space, but it can be repositioned
    /// by pressing the repositioning button
    Sticky {
        /// how far away from your face should the overlay be, when you reposition the overlay.
        #[serde(default = "default_overlay_distance")]
        distance: f32,

        /// internal use, the position to stick to
        #[serde(skip)]
        transform: Affine3<f32>,
    },
    /// the overlay is at a fixed location in space
    Absolute {
        /// transformation matrix for the overlay
        #[serde(
            serialize_with = "serialize_transform",
            deserialize_with = "deserialize_transform"
        )]
        transform: Affine3<f32>,
    },
}

impl PositionMode {
    pub fn transform(&self, hmd_transform: Matrix4<f32>) -> Affine3<f32> {
        match self {
            &PositionMode::Hmd { distance } => {
                let transform = hmd_transform
                    * matrix![
                        1.0, 0.0, 0.0, 0.0;
                        0.0, 1.0, 0.0, 0.0;
                        0.0, 0.0, 1.0, -distance;
                        0.0, 0.0, 0.0, 1.0;
                    ];
                Affine3::from_matrix_unchecked(transform)
            }
            &PositionMode::Absolute { transform, .. } | &PositionMode::Sticky { transform, .. } => {
                transform
            }
        }
    }
    pub fn reposition(&mut self, hmd_transform: Matrix4<f32>) {
        if let PositionMode::Sticky {
            transform,
            distance,
        } = self
        {
            let new_transform = hmd_transform
                * matrix![
                    1.0, 0.0, 0.0, 0.0;
                    0.0, 1.0, 0.0, 0.0;
                    0.0, 0.0, 1.0, -*distance;
                    0.0, 0.0, 0.0, 1.0;
                ];
            *transform = Affine3::from_matrix_unchecked(new_transform);
        }
    }
}

impl Default for PositionMode {
    fn default() -> Self {
        Self::Hmd { distance: 1.0 }
    }
}

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq, Clone, Copy, PartialOrd, Ord)]
pub enum Eye {
    Left,
    Right,
}

pub const fn default_display_eye() -> Eye {
    Eye::Left
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
#[serde(tag = "mode")]
pub enum DisplayMode {
    #[default]
    Direct,
    /// display a stereo image on the overlay. conceptually the overlay becomes a portal from VR
    /// space to real world. you will be able to see more of the real world if the overlay occupys
    /// more of your field of view.
    Stereo {
        /// how is the camera's image projected onto the overlay
        #[serde(default)]
        projection_mode: ProjectionMode,
    },
    /// display one of the camera's image on the overlay
    Flat {
        /// which camera's image to display
        #[serde(default = "default_display_eye")]
        eye: Eye,
    },
}

impl DisplayMode {
    pub(crate) fn projection_mode(&self) -> Option<ProjectionMode> {
        match self {
            DisplayMode::Stereo { projection_mode } => Some(*projection_mode),
            _ => None,
        }
    }
    pub(crate) fn is_stereo(&self) -> bool {
        matches!(self, DisplayMode::Stereo { .. } | DisplayMode::Direct)
    }
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct OverlayConfig {
    /// how is the overlay positioned
    #[serde(default)]
    pub position: PositionMode,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub enum Button {
    Menu,
    Grip,
    Trigger,
    A,
    B,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub enum Backend {
    #[cfg(feature = "openvr")]
    #[serde(alias = "steamvr", alias = "openvr")]
    OpenVR,
    #[cfg(feature = "openxr")]
    #[serde(alias = "openxr")]
    OpenXR,
}
impl Default for Backend {
    fn default() -> Self {
        #[cfg(feature = "openvr")]
        {
            Self::OpenVR
        }
        #[cfg(feature = "openxr")]
        #[cfg(not(feature = "openvr"))]
        {
            Self::OpenXR
        }
    }
}

#[cfg(feature = "openvr")]
impl From<Button> for openvr_sys2::EVRButtonId {
    fn from(b: Button) -> Self {
        use openvr_sys2::EVRButtonId;
        match b {
            Button::Menu => EVRButtonId::k_EButton_ApplicationMenu,
            Button::Grip => EVRButtonId::k_EButton_Grip,
            Button::Trigger => EVRButtonId::k_EButton_Axis1,
            Button::A => EVRButtonId::k_EButton_Grip,
            Button::B => EVRButtonId::k_EButton_ApplicationMenu,
        }
    }
}

#[cfg(feature = "openvr")]
impl PartialEq<openvr_sys2::EVRButtonId> for Button {
    fn eq(&self, other: &openvr_sys2::EVRButtonId) -> bool {
        &openvr_sys2::EVRButtonId::from(*self) == other
    }
}

#[cfg(feature = "openvr")]
impl From<openvr_sys2::EVRButtonId> for Button {
    fn from(value: openvr_sys2::EVRButtonId) -> Self {
        use openvr_sys2::EVRButtonId;
        match value {
            EVRButtonId::k_EButton_ApplicationMenu => Button::Menu,
            EVRButtonId::k_EButton_Grip => Button::Grip,
            EVRButtonId::k_EButton_Axis1 => Button::Trigger,
            _ => panic!("unknown button id"),
        }
    }
}

pub const fn default_toggle_button() -> Button {
    Button::Menu
}

pub const fn default_open_delay() -> std::time::Duration {
    std::time::Duration::ZERO
}

pub const fn default_z_order() -> u32 {
    u32::MAX
}

/// Index camera passthrough
#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    /// VR backend to use
    pub backend: Backend,
    /// camera device to use. auto detect if not set
    #[serde(default)]
    pub camera_device: String,
    /// overlay related configuration
    #[serde(default)]
    pub overlay: OverlayConfig,
    /// how is the camera view displayed on the overlay
    #[serde(default)]
    pub display_mode: DisplayMode,
    /// which button should toggle the overlay visibility. press things
    /// button on both controllers to toggle the overlay.
    #[serde(default = "default_toggle_button")]
    pub toggle_button: Button,
    /// how long does the button need to be held before the overlay open,
    /// closing the overlay is always instantaneous
    #[serde(default = "default_open_delay", with = "humantime_serde")]
    pub open_delay: std::time::Duration,
    /// z order of the overlay. higher z order means the overlay is on top of
    /// other overlays. Not supported on all backends, supported on OpenXR.
    #[serde(default = "default_z_order")]
    pub z_order: u32,
    /// enable debug option, including:
    ///   - use trigger button to do renderdoc capture
    #[serde(default)]
    pub debug: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            camera_device: "".to_owned(),
            #[cfg(feature = "openvr")]
            backend: Backend::OpenVR,
            #[cfg(feature = "openxr")]
            #[cfg(not(feature = "openvr"))]
            backend: Backend::OpenXR,
            overlay: Default::default(),
            display_mode: Default::default(),
            toggle_button: default_toggle_button(),
            open_delay: std::time::Duration::ZERO,
            debug: false,
            z_order: default_z_order(),
        }
    }
}

use anyhow::Result;
use xdg::BaseDirectories;
pub fn load_config(xdg: &BaseDirectories) -> Result<Config> {
    if let Some(f) = xdg.find_config_file("index_camera_passthrough.toml") {
        let cfg = std::fs::read_to_string(f)?;
        Ok(toml::from_str(&cfg)?)
    } else {
        Ok(Default::default())
    }
}
