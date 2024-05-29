use ddsketch_agent::DDSketch;
use plotters::prelude::*;
use plotters::style::full_palette::BLUEGREY;
use plotters_canvas::CanvasBackend;
use rand::SeedableRng;
use rand_distr::{Distribution, Pareto};
use rand_xorshift::XorShiftRng;
use std::collections::HashMap;
use wasm_bindgen::prelude::*;

#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

/// Type used on the JS side to convert screen coordinates to chart
/// coordinates.
#[wasm_bindgen]
pub struct Chart {
    convert: Box<dyn Fn((i32, i32)) -> Option<(f64, f64)>>,
}

/// Result of screen to chart coordinates conversion.
#[wasm_bindgen]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

#[wasm_bindgen]
pub struct SketchView {
    input_canvas_id: String,
    output_canvas_id: String,

    sketch: DDSketch,
    rng: XorShiftRng,
    sampled_points: Vec<f64>,
}

#[wasm_bindgen]
pub struct InputStats {
    pub value_count: usize,
    pub in_memory_size: usize,
}

#[wasm_bindgen]
pub struct OutputStats {
    pub bin_count: usize,
    pub in_memory_size: usize,
    pub p50: f64,
    pub p90: f64,
    pub p99: f64,
}

#[wasm_bindgen]
impl SketchView {
    pub fn new(input_canvas_id: &str, output_canvas_id: &str) -> Self {
        console_error_panic_hook::set_once();

        SketchView {
            input_canvas_id: input_canvas_id.to_string(),
            output_canvas_id: output_canvas_id.to_string(),
            sketch: DDSketch::default(),
            rng: XorShiftRng::from_entropy(),
            sampled_points: Vec::default(),
        }
    }

    pub fn set_bin_limit(&mut self, bin_limit: u16) {
        let config = ddsketch_agent::Config::with_bin_limit(bin_limit);
        self.sketch = DDSketch::with_config(config);
        self.sketch.insert_many(&self.sampled_points);
    }

    pub fn get_input_stats(&self) -> InputStats {
        InputStats {
            value_count: self.sampled_points.len(),
            in_memory_size: self.sampled_points.len() * std::mem::size_of::<f64>(),
        }
    }

    pub fn get_output_stats(&self) -> OutputStats {
        OutputStats {
            bin_count: self.sketch.bin_count(),
            in_memory_size: self.sketch.bin_count() * std::mem::size_of::<ddsketch_agent::Bin>()
                + std::mem::size_of::<DDSketch>(),
            p50: self.sketch.quantile(0.50).unwrap_or_default() / 1_000_000.0,
            p90: self.sketch.quantile(0.90).unwrap_or_default() / 1_000_000.0,
            p99: self.sketch.quantile(0.99).unwrap_or_default() / 1_000_000.0,
        }
    }

    pub fn sample(&mut self, count: usize) {
        // Generate a set of samples that roughly correspond to the latency of a typical web service, in microseconds,
        // with a gamma distribution: big hump at the beginning with a long tail.  We limit this so the samples
        // represent latencies that bottom out at 15 milliseconds and tail off at 1 second.
        let distribution = Pareto::new(1.0, 1.0).expect("pareto distribution should be valid");
        let mut points = distribution
            .sample_iter(&mut self.rng)
            // Scale by 10,000 to get microseconds.
            .map(|n| n * 10_000.)
            .filter(|n| *n > 15_000. && *n < 1_000_000.)
            .take(count)
            .collect::<Vec<_>>();

        self.sketch.insert_many(&points);
        self.sampled_points.append(&mut points);
    }

    pub fn input_chart(&mut self, bin_count: u32) -> Result<Chart, JsValue> {
        Ok(self
            .inner_input_chart(bin_count)
            .map(|_ct| Chart {
                // convert: Box::new(move |coord| ct(coord).map(|(x, y)| (x.into(), y.into()))),
                convert: Box::new(move |_| Some((0.0, 0.0))),
            })
            .map_err(|err| err.to_string())?)
    }

