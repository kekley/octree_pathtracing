use std::collections::HashMap;

use smol_str::SmolStr;

use crate::{RTWImage, Texture};

pub struct TextureManager {
    loaded_textures: HashMap<SmolStr, Texture>,
}
