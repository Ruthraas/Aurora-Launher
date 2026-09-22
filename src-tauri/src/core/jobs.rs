use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use serde::Serialize;
use tauri::{AppHandle, Emitter};
use uuid::Uuid;

/// Registro simples de jobs em background (instalar mod, baixar
/// modpack) — só o suficiente pra cancelamento cooperativo: cada job
/// tem uma flag atômica que ele mesmo confere entre etapas (ex.: antes
/// de baixar o próximo arquivo). Não cancela um download já em voo no
/// meio, só entre um arquivo e outro — suficiente pra UX de "parar
/// isso", sem a complexidade de abortar uma requisição HTTP em curso.
#[derive(Clone, Default)]
pub struct JobRegistry {
    cancelled: Arc<Mutex<HashMap<Uuid, Arc<AtomicBool>>>>,
}

pub struct JobHandle {
    pub id: Uuid,
    cancelled: Arc<AtomicBool>,
}

impl JobHandle {
    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Relaxed)
    }
}

impl JobRegistry {
    pub fn start(&self) -> JobHandle {
        let id = Uuid::new_v4();
        let cancelled = Arc::new(AtomicBool::new(false));
        self.cancelled.lock().unwrap().insert(id, cancelled.clone());
        JobHandle { id, cancelled }
    }

    pub fn finish(&self, id: Uuid) {
        self.cancelled.lock().unwrap().remove(&id);
    }

    pub fn cancel(&self, id: Uuid) {
        if let Some(flag) = self.cancelled.lock().unwrap().get(&id) {
            flag.store(true, Ordering::Relaxed);
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum JobPhase {
    FetchingMetadata,
    CreatingInstance,
    Downloading,
    Extracting,
    ConfiguringOptions,
    Finished,
    Cancelled,
    Failed,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct JobProgressPayload {
    job_id: Uuid,
    phase: JobPhase,
    done: usize,
    total: usize,
    error: Option<String>,
}

/// Emite `job://progress` — mesmo padrão de evento já usado pra
/// instalação de instância (`instance://install-progress`), só que
/// genérico o bastante pra mod/modpack também.
pub fn emit_progress(app: &AppHandle, job_id: Uuid, phase: JobPhase, done: usize, total: usize) {
    let _ = app.emit("job://progress", JobProgressPayload { job_id, phase, done, total, error: None });
}

pub fn emit_failed(app: &AppHandle, job_id: Uuid, error: String) {
    let _ = app.emit(
        "job://progress",
        JobProgressPayload { job_id, phase: JobPhase::Failed, done: 0, total: 0, error: Some(error) },
    );
}
