//! Exécution d'un plugin : module WebAssembly validé, mémoire plafonnée, carburant à chaque appel (ADR 0020).

use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::{Arc, OnceLock};

use alpagamp_core::i18n::lang;
use parking_lot::Mutex;
use wasmi::{Caller, Config, Engine, Extern, Linker, Memory, Module, Store, StoreLimits, StoreLimitsBuilder, TrapCode, TypedFunc, WasmParams, WasmResults};

use crate::manifest::{Kind, Manifest};

/// Ce que les plugins demandent au lecteur.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Request {
    /// 1 lecture, 2 pause, 3 stop, 4 suivant, 5 précédent.
    Command(i32),
    Volume(u8),
    /// Message pour le titre défilant.
    Notify(String),
}

/// Le lien entre les plugins et le lecteur, partagé entre les fils (les effets audio tournent dans le fil de
/// décodage).
pub struct Link {
    /// 0 arrêt, 1 lecture, 2 pause.
    pub state: AtomicU8,
    pub volume: AtomicU8,
    /// Couleurs du visualiseur du skin (`0xRRGGBB`).
    pub colors: Mutex<[u32; 24]>,
    requests: Mutex<Vec<Request>>,
}

impl Default for Link {
    fn default() -> Self {
        Link { state: AtomicU8::new(0), volume: AtomicU8::new(200), colors: Mutex::new([0; 24]), requests: Mutex::new(Vec::new()) }
    }
}

impl Link {
    /// Demandes en attente, une seule fois.
    pub fn take_requests(&self) -> Vec<Request> {
        std::mem::take(&mut *self.requests.lock())
    }

    fn push(&self, r: Request) {
        let mut q = self.requests.lock();
        // Un plugin trop bavard ne remplit pas la mémoire.
        if q.len() < 256 {
            q.push(r);
        }
    }
}

struct Ctx {
    link: Arc<Link>,
    limits: StoreLimits,
    name: String,
}

/// Mémoire maximale d'un plugin.
const MEMORY_LIMIT: usize = 64 << 20;
/// Taille maximale d'un fichier `.wasm`.
pub const MAX_FILE: usize = 16 << 20;
/// Carburant (≈ instructions) par appel.
const LOAD_FUEL: u64 = 200_000_000;
const SMALL_FUEL: u64 = 20_000_000;
const TICK_FUEL: u64 = 20_000_000;
const DSP_FUEL_PER_FRAME: u64 = 4_000;
const VIS_FUEL_PER_PIXEL: u64 = 400;

fn engine() -> &'static Engine {
    static E: OnceLock<Engine> = OnceLock::new();
    E.get_or_init(|| {
        let mut c = Config::default();
        c.consume_fuel(true);
        Engine::new(&c)
    })
}

fn memory_of(caller: &Caller<'_, Ctx>) -> Option<Memory> {
    caller.get_export("memory").and_then(Extern::into_memory)
}

fn read_str(caller: &Caller<'_, Ctx>, ptr: i32, len: i32) -> Option<String> {
    let mem = memory_of(caller)?;
    let start = ptr as u32 as usize;
    let len = (len.max(0) as usize).min(4096);
    let bytes = mem.data(caller).get(start..start.checked_add(len)?)?;
    Some(String::from_utf8_lossy(bytes).into_owned())
}

