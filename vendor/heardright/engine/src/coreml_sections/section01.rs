// Shared native CoreML.framework bridge (Apple Silicon / ANE).
//
// Generic `MLModel`/`MLMultiArray` plumbing reused by every on-device CoreML
// engine — Parakeet TDT (`coreml_asr.rs`) and WhisperKit (`whisper_coreml.rs`).
// Pure Rust on `objc2-core-ml`; no Swift, no subprocess.
//
// THE load-bearing subtlety: CoreML prediction outputs are NOT guaranteed
// C-contiguous — they can carry arbitrary element strides. `read_f16` gathers
// through `MLMultiArray.strides()`; reading `dataPointer` linearly would
// scramble the layout (and min/max/mean wouldn't reveal it). See
// docs/ARCHITECTURE.md.
use std::collections::hash_map::DefaultHasher;
use std::ffi::c_void;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::time::Instant;

use half::f16;
use objc2::rc::{autoreleasepool, Retained};
use objc2::runtime::ProtocolObject;
use objc2::AnyThread;
use objc2_core_ml::{
    MLComputeUnits, MLDictionaryFeatureProvider, MLFeatureProvider, MLFeatureValue, MLModel,
    MLModelConfiguration, MLMultiArray, MLMultiArrayDataType,
};
use objc2_foundation::{NSArray, NSDictionary, NSNumber, NSString, NSURL};

unsafe extern "C" {
    fn heardright_coreml_prediction(
        model: *mut c_void,
        provider: *mut c_void,
        error_out: *mut *mut c_void,
        exception_description_out: *mut *mut c_void,
    ) -> *mut c_void;
    fn heardright_coreml_exception_smoke() -> *mut c_void;
}

/// Process-wide lease for main Core ML ASR predictions.
///
/// CPU Sherpa KWS owns a separate worker and does not take this lease.
pub(crate) struct CoreMlInferenceLease {
    _lease: crate::inference_gate::InferenceLease,
}

pub(crate) fn inference_lease(owner: &'static str) -> CoreMlInferenceLease {
    let wait_started = Instant::now();
    let lease = crate::inference_gate::lease(owner);
    let wait_ms = wait_started.elapsed().as_millis() as u64;
    if wait_ms > 0 {
        tracing::info!(
            target: "coreml_inference",
            owner,
            wait_ms,
            "coreml_inference_waited"
        );
    }
    CoreMlInferenceLease { _lease: lease }
}

/// Diagnostic used by the release stress harness to prove an Objective-C
/// exception cannot cross into Rust even with `panic = "abort"`.
pub fn exception_bridge_smoke() -> Result<String, String> {
    let description = unsafe { heardright_coreml_exception_smoke() };
    let description = unsafe { Retained::<NSString>::from_raw(description.cast()) }
        .ok_or_else(|| "Objective-C exception smoke returned no description".to_string())?;
    Ok(description.to_string())
}

/// One loaded CoreML model stage on the ANE+CPU.
pub struct Stage {
    model: Retained<MLModel>,
    compute_profile: &'static str,
    compile_cache: &'static str,
}

// SAFETY: Apple documents `MLModel` as thread-safe ("You can safely call this
// method [prediction] from multiple threads" / MLModel instances may be shared
// across threads). `objc2` just doesn't encode that. `Stage` holds nothing
// else. Needed so the VAD's background loader thread can hand the loaded
// model to the consumer thread (vad.rs `start_loading`).
unsafe impl Send for Stage {}

impl Stage {
    /// Load a stage. `.mlmodelc` loads directly; `.mlpackage` / `.mlmodel`
    /// are compiled once and cached as `.mlmodelc`. Packaged macOS apps run
    /// from read-only locations such as a mounted DMG, so the shell should set
    /// `HR_COREML_COMPILED_CACHE_DIR` to a writable app-data cache.
    pub fn load(path: &Path) -> Result<Self, String> {
        Self::load_inner(path, None)
    }

    /// Load a stage using a persisted developer compute profile. The
    /// process-level HR_COREML_CU override remains highest priority.
    pub fn load_with_compute_profile(path: &Path, profile: &str) -> Result<Self, String> {
        Self::load_inner(path, Some(profile))
    }

    /// Load a stage pinned to CPU-only compute. For tiny per-frame models
    /// (e.g. the Silero VAD, one 32 ms frame per call) where ANE/GPU dispatch
    /// overhead would exceed the compute itself. `HR_COREML_CU` still wins.
    pub fn load_cpu_only(path: &Path) -> Result<Self, String> {
        Self::load_inner(path, Some("cpu_only"))
    }

