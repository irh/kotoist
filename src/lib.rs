//! VST plugin for live coding using [Koto](https://github.com/koto-lang/koto) programming
//! language.

#![deny(
    nonstandard_style,
    rust_2018_idioms,
    trivial_casts,
    trivial_numeric_casts
)]
#![warn(
    deprecated_in_future,
    missing_docs,
    unused_import_braces,
    unused_labels,
    unused_lifetimes,
    unused_qualifications,
    unreachable_pub
)]

mod editor;
mod parameters;
mod pattern;

use std::sync::{Arc, Mutex, Once};

use log::{info, LevelFilter};

use vst::api::{Event, Events, Supported};
use vst::plugin::{CanDo, Category, HostCallback, Info, Plugin, PluginParameters};
use vst::{buffer::AudioBuffer, editor::Editor, host::Host, plugin_main};

use editor::KotoistEditor;
use parameters::Parameters;
use pattern::{make_module, Orchestrator};

#[cfg(debug_assertions)]
static ONCE: Once = Once::new();
const KOTO_LIB_CODE: &'static str = include_str!("../koto/pattern.koto");

#[derive(Default)]
struct Kotoist {
    host: HostCallback,
    sample_rate: f32,
    block_size: i64,
    parameters: Arc<Parameters>,
    orchestrator: Arc<Mutex<Orchestrator>>,
}

impl Plugin for Kotoist {
    fn new(host: HostCallback) -> Self {
        let mut parameters = Parameters::default();
        parameters.set_host(host.clone());
        let parameters = Arc::new(parameters);
        let orchestrator = Arc::new(Mutex::new(Orchestrator::new(
            host.clone(),
            Arc::clone(&parameters),
        )));
        parameters
            .koto
            .write()
            .unwrap()
            .prelude()
            .add_map("pattern", make_module(Arc::clone(&orchestrator)));

        parameters.eval_code("from pattern import midi_out, print_scales");
        parameters.eval_code(KOTO_LIB_CODE);

        Self {
            host,
            parameters,
            orchestrator,
            ..Default::default()
        }
    }

    fn init(&mut self) {
        #[cfg(debug_assertions)]
        init_log();
    }

    fn set_sample_rate(&mut self, rate: f32) {
        self.sample_rate = rate;
    }

    fn set_block_size(&mut self, size: i64) {
        self.block_size = size;
    }

    fn get_info(&self) -> Info {
        Info {
            name: "Kotoist".to_string(),
            vendor: "Ales Tsurko".to_string(),
            unique_id: 27052021,
            category: Category::Generator,
            preset_chunks: true,
            ..Default::default()
        }
    }

    fn can_do(&self, can_do: CanDo) -> Supported {
        use CanDo::*;
        use Supported::*;

        match can_do {
            SendEvents
            | SendMidiEvent
            | ReceiveEvents
            | ReceiveMidiEvent
            | ReceiveTimeInfo
            | MidiProgramNames
            | Bypass
            | ReceiveSysExEvent
            | MidiSingleNoteTuningChange
            | MidiKeyBasedInstrumentControl => Yes,
            _ => Maybe,
        }
    }

    fn process(&mut self, _buffer: &mut AudioBuffer<'_, f32>) {
        let events = self.orchestrator.lock().unwrap().tick();
        for mut event in events.into_iter() {
            let conv: *mut Event = unsafe { std::mem::transmute(&mut event) };
            let events = Events {
                num_events: 1,
                _reserved: 0,
                events: [conv, conv],
            };
            self.host.process_events(&events);
        }
    }

    fn get_editor(&mut self) -> Option<Box<dyn Editor>> {
        Some(Box::new(KotoistEditor::new(Arc::clone(&self.parameters))))
    }

    fn get_parameter_object(&mut self) -> Arc<dyn PluginParameters> {
        let result = Arc::clone(&self.parameters);
        let result: Arc<dyn PluginParameters> = result;
        result
    }
}

#[cfg(debug_assertions)]
fn init_log() {
    ONCE.call_once(|| {
        let path = format!("{}/Desktop/kotoist.log", std::env::var("HOME").unwrap());
        _init_log(path, LevelFilter::Debug);
        info!("init log");
    });
}

#[cfg(windows)]
fn _init_log(path: String, level: LevelFilter) {
    use simple_logging;
    simple_logging::log_to_file(path, level).unwrap();
}

#[cfg(unix)]
fn _init_log(path: String, level: LevelFilter) {
    use simplelog::{ConfigBuilder, WriteLogger};
    use std::fs::OpenOptions;

    let file = OpenOptions::new()
        .append(true)
        .create(true)
        .open(path)
        .unwrap();
    let config = ConfigBuilder::new().set_time_to_local(true).build();
    let _ = WriteLogger::init(level, config, file).unwrap();
}

#[allow(missing_docs)]
plugin_main!(Kotoist);