fn linker() -> Result<Linker<Ctx>, String> {
    let mut l = <Linker<Ctx>>::new(engine());
    let e = |e: wasmi::errors::LinkerError| e.to_string();
    l.func_wrap("alpagamp", "log", |c: Caller<'_, Ctx>, ptr: i32, len: i32| {
        if let Some(s) = read_str(&c, ptr, len) {
            log::info!("plugin {} : {s}", c.data().name);
        }
    })
    .map_err(e)?;
    l.func_wrap("alpagamp", "notify", |c: Caller<'_, Ctx>, ptr: i32, len: i32| {
        if let Some(s) = read_str(&c, ptr, len) {
            let s: String = s.chars().filter(|ch| !ch.is_control()).take(200).collect();
            c.data().link.push(Request::Notify(s));
        }
    })
    .map_err(e)?;
    l.func_wrap("alpagamp", "player_state", |c: Caller<'_, Ctx>| -> i32 { c.data().link.state.load(Ordering::Relaxed) as i32 }).map_err(e)?;
    l.func_wrap("alpagamp", "command", |c: Caller<'_, Ctx>, n: i32| {
        if (1..=5).contains(&n) {
            c.data().link.push(Request::Command(n));
        }
    })
    .map_err(e)?;
    l.func_wrap("alpagamp", "volume", |c: Caller<'_, Ctx>| -> i32 { c.data().link.volume.load(Ordering::Relaxed) as i32 }).map_err(e)?;
    l.func_wrap("alpagamp", "set_volume", |c: Caller<'_, Ctx>, v: i32| {
        let v = v.clamp(0, 255) as u8;
        let link = &c.data().link;
        link.volume.store(v, Ordering::Relaxed);
        link.push(Request::Volume(v));
    })
    .map_err(e)?;
    l.func_wrap("alpagamp", "skin_color", |c: Caller<'_, Ctx>, i: i32| -> i32 {
        if (0..24).contains(&i) { (c.data().link.colors.lock()[i as usize] & 0x00FF_FFFF) as i32 } else { 0 }
    })
    .map_err(e)?;
    l.func_wrap("alpagamp", "lang", |mut c: Caller<'_, Ctx>, ptr: i32, cap: i32| -> i32 {
        let code = lang().code().as_bytes();
        let Some(mem) = memory_of(&c) else { return 0 };
        let start = ptr as u32 as usize;
        let n = code.len().min(cap.max(0) as usize);
        if let Some(dst) = mem.data_mut(&mut c).get_mut(start..start + n) {
            dst.copy_from_slice(&code[..n]);
        }
        code.len() as i32
    })
    .map_err(e)?;
    Ok(l)
}

fn describe(e: &wasmi::Error) -> String {
    match e.as_trap_code() {
        Some(TrapCode::OutOfFuel) => "trop lent (boucle sans fin ?)".into(),
        Some(TrapCode::UnreachableCodeReached) => "plantage du plugin".into(),
        Some(TrapCode::StackOverflow) => "pile débordée".into(),
        Some(TrapCode::MemoryOutOfBounds) => "accès mémoire hors limites".into(),
        _ => e.to_string(),
    }
}

/// Le son donné aux visualisations : 2 × 576 valeurs par tableau (gauche puis droite).
pub struct Sound {
    pub spectrum: [f32; 1152],
    pub wave: [f32; 1152],
}

impl Default for Sound {
    fn default() -> Self {
        Sound { spectrum: [0.0; 1152], wave: [0.0; 1152] }
    }
}

/// Un plugin chargé.
pub struct Instance {
    store: Store<Ctx>,
    memory: Memory,
    alloc: TypedFunc<i32, i32>,
    option: Option<TypedFunc<(i32, i32), ()>>,
    quit: Option<TypedFunc<(), ()>>,
    dsp_fn: Option<TypedFunc<(i32, i32, i32), ()>>,
    vis_fn: Option<TypedFunc<(i32, i32, i32, i32, i32), i32>>,
    tick_fn: Option<TypedFunc<i64, ()>>,
    /// Zone réservée pour l'audio : (adresse, octets).
    dsp_buf: (u32, usize),
    /// Zones réservées pour le spectre et l'onde.
    vis_bufs: Option<(u32, u32)>,
    pub manifest: Manifest,
    failed: Option<String>,
    quit_done: bool,
}

