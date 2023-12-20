use std::cell::RefCell;
use std::num::NonZeroU8;

use once_cell::sync::Lazy;
use rand::rngs::ThreadRng;
use rand::Rng;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub(crate) mod invocations;

thread_local! {
    static RNG: RefCell<ThreadRng> = RefCell::new(rand::thread_rng());
}

static GIGACHAD_EDGES: Lazy<Vec<Edge>> = Lazy::new(|| {
    let gigachad = include_str!("edges/gigachad.json");

    serde_json::from_str::<Vec<Edge>>(gigachad).unwrap()
});

static ANIME_EDGES: Lazy<Vec<Edge>> = Lazy::new(|| {
    let gigachad = include_str!("edges/anime.json");

    serde_json::from_str::<Vec<Edge>>(gigachad).unwrap()
});

static DEFAULT_EDGES: Lazy<Vec<Edge>> = Lazy::new(|| {
    let gigachad = include_str!("edges/default.json");

    serde_json::from_str::<Vec<Edge>>(gigachad).unwrap()
});

static LEGO_EDGES: Lazy<Vec<Edge>> = Lazy::new(|| {
    let lego = include_str!("edges/lego.json");

    serde_json::from_str::<Vec<Edge>>(lego).unwrap()
});

static KNITTED_EDGES: Lazy<Vec<Edge>> = Lazy::new(|| {
    let lego = include_str!("edges/knit.json");

    serde_json::from_str::<Vec<Edge>>(lego).unwrap()
});

/// Identifier used to link requests to completed images
#[derive(Clone, Copy, Debug, Deserialize, Hash, Eq, PartialEq)]
pub struct BatchId(Uuid);

#[derive(Debug, Serialize)]
pub struct Enqueue {
    prepend: bool,
    batch: Batch,
}

impl From<String> for Enqueue {
    fn from(input: String) -> Self {
        Self::from_prompt(input)
    }
}

impl Enqueue {
    pub fn from_prompt(input: impl Into<String>) -> Self {
        let input = input.into();

        let (data_seed, noise_seed) = RNG.with(|rng| {
            let mut rng = rng.borrow_mut();
            (rng.gen::<usize>(), rng.gen::<usize>())
        });

        Self {
            prepend: false,
            batch: Batch {
                graph: Graph {
                    id: GraphId::TextToImageGraph,
                    nodes: Nodes {
                        model_loader: ModelLoaderVariants::default(),
                        clip_skip: Some(ClipSkip {
                            typ: "clip_skip",
                            id: "clip_skip",
                            skipped_layers: 0,
                            is_intermediate: true,
                        }),
                        positive_conditioning: PositiveConditioning {
                            typ: "compel",
                            id: "positive_conditioning",
                            prompt: input.clone(),
                            is_intermediate: true,
                        },
                        negative_conditioning: NegativeConditioning {
                            typ: "compel",
                            id: "negative_conditioning",
                            prompt: "bad anatomy, low quality, lowres".into(),
                            is_intermediate: true,
                        },
                        noise: Noise {
                            typ: "noise",
                            id: "noise",
                            seed: noise_seed,
                            is_intermediate: true,
                            width: 512,
                            height: 512,
                            use_cpu: true,
                        },
                        denoise_latents: DenoiseLatentsVariants::DenoiseLatents {
                            content: DenoiseLatents {
                                typ: "denoise_latents",
                                id: "denoise_latents",
                                is_intermediate: true,
                                cfg_scale: 7.5,
                                scheduler: "dpmpp_sde_k",
                                steps: NonZeroU8::try_from(30).unwrap(),
                                denoising_start: 0,
                                denoising_end: 1,
                            },
                        },
                        latents_to_image: LatentsToImage {
                            typ: "l2i",
                            id: "latents_to_image",
                            is_intermediate: true,
                            fp32: true,
                        },
                        core_metadata: CoreMetadata {
                            typ: "core_metadata",
                            id: "core_metadata",
                            generation_mode: "txt2img",
                            cfg_scale: 7.5,
                            width: 512,
                            height: 512,
                            positive_prompt: input,
                            negative_prompt: "bad anatomy, low quality, lowres".into(),
                            model: Model {
                                model_name: ModelName::AZovyaPhotorealV2,
                                base_model: BaseModel::Sd1,
                                model_type: ModelType::Main,
                            },
                            steps: NonZeroU8::try_from(30).unwrap(),
                            rand_device: "cpu",
                            scheduler: "dpmpp_sde_k",
                            controlnets: Vec::new(),
                            loras: vec![MetadataLora {
                                lora: Lora {
                                    base_model: BaseModel::Sd1,
                                    model_name: LoraModelName::EpicRealLife,
                                },
                                weight: 0.75,
                            }],
                            ip_adapters: Vec::new(),
                            clip_skip: 0,
                        },
                        linear_ui_output: LinearUiOutput {
                            typ: "linear_ui_output",
                            id: "linear_ui_output",
                            is_intermediate: false,
                            use_cache: false,
                        },
                        lora_loader_epic_real_life: Some(LoraLoader {
                            id: "lora_loader_epiCRealLife",
                            typ: "lora_loader",
                            is_intermediate: true,
                            lora: Lora {
                                base_model: BaseModel::Sd1,
                                model_name: LoraModelName::EpicRealLife,
                            },
                            weight: 0.75,
                        }),
                        lora_loader_gigachad: None,
                        lora_loader_lego: None,
                        lora_loader_knittedstyle2: None,
                    },
                    edges: (*Lazy::force(&DEFAULT_EDGES)).clone(),
                },
                runs: 1,
                data: vec![vec![
                    Data {
                        node_path: NodePath::Noise,
                        field_name: "seed".into(),
                        items: vec![data_seed],
                    },
                    Data {
                        node_path: NodePath::CoreMetadata,
                        field_name: "seed".into(),
                        items: vec![data_seed],
                    },
                ]],
            },
        }
    }

