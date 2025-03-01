use std::path::PathBuf;

use rhai::{Engine, Scope, AST, INT};

use crate::simulation::SimulationParameters;

pub struct Scripting {
    engine: Engine,
    scope: Scope<'static>,

    script: AST,
}

#[derive(Clone)]
struct Context {
    params: SimulationParameters,
    response: PostTickResponse,
}

#[derive(Default, Clone)]
pub struct PostTickResponse {
    pub reset: bool,
    pub snapshot_state: bool,
    pub snapshot_energy: bool,
}

impl Scripting {
    pub fn from_file(path: PathBuf) -> Self {
        let mut engine = Engine::new();
        let mut scope = Scope::new();
        engine
            .register_type::<Context>()
            .register_get("tick", Context::get_tick)
            .register_fn("pause", Context::pause)
            .register_fn("reset", Context::reset)
            .register_fn("snapshot_state", Context::snapshot_state)
            .register_fn("snapshot_energy", Context::snapshot_energy)
            .register_get_set("v", Context::get_v, Context::set_v)
            .register_get_set("dt", Context::get_dt, Context::set_dt)
            .register_get_set("dx", Context::get_dx, Context::set_dx)
            .register_get_set("amplitude", Context::get_amplitude, Context::set_amplitude)
            .register_get_set("frequency", Context::get_frequency, Context::set_frequency);

        let script = engine.compile_file(path).unwrap();
        engine.run_ast_with_scope(&mut scope, &script).unwrap();

        Self {
            engine,
            scope,
            script,
        }
    }

    pub fn post_tick(&mut self, params: &mut SimulationParameters) -> PostTickResponse {
        let ctx = Context {
            params: params.clone(),
            response: PostTickResponse::default(),
        };

        self.scope.set_value("sim", ctx);
        self.engine
            .call_fn::<()>(&mut self.scope, &self.script, "post_tick", ())
            .unwrap();

        let ctx = self.scope.get_value::<Context>("sim").unwrap();
        *params = ctx.params;
        ctx.response
    }
}

impl Context {
    fn get_tick(&mut self) -> INT {
        self.params.tick as INT
    }

    fn pause(&mut self) {
        self.params.running = false;
    }

    fn reset(&mut self) {
        self.response.reset = true;
    }

    fn snapshot_state(&mut self) {
        self.response.snapshot_state = true;
    }

    fn snapshot_energy(&mut self) {
        self.response.snapshot_energy = true;
    }

    fn set_v(&mut self, v: f64) {
        self.params.v = v as f32;
    }

    fn get_v(&mut self) -> f64 {
        self.params.v as f64
    }

    fn set_dt(&mut self, dt: f64) {
        self.params.dt = dt as f32;
    }

    fn get_dt(&mut self) -> f64 {
        self.params.dt as f64
    }

    fn set_dx(&mut self, dx: f64) {
        self.params.dx = dx as f32;
    }

    fn get_dx(&mut self) -> f64 {
        self.params.dx as f64
    }

    fn set_amplitude(&mut self, amplitude: f64) {
        self.params.amplitude = amplitude as f32;
    }

    fn get_amplitude(&mut self) -> f64 {
        self.params.amplitude as f64
    }

    fn set_frequency(&mut self, frequency: f64) {
        self.params.frequency = frequency as f32;
    }

    fn get_frequency(&mut self) -> f64 {
        self.params.frequency as f64
    }
}
