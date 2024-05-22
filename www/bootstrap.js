init();

async function init() {
    if (typeof process == "object") {
        // We run in the npm/webpack environment.
        const [{ Chart, SketchView }, { main, setup }] = await Promise.all([
            import("wasm-demo"),
            import("./index.js"),
        ]);
        setup(Chart, SketchView);
        main();
    } else {
        const [{ Chart, SketchView, default: init }, { main, setup }] = await Promise.all([
            import("../pkg/wasm_demo.js"),
            import("./index.js"),
        ]);
        await init();
        setup(Chart, SketchView);
        main();
    }
}