    pub fn prompt(&self) -> &str {
        &self.batch.graph.nodes.core_metadata.positive_prompt
    }

    fn set_resolution(&mut self, width: usize, height: usize) {
        self.batch.graph.nodes.noise.width = width;
        self.batch.graph.nodes.noise.height = height;
        self.batch.graph.nodes.core_metadata.width = width;
        self.batch.graph.nodes.core_metadata.height = height;
    }

    fn ensure_prompt_contains(&mut self, needs: &'static str) {
        let prompt = self.batch.graph.nodes.positive_conditioning.prompt.as_str();
        if !prompt.to_lowercase().contains(&needs.to_lowercase()) {
            self.batch.graph.nodes.positive_conditioning.prompt = format!("{needs}, {prompt}");
        }
    }

    fn set_negative_prompt<T: Into<String> + Clone>(&mut self, prompt: T) {
        self.batch.graph.nodes.negative_conditioning.prompt = prompt.clone().into();
        self.batch.graph.nodes.core_metadata.negative_prompt = prompt.into();
    }

    pub fn drawing(mut self) -> Self {
        let model = ModelName::ChildrensStoriesV1SemiReal;
        let loader = ModelLoader::sd1_with_model(model);

        self.batch.graph.nodes.model_loader = ModelLoaderVariants::from(loader);
        self.batch.graph.nodes.core_metadata.model.model_name = model;
        self
    }

    pub fn gigachad(mut self) -> Self {
        let model = ModelName::AZovyaPhotorealV2;
        let loader = ModelLoader::sd1_with_model(model);

        self.batch.graph.nodes.model_loader = ModelLoaderVariants::from(loader);

        // Make sure Gigachad is part of the promopt
        self.ensure_prompt_contains("Gigachad");

        let lora = Lora {
            base_model: BaseModel::Sd1,
            model_name: LoraModelName::GigaChad,
        };

        self.batch.graph.nodes.lora_loader_gigachad = Some(LoraLoader {
            id: "lora_loader_Gigachadv1",
            typ: "lora_loader",
            is_intermediate: true,
            lora,
            weight: 1.0,
        });

        self.batch
            .graph
            .nodes
            .core_metadata
            .loras
            .push(MetadataLora { lora, weight: 1.0 });

        self.batch.graph.edges = (*Lazy::force(&GIGACHAD_EDGES)).clone();

        self
    }

    pub fn anime(mut self) -> Self {
        let model = ModelName::CounterfeitV30;
        let loader = ModelLoader::sd1_with_model(model);

        self.batch.graph.nodes.model_loader = ModelLoaderVariants::from(loader);
        self.batch.graph.nodes.core_metadata.model.model_name = model;
        self.batch.graph.edges = (*Lazy::force(&ANIME_EDGES)).clone();

        // 720p resolution
        self.set_resolution(1280, 720);

        self
    }

