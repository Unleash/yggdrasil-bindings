use flatbuffers::{FlatBufferBuilder, Follow, WIPOffset};
use std::collections::BTreeMap;
use std::{
    cell::RefCell,
    fmt::{Display, Formatter},
};
use unleash_types::client_metrics::MetricBucket;
use unleash_yggdrasil::{EvalWarning, ExtendedVariantDef, ToggleDefinition};

use crate::flat::messaging::messaging::{
    BuiltInStrategies, BuiltInStrategiesBuilder, CoreVersion, CoreVersionBuilder,
    FeatureDefBuilder, FeatureDefs, FeatureDefsBuilder, MetricsResponse, MetricsResponseBuilder,
    Response, ResponseBuilder, StrategyDefinition, StrategyDefinitionArgs, StrategyFeature,
    StrategyFeatureArgs, StrategyParameter, StrategyParameterArgs, TakeStateResponse,
    TakeStateResponseArgs, TakeStateResponseBuilder, ToggleEntryBuilder, ToggleStatsBuilder,
    Variant, VariantBuilder, VariantEntryBuilder, VariantPayloadBuilder,
};

thread_local! {
    static BUILDER: RefCell<FlatBufferBuilder<'static>> =
        RefCell::new(FlatBufferBuilder::with_capacity(128));
}

#[derive(Debug)]
pub enum FlatError {
    InvalidContext(String),
    InvalidState(String),
    InvalidPointer,
    MissingFlagName,
    Panic,
    NullError,
}

pub struct ResponseMessage<T> {
    pub message: Option<T>,
    pub impression_data: bool,
}

pub struct TakeStateResult {
    pub warnings: Vec<EvalWarning>,
    pub error: Option<String>,
    pub feature_strategies_map: BTreeMap<String, BTreeMap<String, BTreeMap<String, String>>>,
}

#[repr(C)]
pub struct Buf {
    pub ptr: *mut u8, // points to heap memory owned by Rust
    pub len: usize,   // length in bytes
    pub cap: usize,
}

impl Display for FlatError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            FlatError::InvalidContext(msg) => write!(f, "Invalid context: {}", msg),
            FlatError::InvalidState(msg) => write!(f, "Invalid state: {}", msg),
            FlatError::InvalidPointer => write!(f, "Invalid pointer encountered"),
            FlatError::MissingFlagName => {
                write!(f, "Flag name was missing when building extended context")
            }
            FlatError::Panic => write!(f, "Engine panicked while processing the request. Please report this as a bug with the accompanying stack trace if available."),
            FlatError::NullError => write!(f, "Null error detected, this is a serious issue and you should report this as a bug.")
        }
    }
}

pub trait FlatMessage<TInput>: Follow<'static> + Sized {
    fn as_flat_buffer(builder: &mut FlatBufferBuilder<'static>, input: TInput) -> WIPOffset<Self>;

    fn build_response(input: TInput) -> Buf {
        let bytes: Vec<u8> = BUILDER.with(|cell| {
            let mut builder = cell.borrow_mut();
            builder.reset();
            let off = Self::as_flat_buffer(&mut builder, input);
            builder.finish(off, None);
            builder.finished_data().to_vec()
        });
        let mut v = bytes;
        let buf = Buf {
            ptr: v.as_mut_ptr(),
            len: v.len(),
            cap: v.capacity(),
        };
        std::mem::forget(v);
        buf
    }
}

impl FlatMessage<Result<Option<ResponseMessage<bool>>, FlatError>> for Response<'static> {
    fn as_flat_buffer(
        builder: &mut FlatBufferBuilder<'static>,
        from: Result<Option<ResponseMessage<bool>>, FlatError>,
    ) -> WIPOffset<Response<'static>> {
        match from {
            Ok(Some(response)) => match response.message {
                Some(flag) => {
                    let mut response_builder = ResponseBuilder::new(builder);
                    response_builder.add_impression_data(response.impression_data);
                    response_builder.add_enabled(flag);
                    response_builder.add_has_enabled(true);
                    response_builder.finish()
                }
                None => {
                    let mut response_builder = ResponseBuilder::new(builder);
                    response_builder.add_has_enabled(false);
                    response_builder.finish()
                }
            },
            Ok(None) => {
                let mut response_builder = ResponseBuilder::new(builder);
                response_builder.add_has_enabled(false);
                response_builder.finish()
            }
            Err(err) => {
                let error_offset = builder.create_string(&err.to_string());
                let mut response_builder = ResponseBuilder::new(builder);
                response_builder.add_has_enabled(false);
                response_builder.add_error(error_offset);
                response_builder.finish()
            }
        }
    }
}