impl Instance {
    /// Charge un plugin : validation, instanciation, version d'API, description, `alp_init`.
    pub fn load(bytes: &[u8], link: Arc<Link>, name: &str) -> Result<Instance, String> {
        if bytes.len() > MAX_FILE {
            return Err("fichier trop gros".into());
        }
        let module = Module::new(engine(), bytes).map_err(|e| format!("module WebAssembly invalide : {e}"))?;
        let limits = StoreLimitsBuilder::new().memory_size(MEMORY_LIMIT).memories(1).tables(4).table_elements(100_000).instances(1).build();
        let mut store = Store::new(engine(), Ctx { link, limits, name: name.to_string() });
        store.limiter(|c| &mut c.limits);
        store.set_fuel(LOAD_FUEL).map_err(|e| e.to_string())?;
        let instance = linker()?.instantiate_and_start(&mut store, &module).map_err(|e| format!("démarrage impossible : {}", describe(&e)))?;
        let memory = instance.get_memory(&store, "memory").ok_or("le plugin n'exporte pas sa mémoire")?;
        let api = instance.get_typed_func::<(), i32>(&store, "alp_api").map_err(|_| "pas un plugin Alpagamp (alp_api manquant)".to_string())?;
        let v = api.call(&mut store, ()).map_err(|e| describe(&e))?;
        if v != 1 {
            return Err(format!("version d'API {v} non prise en charge (Alpagamp connaît la 1)"));
        }
        let info = instance.get_typed_func::<(), i32>(&store, "alp_info").map_err(|_| "alp_info manquant".to_string())?;
        let at = info.call(&mut store, ()).map_err(|e| describe(&e))? as u32 as usize;
        let json = {
            let data = memory.data(&store);
            let rest = data.get(at..).ok_or("alp_info : adresse invalide")?;
            let rest = &rest[..rest.len().min(64 * 1024)];
            let end = rest.iter().position(|&b| b == 0).ok_or("alp_info : texte sans fin")?;
            String::from_utf8_lossy(&rest[..end]).into_owned()
        };
        let manifest = Manifest::parse(&json)?;
        let alloc = instance.get_typed_func::<i32, i32>(&store, "alp_alloc").map_err(|_| "alp_alloc manquant".to_string())?;
        let opt = |n: &str| instance.get_func(&store, n).is_some();
        let typed_err = |n: &str| format!("{n} : mauvaise signature");
        let option = if opt("alp_option") { Some(instance.get_typed_func(&store, "alp_option").map_err(|_| typed_err("alp_option"))?) } else { None };
        let quit = if opt("alp_quit") { Some(instance.get_typed_func(&store, "alp_quit").map_err(|_| typed_err("alp_quit"))?) } else { None };
        let init: Option<TypedFunc<(), ()>> = if opt("alp_init") { Some(instance.get_typed_func(&store, "alp_init").map_err(|_| typed_err("alp_init"))?) } else { None };
        let (mut dsp_fn, mut vis_fn, mut tick_fn) = (None, None, None);
        let missing = |n: &str| format!("{n} manquant ou mal déclaré");
        match manifest.kind {
            Kind::Dsp => dsp_fn = Some(instance.get_typed_func(&store, "alp_dsp").map_err(|_| missing("alp_dsp"))?),
            Kind::Vis => vis_fn = Some(instance.get_typed_func(&store, "alp_vis").map_err(|_| missing("alp_vis"))?),
            Kind::General => tick_fn = Some(instance.get_typed_func(&store, "alp_tick").map_err(|_| missing("alp_tick"))?),
        }
        let mut p = Instance {
            store,
            memory,
            alloc,
            option,
            quit,
            dsp_fn,
            vis_fn,
            tick_fn,
            dsp_buf: (0, 0),
            vis_bufs: None,
            manifest,
            failed: None,
            quit_done: false,
        };
        if let Some(f) = init {
            p.run(f, (), LOAD_FUEL);
        }
        match p.failed.take() {
            Some(e) => Err(e),
            None => Ok(p),
        }
    }

    /// Raison de l'arrêt du plugin, s'il a échoué.
    pub fn failed(&self) -> Option<&str> {
        self.failed.as_deref()
    }

    fn run<P: WasmParams, R: WasmResults>(&mut self, f: TypedFunc<P, R>, args: P, fuel: u64) -> Option<R> {
        if self.failed.is_some() {
            return None;
        }
        if let Err(e) = self.store.set_fuel(fuel) {
            self.failed = Some(e.to_string());
            return None;
        }
        match f.call(&mut self.store, args) {
            Ok(r) => Some(r),
            Err(e) => {
                let msg = describe(&e);
                log::warn!("plugin {} arrêté : {msg}", self.store.data().name);
                self.failed = Some(msg);
                None
            }
        }
    }

    /// Réserve `bytes` octets dans la mémoire du plugin.
    fn reserve(&mut self, bytes: usize) -> Option<u32> {
        let p = self.run(self.alloc, bytes as i32, SMALL_FUEL)? as u32;
        let size = self.memory.data_size(&self.store);
        if p == 0 || (p as usize).checked_add(bytes).is_none_or(|end| end > size) {
            self.failed = Some("alp_alloc : adresse invalide".into());
            return None;
        }
        Some(p)
    }