    fn load_inner(path: &Path, configured_profile: Option<&str>) -> Result<Self, String> {
        autoreleasepool(|_| unsafe {
            let config = MLModelConfiguration::new();
            // Compute units overridable via HR_COREML_CU for variable isolation:
            // `cpuOnly` skips the ANE compile entirely (loads in seconds — use it to
            // debug decode correctness without the ~10-min cold ANE compile),
            // `all` = CPU+GPU+ANE, default `cpuAndNeuralEngine`.
            #[cfg(not(target_arch = "x86_64"))]
            let fallback_cu = MLComputeUnits::CPUAndNeuralEngine;
            #[cfg(target_arch = "x86_64")]
            let fallback_cu = MLComputeUnits::CPUAndGPU;
            let (cu, compute_profile) = match std::env::var("HR_COREML_CU").as_deref() {
                Ok("cpuOnly") => (MLComputeUnits::CPUOnly, "env_cpu_only"),
                Ok("all") => (MLComputeUnits::All, "env_all"),
                Ok("cpuAndGPU") => (MLComputeUnits::CPUAndGPU, "env_cpu_gpu"),
                Ok("cpuAndNeuralEngine") => {
                    (MLComputeUnits::CPUAndNeuralEngine, "env_neural_engine")
                }
                _ => match configured_profile {
                    Some("neural_engine") => (MLComputeUnits::CPUAndNeuralEngine, "neural_engine"),
                    Some("cpu_gpu") => (MLComputeUnits::CPUAndGPU, "cpu_gpu"),
                    Some("cpu_only") => (MLComputeUnits::CPUOnly, "cpu_only"),
                    _ => (fallback_cu, "automatic"),
                },
            };
            config.setComputeUnits(cu);
            let already_compiled = path.extension().and_then(|e| e.to_str()) == Some("mlmodelc");
            let (load_path, compile_cache) = if already_compiled {
                (path.to_path_buf(), "precompiled_bundle")
            } else {
                let cache_path = compiled_model_cache_path(path);
                let cache_hit = cache_path.join("model.mil").exists()
                    || cache_path.join("coremldata.bin").exists();
                (
                    persistent_compiled_model(path)?,
                    if cache_hit {
                        "cache_hit"
                    } else {
                        "fresh_compile"
                    },
                )
            };
            let p = NSString::from_str(&load_path.to_string_lossy());
            let url = NSURL::fileURLWithPath(&p);
            let model = MLModel::modelWithContentsOfURL_configuration_error(&url, &config)
                .map_err(|e| format!("load {}: {}", load_path.display(), nserr(&e)))?;
            Ok(Self {
                model,
                compute_profile,
                compile_cache,
            })
        })
    }

