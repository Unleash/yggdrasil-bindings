package io.getunleash.engine;

import com.fasterxml.jackson.annotation.JsonCreator;
import com.fasterxml.jackson.annotation.JsonProperty;

public class VariantDefMixin {
    @JsonCreator
    public VariantDefMixin(
            @JsonProperty("name") String name,
            @JsonProperty("payload") Payload payload,
            @JsonProperty("enabled") Boolean enabled,
            @JsonProperty("feature_enabled") Boolean featureEnabled
    ) {}
}