    pub fn lego(mut self) -> Self {
        self.batch.graph.id = GraphId::SdxlTextToImageGraph;
        let model = ModelName::StableDiffusionXlBase1;
        let loader = ModelLoader::sdxl_with_model(model);
        self.batch.graph.nodes.model_loader = ModelLoaderVariants::from(loader);
        self.batch.graph.nodes.core_metadata.generation_mode = "sdxl_txt2img";
        self.batch.graph.nodes.core_metadata.model.model_name = model;
        self.batch.graph.nodes.core_metadata.model.base_model = BaseModel::Sdxl;

        // Higher is too slow, lower is worse generations, trained to be portrait mode
        self.set_resolution(704, 1056);

        // Make sure LEGO is part of the promopt
        self.ensure_prompt_contains("LEGO");

        self.batch.graph.nodes.positive_conditioning.typ = "sdxl_compel_prompt";
        self.batch.graph.nodes.negative_conditioning.typ = "sdxl_compel_prompt";

        self.batch.graph.nodes.denoise_latents = DenoiseLatentsVariants::SdxlDenoiseLatents {
            content: DenoiseLatents {
                typ: "denoise_latents",
                id: "sdxl_denoise_latents",
                is_intermediate: true,
                cfg_scale: 7.5,
                scheduler: "dpmpp_sde_k",
                steps: 30.try_into().unwrap(),
                denoising_start: 0,
                denoising_end: 1,
            },
        };

        // Lora
        self.batch.graph.nodes.lora_loader_lego = Some(LoraLoader {
            id: "lora_loader_lego_v2_0_XL_32",
            typ: "sdxl_lora_loader",
            is_intermediate: true,
            lora: Lora {
                base_model: BaseModel::Sdxl,
                model_name: LoraModelName::Lego,
            },
            weight: 1.0,
        });
        self.batch.graph.nodes.core_metadata.loras = vec![MetadataLora {
            lora: Lora {
                base_model: BaseModel::Sdxl,
                model_name: LoraModelName::Lego,
            },
            weight: 1.0,
        }];
        self.batch.graph.nodes.lora_loader_epic_real_life = None;
        self.batch.graph.nodes.clip_skip = None;

        // Edges
        self.batch.graph.edges = (*Lazy::force(&LEGO_EDGES)).clone();

        self
    }

    pub fn knit(mut self) -> Self {
        self.batch.graph.nodes.lora_loader_epic_real_life = None;
        self.batch.graph.nodes.lora_loader_knittedstyle2 = Some(LoraLoader {
            id: "lora_loader_knittedstyle2",
            typ: "lora_loader",
            is_intermediate: true,
            lora: Lora {
                base_model: BaseModel::Sd1,
                model_name: LoraModelName::KnittedStyle2,
            },
            weight: 1.0,
        });

        self.batch.graph.edges = (*Lazy::force(&KNITTED_EDGES)).clone();

        self.set_resolution(768, 1152);
        self.ensure_prompt_contains("knittedstyle");
        self.set_negative_prompt("");

        self
    }
}

#[derive(Debug, Serialize)]
struct Batch {
    graph: Graph,
    runs: usize,
    data: Vec<Vec<Data>>,
}

#[derive(Debug, Serialize)]
struct Graph {
    id: GraphId,
    nodes: Nodes,
    edges: Vec<Edge>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
enum GraphId {
    TextToImageGraph,
    SdxlTextToImageGraph,
}

#[derive(Debug, Serialize)]
struct Nodes {
    #[serde(flatten)]
    model_loader: ModelLoaderVariants,
    #[serde(skip_serializing_if = "Option::is_none")]
    clip_skip: Option<ClipSkip>,
    positive_conditioning: PositiveConditioning,
    negative_conditioning: NegativeConditioning,
    noise: Noise,
    #[serde(flatten)]
    denoise_latents: DenoiseLatentsVariants,
    latents_to_image: LatentsToImage,
    core_metadata: CoreMetadata,
    #[serde(
        rename = "lora_loader_epiCRealLife",
        skip_serializing_if = "Option::is_none"
    )]
    lora_loader_epic_real_life: Option<LoraLoader>,
    #[serde(
        rename = "lora_loader_Gigachadv1",
        skip_serializing_if = "Option::is_none"
    )]
    lora_loader_gigachad: Option<LoraLoader>,
    #[serde(
        rename = "lora_loader_lego_v2_0_XL_32",
        skip_serializing_if = "Option::is_none"
    )]
    lora_loader_lego: Option<LoraLoader>,
    #[serde(
        rename = "lora_loader_knittedstyle2",
        skip_serializing_if = "Option::is_none"
    )]
    lora_loader_knittedstyle2: Option<LoraLoader>,
    linear_ui_output: LinearUiOutput,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