    fn write_f32(&mut self, at: u32, data: &[f32]) -> bool {
        let start = at as usize;
        let mem = self.memory.data_mut(&mut self.store);
        let Some(dst) = mem.get_mut(start..start + data.len() * 4) else { return false };
        for (d, s) in dst.chunks_exact_mut(4).zip(data) {
            d.copy_from_slice(&s.to_le_bytes());
        }
        true
    }

    /// Règle l'option n° `index`.
    pub fn set_option(&mut self, index: usize, value: i32) {
        if let Some(f) = self.option {
            self.run(f, (index as i32, value), SMALL_FUEL);
        }
    }

    /// Ne pas appeler `alp_quit` à la destruction (simple lecture de la description).
    pub fn forget(&mut self) {
        self.quit_done = true;
    }

    /// Prévient le plugin qu'il s'arrête (une seule fois).
    pub fn quit(&mut self) {
        if !self.quit_done {
            self.quit_done = true;
            if let Some(f) = self.quit {
                self.run(f, (), SMALL_FUEL);
            }
        }
    }

    /// Effet audio sur `samples` (stéréo entrelacée). `false` si le plugin a échoué.
    pub fn dsp(&mut self, samples: &mut [f32], rate: u32) -> bool {
        let Some(f) = self.dsp_fn else { return false };
        if self.failed.is_some() {
            return false;
        }
        let bytes = samples.len() * 4;
        if self.dsp_buf.1 < bytes {
            let want = bytes.next_power_of_two().max(32 * 1024);
            match self.reserve(want) {
                Some(p) => self.dsp_buf = (p, want),
                None => return false,
            }
        }
        let at = self.dsp_buf.0;
        if !self.write_f32(at, samples) {
            return false;
        }
        let frames = samples.len() / 2;
        if self.run(f, (at as i32, frames as i32, rate as i32), 500_000 + DSP_FUEL_PER_FRAME * frames as u64).is_none() {
            return false;
        }
        let start = at as usize;
        let Some(src) = self.memory.data(&self.store).get(start..start + bytes) else { return false };
        for (s, b) in samples.iter_mut().zip(src.chunks_exact(4)) {
            let v = f32::from_le_bytes([b[0], b[1], b[2], b[3]]);
            // Un plugin qui renvoie n'importe quoi ne doit pas abîmer les oreilles.
            *s = if v.is_finite() { v.clamp(-2.0, 2.0) } else { 0.0 };
        }
        true
    }

    /// Une image de visualisation (`w × h` pixels `0xAARRGGBB` dans `out`). `false` si le plugin a échoué.
    pub fn vis(&mut self, w: u32, h: u32, time_ms: u32, sound: &Sound, out: &mut Vec<u32>) -> bool {
        let Some(f) = self.vis_fn else { return false };
        if self.failed.is_some() || w == 0 || h == 0 || w > 4096 || h > 4096 {
            return false;
        }
        let (sp, wv) = match self.vis_bufs {
            Some(b) => b,
            None => {
                let (Some(a), Some(b)) = (self.reserve(1152 * 4), self.reserve(1152 * 4)) else { return false };
                self.vis_bufs = Some((a, b));
                (a, b)
            }
        };
        if !self.write_f32(sp, &sound.spectrum) || !self.write_f32(wv, &sound.wave) {
            return false;
        }
        let px = (w * h) as u64;
        let fuel = 5_000_000 + VIS_FUEL_PER_PIXEL * px;
        let Some(at) = self.run(f, (w as i32, h as i32, time_ms as i32, sp as i32, wv as i32), fuel) else { return false };
        let start = at as u32 as usize;
        let Some(src) = self.memory.data(&self.store).get(start..start + px as usize * 4) else {
            self.failed = Some("alp_vis : image hors de la mémoire".into());
            return false;
        };
        out.clear();
        out.extend(src.chunks_exact(4).map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]) | 0xFF00_0000));
        true
    }

    /// Tic d'un plugin général. `false` si le plugin a échoué.
    pub fn tick(&mut self, time_ms: i64) -> bool {
        let Some(f) = self.tick_fn else { return false };
        self.run(f, time_ms, TICK_FUEL).is_some()
    }
}

impl std::fmt::Debug for Instance {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Instance").field("name", &self.store.data().name).field("kind", &self.manifest.kind).field("failed", &self.failed).finish()
    }
}

impl Drop for Instance {
    fn drop(&mut self) {
        self.quit();
    }
}