impl FlatMessage<Result<Option<TakeStateResult>, FlatError>> for TakeStateResponse<'static> {
    fn as_flat_buffer(
        builder: &mut FlatBufferBuilder<'static>,
        from: Result<Option<TakeStateResult>, FlatError>,
    ) -> WIPOffset<Self> {
        let mut features_vec = None;
        let mut warnings_vec = None;
        let mut error_str = None;

        match from {
            Ok(Some(res)) => {
                if !res.warnings.is_empty() {
                    let warning_strings: Vec<_> = res
                        .warnings
                        .iter()
                        .map(|w| {
                            builder.create_string(&format!(
                                "toggle: {} warned {}",
                                w.toggle_name, w.message
                            ))
                        })
                        .collect();
                    warnings_vec = Some(builder.create_vector(&warning_strings));
                }

                // error: string (optional)
                if let Some(err) = res.error {
                    error_str = Some(builder.create_string(&err));
                }
                // features: [StrategyFeature]
                if !res.feature_strategies_map.is_empty() {
                    let features: Vec<WIPOffset<StrategyFeature>> = res
                        .feature_strategies_map
                        .into_iter()
                        .map(|(feature_name, strategy_map)| {
                            // Build [StrategyDefinition] for this feature
                            let defs: Vec<WIPOffset<StrategyDefinition>> = strategy_map
                                .into_iter()
                                .map(|(strategy_name, params)| {
                                    // Build [StrategyParameter] for this definition
                                    let params_vec: Vec<WIPOffset<StrategyParameter>> = params
                                        .into_iter()
                                        .map(|(k, v)| {
                                            let key = builder.create_string(&k);
                                            let val = builder.create_string(&v);
                                            StrategyParameter::create(
                                                builder,
                                                &StrategyParameterArgs {
                                                    key: Some(key),
                                                    value: Some(val),
                                                },
                                            )
                                        })
                                        .collect();

                                    let params_off = builder.create_vector(&params_vec);
                                    let name_off = builder.create_string(&strategy_name);

                                    StrategyDefinition::create(
                                        builder,
                                        &StrategyDefinitionArgs {
                                            name: Some(name_off),
                                            parameters: Some(params_off),
                                        },
                                    )
                                })
                                .collect();

                            let defs_off = builder.create_vector(&defs);
                            let fname_off = builder.create_string(&feature_name);

                            StrategyFeature::create(
                                builder,
                                &StrategyFeatureArgs {
                                    feature_name: Some(fname_off),
                                    strategies: Some(defs_off),
                                },
                            )
                        })
                        .collect();

                    features_vec = Some(builder.create_vector(&features));
                }
                TakeStateResponse::create(
                    builder,
                    &TakeStateResponseArgs {
                        features: features_vec,
                        warnings: warnings_vec,
                        error: error_str,
                    },
                )
            }
            Ok(None) => TakeStateResponse::create(
                builder,
                &TakeStateResponseArgs {
                    features: features_vec,
                    warnings: warnings_vec,
                    error: error_str,
                },
            ),
            Err(e) => {
                let err = builder.create_string(&e.to_string());
                TakeStateResponse::create(
                    builder,
                    &TakeStateResponseArgs {
                        features: None,
                        warnings: None,
                        error: Some(err),
                    },
                )
            }
        }
    }
}

impl FlatMessage<Result<Option<ResponseMessage<ExtendedVariantDef>>, FlatError>>
    for Variant<'static>
{
    fn as_flat_buffer(
        builder: &mut FlatBufferBuilder<'static>,
        from: Result<Option<ResponseMessage<ExtendedVariantDef>>, FlatError>,
    ) -> WIPOffset<Self> {
        match from {
            Ok(Some(response)) => match response.message {
                Some(variant) => {
                    let payload_offset = variant.payload.as_ref().map(|payload| {
                        let payload_type_offset = builder.create_string(&payload.payload_type);
                        let value_offset = builder.create_string(&payload.value);

                        let mut variant_payload = VariantPayloadBuilder::new(builder);
                        variant_payload.add_payload_type(payload_type_offset);
                        variant_payload.add_value(value_offset);

                        variant_payload.finish()
                    });

                    let variant_name_offset = builder.create_string(&variant.name);

                    let mut variant_builder = VariantBuilder::new(builder);
                    variant_builder.add_feature_enabled(variant.feature_enabled);
                    variant_builder.add_impression_data(response.impression_data);
                    variant_builder.add_enabled(variant.enabled);
                    variant_builder.add_name(variant_name_offset);
                    if let Some(payload_offset) = payload_offset {
                        variant_builder.add_payload(payload_offset);
                    }

                    variant_builder.finish()
                }
                None => {
                    let resp_builder = VariantBuilder::new(builder);
                    resp_builder.finish()
                }
            },
            Ok(None) => {
                let resp_builder = VariantBuilder::new(builder);
                resp_builder.finish()
            }
            Err(err) => {
                let error_offset = builder.create_string(&err.to_string());
                let mut response_builder = VariantBuilder::new(builder);
                response_builder.add_error(error_offset);
                response_builder.finish()
            }
        }
    }
}