enum ModelLoaderVariants {
    MainModelLoader {
        #[serde(flatten)]
        loader: ModelLoader,
    },
    SdxlModelLoader {
        #[serde(flatten)]
        loader: ModelLoader,
    },
}

impl Default for ModelLoaderVariants {
    fn default() -> Self {
        Self::MainModelLoader {
            loader: ModelLoader::default(),
        }
    }
}

impl From<ModelLoader> for ModelLoaderVariants {
    fn from(loader: ModelLoader) -> Self {
        match loader.model.base_model {
            BaseModel::Sd1 => ModelLoaderVariants::MainModelLoader { loader },
            BaseModel::Sdxl => ModelLoaderVariants::SdxlModelLoader { loader },
        }
    }
}

#[derive(Debug, Serialize)]
struct ModelLoader {
    #[serde(rename = "type")]
    typ: &'static str,
    id: &'static str,
    is_intermediate: bool,
    model: Model,
}

impl ModelLoader {
    fn sd1_with_model(model: ModelName) -> Self {
        Self {
            model: Model {
                model_name: model,
                model_type: ModelType::Main,
                base_model: BaseModel::Sd1,
            },
            ..Default::default()
        }
    }

    fn sdxl_with_model(model: ModelName) -> Self {
        Self {
            typ: "sdxl_model_loader",
            id: "sdxl_model_loader",
            is_intermediate: true,
            model: Model {
                model_name: model,
                base_model: BaseModel::Sdxl,
                model_type: ModelType::Main,
            },
        }
    }
}

impl Default for ModelLoader {
    fn default() -> Self {
        Self {
            typ: "main_model_loader",
            id: "main_model_loader",
            is_intermediate: true,
            model: Model {
                model_name: ModelName::AZovyaPhotorealV2,
                base_model: BaseModel::Sd1,
                model_type: ModelType::Main,
            },
        }
    }
}

#[derive(Debug, Serialize)]
struct Model {
    model_name: ModelName,
    base_model: BaseModel,
    model_type: ModelType,
}

#[allow(unused)]
#[derive(Clone, Copy, Debug, Serialize)]
enum ModelName {
    #[serde(rename = "a-zovya-photoreal-v2")]
    AZovyaPhotorealV2,

    /// Realistic anime-esque drawings
    #[serde(rename = "childrens-stories-v1-semi-real")]
    ChildrensStoriesV1SemiReal,

    /// Foto realistic portraits etc
    #[serde(rename = "epicphotogasm_v1")]
    EpicPhotogasmV1,

    /// Anime thingies
    #[serde(rename = "CounterfeitV30_v30")]
    CounterfeitV30,

    #[serde(rename = "epicrealism_naturalSinRC1VAE")]
    EpicRealism,

    /// Sdxl Model
    #[serde(rename = "stable-diffusion-xl-base-1-0")]
    StableDiffusionXlBase1,
}

#[derive(Clone, Copy, Debug, Serialize)]
enum LoraModelName {
    #[serde(rename = "epiCRealLife")]
    EpicRealLife,
    #[serde(rename = "Gigachadv1")]
    GigaChad,
    /// Sdxl, requires "LEGO" in prompt
    #[serde(rename = "lego_v2.0_XL_32")]
    Lego,
    /// Required "knittedstyle" in prompt
    #[serde(rename = "knittedstyle2")]
    KnittedStyle2,
}

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
enum BaseModel {
    #[serde(rename = "sd-1")]
    Sd1,
    #[serde(rename = "sdxl")]
    Sdxl,
}

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
enum ModelType {
    Main,
}

#[derive(Debug, Serialize)]
struct ClipSkip {
    #[serde(rename = "type")]
    typ: &'static str,
    id: &'static str,
    skipped_layers: usize,
    is_intermediate: bool,
}

#[derive(Debug, Serialize)]
struct PositiveConditioning {
    #[serde(rename = "type")]
    typ: &'static str,
    id: &'static str,
    prompt: String,
    is_intermediate: bool,
}

#[derive(Debug, Serialize)]
struct NegativeConditioning {
    #[serde(rename = "type")]
    typ: &'static str,
    id: &'static str,
    prompt: String,
    is_intermediate: bool,
}

