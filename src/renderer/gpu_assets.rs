use crate::assets::{AssetHandle, AssetId, Assets, Geom, Material, Texture};
use crate::gpu::GPU;
use crate::renderer::gpu_geom::GPUGeom;
use crate::renderer::gpu_pipeline::GPUPipeline;
use crate::renderer::gpu_texture::GPUTexture;
use crate::renderer::ForwardRenderer;
use ash::vk;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

pub struct GPUAssets {
    gpu: Rc<GPU>,
    assets: Rc<RefCell<Assets>>,

    pipeline_pool: RefCell<HashMap<AssetId, HashMap<vk::RenderPass, GPUPipeline>>>,
    geom_pool: RefCell<HashMap<AssetId, GPUGeom>>,
    texture_pool: RefCell<HashMap<AssetId, GPUTexture>>,
}

impl GPUAssets {
    pub fn new(gpu: Rc<GPU>, assets: Rc<RefCell<Assets>>) -> Self {
        GPUAssets {
            gpu,
            assets,
            pipeline_pool: RefCell::new(HashMap::new()),
            geom_pool: RefCell::new(HashMap::new()),
            texture_pool: RefCell::new(HashMap::new()),
        }
    }

    pub fn get_texture(&self, handle: AssetHandle<Texture>) -> Option<GPUTexture> {
        let mut texture_pool = self.texture_pool.borrow_mut();
        match texture_pool.get(&handle.id) {
            None => {
                let assets = self.assets.borrow();
                let texture = assets.load(&handle)?;
                let tex_gpu = GPUTexture::new(&self.gpu, &texture);

                texture_pool.insert(handle.id, tex_gpu)
            }
            Some(tex) => Some(tex.to_owned()),
        }
    }

    pub fn get_pipeline(
        &self,
        handle: &AssetHandle<Material>,
        renderer: &ForwardRenderer,
    ) -> Option<GPUPipeline> {
        let mut pipeline_pool = self.pipeline_pool.borrow_mut();
        let pipelines = pipeline_pool.entry(handle.id).or_insert(HashMap::new());

        match pipelines.get(&renderer.render_pass) {
            None => {
                let assets = self.assets.borrow();
                let material = assets.load(&handle)?;
                let pipeline_gpu = GPUPipeline::new(&self.gpu, &material, renderer);
                pipelines.insert(renderer.render_pass, pipeline_gpu)
            }
            Some(pipeline) => Some(pipeline.to_owned()),
        }
    }

    pub fn get_material(
        &self,
        handle: &AssetHandle<Material>,
        renderer: &ForwardRenderer,
    ) -> Option<(GPUPipeline, HashMap<&str, Option<GPUTexture>>)> {
        let mut pipeline_pool = self.pipeline_pool.borrow_mut();
        let pipelines = pipeline_pool.entry(handle.id).or_insert(HashMap::new());

        let assets = self.assets.borrow();
        let material = assets.load(&handle)?;

        let pipeline = match pipelines.get(&renderer.render_pass) {
            None => {
                let pipeline = GPUPipeline::new(&self.gpu, &material, renderer);
                pipelines.insert(renderer.render_pass, pipeline)?
            }
            Some(pipeline) => pipeline.to_owned(),
        };

        let mut properties = HashMap::new();
        if let Some(value) = material.get_texture("texture") {
            properties.insert("texture", self.get_texture(value));
        }

        Some((pipeline, properties))
    }

    pub fn get_geom(&mut self, handle: &AssetHandle<Geom>) -> Option<GPUGeom> {
        let mut geom_pool = self.geom_pool.borrow_mut();
        match geom_pool.get(&handle.id) {
            None => {
                let assets = self.assets.borrow();
                let geom = assets.load(&handle)?;
                let geom_gpu = GPUGeom::new(&self.gpu, geom);

                geom_pool.insert(handle.id, geom_gpu)
            }
            Some(geom) => Some(geom.to_owned()),
        }
    }
}

impl Drop for GPUAssets {
    fn drop(&mut self) {
        self.pipeline_pool
            .borrow_mut()
            .values_mut()
            .for_each(|map| {
                map.values_mut()
                    .for_each(|pipeline| pipeline.drop(&self.gpu))
            });

        self.geom_pool
            .borrow_mut()
            .values_mut()
            .for_each(|geom| geom.drop(&self.gpu));

        self.texture_pool
            .borrow_mut()
            .values_mut()
            .for_each(|tex| tex.drop(&self.gpu));
    }
}