    pub fn compute_profile(&self) -> &'static str {
        self.compute_profile
    }

    pub fn compile_cache(&self) -> &'static str {
        self.compile_cache
    }

    /// Output feature names, in dictionary order.
    pub fn output_names(&self) -> Vec<String> {
        unsafe {
            let desc = self.model.modelDescription();
            let by_name = desc.outputDescriptionsByName();
            by_name.allKeys().iter().map(|k| k.to_string()).collect()
        }
    }

    /// Input feature names, in dictionary order.
    pub fn input_names(&self) -> Vec<String> {
        unsafe {
            let desc = self.model.modelDescription();
            let by_name = desc.inputDescriptionsByName();
            by_name.allKeys().iter().map(|k| k.to_string()).collect()
        }
    }

    /// Human-readable dump of every input/output feature + its multiarray shape.
    /// Used to reverse the IO contract of a new model (e.g. WhisperKit stages).
    pub fn describe(&self) -> String {
        unsafe {
            let desc = self.model.modelDescription();
            let mut s = String::new();
            let dump = |label: &str,
                        by: &objc2_foundation::NSDictionary<
                NSString,
                objc2_core_ml::MLFeatureDescription,
            >,
                        s: &mut String| {
                for key in by.allKeys().iter() {
                    if let Some(fd) = by.objectForKey(&key) {
                        let shape = fd.multiArrayConstraint().map(|c| {
                            c.shape()
                                .iter()
                                .map(|n| n.integerValue())
                                .collect::<Vec<_>>()
                        });
                        s.push_str(&format!("  {label}: {key} {shape:?}\n"));
                    }
                }
            };
            dump("in ", &desc.inputDescriptionsByName(), &mut s);
            dump("out", &desc.outputDescriptionsByName(), &mut s);
            s
        }
    }

    /// The single output whose multiarray shape equals `want`.
    pub fn output_by_shape(&self, want: &[usize]) -> Result<String, String> {
        self.outputs_by_shape(want)
            .into_iter()
            .next()
            .ok_or_else(|| format!("no output with shape {want:?}"))
    }

    /// All outputs whose shape equals `want`, sorted by name (autogen `var_NN`
    /// names sort into spec/declaration order).
    pub fn outputs_by_shape(&self, want: &[usize]) -> Vec<String> {
        unsafe {
            let desc = self.model.modelDescription();
            let by_name = desc.outputDescriptionsByName();
            let mut hits: Vec<String> = Vec::new();
            for key in by_name.allKeys().iter() {
                if let Some(fd) = by_name.objectForKey(&key) {
                    if let Some(c) = fd.multiArrayConstraint() {
                        let shape: Vec<usize> = c
                            .shape()
                            .iter()
                            .map(|n| n.integerValue() as usize)
                            .collect();
                        if shape == want {
                            hits.push(key.to_string());
                        }
                    }
                }
            }
            hits.sort();
            hits
        }
    }

    /// Run a prediction, return one named output's MLMultiArray.
    pub fn predict(
        &self,
        inputs: &[(&str, &Retained<MLMultiArray>)],
        out_name: &str,
    ) -> Result<Retained<MLMultiArray>, String> {
        Ok(self
            .predict_multi(inputs, &[out_name])?
            .into_iter()
            .next()
            .unwrap())
    }

    /// Run a prediction, return several named outputs (in `out_names` order).
    pub fn predict_multi(
        &self,
        inputs: &[(&str, &Retained<MLMultiArray>)],
        out_names: &[&str],
    ) -> Result<Vec<Retained<MLMultiArray>>, String> {
        unsafe {
            let keys: Vec<Retained<NSString>> =
                inputs.iter().map(|(k, _)| NSString::from_str(k)).collect();
            let vals: Vec<Retained<MLFeatureValue>> = inputs
                .iter()
                .map(|(_, v)| MLFeatureValue::featureValueWithMultiArray(v))
                .collect();
            let key_refs: Vec<&NSString> = keys.iter().map(|k| k.as_ref()).collect();
            let val_refs: Vec<&MLFeatureValue> = vals.iter().map(|v| v.as_ref()).collect();
            let dict: Retained<NSDictionary<NSString, MLFeatureValue>> =
                NSDictionary::from_slices(&key_refs, &val_refs);
            let provider = MLDictionaryFeatureProvider::initWithDictionary_error(
                MLDictionaryFeatureProvider::alloc(),
                // SAFETY: NSDictionary<NSString, MLFeatureValue> is layout-identical to
                // NSDictionary<NSString, AnyObject> (MLFeatureValue: NSObject).
                std::mem::transmute::<
                    &NSDictionary<NSString, MLFeatureValue>,
                    &NSDictionary<NSString, objc2::runtime::AnyObject>,
                >(&dict),
            )
            .map_err(|e| format!("feature provider: {}", nserr(&e)))?;

            let provider_obj: &ProtocolObject<dyn MLFeatureProvider> =
                ProtocolObject::from_ref(&*provider);
            // `NSError` does not cover Objective-C exceptions. Keep the message
            // send wholly inside native `@try/@catch`; allowing NSException to
            // unwind through Rust aborts release builds (`panic = "abort"`).
            let mut error = std::ptr::null_mut();
            let mut exception_description = std::ptr::null_mut();
            let result = heardright_coreml_prediction(
                Retained::as_ptr(&self.model).cast_mut().cast(),
                provider_obj.as_ref() as *const objc2::runtime::AnyObject as *mut c_void,
                &mut error,
                &mut exception_description,
            );
            if !exception_description.is_null() {
                let description = Retained::<NSString>::from_raw(exception_description.cast())
                    .expect("bridge returned a non-null exception description");
                return Err(format!("prediction exception: {description}"));
            }
            if !error.is_null() {
                let error = Retained::<objc2_foundation::NSError>::from_raw(error.cast())
                    .expect("bridge returned a non-null NSError");
                return Err(format!("prediction: {}", nserr(&error)));
            }
            let result = Retained::<objc2::runtime::AnyObject>::from_raw(result.cast())
                .ok_or_else(|| "prediction returned no result or error".to_string())?;
            // SAFETY: the Objective-C bridge's result is declared
            // `id<MLFeatureProvider>` and only returns that value on success.
            let result: Retained<ProtocolObject<dyn MLFeatureProvider>> =
                Retained::cast_unchecked(result);

            let mut outs = Vec::with_capacity(out_names.len());
            for name in out_names {
                let ns = NSString::from_str(name);
                let fv = result
                    .featureValueForName(&ns)
                    .ok_or_else(|| format!("output `{name}` missing"))?;
                let arr = fv
                    .multiArrayValue()
                    .ok_or_else(|| format!("output `{name}` is not a multiarray"))?;
                outs.push(arr);
            }
            Ok(outs)
        }
    }
}