    fn inner_input_chart(&mut self, bin_count: u32) -> Result<(), Box<dyn std::error::Error>> {
        // Arrange values into `bin_count` buckets
        let min_value = self.sketch.min().unwrap_or_default();
        let max_value = self.sketch.max().unwrap_or_default();
        let bin_width = (max_value - min_value) / bin_count as f64;
        let mut grouped_values = HashMap::<i64, u32>::default();
        for v in self.sampled_points.iter() {
            let key = v / bin_width;
            let key = key.floor() as i64;
            let count = grouped_values.entry(key).or_insert(0u32);
            *count += 1;
        }

        // todo: share the same min & max for both charts

        let min_value = grouped_values.keys().min().copied().unwrap_or_default() as f64 * bin_width;
        let max_value = grouped_values.keys().max().copied().unwrap_or_default() as f64 * bin_width;
        let max_count = *grouped_values.values().max().unwrap_or(&1) as f64 * 1.1;

        let backend = CanvasBackend::new(&self.input_canvas_id).expect("cannot find canvas");
        let root = backend.into_drawing_area();

        root.fill(&WHITE)?;

        let mut chart = ChartBuilder::on(&root)
            .set_label_area_size(LabelAreaPosition::Left, 60)
            .set_label_area_size(LabelAreaPosition::Bottom, 60)
            .build_cartesian_2d(min_value..max_value, 0f64..max_count)?;

        chart
            .configure_mesh()
            .x_labels(15)
            .y_labels(15)
            .x_label_formatter(&|x| format!("{:.1}", *x / 1_000_000.))
            .axis_style(BLACK.mix(0.1))
            .disable_x_mesh()
            .disable_y_mesh()
            .draw()?;

        chart.draw_series(grouped_values.iter().map(|(bin, count)| {
            Rectangle::new(
                [
                    (*bin as f64 * bin_width, *count as f64),
                    (((*bin + 1) as f64) * bin_width, 0.),
                ],
                BLUEGREY.filled(),
            )
        }))?;

        root.present()?;
        Ok(())
    }

    pub fn output_chart(&mut self) -> Result<Chart, JsValue> {
        Ok(self
            .output_chart_inner()
            .map(|_ct| Chart {
                convert: Box::new(move |_| Some((0.0, 0.0))),
            })
            .map_err(|err| err.to_string())?)
    }

    fn output_chart_inner(&self) -> Result<(), Box<dyn std::error::Error>> {
        let bins = self
            .sketch
            .bins()
            .iter()
            // Convert DDSketch bins to a map of `(key, count)` (handle bin
            // overflow - multiple bins with the same key)
            .fold(HashMap::new(), |mut map, b| {
                if map.contains_key(&b.k) {
                    let v = map.get_mut(&b.k).unwrap();
                    *v += b.n as i64;
                } else {
                    map.insert(b.k, b.n as i64);
                }
                map
            });

        let max_count = *bins.values().max().unwrap_or(&0);
        let min_key = *bins.keys().min().unwrap_or(&0);
        let max_key = *bins.keys().max().unwrap_or(&0);
        let min_value = self.sketch.config().bin_lower_bound(min_key);
        let max_value = self.sketch.config().bin_upper_bound(max_key);

        let backend = CanvasBackend::new(&self.output_canvas_id).expect("cannot find canvas");
        let root = backend.into_drawing_area();

        root.fill(&WHITE)?;

        let mut chart = ChartBuilder::on(&root)
            .margin(5)
            .set_label_area_size(LabelAreaPosition::Left, 60)
            .set_label_area_size(LabelAreaPosition::Bottom, 60)
            .build_cartesian_2d(
                // (floor(min) as f64)..(ceil(max) as f64),
                (min_value as f64)..(max_value as f64),
                0f64..(max_count as f64),
            )?;

        chart
            .configure_mesh()
            .x_labels(15)
            .y_labels(15)
            .axis_style(BLACK.mix(0.1))
            .x_label_formatter(&|x| format!("{:.1}", *x / 1_000_000.))
            .disable_x_mesh()
            .disable_y_mesh()
            .label_style(("sans-serif", 20))
            .draw()?;

        let bins = bins.into_iter().map(|(k, n)| {
            let key_min = self.sketch.config().bin_lower_bound(k);
            let key_max = self.sketch.config().bin_upper_bound(k);

            Rectangle::new([(key_min, n as f64), (key_max, 0.)], BLUEGREY)
        });

        chart.draw_series(bins)?;

        root.present()?;

        // Ok(chart.into_coord_trans())

        Ok(())
    }
}

#[wasm_bindgen]
impl Chart {
    /// This function can be used to convert screen coordinates to
    /// chart coordinates.
    pub fn coord(&self, x: i32, y: i32) -> Option<Point> {
        (self.convert)((x, y)).map(|(x, y)| Point { x, y })
    }
}
