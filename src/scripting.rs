use std::path::PathBuf;

use rhai::{Engine, Scope, AST, INT};

pub struct Scripting {
    engine: Engine,
    scope: Scope<'static>,

    script: AST,
}

#[derive(Clone, Default)]
struct Context {
    tick: INT,
    running: bool,
    do_reset: bool,
    snapshot_state: bool,
    snapshot_energy: bool,
}

pub struct PostTickResponse {
    pub running: bool,
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
            .register_fn("snapshot_energy", Context::snapshot_energy);

        let script = engine.compile_file(path).unwrap();
        engine.run_ast_with_scope(&mut scope, &script).unwrap();

        Self {
            engine,
            scope,
            script,
        }
    }

    pub fn post_tick(&mut self, tick: u64, running: bool) -> PostTickResponse {
        let ctx = Context {
            tick: tick as _,
            running,
            ..Default::default()
        };

        self.scope.set_value("sim", ctx);
        self.engine
            .call_fn::<()>(&mut self.scope, &self.script, "post_tick", ())
            .unwrap();

        let ctx = self.scope.get_value::<Context>("sim").unwrap();

        PostTickResponse {
            running: ctx.running,
            reset: ctx.do_reset,
            snapshot_state: ctx.snapshot_state,
            snapshot_energy: ctx.snapshot_energy,
        }
    }
}

impl Context {
    fn get_tick(&mut self) -> INT {
        self.tick
    }

    fn pause(&mut self) {
        self.running = false;
    }

    fn reset(&mut self) {
        self.do_reset = true;
    }

    fn snapshot_state(&mut self) {
        self.snapshot_state = true;
    }

    fn snapshot_energy(&mut self) {
        self.snapshot_energy = true;
    }
}