#[derive(Debug, Serialize)]
struct Noise {
    #[serde(rename = "type")]
    typ: &'static str,
    id: &'static str,
    seed: usize,
    width: usize,
    height: usize,
    use_cpu: bool,
    is_intermediate: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
enum DenoiseLatentsVariants {
    DenoiseLatents {
        #[serde(flatten)]
        content: DenoiseLatents,
    },
    SdxlDenoiseLatents {
        #[serde(flatten)]
        content: DenoiseLatents,
    },
}

#[derive(Debug, Serialize)]
struct DenoiseLatents {
    #[serde(rename = "type")]
    typ: &'static str,
    id: &'static str,
    is_intermediate: bool,
    cfg_scale: f32,
    scheduler: &'static str,
    steps: NonZeroU8,
    denoising_start: usize,
    denoising_end: usize,
}

#[derive(Debug, Serialize)]
struct LatentsToImage {
    #[serde(rename = "type")]
    typ: &'static str,
    id: &'static str,
    fp32: bool,
    is_intermediate: bool,
}

#[derive(Debug, Serialize)]
struct CoreMetadata {
    id: &'static str,
    #[serde(rename = "type")]
    typ: &'static str,
    generation_mode: &'static str,
    cfg_scale: f32,
    height: usize,
    width: usize,
    positive_prompt: String,
    negative_prompt: String,
    model: Model,
    steps: NonZeroU8,
    rand_device: &'static str,
    scheduler: &'static str,
    controlnets: Vec<()>,
    loras: Vec<MetadataLora>,
    #[serde(rename = "ipAdapters")]
    ip_adapters: Vec<()>,
    clip_skip: usize,
}

#[derive(Debug, Serialize)]
struct LinearUiOutput {
    id: &'static str,
    #[serde(rename = "type")]
    typ: &'static str,
    is_intermediate: bool,
    use_cache: bool,
}

#[derive(Debug, Serialize)]
struct LoraLoader {
    id: &'static str,
    #[serde(rename = "type")]
    typ: &'static str,
    is_intermediate: bool,
    lora: Lora,
    weight: f32,
}

#[derive(Clone, Copy, Debug, Serialize)]
struct Lora {
    base_model: BaseModel,
    model_name: LoraModelName,
}

#[derive(Clone, Copy, Debug, Serialize)]
struct MetadataLora {
    lora: Lora,
    weight: f32,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
struct Edge {
    source: EdgeNode,
    destination: EdgeNode,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
struct EdgeNode {
    node_id: EdgeNodeId,
    field: EdgeField,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum EdgeNodeId {
    MainModelLoader,
    SdxlModelLoader,
    ClipSkip,
    PositiveConditioning,
    NegativeConditioning,
    DenoiseLatents,
    SdxlDenoiseLatents,
    Noise,
    CoreMetadata,
    LatentsToImage,
    SaveImage,
    LinearUiOutput,
    #[serde(rename = "lora_loader_Gigachadv1")]
    GigaChad,
    #[serde(rename = "lora_loader_epiCRealLife")]
    EpicRealLife,
    #[serde(rename = "lora_loader_lego_v2_0_XL_32")]
    Lego,
    #[serde(rename = "lora_loader_knittedstyle2")]
    Knit,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum EdgeField {
    Unet,
    Clip,
    Clip2,
    Noise,
    Conditioning,
    PositiveConditioning,
    NegativeConditioning,
    Latents,
    Metadata,
    Vae,
    Image,
}

#[derive(Debug, Serialize)]
struct Data {
    node_path: NodePath,
    field_name: String,
    items: Vec<usize>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
enum NodePath {
    Noise,
    CoreMetadata,
}

#[allow(unused)]
#[derive(Debug, Deserialize)]
pub struct EnqueueResult {
    queue_id: String,
    batch: BatchResult,
}

impl EnqueueResult {
    pub fn id(&self) -> BatchId {
        self.batch.batch_id
    }
}

#[allow(unused)]
#[derive(Debug, Deserialize)]
struct BatchResult {
    batch_id: BatchId,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_conversion() {
        let default = Enqueue::from_prompt("random prompt");
        assert!(serde_json::to_value(&default).is_ok());

        let drawing = Enqueue::from_prompt("random prompt").drawing();
        assert!(serde_json::to_value(&drawing).is_ok());

        let gigachad = Enqueue::from_prompt("random prompt").gigachad();
        assert!(serde_json::to_value(&gigachad).is_ok());

        let anime = Enqueue::from_prompt("random prompt").anime();
        assert!(serde_json::to_value(&anime).is_ok());
    }

    #[allow(unused)]
    #[test]
    fn print_to_file() {
        use std::fs::File;
        use std::io::prelude::*;

        let lego = Enqueue::from_prompt("random prompt").lego();
        let json = serde_json::to_string_pretty(&lego).unwrap();

        let mut file = File::create("_output.json").unwrap();
        file.write_all(json.as_bytes()).unwrap();
    }
}
