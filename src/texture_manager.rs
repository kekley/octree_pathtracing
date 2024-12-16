use std::{collections::HashMap, fmt::Write, u16};

use smol_str::{SmolStr, SmolStrBuilder};

use crate::{Material, RTWImage, Texture};

const ASSET_PATH: &str = "./assets/default_resource_pack/assets/minecraft/textures/block";
#[derive(Debug, Clone)]
pub struct TextureManager {
    loaded_textures: Vec<Texture>,
    created_materials: Vec<Material>,
    num_materials: u32,
    resource_map: HashMap<SmolStr, u32>,
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
    pub fn get_or_make_material_idx(&mut self, resource_name: &str) -> Result<u32, String> {
        let mut replacement_string = resource_name.to_owned();
        if replacement_string.contains("leaves")
            || replacement_string.contains("vine")
            || replacement_string.contains("fence")
            || replacement_string.contains("lectern")
            || replacement_string.contains("trapdoor")
        {
            return Err("removed".to_string());
        }
        if replacement_string.contains("brick") {
            replacement_string = replacement_string.replace("_stairs", "");
        }
        replacement_string = replacement_string.replace("stairs", "planks");
        replacement_string = replacement_string.replace("slab", "planks");

        if self.resource_map.contains_key(replacement_string.as_str()) {
            return Ok((*self.resource_map.get(replacement_string.as_str()).unwrap()).into());
        } else {
            let image = RTWImage::load(
                &(ASSET_PATH.to_owned()
                    + "/"
                    + &replacement_string.trim_start_matches("minecraft:")
                    + ".png"),
            )?;

            let texture = Texture::Image(image);

            self.loaded_textures.push(texture);

            let material = Material::Lambertian {
                texture: self.loaded_textures[self.num_materials as usize].clone(),
            };

            self.created_materials.push(material);

            self.resource_map
                .insert(replacement_string.clone().into(), self.num_materials);

            self.num_materials += 1;
            return Ok(*self
                .resource_map
                .get(&SmolStr::from(replacement_string))
                .unwrap());
        }
    }

    pub fn get_material(&self, idx: u32) -> &Material {
        &self.created_materials[idx as usize]
    }
}
