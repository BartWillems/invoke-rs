use std::cell::RefCell;
use std::num::NonZeroU8;

use once_cell::sync::Lazy;
use rand::rngs::ThreadRng;
use rand::Rng;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

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
                        main_model_loader: MainModelLoader {
                            typ: "main_model_loader",
                            id: "main_model_loader",
                            is_intermediate: true,
                            model: Model {
                                model_name: ModelName::EpicRealism,
                                base_model: BaseModel::Sd1,
                                model_type: ModelType::Main,
                            },
                        },
                        clip_skip: ClipSkip {
                            typ: "clip_skip",
                            id: "clip_skip",
                            skipped_layers: 0,
                            is_intermediate: true,
                        },
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
                        denoise_latents: DenoiseLatents {
                            typ: "denoise_latents",
                            id: "denoise_latents",
                            is_intermediate: true,
                            cfg_scale: 7.5,
                            scheduler: "euler",
                            steps: NonZeroU8::try_from(50).unwrap(),
                            denoising_start: 0,
                            denoising_end: 1,
                        },
                        latents_to_image: LatentsToImage {
                            typ: "l2i",
                            id: "latents_to_image",
                            is_intermediate: true,
                            fp32: true,
                        },
                        metadata_accumulator: MetadataAccumulator {
                            typ: "metadata_accumulator",
                            id: "metadata_accumulator",
                            generation_mode: "txt2img",
                            cfg_scale: 7.5,
                            width: 512,
                            height: 512,
                            positive_prompt: input,
                            negative_prompt: "bad anatomy, low quality, lowres".into(),
                            model: Model {
                                model_name: ModelName::EpicRealism,
                                base_model: BaseModel::Sd1,
                                model_type: ModelType::Main,
                            },
                            steps: NonZeroU8::try_from(50).unwrap(),
                            rand_device: "cpu",
                            scheduler: "euler",
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
                        save_image: SaveImage {
                            typ: "save_image",
                            id: "save_image",
                            is_intermediate: false,
                            use_cache: false,
                        },
                        lora_loader_epic_real_life: LoraLoader {
                            id: "lora_loader_epiCRealLife",
                            typ: "lora_loader",
                            is_intermediate: true,
                            lora: Lora {
                                base_model: BaseModel::Sd1,
                                model_name: LoraModelName::EpicRealLife,
                            },
                            weight: 0.75,
                        },
                        lora_loader_gigachad: None,
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
                        node_path: NodePath::MetadataAccumulator,
                        field_name: "seed".into(),
                        items: vec![data_seed],
                    },
                ]],
            },
        }
    }

    pub fn drawing(mut self) -> Self {
        let model = ModelName::ChildrensStoriesV1SemiReal;
        self.batch.graph.nodes.main_model_loader.model.model_name = model;
        self.batch.graph.nodes.metadata_accumulator.model.model_name = model;
        self
    }

    pub fn gigachad(mut self) -> Self {
        self.batch.graph.nodes.main_model_loader.model.model_name = ModelName::AZovyaPhotorealV2;

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
            .metadata_accumulator
            .loras
            .push(MetadataLora { lora, weight: 1.0 });

        self.batch.graph.edges = (*Lazy::force(&GIGACHAD_EDGES)).clone();

        self
    }

    pub fn anime(mut self) -> Self {
        self.batch.graph.nodes.main_model_loader.model.model_name = ModelName::CounterfeitV30;
        self.batch.graph.nodes.metadata_accumulator.model.model_name = ModelName::CounterfeitV30;
        self.batch.graph.edges = (*Lazy::force(&ANIME_EDGES)).clone();

        // Less steps are needed for drawings
        self.batch.graph.nodes.denoise_latents.steps = 25.try_into().unwrap();
        self.batch.graph.nodes.metadata_accumulator.steps = 25.try_into().unwrap();

        // 720p resolution
        self.batch.graph.nodes.noise.width = 1280;
        self.batch.graph.nodes.noise.height = 720;

        // Use different sampler because idk
        self.batch.graph.nodes.denoise_latents.scheduler = "dpmpp_2m_k";
        self.batch.graph.nodes.metadata_accumulator.scheduler = "dpmpp_2m_k";
        self.batch.graph.nodes.denoise_latents.cfg_scale = 10.0;
        self.batch.graph.nodes.metadata_accumulator.cfg_scale = 10.0;

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
}

#[derive(Debug, Serialize)]
struct Nodes {
    main_model_loader: MainModelLoader,
    clip_skip: ClipSkip,
    positive_conditioning: PositiveConditioning,
    negative_conditioning: NegativeConditioning,
    noise: Noise,
    denoise_latents: DenoiseLatents,
    latents_to_image: LatentsToImage,
    metadata_accumulator: MetadataAccumulator,
    #[serde(rename = "lora_loader_epiCRealLife")]
    lora_loader_epic_real_life: LoraLoader,
    #[serde(
        rename = "lora_loader_Gigachadv1",
        skip_serializing_if = "Option::is_none"
    )]
    lora_loader_gigachad: Option<LoraLoader>,
    save_image: SaveImage,
}

#[derive(Debug, Serialize)]
struct MainModelLoader {
    #[serde(rename = "type")]
    typ: &'static str,
    id: &'static str,
    is_intermediate: bool,
    model: Model,
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
}

#[derive(Clone, Copy, Debug, Serialize)]
enum LoraModelName {
    #[serde(rename = "epiCRealLife")]
    EpicRealLife,
    #[serde(rename = "Gigachadv1")]
    GigaChad,
}

#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "kebab-case")]
enum BaseModel {
    #[serde(rename = "sd-1")]
    Sd1,
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
struct MetadataAccumulator {
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
struct SaveImage {
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
    ClipSkip,
    PositiveConditioning,
    NegativeConditioning,
    DenoiseLatents,
    Noise,
    MetadataAccumulator,
    LatentsToImage,
    SaveImage,
    #[serde(rename = "lora_loader_Gigachadv1")]
    GigaChad,
    #[serde(rename = "lora_loader_epiCRealLife")]
    EpicRealLife,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum EdgeField {
    Unet,
    Clip,
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
    MetadataAccumulator,
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

#[allow(unused)]
#[derive(Debug, Deserialize)]
pub struct InvocationComplete {
    queue_id: String,
    queue_item_id: usize,
    queue_batch_id: BatchId,
    node: InvocationNode,
    result: InvocationResult,
}

impl InvocationComplete {
    pub fn id(&self) -> BatchId {
        self.queue_batch_id
    }

    pub fn still_in_progress(&self) -> bool {
        self.node.is_intermediate
    }

    pub fn image_path(&self) -> Option<String> {
        self.result
            .image
            .as_ref()
            .map(|image| image.image_name.clone())
    }
}

#[derive(Debug, Deserialize)]
struct InvocationNode {
    is_intermediate: bool,
}

#[derive(Debug, Deserialize)]
struct InvocationResult {
    image: Option<Image>,
}

#[derive(Debug, Deserialize)]
struct Image {
    image_name: String,
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
}
