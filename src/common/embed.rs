use anyhow::Result;
use model2vec_rs::model::StaticModel;
use pgvector::Vector;

pub struct Embedder(StaticModel);

impl Embedder {
    pub fn load() -> Result<Embedder> {
        let model = StaticModel::from_bytes(
            include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/embed/tokenizer.json")),
            include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/embed/model.safetensors")),
            include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/embed/config.json")),
            None,
        )?;
        Ok(Embedder(model))
    }

    pub fn embed(&self, text: &str) -> Vector {
        Vector::from(self.0.encode_single(text))
    }

    pub fn embed_all(&self, texts: &[String]) -> Vec<Vector> {
        self.0.encode(texts).into_iter().map(Vector::from).collect()
    }
}
