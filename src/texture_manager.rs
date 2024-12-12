use std::{collections::HashMap, u16};

use smol_str::SmolStr;

use crate::{Material, RTWImage, Texture};

const ASSET_PATH: &str = "./assets/default_resource_pack/assets/minecraft/textures/block";
#[derive(Debug, Clone)]
pub struct TextureManager {
    loaded_textures: Vec<Texture>,
    created_materials: Vec<Material>,
    num_materials: u16,
    resource_map: HashMap<SmolStr, u16>,
}

impl TextureManager {
    pub fn new() -> Self {
        Self {
            loaded_textures: vec![],
            created_materials: vec![],
            num_materials: 0,
            resource_map: HashMap::default(),
        }
    }
    pub fn get_or_make_material_idx(&mut self, resource_name: &str) -> Result<u16, ()> {
        if self.resource_map.contains_key(resource_name) {
            return Ok(*self.resource_map.get(resource_name).unwrap());
        } else {
            let image = RTWImage::load(
                &(ASSET_PATH.to_owned()
                    + "/"
                    + resource_name.trim_start_matches("minecraft:")
                    + ".png"),
            )?;

            let texture = Texture::Image(image);

            self.loaded_textures.push(texture);

            let material = Material::Lambertian {
                texture: self.loaded_textures[self.num_materials as usize].clone(),
            };

            self.created_materials.push(material);

            self.resource_map
                .insert(resource_name.into(), self.num_materials);

            self.num_materials += 1;
            return Ok(*self.resource_map.get(resource_name).unwrap());
        }
    }

    pub fn get_material(&self, idx: u16) -> &Material {
        &self.created_materials[idx as usize]
    }
}
