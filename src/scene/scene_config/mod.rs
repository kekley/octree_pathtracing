pub mod emitters;
pub mod sun;

use crate::scene::scene_config::{emitters::EmittersConfig, sun::SunConfig};
pub struct SceneConfig {
    pub sun: SunConfig,
    pub emitters: EmittersConfig,
}