pub fn nserr(e: &objc2_foundation::NSError) -> String {
    e.localizedDescription().to_string()
}

fn persistent_compiled_model(path: &Path) -> Result<PathBuf, String> {
    let compiled_path = compiled_model_cache_path(path);
    if compiled_path.join("model.mil").exists() || compiled_path.join("coremldata.bin").exists() {
        return Ok(compiled_path);
    }

    tracing::info!(
        "coreml_compile_start {} -> {}",
        path.display(),
        compiled_path.display()
    );
    let compiled_tmp = autoreleasepool(|_| unsafe {
        let p = NSString::from_str(&path.to_string_lossy());
        let url = NSURL::fileURLWithPath(&p);
        MLModel::compileModelAtURL_error(&url)
            .map_err(|e| format!("compile {}: {}", path.display(), nserr(&e)))
            .and_then(|url| {
                let compiled = url.path().ok_or_else(|| {
                    format!("compiled model URL has no path for {}", path.display())
                })?;
                Ok(PathBuf::from(compiled.to_string()))
            })
    })?;

    let tmp_destination = compiled_path.with_extension("mlmodelc.tmp");
    if tmp_destination.exists() {
        std::fs::remove_dir_all(&tmp_destination)
            .map_err(|e| format!("remove stale {}: {e}", tmp_destination.display()))?;
    }
    copy_dir_recursive(&compiled_tmp, &tmp_destination)?;
    if compiled_path.exists() {
        std::fs::remove_dir_all(&compiled_path)
            .map_err(|e| format!("replace {}: {e}", compiled_path.display()))?;
    }
    std::fs::rename(&tmp_destination, &compiled_path).map_err(|e| {
        format!(
            "install compiled model {} -> {}: {e}",
            tmp_destination.display(),
            compiled_path.display()
        )
    })?;
    tracing::info!("coreml_compile_done {}", compiled_path.display());
    Ok(compiled_path)
}

fn compiled_model_cache_path(path: &Path) -> PathBuf {
    if let Some(cache_dir) = std::env::var_os("HR_COREML_COMPILED_CACHE_DIR") {
        let mut hasher = DefaultHasher::new();
        path.to_string_lossy().hash(&mut hasher);
        let hash = hasher.finish();
        let stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("model")
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                    c
                } else {
                    '-'
                }
            })
            .collect::<String>();
        PathBuf::from(cache_dir).join(format!("{stem}-{hash:016x}.mlmodelc"))
    } else {
        path.with_extension("mlmodelc")
    }
}

fn copy_dir_recursive(from: &Path, to: &Path) -> Result<(), String> {
    if let Some(parent) = to.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("create {}: {e}", parent.display()))?;
    }
    std::fs::create_dir_all(to).map_err(|e| format!("create {}: {e}", to.display()))?;
    for entry in std::fs::read_dir(from).map_err(|e| format!("read {}: {e}", from.display()))? {
        let entry = entry.map_err(|e| format!("read entry in {}: {e}", from.display()))?;
        let source = entry.path();
        let destination = to.join(entry.file_name());
        let file_type = entry
            .file_type()
            .map_err(|e| format!("stat {}: {e}", source.display()))?;
        if file_type.is_dir() {
            copy_dir_recursive(&source, &destination)?;
        } else if file_type.is_file() {
            std::fs::copy(&source, &destination).map_err(|e| {
                format!(
                    "copy {} -> {}: {e}",
                    source.display(),
                    destination.display()
                )
            })?;
        }
    }
    Ok(())
}

/// Build an f16 MLMultiArray from f32 data (row-major, C-contiguous).
pub fn ml_f16(dims: &[usize], data: &[f32]) -> Result<Retained<MLMultiArray>, String> {
    unsafe {
        let arr = new_marray(dims, MLMultiArrayDataType::Float16)?;
        let ptr = arr.dataPointer().as_ptr() as *mut f16;
        for (i, &x) in data.iter().enumerate() {
            *ptr.add(i) = f16::from_f32(x);
        }
        Ok(arr)
    }
}