impl FlatMessage<Option<MetricBucket>> for MetricsResponse<'static> {
    fn as_flat_buffer(
        builder: &mut FlatBufferBuilder<'static>,
        metrics: Option<MetricBucket>,
    ) -> WIPOffset<Self> {
        if let Some(metrics) = metrics {
            let items: Vec<_> = metrics
                .toggles
                .iter()
                .map(|(toggle_key, stats)| {
                    let variant_items: Vec<_> = stats
                        .variants
                        .iter()
                        .map(|(variant_key, count)| {
                            let variant_key = builder.create_string(variant_key);
                            let mut variant_builder = VariantEntryBuilder::new(builder);
                            variant_builder.add_key(variant_key);
                            variant_builder.add_value(*count);
                            variant_builder.finish()
                        })
                        .collect();
                    let variant_vector = builder.create_vector(&variant_items);

                    let toggle_key = builder.create_string(toggle_key);
                    let mut toggle_builder = ToggleStatsBuilder::new(builder);
                    toggle_builder.add_no(stats.no);
                    toggle_builder.add_yes(stats.yes);
                    toggle_builder.add_variants(variant_vector);
                    let toggle_value = toggle_builder.finish();
                    let mut toggle_entry_builder = ToggleEntryBuilder::new(builder);
                    toggle_entry_builder.add_value(toggle_value);
                    toggle_entry_builder.add_key(toggle_key);
                    toggle_entry_builder.finish()
                })
                .collect();
            let toggle_vector = builder.create_vector(&items);
            let mut resp_builder = MetricsResponseBuilder::new(builder);
            resp_builder.add_start(metrics.start.timestamp_millis());
            resp_builder.add_stop(metrics.stop.timestamp_millis());
            resp_builder.add_toggles(toggle_vector);
            resp_builder.finish()
        } else {
            let resp_builder = MetricsResponseBuilder::new(builder);
            resp_builder.finish()
        }
    }
}

impl FlatMessage<Vec<ToggleDefinition>> for FeatureDefs<'static> {
    fn as_flat_buffer(
        builder: &mut FlatBufferBuilder<'static>,
        known_toggles: Vec<ToggleDefinition>,
    ) -> WIPOffset<Self> {
        let items: Vec<_> = known_toggles
            .iter()
            .map(|toggle| {
                let toggle_name_offset = builder.create_string(&toggle.name);
                let project_offset = builder.create_string(&toggle.project);
                let feature_type_offset = toggle
                    .feature_type
                    .as_ref()
                    .map(|f| builder.create_string(f));

                let mut feature_def_builder = FeatureDefBuilder::new(builder);

                feature_def_builder.add_name(toggle_name_offset);
                feature_def_builder.add_project(project_offset);
                feature_def_builder.add_enabled(toggle.enabled);
                if let Some(offset) = feature_type_offset {
                    feature_def_builder.add_type_(offset);
                }
                feature_def_builder.finish()
            })
            .collect();

        let toggle_vector = builder.create_vector(&items);

        let mut resp_builder = FeatureDefsBuilder::new(builder);
        resp_builder.add_items(toggle_vector);
        resp_builder.finish()
    }
}

impl FlatMessage<&str> for CoreVersion<'static> {
    fn as_flat_buffer(builder: &mut FlatBufferBuilder<'static>, version: &str) -> WIPOffset<Self> {
        let version_offset = builder.create_string(version);
        let mut resp_builder = CoreVersionBuilder::new(builder);
        resp_builder.add_version(version_offset);
        resp_builder.finish()
    }
}

impl FlatMessage<[&'static str; 8]> for BuiltInStrategies<'static> {
    fn as_flat_buffer(
        builder: &mut FlatBufferBuilder<'static>,
        strategies: [&'static str; 8],
    ) -> WIPOffset<Self> {
        let items: Vec<_> = strategies
            .iter()
            .map(|strategy| builder.create_string(strategy))
            .collect();

        let strategy_vector = builder.create_vector(&items);

        let mut resp_builder = BuiltInStrategiesBuilder::new(builder);
        resp_builder.add_values(strategy_vector);
        resp_builder.finish()
    }
}

impl FlatMessage<Result<Option<Vec<EvalWarning>>, FlatError>> for TakeStateResponse<'static> {
    fn as_flat_buffer(
        builder: &mut FlatBufferBuilder<'static>,
        response: Result<Option<Vec<EvalWarning>>, FlatError>,
    ) -> WIPOffset<Self> {
        match response {
            Ok(Some(state)) => {
                let warnings: Vec<_> = state
                    .iter()
                    .map(|warning| {
                        builder
                            .create_string(&format!("{}:{}", warning.toggle_name, warning.message))
                    })
                    .collect();

                let warning_vector = builder.create_vector(&warnings);
                let mut resp_builder = TakeStateResponseBuilder::new(builder);
                resp_builder.add_warnings(warning_vector);
                resp_builder.finish()
            }
            Ok(None) => {
                let resp_builder = TakeStateResponseBuilder::new(builder);
                resp_builder.finish()
            }
            Err(err) => {
                let error_offset = builder.create_string(&err.to_string());
                let mut resp_builder = TakeStateResponseBuilder::new(builder);
                resp_builder.add_error(error_offset);
                resp_builder.finish()
            }
        }
    }
}
