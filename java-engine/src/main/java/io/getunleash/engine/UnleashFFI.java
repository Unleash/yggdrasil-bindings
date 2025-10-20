interface UnleashFFI extends Library {

    Pointer newEngine();

    void freeEngine(Pointer ptr);

    Pointer takeState(Pointer ptr, Pointer toggles);

    Pointer checkEnabled(Pointer ptr, Pointer name, Pointer context, Pointer customStrategyResults);

    Pointer checkVariant(Pointer ptr, Pointer name, Pointer context, Pointer customStrategyResults);

    Pointer countToggle(Pointer ptr, Pointer name, boolean enabled);

    Pointer countVariant(Pointer ptr, Pointer name, Pointer variantName);

    Pointer getMetrics(Pointer ptr);

    Pointer shouldEmitImpressionEvent(Pointer ptr, Pointer name);

    Pointer builtInStrategies();

    void freeResponse(Pointer pointer);

    Pointer listKnownToggles(Pointer ptr);

    Pointer getCoreVersion();

    static UnleashFFI getInstance() {
        return NativeLoader.NATIVE_INTERFACE;
    }

    static Pointer getYggdrasilCoreVersion() {
        return NativeLoader.NATIVE_INTERFACE.getCoreVersion();
    }
}
