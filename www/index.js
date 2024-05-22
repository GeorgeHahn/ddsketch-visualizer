// If you only use `npm` you can simply
// import { Chart } from "wasm-demo" and remove `setup` call from `bootstrap.js`.
class Chart { }
class SketchView { }

const input_canvas = document.getElementById("input_canvas");
const output_canvas = document.getElementById("output_canvas");
const coord = document.getElementById("coord");
const status = document.getElementById("status");
const output_stats = document.getElementById("output_stats");
const input_stats = document.getElementById("input_stats");
const input_bin_size = document.getElementById("input_bin_size");
const bin_count_label = document.getElementById("bin_count_label");
const add_btn = document.getElementById("add_btn");
const add_count = document.getElementById("add_count");
const sketch_bin_limit = document.getElementById("sketch_bin_limit");
const sketch_bin_limit_label = document.getElementById("sketch_bin_limit_label");

let input_chart = null;
let output_chart = null;
let sketch = null;

export function setup(WasmChart, SV) {
	Chart = WasmChart;
	SketchView = SV;
}

export function main() {
	setupUI();
	setupCanvas(input_canvas);
	setupCanvas(output_canvas);
	sketch = SketchView.new(input_canvas.id, output_canvas.id);

	updatePlot();

	window.addEventListener("resize", setupCanvas);
	status.addEventListener("click", updatePlot);
	window.addEventListener("mousemove", onMouseMove);

	add_btn.addEventListener("click", addData);

	input_bin_size.addEventListener("change", updateBinSize);
	input_bin_size.addEventListener("input", updateBinSize);
	add_count.addEventListener("change", updateAddBtn);
	add_count.addEventListener("input", updateAddBtn);
	sketch_bin_limit.addEventListener("change", updateBinLimit);
	sketch_bin_limit.addEventListener("input", updateBinLimit);

	updateBinLimit();
	updateAddBtn();
	updateBinSize();
}

function setupUI() {
	status.innerText = "WebAssembly loaded!";
}

function setupCanvas(canvas) {
	const dpr = window.devicePixelRatio || 1.0;
	const aspectRatio = canvas.width / canvas.height;
	const size = canvas.parentNode.offsetWidth * 0.8;
	canvas.style.width = size + "px";
	canvas.style.height = size / aspectRatio + "px";
	canvas.width = size;
	canvas.height = size / aspectRatio;
}

function updateBinLimit(event) {
	sketch_bin_limit_label.innerText = sketch_bin_limit.value;
	sketch.set_bin_limit(sketch_bin_limit.value);
	updatePlot(event);
}

function updateBinSize(event) {
	bin_count_label.innerText = input_bin_size.value;
	if (event !== undefined) {
		updatePlot(event);
	}
}

function updateAddBtn(event) {
	add_btn.value = `Add ${add_count.value}`;
}

function addData(event) {
	sketch.sample(add_count.value);
	updatePlot();
}

/** Update displayed coordinates. */
function onMouseMove(event) {
	if (!sketch) return;
	var text = "";

	if (event.target == input_canvas || event.target == output_canvas) {
		let name = "unknown";
		let chart = null;
		if (event.target == input_canvas) {
			name = "input";
			chart = input_chart;
		} else if (event.target == output_canvas) {
			name = "output";
			chart = output_chart;
		}

		let actualRect = event.target.getBoundingClientRect();
		let logicX = event.offsetX * event.target.width / actualRect.width;
		let logicY = event.offsetY * event.target.height / actualRect.height;
		const point = chart.coord(logicX, logicY);
		text = (point)
			? `${name}: (${point.x.toFixed(2)}, ${point.y.toFixed(2)})`
			: text;
	}
	coord.innerText = text;
}

/** Redraw currently selected plot. */
function updatePlot() {
	if (!sketch) return;

	let is = sketch.get_input_stats();
	input_stats.innerText = `Value count: ${is.value_count}, in-memory size: ${is.in_memory_size}`

	let os = sketch.get_output_stats();
	output_stats.innerHTML = `Bin count: ${os.bin_count}, in-memory size: ${os.in_memory_size}<br>P50: ${os.p50.toFixed(4)}, P90: ${os.p90.toFixed(4)}, P99: ${os.p99.toFixed(4)}`

	status.innerText = `Rendering...`;
	const start = performance.now();
	input_chart = sketch.input_chart(input_bin_size.value)
	output_chart = sketch.output_chart()
	const end = performance.now();
	status.innerText = `Rendered in ${Math.ceil(end - start)}ms`;
}
