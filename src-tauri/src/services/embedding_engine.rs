// Embedding engine: ONNX (multilingual-e5-small) + tokenizers, mean pooling, L2 norm

use anyhow::Result;
use ndarray::Array2;
use ort::session::{builder::GraphOptimizationLevel, Session};
use ort::value::Tensor;
use std::path::PathBuf;
use std::sync::OnceLock;
use tokenizers::{PaddingParams, PaddingStrategy, Tokenizer, TruncationParams, TruncationStrategy};

const EMBEDDING_DIM: usize = 384;
const MAX_LENGTH: usize = 512;

static ENGINE: OnceLock<EmbeddingEngine> = OnceLock::new();

pub struct EmbeddingEngine {
    session: std::sync::Mutex<Session>,
    tokenizer: Tokenizer,
}

impl EmbeddingEngine {
    /// Инициализация движка. Вызывать один раз при старте приложения.
    /// Проверяет сначала appDataDir/models (обновляемая), потом bundled resource (из инсталлятора).
    pub fn init(app_data_model_dir: PathBuf, bundled_model_dir: PathBuf) -> Result<(), String> {
        let model_dir = if app_data_model_dir.join("model_int8.onnx").exists() {
            log::info!("Using model from appDataDir: {}", app_data_model_dir.display());
            app_data_model_dir
        } else if bundled_model_dir.join("model_int8.onnx").exists() {
            log::info!("Using bundled model from: {}", bundled_model_dir.display());
            bundled_model_dir
        } else {
            return Err("Embedding model not found".to_string());
        };
        let engine = Self::new(model_dir).map_err(|e| e.to_string())?;
        ENGINE
            .set(engine)
            .map_err(|_| "EmbeddingEngine already initialized".to_string())
    }

    /// Инициализация с одним путём (для dev fallback)
    pub fn init_single(model_dir: PathBuf) -> Result<(), String> {
        if !model_dir.join("model_int8.onnx").exists() {
            return Err(format!("Model not found in {}", model_dir.display()));
        }
        log::info!("Using model from: {}", model_dir.display());
        let engine = Self::new(model_dir).map_err(|e| e.to_string())?;
        ENGINE
            .set(engine)
            .map_err(|_| "EmbeddingEngine already initialized".to_string())
    }

    pub fn get() -> Option<&'static EmbeddingEngine> {
        ENGINE.get()
    }

    fn new(model_dir: PathBuf) -> Result<Self> {
        let tokenizer_path = model_dir.join("tokenizer.json");
        let mut tokenizer = Tokenizer::from_file(tokenizer_path).map_err(|e| anyhow::anyhow!("Tokenizer: {}", e))?;

        tokenizer.with_truncation(Some(TruncationParams {
            max_length: MAX_LENGTH,
            strategy: TruncationStrategy::LongestFirst,
            ..Default::default()
        }));
        tokenizer.with_padding(Some(PaddingParams {
            strategy: PaddingStrategy::BatchLongest,
            ..Default::default()
        }));

        let model_path = model_dir.join("model_int8.onnx");
        let session = Session::builder()?
            .with_optimization_level(GraphOptimizationLevel::Level3)?
            .with_intra_threads(4)?
            .commit_from_file(model_path)?;

        Ok(Self {
            session: std::sync::Mutex::new(session),
            tokenizer,
        })
    }

    /// Генерирует эмбеддинги для батча текстов. Тексты должны уже содержать префикс "query: " или "passage: ".
    pub fn embed(&self, texts: &[String]) -> Result<Vec<Vec<f32>>> {
        if texts.is_empty() {
            return Ok(Vec::new());
        }

        let encodings = self
            .tokenizer
            .encode_batch(texts.iter().map(String::as_str).collect::<Vec<_>>(), true)
            .map_err(|e| anyhow::anyhow!("Encode: {}", e))?;

        let seq_len = encodings
            .iter()
            .map(|e| e.get_ids().len())
            .max()
            .unwrap_or(0);
        let batch_size = encodings.len();

        let mut input_ids = Array2::<i64>::zeros((batch_size, seq_len));
        let mut attention_mask = Array2::<i64>::zeros((batch_size, seq_len));
        let mut token_type_ids = Array2::<i64>::zeros((batch_size, seq_len));

        for (i, enc) in encodings.iter().enumerate() {
            let ids = enc.get_ids();
            let type_ids = enc.get_type_ids();
            for j in 0..seq_len.min(ids.len()) {
                input_ids[[i, j]] = ids[j] as i64;
                attention_mask[[i, j]] =
                    *enc.get_attention_mask().get(j).unwrap_or(&1) as i64;
                token_type_ids[[i, j]] = *type_ids.get(j).unwrap_or(&0) as i64;
            }
        }

        let input_ids_value = Tensor::from_array(input_ids.clone())?;
        let attention_mask_value = Tensor::from_array(attention_mask.clone())?;
        let token_type_ids_value = Tensor::from_array(token_type_ids.clone())?;

        let inputs = ort::inputs![
            "input_ids" => input_ids_value,
            "attention_mask" => attention_mask_value,
            "token_type_ids" => token_type_ids_value,
        ];
        let mut session = self
            .session
            .lock()
            .map_err(|e| anyhow::anyhow!("Session lock: {}", e))?;
        let outputs = session.run(inputs)?;

        let last_hidden = outputs
            .get("last_hidden_state")
            .ok_or_else(|| anyhow::anyhow!("No output from ONNX"))?;
        let (shape, hidden) = last_hidden.try_extract_tensor::<f32>()?;
        let dims = shape.as_ref();
        if dims.len() != 3 || dims[2] as usize != EMBEDDING_DIM {
            anyhow::bail!("Unexpected output shape: {:?}", dims);
        }
        let batch = dims[0] as usize;
        let seq = dims[1] as usize;
        let dim = dims[2] as usize;

        let mut result = Vec::with_capacity(batch);
        for i in 0..batch {
            let mut sum = vec![0.0f32; EMBEDDING_DIM];
            let mut count: f32 = 0.0;
            for j in 0..seq {
                let mask_val = attention_mask[[i, j]];
                if mask_val == 0 {
                    continue;
                }
                for d in 0..dim {
                    sum[d] += hidden[i * seq * dim + j * dim + d];
                }
                count += 1.0;
            }
            if count > 0.0 {
                for x in &mut sum {
                    *x /= count;
                }
            }
            let norm: f32 = sum.iter().map(|x| x * x).sum::<f32>().sqrt();
            if norm > 1e-9 {
                for x in &mut sum {
                    *x /= norm;
                }
            }
            result.push(sum);
        }
        Ok(result)
    }
}
